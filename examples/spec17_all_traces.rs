//! SPEC17 All Traces Runner - Generate Konata visualizations for all SPEC17 traces
//!
//! Usage:
//!   cargo run --example spec17_all_traces --release

use arm_cpu_emulator::{
    CPUConfig, CPUEmulator, ChampSimTraceParser, InstructionSource,
};
use arm_cpu_emulator::visualization::{KonataSnapshot, KonataOp, StageId, KonataDependencyType};
use std::fs::File;
use std::io::{Write, BufWriter};
use std::time::Instant;
use std::path::Path;

/// SPEC17 workload characteristics
#[derive(Debug, Clone)]
pub struct WorkloadCharacteristics {
    pub name: String,
    pub load_ratio: f64,
    pub store_ratio: f64,
    pub branch_ratio: f64,
    pub compute_ratio: f64,
    pub branch_taken_rate: f64,
    pub spatial_locality: f64,
}

impl WorkloadCharacteristics {
    pub fn perlbench() -> Self {
        Self { name: "600.perlbench_s".into(), load_ratio: 0.28, store_ratio: 0.12, branch_ratio: 0.18, compute_ratio: 0.42, branch_taken_rate: 0.55, spatial_locality: 0.7 }
    }
    pub fn gcc() -> Self {
        Self { name: "602.gcc_s".into(), load_ratio: 0.25, store_ratio: 0.15, branch_ratio: 0.15, compute_ratio: 0.45, branch_taken_rate: 0.52, spatial_locality: 0.75 }
    }
    pub fn mcf() -> Self {
        Self { name: "505.mcf_r".into(), load_ratio: 0.40, store_ratio: 0.20, branch_ratio: 0.10, compute_ratio: 0.30, branch_taken_rate: 0.45, spatial_locality: 0.3 }
    }
    pub fn bwaves() -> Self {
        Self { name: "603.bwaves_s".into(), load_ratio: 0.22, store_ratio: 0.10, branch_ratio: 0.05, compute_ratio: 0.63, branch_taken_rate: 0.60, spatial_locality: 0.9 }
    }
    pub fn cam4() -> Self {
        Self { name: "627.cam4_s".into(), load_ratio: 0.24, store_ratio: 0.12, branch_ratio: 0.08, compute_ratio: 0.56, branch_taken_rate: 0.50, spatial_locality: 0.85 }
    }
}

/// Simple LCG RNG
struct LcgRng { state: u64 }

impl LcgRng {
    fn new(seed: u64) -> Self { Self { state: seed } }
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }
    fn next_f64(&mut self) -> f64 { (self.next() % 10000) as f64 / 10000.0 }
    fn next_in_range(&mut self, max: u64) -> u64 { self.next() % max }
}

/// Generate ChampSim trace
fn generate_trace(path: &str, chars: &WorkloadCharacteristics, num_instrs: usize) -> std::io::Result<()> {
    let file = File::create(path)?;
    let mut writer = BufWriter::new(file);
    let mut rng = LcgRng::new(12345);
    let mut pc: u64 = 0x400000;
    let mut mem_addr: u64 = 0x10000;
    let mut last_addr: u64 = 0x10000;

    for _ in 0..num_instrs {
        let mut buf = [0u8; 64];
        let r = rng.next_f64();

        let (is_branch, taken) = if r < chars.branch_ratio {
            (true, rng.next_f64() < chars.branch_taken_rate)
        } else {
            (false, false)
        };

        if is_branch && taken {
            pc = 0x400000 + rng.next_in_range(0x10000) * 4;
        } else {
            pc += 4;
        }

        buf[0..8].copy_from_slice(&pc.to_le_bytes());
        buf[8] = if is_branch { 1 } else { 0 };
        buf[9] = if taken { 1 } else { 0 };

        if r < chars.load_ratio {
            buf[10] = (rng.next_in_range(28) + 1) as u8;
            buf[12] = 31;
            if rng.next_f64() < chars.spatial_locality {
                last_addr += 8 + rng.next_in_range(64);
            } else {
                last_addr = mem_addr + rng.next_in_range(0x100000);
            }
            buf[32..40].copy_from_slice(&last_addr.to_le_bytes());
            mem_addr = mem_addr.max(last_addr + 64);
        } else if r < chars.load_ratio + chars.store_ratio {
            buf[12] = (rng.next_in_range(28) + 1) as u8;
            if rng.next_f64() < chars.spatial_locality {
                last_addr += 8 + rng.next_in_range(64);
            } else {
                last_addr = mem_addr + rng.next_in_range(0x100000);
            }
            buf[16..24].copy_from_slice(&last_addr.to_le_bytes());
            mem_addr = mem_addr.max(last_addr + 64);
        } else if r < chars.load_ratio + chars.store_ratio + chars.branch_ratio {
            buf[12] = (rng.next_in_range(28) + 1) as u8;
            buf[13] = (rng.next_in_range(28) + 1) as u8;
        } else {
            buf[10] = (rng.next_in_range(28) + 1) as u8;
            buf[12] = (rng.next_in_range(28) + 1) as u8;
            buf[13] = (rng.next_in_range(28) + 1) as u8;
        }

        writer.write_all(&buf)?;
    }
    writer.flush()
}

/// Run simulation and generate Konata JSON
fn run_and_generate(trace_path: &str, output_path: &str, name: &str, chars: &WorkloadCharacteristics, num_instrs: usize) {
    println!("\n📊 Processing: {}", name);

    // Generate trace if needed
    if !Path::new(trace_path).exists() {
        println!("   Generating trace...");
        if let Err(e) = generate_trace(trace_path, chars, num_instrs) {
            eprintln!("   ❌ Failed to generate trace: {}", e);
            return;
        }
    }

    // Create parser
    let mut parser = match ChampSimTraceParser::from_file(trace_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("   ❌ Failed to load trace: {}", e);
            return;
        }
    };

    // CPU config
    let config = CPUConfig {
        window_size: 256,
        issue_width: 6,
        commit_width: 6,
        fetch_width: 6,
        l1_size: 64 * 1024,
        l1_associativity: 4,
        l2_size: 512 * 1024,
        l2_associativity: 8,
        enable_trace_output: true,
        max_trace_output: 10000,
        ..CPUConfig::default()
    };

    let mut cpu = CPUEmulator::new(config).unwrap();

    // Run simulation
    let start = Instant::now();
    let metrics = cpu.run(&mut parser).unwrap();
    println!("   Instructions: {}, Cycles: {}, IPC: {:.4}", metrics.total_instructions, metrics.total_cycles, metrics.ipc);
    println!("   L1 Hit: {:.2}%, Time: {:?}", metrics.l1_hit_rate * 100.0, start.elapsed());

    // Generate Konata
    let trace = cpu.trace();
    let pipeline_tracker = cpu.pipeline_tracker();
    let mut konata = KonataSnapshot::new(metrics.total_cycles, metrics.total_instructions);

    let mut total_deps = 0;
    let mut mem_deps = 0;

    for entry in trace.entries().iter() {
        use arm_cpu_emulator::types::InstructionId;

        let instr_id = InstructionId(entry.id);
        let viz_id = pipeline_tracker.get_viz_id(instr_id).unwrap_or(entry.id);

        let mut op = KonataOp::new(viz_id, entry.id, entry.pc, entry.opcode.clone());

        let dispatch = entry.dispatch_cycle;
        let issue = entry.issue_cycle.unwrap_or(dispatch + 1);
        let complete = entry.complete_cycle.unwrap_or(issue + 2);
        let commit = entry.commit_cycle.unwrap_or(complete + 1);

        op.fetched_cycle = dispatch;
        op.retired_cycle = Some(commit);
        op.is_memory = entry.mem_addr.is_some();
        op.mem_addr = entry.mem_addr;
        op.src_regs = entry.src_regs.clone();
        op.dst_regs = entry.dst_regs.clone();

        op.add_stage(StageId::F, dispatch, dispatch + 1);
        op.add_stage(StageId::Dc, dispatch + 1, dispatch + 1);
        op.add_stage(StageId::Rn, dispatch + 1, issue);
        op.add_stage(StageId::Ds, issue, issue);
        op.add_stage(StageId::Is, issue, issue + 1);
        op.add_stage(StageId::Ex, issue + 1, complete);
        op.add_stage(StageId::Cm, complete, complete + 1);
        op.add_stage(StageId::Rt, commit, commit + 1);

        if let Some(deps) = pipeline_tracker.get_dependencies(instr_id) {
            for dep in deps {
                op.prods.push(dep.clone());
                total_deps += 1;
                if dep.dep_type == KonataDependencyType::Memory {
                    mem_deps += 1;
                }
            }
        }

        konata.add_op(op);
    }

    println!("   Dependencies: {} (mem: {}, reg: {})", total_deps, mem_deps, total_deps - mem_deps);

    // Save JSON
    let json = serde_json::to_string_pretty(&konata).unwrap();
    let mut file = File::create(output_path).unwrap();
    file.write_all(json.as_bytes()).unwrap();

    let size = std::fs::metadata(output_path).unwrap().len();
    println!("   ✅ Saved: {} ({:.2} KB)", output_path, size as f64 / 1024.0);
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║       SPEC CPU 2017 - All Traces Konata Visualization Generator       ║");
    println!("╚═══════════════════════════════════════════════════════════════════════╝");

    std::fs::create_dir_all("traces").ok();
    std::fs::create_dir_all("konata").ok();

    let num_instrs = 50_000;
    let start = Instant::now();

    let workloads: Vec<(&str, &str, &str, WorkloadCharacteristics)> = vec![
        ("traces/600.perlbench_s.trace", "konata/600.perlbench_s.json", "600.perlbench_s", WorkloadCharacteristics::perlbench()),
        ("traces/602.gcc_s.trace", "konata/602.gcc_s.json", "602.gcc_s", WorkloadCharacteristics::gcc()),
        ("traces/505.mcf_r.trace", "konata/505.mcf_r.json", "505.mcf_r", WorkloadCharacteristics::mcf()),
        ("traces/603.bwaves_s.trace", "konata/603.bwaves_s.json", "603.bwaves_s", WorkloadCharacteristics::bwaves()),
        ("traces/627.cam4_s.trace", "konata/627.cam4_s.json", "627.cam4_s", WorkloadCharacteristics::cam4()),
    ];

    for (trace_path, output_path, name, chars) in &workloads {
        run_and_generate(trace_path, output_path, name, chars, num_instrs);
    }

    println!("\n╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║                          SUMMARY                                       ║");
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║  Total time: {:?}                                              ", start.elapsed());
    println!("║  Generated files:                                                      ║");
    println!("║    - konata/600.perlbench_s.json                                       ║");
    println!("║    - konata/602.gcc_s.json                                             ║");
    println!("║    - konata/505.mcf_r.json                                             ║");
    println!("║    - konata/603.bwaves_s.json                                          ║");
    println!("║    - konata/627.cam4_s.json                                            ║");
    println!("╚═══════════════════════════════════════════════════════════════════════╝");

    println!("\n✅ All SPEC17 traces processed!");
}
