//! SPEC17 Trace Runner - Load and simulate SPEC17 traces with visualization
//!
//! This example loads a SPEC CPU 2017 trace file (ChampSim format) and runs
//! it through the ARM CPU emulator with full visualization output.
//!
//! Usage:
//!   cargo run --example spec17_trace_runner --release

use arm_cpu_emulator::{
    CPUConfig, CPUEmulator, ChampSimTraceParser, InstructionSource,
};
use arm_cpu_emulator::visualization::{KonataSnapshot, KonataOp, StageId, KonataDependencyRef, KonataDependencyType};
use std::fs::File;
use std::io::Write;
use std::time::Instant;

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║       SPEC CPU 2017 Trace Runner - ARM CPU Emulator                   ║");
    println!("╚═══════════════════════════════════════════════════════════════════════╝\n");

    // Select trace file
    let trace_path = "traces/spec17_mcf.trace";

    println!("📁 Loading trace: {}", trace_path);

    // Create trace parser
    let mut parser = match ChampSimTraceParser::from_file(trace_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Failed to load trace: {}", e);
            eprintln!("\n请先运行以下命令生成trace文件:");
            eprintln!("  cargo run --example spec17_trace_generator --release");
            return;
        }
    };

    let total_instructions = parser.total_count().unwrap_or(0);
    println!("   Total instructions in trace: {}", total_instructions);

    // Create high-performance CPU configuration
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
        max_trace_output: 5000, // Capture 5000 instructions for visualization
        ..CPUConfig::default()
    };

    println!("\n⚙️  CPU Configuration:");
    println!("   Window size:    {} entries", config.window_size);
    println!("   Issue width:    {} instructions/cycle", config.issue_width);
    println!("   Commit width:   {} instructions/cycle", config.commit_width);
    println!("   L1 cache:       {}KB, {}-way", config.l1_size / 1024, config.l1_associativity);
    println!("   L2 cache:       {}KB, {}-way", config.l2_size / 1024, config.l2_associativity);

    // Create CPU emulator
    let mut cpu = CPUEmulator::new(config.clone()).unwrap();

    println!("\n🚀 Running simulation...\n");

    // Run simulation
    let start_time = Instant::now();
    let metrics = cpu.run(&mut parser).unwrap();
    let elapsed = start_time.elapsed();

    // Print results
    println!("╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║                      SIMULATION RESULTS                                ║");
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║  Instructions:    {:>12}                                  ║", metrics.total_instructions);
    println!("║  Cycles:          {:>12}                                  ║", metrics.total_cycles);
    println!("║  IPC:             {:>12.4}                                  ║", metrics.ipc);
    println!("║  CPI:             {:>12.4}                                  ║", metrics.cpi);
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║  L1 Hit Rate:     {:>12.2}%                                ║", metrics.l1_hit_rate * 100.0);
    println!("║  L1 MPKI:         {:>12.2}                                  ║", metrics.l1_mpki);
    println!("║  L2 Hit Rate:     {:>12.2}%                                ║", metrics.l2_hit_rate * 100.0);
    println!("║  L2 MPKI:         {:>12.2}                                  ║", metrics.l2_mpki);
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║  Memory Ops:      {:>12.2}%                                ║", metrics.memory_instr_pct);
    println!("║  Branch Ops:      {:>12.2}%                                ║", metrics.branch_instr_pct);
    println!("║  Avg Load Lat:    {:>12.2} cycles                          ║", metrics.avg_load_latency);
    println!("║  Avg Store Lat:   {:>12.2} cycles                          ║", metrics.avg_store_latency);
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║  Simulation Time: {:12?}                               ║", elapsed);
    println!("╚═══════════════════════════════════════════════════════════════════════╝");

    // Generate Konata visualization
    println!("\n📊 Generating Konata visualization...");

    let trace = cpu.trace();
    let trace_len = trace.len();
    println!("   Captured {} instruction trace entries", trace_len);

    // Get pipeline tracker with dependency information
    let pipeline_tracker = cpu.pipeline_tracker();
    println!("   Pipeline tracker has {} timings", pipeline_tracker.timings.len());

    // Build Konata snapshot
    let mut konata = KonataSnapshot::new(metrics.total_cycles, metrics.total_instructions);

    // Count dependencies
    let mut total_deps = 0;
    let mut mem_deps = 0;

    for entry in trace.entries().iter() {
        use arm_cpu_emulator::types::InstructionId;

        // Get the viz_id from pipeline tracker to ensure ID consistency with dependencies
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

        // Add pipeline stages
        op.add_stage(StageId::F, dispatch, dispatch + 1);
        op.add_stage(StageId::Dc, dispatch + 1, dispatch + 1);
        op.add_stage(StageId::Rn, dispatch + 1, issue);
        op.add_stage(StageId::Ds, issue, issue);
        op.add_stage(StageId::Is, issue, issue + 1);
        op.add_stage(StageId::Ex, issue + 1, complete);
        op.add_stage(StageId::Cm, complete, complete + 1);
        op.add_stage(StageId::Rt, commit, commit + 1);

        // Add dependencies from pipeline tracker
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

    println!("   Total dependencies: {} (memory: {}, register: {})",
        total_deps, mem_deps, total_deps - mem_deps);

    // Save to JSON
    let json = serde_json::to_string_pretty(&konata).unwrap();
    let output_path = "spec17_konata_visualization.json";
    let mut file = File::create(output_path).unwrap();
    file.write_all(json.as_bytes()).unwrap();

    let file_size = std::fs::metadata(output_path).unwrap().len();
    println!("   ✅ Saved to: {} ({:.2} KB)", output_path, file_size as f64 / 1024.0);

    // Print sample trace entries
    println!("\n📋 Sample Trace Entries (first 20):");
    println!("┌──────┬──────────────┬──────────────────┬─────────┬─────────┬─────────┐");
    println!("│  ID  │     PC       │     Opcode       │  Disp   │  Issue  │ Commit  │");
    println!("├──────┼──────────────┼──────────────────┼─────────┼─────────┼─────────┤");

    for (i, entry) in trace.entries().iter().take(20).enumerate() {
        let opcode = if entry.opcode.len() > 14 {
            &entry.opcode[..14]
        } else {
            &entry.opcode
        };
        println!("│ {:>4} │ {:>#10x} │ {:<16} │ {:>7} │ {:>7} │ {:>7} │",
            i,
            entry.pc,
            opcode,
            entry.dispatch_cycle,
            entry.issue_cycle.unwrap_or(0),
            entry.commit_cycle.unwrap_or(0)
        );
    }
    println!("└──────┴──────────────┴──────────────────┴─────────┴─────────┴─────────┘");

    println!("\n✅ Trace simulation completed successfully!");
    println!("\n💡 可视化文件可用于:");
    println!("   1. 在浏览器中打开 viz_server 查看流水线动画");
    println!("   2. 将 JSON 文件导入到 Konata 可视化工具");
}
