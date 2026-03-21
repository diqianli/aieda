//! Test dependency timing in simulation
//!
//! Run: cargo run --features visualization --example test_dependency_timing

use arm_cpu_emulator::{
    types::{Instruction, InstructionId, OpcodeType, Reg},
    CPUConfig, CPUEmulator, InstructionSource, TraceInput,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Dependency Timing Test ===\n");

    // Create CPU emulator with visualization
    let config = CPUConfig {
        window_size: 32,
        issue_width: 4,
        commit_width: 4,
        fetch_width: 4,
        l1_hit_latency: 4,
        l2_hit_latency: 12,
        l2_miss_latency: 100,
        ..Default::default()
    };

    println!("Configuration:");
    println!("  L1 hit latency: {} cycles", config.l1_hit_latency);
    println!("  L2 hit latency: {} cycles", config.l2_hit_latency);
    println!("  L2 miss latency: {} cycles", config.l2_miss_latency);
    println!("  Issue width: {}", config.issue_width);
    println!("  Fetch width: {}", config.fetch_width);
    println!();

    let viz_config = arm_cpu_emulator::VisualizationConfig {
        enabled: true,
        port: 3000,
        max_snapshots: 10000,
        animation_speed: 10,
    };

    let mut cpu = CPUEmulator::with_visualization(config.clone(), viz_config)?;

    // Create instruction trace with explicit dependency chain
    let mut input = TraceInput::new();

    // Instruction 0: ADD X2, X0, X1 (writes X2)
    input.builder(0x1000, OpcodeType::Add)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(2))
        .disasm("ADD X2, X0, X1")
        .build();

    // Instruction 1: ADD X3, X2, X0 (reads X2, depends on instr 0)
    input.builder(0x1004, OpcodeType::Add)
        .src_reg(Reg(2))  // Depends on instruction 0
        .src_reg(Reg(0))
        .dst_reg(Reg(3))
        .disasm("ADD X3, X2, X0")
        .build();

    // Instruction 2: ADD X4, X3, X0 (reads X3, depends on instr 1)
    input.builder(0x1008, OpcodeType::Add)
        .src_reg(Reg(3))  // Depends on instruction 1
        .src_reg(Reg(0))
        .dst_reg(Reg(4))
        .disasm("ADD X4, X3, X0")
        .build();

    // Instruction 3: ADD X5, X0, X0 (independent)
    input.builder(0x100C, OpcodeType::Add)
        .src_reg(Reg(0))
        .src_reg(Reg(0))
        .dst_reg(Reg(5))
        .disasm("ADD X5, X0, X0")
        .build();

    println!("Instruction trace:");
    println!("  0: ADD X2, X0, X1  -> writes X2");
    println!("  1: ADD X3, X2, X0  -> reads X2 (depends on 0)");
    println!("  2: ADD X4, X3, X0  -> reads X3 (depends on 1)");
    println!("  3: ADD X5, X0, X0  -> independent");
    println!();

    // Run simulation
    let result = cpu.run(&mut input)?;

    println!("Simulation results:");
    println!("  Total cycles: {}", result.total_cycles);
    println!("  Instructions committed: {}", result.total_instructions);
    println!("  IPC: {:.2}", result.ipc);
    println!();

    // Check timing
    let tracker = cpu.pipeline_tracker();

    println!("Timing analysis:");
    for i in 0..4 {
        if let Some(timing) = tracker.get_timing(InstructionId(i)) {
            println!("  Instruction {}:", i);
            println!("    Fetch: {:?} - {:?}", timing.fetch_start, timing.fetch_end);
            println!("    Issue: {:?} - {:?}", timing.issue_start, timing.issue_end);
            println!("    Execute: {:?} - {:?}", timing.execute_start, timing.execute_end);
            println!("    Complete: {:?} (cycle)", timing.complete_cycle);
            println!("    Retire: {:?} (cycle)", timing.retire_cycle);

            if let Some(deps) = tracker.get_dependencies(InstructionId(i)) {
                if !deps.is_empty() {
                    println!("    Dependencies: {:?}", deps.iter().map(|d| d.producer_id).collect::<Vec<_>>());
                }
            }
        }
    }
    println!();

    // Verify dependency constraints
    println!("Dependency constraint verification:");

    let timing0 = tracker.get_timing(InstructionId(0)).unwrap();
    let timing1 = tracker.get_timing(InstructionId(1)).unwrap();
    let timing2 = tracker.get_timing(InstructionId(2)).unwrap();

    // Instruction 1 should start Execute AFTER instruction 0 Completes
    let instr0_complete = timing0.complete_cycle.unwrap_or(0);
    let instr1_exec_start = timing1.execute_start.unwrap_or(0);

    if instr1_exec_start >= instr0_complete {
        println!("  ✓ Instruction 1 Execute ({}) >= Instruction 0 Complete ({})",
            instr1_exec_start, instr0_complete);
    } else {
        println!("  ✗ VIOLATION: Instruction 1 Execute ({}) < Instruction 0 Complete ({})",
            instr1_exec_start, instr0_complete);
    }

    // Instruction 2 should start Execute AFTER instruction 1 Completes
    let instr1_complete = timing1.complete_cycle.unwrap_or(0);
    let instr2_exec_start = timing2.execute_start.unwrap_or(0);

    if instr2_exec_start >= instr1_complete {
        println!("  ✓ Instruction 2 Execute ({}) >= Instruction 1 Complete ({})",
            instr2_exec_start, instr1_complete);
    } else {
        println!("  ✗ VIOLATION: Instruction 2 Execute ({}) < Instruction 1 Complete ({})",
            instr2_exec_start, instr1_complete);
    }

    println!("\n=== Test Complete ===");

    Ok(())
}
