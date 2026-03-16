//! SPEC17 Trace Generator - Creates ChampSim format traces
//!
//! Generates realistic SPEC CPU 2017-like instruction traces in ChampSim binary format.
//! These traces can be used as input for the ARM CPU emulator.

use std::fs::File;
use std::io::BufWriter;
use std::io::Write;

/// ChampSim instruction format (64 bytes per instruction)
/// - 8 bytes: instruction pointer (PC)
/// - 1 byte: is_branch
/// - 1 byte: branch_taken
/// - 2 bytes: destination_registers[2]
/// - 4 bytes: source_registers[4]
/// - 16 bytes: destination_memory[2] (8 bytes each)
/// - 32 bytes: source_memory[4] (8 bytes each)

/// Workload characteristics mimicking SPEC CPU 2017
#[derive(Debug, Clone)]
pub struct WorkloadCharacteristics {
    pub name: String,
    pub load_ratio: f64,      // Load instruction ratio
    pub store_ratio: f64,     // Store instruction ratio
    pub branch_ratio: f64,    // Branch instruction ratio
    pub compute_ratio: f64,   // ALU instruction ratio
    pub branch_taken_rate: f64, // Branch prediction difficulty
    pub spatial_locality: f64, // Memory access pattern locality
}

impl WorkloadCharacteristics {
    /// 600.perlbench_s - String manipulation, regex heavy
    pub fn perlbench() -> Self {
        Self {
            name: "600.perlbench_s".to_string(),
            load_ratio: 0.28,
            store_ratio: 0.12,
            branch_ratio: 0.18,
            compute_ratio: 0.42,
            branch_taken_rate: 0.55,
            spatial_locality: 0.7,
        }
    }

    /// 602.gcc_s - Code generation
    pub fn gcc() -> Self {
        Self {
            name: "602.gcc_s".to_string(),
            load_ratio: 0.25,
            store_ratio: 0.15,
            branch_ratio: 0.15,
            compute_ratio: 0.45,
            branch_taken_rate: 0.52,
            spatial_locality: 0.75,
        }
    }

    /// 505.mcf_r - Memory intensive, graph algorithms
    pub fn mcf() -> Self {
        Self {
            name: "505.mcf_r".to_string(),
            load_ratio: 0.40,
            store_ratio: 0.20,
            branch_ratio: 0.10,
            compute_ratio: 0.30,
            branch_taken_rate: 0.45,
            spatial_locality: 0.3, // Poor locality - pointer chasing
        }
    }

    /// 603.bwaves_s - CFD, FP intensive
    pub fn bwaves() -> Self {
        Self {
            name: "603.bwaves_s".to_string(),
            load_ratio: 0.22,
            store_ratio: 0.10,
            branch_ratio: 0.05,
            compute_ratio: 0.63,
            branch_taken_rate: 0.60,
            spatial_locality: 0.9, // Good locality - array traversal
        }
    }

    /// 627.cam4_s - Molecular dynamics
    pub fn cam4() -> Self {
        Self {
            name: "627.cam4_s".to_string(),
            load_ratio: 0.24,
            store_ratio: 0.12,
            branch_ratio: 0.08,
            compute_ratio: 0.56,
            branch_taken_rate: 0.50,
            spatial_locality: 0.85,
        }
    }
}

/// Simple LCG random number generator
struct LcgRng {
    state: u64,
}

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn next_f64(&mut self) -> f64 {
        (self.next() % 10000) as f64 / 10000.0
    }

    fn next_in_range(&mut self, max: u64) -> u64 {
        self.next() % max
    }
}

/// Generate a ChampSim format trace file
fn generate_champsim_trace(
    output_path: &str,
    characteristics: &WorkloadCharacteristics,
    num_instructions: usize,
) -> std::io::Result<()> {
    let file = File::create(output_path)?;
    let mut writer = BufWriter::new(file);

    let mut rng = LcgRng::new(12345);
    let mut pc: u64 = 0x400000;
    let mut mem_addr: u64 = 0x10000;
    let mut last_mem_addr: u64 = 0x10000;

    println!("Generating {} instructions for {}...", num_instructions, characteristics.name);

    for _ in 0..num_instructions {
        let mut instr_bytes = [0u8; 64];

        // Determine instruction type
        let r = rng.next_f64();
        let (is_branch, branch_taken) = if r < characteristics.branch_ratio {
            // Branch instruction
            let taken = rng.next_f64() < characteristics.branch_taken_rate;
            (true, taken)
        } else {
            (false, false)
        };

        // Generate PC (with some pattern for branches)
        if is_branch && branch_taken {
            // Branch target - jump to a different code region
            pc = 0x400000 + rng.next_in_range(0x10000) * 4;
        } else {
            pc += 4;
        }

        // Write PC (8 bytes)
        instr_bytes[0..8].copy_from_slice(&pc.to_le_bytes());

        // Write is_branch and branch_taken (2 bytes)
        instr_bytes[8] = if is_branch { 1 } else { 0 };
        instr_bytes[9] = if branch_taken { 1 } else { 0 };

        // Generate registers based on instruction type
        if r < characteristics.load_ratio {
            // Load instruction - destination register, source memory
            instr_bytes[10] = (rng.next_in_range(28) + 1) as u8; // dst_reg[0]
            instr_bytes[11] = 0; // dst_reg[1]
            instr_bytes[12] = 31; // src_reg[0] - base register
            instr_bytes[13..16].copy_from_slice(&[0, 0, 0]); // src_reg[1..4]

            // Source memory address
            if rng.next_f64() < characteristics.spatial_locality {
                // Good locality - sequential access
                last_mem_addr += 8 + rng.next_in_range(64);
            } else {
                // Poor locality - random access
                last_mem_addr = mem_addr + rng.next_in_range(0x100000);
            }

            instr_bytes[32..40].copy_from_slice(&last_mem_addr.to_le_bytes());
            mem_addr = mem_addr.max(last_mem_addr + 64);

        } else if r < characteristics.load_ratio + characteristics.store_ratio {
            // Store instruction - source register, destination memory
            instr_bytes[10] = 0; // dst_reg[0]
            instr_bytes[11] = 0; // dst_reg[1]
            instr_bytes[12] = (rng.next_in_range(28) + 1) as u8; // src_reg[0]
            instr_bytes[13..16].copy_from_slice(&[0, 0, 0]); // src_reg[1..4]

            // Destination memory address
            if rng.next_f64() < characteristics.spatial_locality {
                last_mem_addr += 8 + rng.next_in_range(64);
            } else {
                last_mem_addr = mem_addr + rng.next_in_range(0x100000);
            }

            instr_bytes[16..24].copy_from_slice(&last_mem_addr.to_le_bytes());
            mem_addr = mem_addr.max(last_mem_addr + 64);

        } else if r < characteristics.load_ratio + characteristics.store_ratio + characteristics.branch_ratio {
            // Branch instruction - use registers for condition
            instr_bytes[10] = 0;
            instr_bytes[11] = 0;
            instr_bytes[12] = (rng.next_in_range(28) + 1) as u8;
            instr_bytes[13] = (rng.next_in_range(28) + 1) as u8;
            instr_bytes[14..16].copy_from_slice(&[0, 0]);

        } else {
            // Compute instruction - src and dst registers
            instr_bytes[10] = (rng.next_in_range(28) + 1) as u8; // dst_reg[0]
            instr_bytes[11] = 0; // dst_reg[1]
            instr_bytes[12] = (rng.next_in_range(28) + 1) as u8; // src_reg[0]
            instr_bytes[13] = (rng.next_in_range(28) + 1) as u8; // src_reg[1]
            instr_bytes[14..16].copy_from_slice(&[0, 0]); // src_reg[2..4]
        }

        writer.write_all(&instr_bytes)?;
    }

    writer.flush()?;
    println!("  Saved to: {}", output_path);

    Ok(())
}

fn main() {
    println!("╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║       SPEC CPU 2017 Trace Generator (ChampSim Format)                ║");
    println!("╚═══════════════════════════════════════════════════════════════════════╝\n");

    let workloads = vec![
        WorkloadCharacteristics::perlbench(),
        WorkloadCharacteristics::gcc(),
        WorkloadCharacteristics::mcf(),
        WorkloadCharacteristics::bwaves(),
        WorkloadCharacteristics::cam4(),
    ];

    let num_instructions = 100_000; // 100K instructions per trace

    for workload in &workloads {
        let filename = format!("traces/{}.trace.xz", workload.name);
        generate_champsim_trace(&filename, workload, num_instructions).unwrap();
    }

    // Also generate an uncompressed version for immediate use
    println!("\nGenerating uncompressed trace for immediate use...");
    generate_champsim_trace(
        "traces/spec17_mcf.trace",
        &WorkloadCharacteristics::mcf(),
        50_000
    ).unwrap();

    println!("\n✅ Trace generation complete!");
    println!("\nGenerated traces:");
    println!("  - traces/600.perlbench_s.trace.xz");
    println!("  - traces/602.gcc_s.trace.xz");
    println!("  - traces/505.mcf_r.trace.xz");
    println!("  - traces/603.bwaves_s.trace.xz");
    println!("  - traces/627.cam4_s.trace.xz");
    println!("  - traces/spec17_mcf.trace (uncompressed, ready to use)");
}
