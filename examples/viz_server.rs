//! Visualization Server Example
//!
//! Run with: cargo run --features visualization --example viz_server
//!
//! Then open http://localhost:3000 in your browser.

use arm_cpu_emulator::{
    CPUConfig, CPUEmulator, OpcodeType, Reg, TraceInput,
    VisualizationConfig, VisualizationServer,
};

#[cfg(feature = "visualization")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Create visualization configuration
    let viz_config = VisualizationConfig {
        enabled: true,
        port: 3000,
        max_snapshots: 10000,
        animation_speed: 10,
    };

    // Create the visualization server
    let server = VisualizationServer::new(viz_config.clone());
    let state = server.state();

    // Run the server in the background
    let server_handle = server.run_in_background();

    println!("Visualization server started on http://localhost:{}", viz_config.port);
    println!("Open this URL in your browser to see the visualization.");

    // Create CPU emulator with visualization enabled
    let config = CPUConfig {
        window_size: 256,
        issue_width: 6,
        commit_width: 6,
        enable_trace_output: true,
        ..Default::default()
    };

    let mut cpu = CPUEmulator::with_visualization(config, viz_config)?;

    // Create a sample instruction trace
    let mut input = TraceInput::new();

    // Add some instructions with dependencies
    for i in 0..100 {
        let pc = 0x1000 + i as u64 * 4;

        // Mix of different instruction types
        match i % 5 {
            0 => {
                // ADD with dependency chain
                input.builder(pc, OpcodeType::Add)
                    .src_reg(Reg((i % 30) as u8))
                    .src_reg(Reg(((i + 1) % 30) as u8))
                    .dst_reg(Reg(((i + 2) % 30) as u8))
                    .disasm(format!("ADD X{}, X{}, X{}", (i + 2) % 30, i % 30, (i + 1) % 30))
                    .build();
            }
            1 => {
                // LOAD instruction
                let addr = 0x2000 + (i as u64 * 64);
                input.builder(pc, OpcodeType::Load)
                    .dst_reg(Reg((i % 30) as u8))
                    .mem_access(addr, 8, true)
                    .disasm(format!("LDR X{}, [X{}, #{}]", i % 30, 31, addr))
                    .build();
            }
            2 => {
                // MUL instruction (higher latency)
                input.builder(pc, OpcodeType::Mul)
                    .src_reg(Reg((i % 30) as u8))
                    .src_reg(Reg(((i + 1) % 30) as u8))
                    .dst_reg(Reg(((i + 2) % 30) as u8))
                    .disasm(format!("MUL X{}, X{}, X{}", (i + 2) % 30, i % 30, (i + 1) % 30))
                    .build();
            }
            3 => {
                // STORE instruction
                let addr = 0x3000 + (i as u64 * 64);
                input.builder(pc, OpcodeType::Store)
                    .src_reg(Reg((i % 30) as u8))
                    .mem_access(addr, 8, false)
                    .disasm(format!("STR X{}, [X{}, #{}]", i % 30, 31, addr))
                    .build();
            }
            _ => {
                // SUB instruction
                input.builder(pc, OpcodeType::Sub)
                    .src_reg(Reg((i % 30) as u8))
                    .src_reg(Reg(((i + 1) % 30) as u8))
                    .dst_reg(Reg(((i + 2) % 30) as u8))
                    .disasm(format!("SUB X{}, X{}, X{}", (i + 2) % 30, i % 30, (i + 1) % 30))
                    .build();
            }
        }
    }

    println!("Running simulation with {} instructions...", 100);

    // Run the simulation step by step
    let mut total_committed = 0u64;
    let mut cycles = 0u64;
    let max_cycles = 1000;

    // First, dispatch all instructions
    for _ in 0..100 {
        if let Some(Ok(instr)) = input.next() {
            let _ = cpu.dispatch(instr);
        }
    }

    // Run simulation loop
    while total_committed < 100 && cycles < max_cycles {
        // Execute one cycle
        cpu.step();
        cycles += 1;
        total_committed = cpu.committed_count();

        // Get the latest snapshot and send to server
        let snapshot = cpu.visualization().latest_snapshot().cloned();
        if let Some(snap) = snapshot {
            state.add_snapshot(snap).await;
        }

        // Also get and send the latest Konata snapshot
        let konata_snapshot = cpu.visualization().latest_konata_snapshot().cloned();
        if let Some(konata_snap) = konata_snapshot {
            state.add_konata_snapshot(konata_snap).await;
        }

        // Small delay to allow real-time visualization
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    }

    println!("Simulation complete after {} cycles.", cycles);
    println!("Committed {} instructions.", total_committed);
    let metrics = cpu.get_metrics();
    println!("Final IPC: {:.3}", metrics.ipc);
    println!("L1 Hit Rate: {:.2}%", metrics.l1_hit_rate * 100.0);

    // Keep the server running
    println!("\nPress Ctrl+C to stop the server.");
    println!("Open http://localhost:3000 in your browser to see the visualization.");

    server_handle.await?;

    Ok(())
}

#[cfg(not(feature = "visualization"))]
fn main() {
    eprintln!("This example requires the 'visualization' feature.");
    eprintln!("Run with: cargo run --features visualization --example viz_server");
}
