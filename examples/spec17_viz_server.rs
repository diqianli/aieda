//! SPEC17 Visualization Server
//!
//! Run with: cargo run --features visualization --example spec17_viz_server --release
//!
//! Then open http://localhost:3000 in your browser.

use arm_cpu_emulator::{
    CPUConfig, CPUEmulator, ChampSimTraceParser, InstructionSource,
    VisualizationConfig, VisualizationServer,
};

#[cfg(feature = "visualization")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Select trace file
    let trace_path = "traces/spec17_mcf.trace";

    println!("╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║       SPEC17 Visualization Server - ARM CPU Emulator                  ║");
    println!("╚═══════════════════════════════════════════════════════════════════════╝\n");

    println!("📁 Loading trace: {}", trace_path);

    // Create trace parser
    let mut parser = match ChampSimTraceParser::from_file(trace_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Failed to load trace: {}", e);
            eprintln!("\n请先运行以下命令生成trace文件:");
            eprintln!("  cargo run --example spec17_trace_generator --release");
            std::process::exit(1);
        }
    };

    let total_instructions = parser.total_count().unwrap_or(0);
    println!("   Total instructions: {}", total_instructions);

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

    println!("\n✅ Visualization server started on http://localhost:{}", viz_config.port);
    println!("   Open this URL in your browser to see the visualization.\n");

    // Create CPU emulator with visualization enabled
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
        max_trace_output: 5000,
        ..Default::default()
    };

    let mut cpu = CPUEmulator::with_visualization(config, viz_config)?;

    println!("🚀 Running simulation with {} instructions...", total_instructions);
    println!("   (Visualization updates will be sent to connected browsers)\n");

    // Run simulation
    let metrics = cpu.run(&mut parser)?;

    println!("╔═══════════════════════════════════════════════════════════════════════╗");
    println!("║                      SIMULATION COMPLETE                               ║");
    println!("╠═══════════════════════════════════════════════════════════════════════╣");
    println!("║  Instructions:  {:>12}                                        ║", metrics.total_instructions);
    println!("║  Cycles:        {:>12}                                        ║", metrics.total_cycles);
    println!("║  IPC:           {:>12.4}                                        ║", metrics.ipc);
    println!("║  L1 Hit Rate:   {:>12.2}%                                      ║", metrics.l1_hit_rate * 100.0);
    println!("║  Memory Ops:    {:>12.2}%                                      ║", metrics.memory_instr_pct);
    println!("╚═══════════════════════════════════════════════════════════════════════╝");

    println!("\n📊 Visualization data available at http://localhost:3000");
    println!("   - Pipeline view shows instruction flow through stages");
    println!("   - Dependency arrows show data dependencies");
    println!("   - Use playback controls to animate the simulation\n");

    println!("Press Ctrl+C to stop the server...");

    // Wait for the server to finish (it runs indefinitely)
    let _ = server_handle.await;

    Ok(())
}

#[cfg(not(feature = "visualization"))]
fn main() {
    eprintln!("This example requires the 'visualization' feature.");
    eprintln!("Run with: cargo run --features visualization --example spec17_viz_server --release");
}
