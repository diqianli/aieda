//! Run ELF executable with visualization
//!
//! Usage: cargo run --features visualization --example elf_viz -- /path/to/elf

use arm_cpu_emulator::{
    elf::{ElfLoader, Arm64Decoder},
    types::{Instruction, InstructionId, Reg},
    CPUConfig, CPUEmulator, InstructionSource, VisualizationConfig, VisualizationServer,
};
use std::path::PathBuf;

/// Maximum instructions to simulate
const MAX_INSTRUCTIONS: u64 = 10000;

/// Create an instruction iterator from ELF file
struct ElfInstructionSource {
    loader: ElfLoader,
    decoder: Arm64Decoder,
    pc: u64,
    end_pc: u64,
    count: u64,
    max_count: u64,
}

impl ElfInstructionSource {
    fn new(loader: ElfLoader, max_count: u64) -> Self {
        let entry = loader.entry_point();

        // Find end of code segment
        let end_pc = loader.segments()
            .iter()
            .filter(|s| s.executable)
            .map(|s| s.vaddr + s.size as u64)
            .max()
            .unwrap_or(entry + 0x1000);

        Self {
            loader,
            decoder: Arm64Decoder::new(),
            pc: entry,
            end_pc,
            count: 0,
            max_count,
        }
    }
}

impl Iterator for ElfInstructionSource {
    type Item = arm_cpu_emulator::types::Result<Instruction>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count >= self.max_count {
            return None;
        }

        // Read instruction at current PC
        match self.loader.read_instruction(self.pc) {
            Some(raw) => {
                let decoded = self.decoder.decode(self.pc, raw);

                // Build instruction
                let mut instr = Instruction::new(
                    InstructionId(self.count),
                    self.pc,
                    raw,
                    decoded.opcode,
                );

                // Add registers (decoded.src_regs is already Vec<Reg>)
                for reg in decoded.src_regs.clone() {
                    instr = instr.with_src_reg(reg);
                }
                for reg in decoded.dst_regs.clone() {
                    instr = instr.with_dst_reg(reg);
                }

                // Add disassembly
                instr = instr.with_disasm(decoded.disasm.clone());

                // Handle memory operations
                if decoded.mem_addr.is_some() {
                    let addr = decoded.mem_addr.unwrap_or(0x10000 + self.count * 64);
                    instr = instr.with_mem_access(addr, decoded.mem_size.unwrap_or(8), decoded.is_load);
                }

                // Handle branches
                if let Some(target) = decoded.branch_target {
                    instr = instr.with_branch(target, decoded.is_conditional, false);
                }

                self.pc += 4;
                self.count += 1;

                Some(Ok(instr))
            }
            None => None,
        }
    }
}

impl InstructionSource for ElfInstructionSource {
    fn total_count(&self) -> Option<usize> {
        Some(self.max_count as usize)
    }

    fn reset(&mut self) -> arm_cpu_emulator::types::Result<()> {
        self.pc = self.loader.entry_point();
        self.count = 0;
        Ok(())
    }
}

#[cfg(feature = "visualization")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::env;

    // Parse arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <elf_file> [max_instructions]", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} /path/to/program.elf 5000", args[0]);
        std::process::exit(1);
    }

    let elf_path = PathBuf::from(&args[1]);
    let max_instructions = args.get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(MAX_INSTRUCTIONS);

    println!("=== ELF Visualization ===");
    println!("Loading ELF: {:?}", elf_path);
    println!("Max instructions: {}", max_instructions);

    // Load ELF file
    let loader = match ElfLoader::load(&elf_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to load ELF: {:?}", e);
            std::process::exit(1);
        }
    };

    println!("\n--- ELF Info ---");
    println!("Entry point: {:#X}", loader.entry_point());
    println!("Segments: {}", loader.segments().len());

    for (i, seg) in loader.segments().iter().enumerate() {
        println!(
            "  Segment {}: {:#X}-{:#X} ({}, {})",
            i, seg.vaddr, seg.vaddr + seg.size as u64,
            if seg.executable { "exec" } else { "" },
            if seg.writable { "write" } else { "" }
        );
    }

    // Show first few instructions
    println!("\n--- First Instructions ---");
    let decoder = Arm64Decoder::new();
    let mut pc = loader.entry_point();
    for _ in 0..10 {
        if let Some(raw) = loader.read_instruction(pc) {
            let decoded = decoder.decode(pc, raw);
            println!("{:#X}: {}", pc, decoded.disasm);
            pc += 4;
        } else {
            break;
        }
    }

    // Create visualization configuration
    let viz_config = VisualizationConfig {
        enabled: true,
        port: 3000,
        max_snapshots: 10000,
        animation_speed: 10,
    };

    // Create the visualization server
    let server = VisualizationServer::new(viz_config.clone());
    let server_state = server.state();

    // Run the server in the background
    let server_handle = server.run_in_background();

    println!("\n--- Starting Visualization Server ---");
    println!("Server running at http://localhost:{}", viz_config.port);
    println!("Open this URL in your browser to see the visualization.");

    // Wait a bit for server to start
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Create CPU emulator with visualization
    let config = CPUConfig {
        window_size: 256,
        issue_width: 6,
        commit_width: 6,
        enable_trace_output: true,
        ..Default::default()
    };

    let mut cpu = CPUEmulator::with_visualization(config, viz_config.clone())?;

    // Create instruction source from ELF
    let mut source = ElfInstructionSource::new(loader, max_instructions);

    println!("\n--- Running Simulation ---");
    println!("Simulating {} instructions...", max_instructions);

    // Run simulation
    let result = cpu.run(&mut source);

    // Transfer visualization data from CPU to server
    println!("\n--- Transferring Visualization Data ---");
    let cpu_viz = cpu.visualization();

    // Transfer regular snapshots
    let snapshots = cpu_viz.snapshots();
    println!("Transferring {} regular snapshots...", snapshots.len());
    for snapshot in snapshots {
        server_state.add_snapshot(snapshot.clone()).await;
    }

    // Transfer Konata snapshots
    let konata_snapshots = cpu_viz.konata_snapshots();
    println!("Transferring {} Konata snapshots...", konata_snapshots.len());
    for snapshot in konata_snapshots {
        server_state.add_konata_snapshot(snapshot.clone()).await;
    }

    match result {
        Ok(stats) => {
            println!("\n--- Simulation Complete ---");
            println!("Total cycles: {}", stats.total_cycles);
            println!("Instructions committed: {}", stats.total_instructions);
            println!("IPC: {:.2}", stats.ipc);
        }
        Err(e) => {
            eprintln!("Simulation error: {:?}", e);
        }
    }

    println!("\n=== Visualization Ready ===");
    println!("Open http://localhost:3000 in your browser");
    println!("Click on 'Konata View' tab to see the pipeline visualization.");
    println!("");
    println!("Konata JSON data available at:");
    println!("  - http://localhost:3000/api/konata (all snapshots)");
    println!("  - http://localhost:3000/api/konata?limit=100 (last 100)");
    println!("  - http://localhost:3000/api/export/konata (full export)");
    println!("");
    println!("Press Ctrl+C to stop the server.");

    // Keep server running
    server_handle.await?;

    Ok(())
}

#[cfg(not(feature = "visualization"))]
fn main() {
    eprintln!("This example requires the 'visualization' feature.");
    eprintln!("Run with: cargo run --features visualization --example elf_viz -- /path/to/elf");
}
