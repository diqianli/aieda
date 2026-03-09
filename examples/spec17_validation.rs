//! SPEC CPU 2017 trace validation example
//!
//! This example demonstrates how to use the ARM CPU emulator with SPEC CPU 2017
//! traces from ChampSim.
//!
//! # Downloading SPEC17 Traces
//!
//! ChampSim provides pre-generated SPEC CPU 2017 traces at:
//! http://hpca23.cse.tamu.edu/champsim-traces/speccpu/
//!
//! For example, to download a trace:
//! ```bash
//! wget http://hpca23.cse.tamu.edu/champsim-traces/speccpu/600.perlbench_s-210B.champsimtrace.xz
//! ```
//!
//! # Running the Example
//!
//! ```bash
//! cargo run --example spec17_validation -- /path/to/trace.champsimtrace.xz
//! ```

use arm_cpu_emulator::{
    CPUConfig, CPUEmulator, ChampSimXzTraceParser, InstructionSource, OpcodeType, Reg, TraceInput,
};
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("SPEC CPU 2017 Trace Validation");
        println!("================================");
        println!();
        println!("Usage: {} <trace_file> [max_instructions]", args[0]);
        println!();
        println!("Downloading SPEC17 Traces:");
        println!("  wget http://hpca23.cse.tamu.edu/champsim-traces/speccpu/600.perlbench_s-210B.champsimtrace.xz");
        println!();
        println!("Available traces include:");
        println!("  - 500.perlbench_r-210B.champsimtrace.xz (Integer Rate)");
        println!("  - 600.perlbench_s-210B.champsimtrace.xz (Integer Speed)");
        println!("  - 502.gcc_r-210B.champsimtrace.xz");
        println!("  - 505.mcf_r-210B.champsimtrace.xz");
        println!("  - 520.omnetpp_r-210B.champsimtrace.xz");
        println!("  - And many more...");
        println!();
        println!("Example:");
        println!("  {} 600.perlbench_s-210B.champsimtrace.xz 1000000", args[0]);
        return;
    }

    let trace_path = &args[1];
    let max_instructions: usize = if args.len() > 2 {
        args[2].parse().unwrap_or(1_000_000)
    } else {
        1_000_000 // Default: 1 million instructions
    };

    println!("SPEC CPU 2017 Trace Validation");
    println!("================================");
    println!();
    println!("Trace file: {}", trace_path);
    println!("Max instructions: {}", max_instructions);
    println!();

    // Create CPU configuration
    let config = CPUConfig {
        window_size: 128,
        issue_width: 4,
        commit_width: 4,
        l1_size: 64 * 1024,
        l2_size: 512 * 1024,
        ..Default::default()
    };

    println!("CPU Configuration:");
    println!("  Window size: {}", config.window_size);
    println!("  Issue width: {}", config.issue_width);
    println!("  Commit width: {}", config.commit_width);
    println!("  L1 cache: {}KB, {}-way", config.l1_size / 1024, config.l1_associativity);
    println!("  L2 cache: {}KB, {}-way", config.l2_size / 1024, config.l2_associativity);
    println!();

    // Create emulator
    let mut cpu = match CPUEmulator::new(config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create emulator: {}", e);
            return;
        }
    };

    // Open trace file
    println!("Loading trace file...");
    let parser = match ChampSimXzTraceParser::from_file(trace_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to open trace file: {}", e);
            return;
        }
    };

    // Create a limited iterator
    let mut instruction_count = 0;
    let mut input = TraceInput::with_capacity(10000);

    println!("Parsing trace (this may take a while for compressed traces)...");

    // Parse and add instructions
    for result in parser.take(max_instructions) {
        match result {
            Ok(instr) => {
                input.push(instr);
                instruction_count += 1;

                if instruction_count % 100000 == 0 {
                    print!("\r  Parsed {} instructions...", instruction_count);
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                }
            }
            Err(e) => {
                eprintln!("\nError parsing instruction: {}", e);
                break;
            }
        }
    }

    println!("\n\nRunning simulation...");
    println!("  Instructions to simulate: {}", instruction_count);
    println!();

    // Run simulation
    let start_time = std::time::Instant::now();

    // Run the simulation with all instructions
    let metrics = match cpu.run(&mut input) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Simulation error: {}", e);
            return;
        }
    };

    let elapsed = start_time.elapsed();

    println!("\n\nSimulation completed!");
    println!();

    // Print results
    let metrics = cpu.get_metrics();

    println!("Results:");
    println!("--------");
    println!("  Total instructions: {}", metrics.total_instructions);
    println!("  Total cycles: {}", metrics.total_cycles);
    println!("  IPC: {:.4}", metrics.ipc);
    println!("  CPI: {:.4}", metrics.cpi);
    println!();
    println!("  L1 hit rate: {:.2}%", metrics.l1_hit_rate * 100.0);
    println!("  L1 MPKI: {:.2}", metrics.l1_mpki);
    println!("  L2 hit rate: {:.2}%", metrics.l2_hit_rate * 100.0);
    println!("  L2 MPKI: {:.2}", metrics.l2_mpki);
    println!();
    println!("  Memory instructions: {:.2}%", metrics.memory_instr_pct);
    println!("  Branch instructions: {:.2}%", metrics.branch_instr_pct);
    println!("  Avg load latency: {:.2}", metrics.avg_load_latency);
    println!("  Avg store latency: {:.2}", metrics.avg_store_latency);
    println!();
    println!("  Simulation time: {:?}", elapsed);
    println!("  Instructions/second: {:.0}", instruction_count as f64 / elapsed.as_secs_f64());
}

/// Run a simple validation with synthetic instructions
#[allow(dead_code)]
fn run_simple_validation() {
    println!("Running simple validation with synthetic instructions...");
    println!();

    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::with_capacity(1000);

    // Create a mix of instructions
    for i in 0..100 {
        // Compute instruction
        input.builder(0x1000 + i * 4, OpcodeType::Add)
            .src_reg(Reg(0))
            .src_reg(Reg(1))
            .dst_reg(Reg(2))
            .build();

        // Load instruction
        input.builder(0x2000 + i * 4, OpcodeType::Load)
            .dst_reg(Reg(3))
            .mem_access(0x8000 + i * 8, 8, true)
            .build();

        // Store instruction
        input.builder(0x3000 + i * 4, OpcodeType::Store)
            .src_reg(Reg(3))
            .mem_access(0x9000 + i * 8, 8, false)
            .build();
    }

    let metrics = cpu.run(&mut input).unwrap();

    println!("Simple validation results:");
    println!("  Total instructions: {}", metrics.total_instructions);
    println!("  IPC: {:.4}", metrics.ipc);
    println!("  L1 hit rate: {:.2}%", metrics.l1_hit_rate * 100.0);
}
