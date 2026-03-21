//! Test to verify Konata timing matches simulation timing
//!
//! Run: cargo run --features visualization --example test_konata_timing

use arm_cpu_emulator::{
    types::{Instruction, InstructionId, OpcodeType, Reg},
    CPUConfig, CPUEmulator, InstructionSource, TraceInput,
};

#[cfg(feature = "visualization")]
use arm_cpu_emulator::visualization::KonataOp;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Konata Timing Verification Test ===\n");

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

    let viz_config = arm_cpu_emulator::VisualizationConfig {
        enabled: true,
        port: 3000,
        max_snapshots: 10000,
        animation_speed: 10,
    };

    let mut cpu = CPUEmulator::with_visualization(config.clone(), viz_config)?;

    // Create the SAME instruction pattern as generate_konata.rs
    let mut input = TraceInput::new();

    // i=0: ADD X2, X0, X1 (writes X2)
    input.builder(0x1000, OpcodeType::Add)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(2))
        .disasm("ADD X2, X0, X1")
        .build();

    // i=1: LDR X1, [X31, #8256] (writes X1)
    input.builder(0x1004, OpcodeType::Load)
        .dst_reg(Reg(1))
        .mem_access(0x2000, 8, true)
        .disasm("LDR X1, [X31, #8256]")
        .build();

    // i=2: MUL X4, X2, X3 (reads X2 - depends on i=0)
    input.builder(0x1008, OpcodeType::Mul)
        .src_reg(Reg(2))  // Depends on instruction 0
        .src_reg(Reg(3))
        .dst_reg(Reg(4))
        .disasm("MUL X4, X2, X3")
        .build();

    // i=3: STR X3, [X31, #12480]
    input.builder(0x100C, OpcodeType::Store)
        .src_reg(Reg(3))
        .mem_access(0x3000, 8, false)
        .disasm("STR X3, [X31, #12480]")
        .build();

    // i=4: SUB X6, X4, X5 (reads X4 - depends on i=2)
    input.builder(0x1010, OpcodeType::Sub)
        .src_reg(Reg(4))  // Depends on instruction 2
        .src_reg(Reg(5))
        .dst_reg(Reg(6))
        .disasm("SUB X6, X4, X5")
        .build();

    println!("Instruction trace:");
    println!("  0: ADD X2, X0, X1  -> writes X2");
    println!("  1: LDR X1, [...]   -> writes X1");
    println!("  2: MUL X4, X2, X3  -> reads X2 (depends on 0)");
    println!("  3: STR X3, [...]   -> no register dependency");
    println!("  4: SUB X6, X4, X5  -> reads X4 (depends on 2)");
    println!();

    // Run simulation
    let result = cpu.run(&mut input)?;

    println!("Simulation results:");
    println!("  Total cycles: {}", result.total_cycles);
    println!("  Instructions committed: {}", result.total_instructions);
    println!();

    // Compare timing from both trackers
    println!("Comparing CPU's pipeline_tracker vs visualization's pipeline_tracker:");
    for i in 0..5 {
        let cpu_timing = cpu.pipeline_tracker().get_timing(InstructionId(i));
        let viz_timing = cpu.visualization().pipeline_tracker().get_timing(InstructionId(i));

        println!("  Instruction {}:", i);
        println!("    CPU tracker execute: {:?}", cpu_timing.map(|t| (t.execute_start, t.execute_end)));
        println!("    Viz tracker execute: {:?}", viz_timing.map(|t| (t.execute_start, t.execute_end)));
    }
    println!();

    // Check timing from pipeline_tracker (same as test_dependency_timing.rs)
    let tracker = cpu.pipeline_tracker();

    println!("Timing from cpu.pipeline_tracker():");
    for i in 0..5 {
        if let Some(timing) = tracker.get_timing(InstructionId(i)) {
            println!("  Instruction {}:", i);
            println!("    Execute: {:?} - {:?}", timing.execute_start, timing.execute_end);
            println!("    Complete: {:?}", timing.complete_cycle);

            if let Some(deps) = tracker.get_dependencies(InstructionId(i)) {
                if !deps.is_empty() {
                    println!("    Dependencies: {:?}", deps.iter().map(|d| d.producer_id).collect::<Vec<_>>());
                }
            }
        }
    }
    println!();

    // Check timing from visualization.konata_snapshots()
    #[cfg(feature = "visualization")]
    {
        let viz = cpu.visualization();
        let snapshots = viz.konata_snapshots();
        println!("Collected {} Konata snapshots", snapshots.len());

        // Find the latest snapshot that has instruction data
        let mut latest_ops: std::collections::HashMap<u64, &KonataOp> = std::collections::HashMap::new();
        for snapshot in snapshots {
            for op in &snapshot.ops {
                latest_ops.insert(op.gid, op);
            }
        }

        println!("\nTiming from Konata snapshots:");
        for i in 0..5u64 {
            if let Some(op) = latest_ops.get(&i) {
                // Find Execute stage
                let exec_stage = op.lanes.get("main").and_then(|lane| {
                    lane.stages.iter().find(|s| s.name == "Ex" || s.name == "Me")
                });

                println!("  Instruction {} ({}):", i, op.label_name);
                if let Some(stage) = exec_stage {
                    println!("    Execute: {} - {}", stage.start_cycle, stage.end_cycle);
                } else {
                    println!("    Execute: NOT FOUND");
                }
                println!("    Dependencies: {:?}", op.prods.iter().map(|d| d.producer_id).collect::<Vec<_>>());
            }
        }
        println!();
    }

    // Verify dependency constraints using pipeline_tracker
    println!("Dependency constraint verification (from pipeline_tracker):");

    let timing0 = tracker.get_timing(InstructionId(0)).unwrap();
    let timing2 = tracker.get_timing(InstructionId(2)).unwrap();

    // Instruction 2 should start Execute AFTER instruction 0 Completes
    let instr0_complete = timing0.complete_cycle.unwrap_or(0);
    let instr2_exec_start = timing2.execute_start.unwrap_or(0);

    if instr2_exec_start >= instr0_complete {
        println!("  ✓ Instruction 2 Execute ({}) >= Instruction 0 Complete ({})",
            instr2_exec_start, instr0_complete);
    } else {
        println!("  ✗ VIOLATION: Instruction 2 Execute ({}) < Instruction 0 Complete ({})",
            instr2_exec_start, instr0_complete);
    }

    println!("\n=== Test Complete ===");

    Ok(())
}
