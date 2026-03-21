//! Run ELF executable and export Konata JSON for visualization
//!
//! Usage: cargo run --features visualization --example elf_to_konata -- /path/to/elf [output.json]
//!
//! This example:
//! 1. Loads and runs the ELF simulation
//! 2. Exports all pipeline data to a Konata JSON file
//! 3. The frontend loads this file directly (no WebSocket needed)

use arm_cpu_emulator::{
    elf::{ElfLoader, Arm64Decoder},
    types::{Instruction, InstructionId},
    CPUConfig, CPUEmulator, InstructionSource,
};

#[cfg(feature = "visualization")]
use arm_cpu_emulator::visualization::KonataOp;

use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

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
        if self.count >= self.max_count || self.pc >= self.end_pc {
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

                // Add registers
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
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::env;
    use arm_cpu_emulator::visualization::{VisualizationConfig, KonataOp, KonataSnapshot};

    // Parse arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <elf_file> [max_instructions] [output.json]", args[0]);
        eprintln!("\nExample:");
        eprintln!("  {} /path/to/program.elf 5000 konata_data.json", args[0]);
        std::process::exit(1);
    }

    let elf_path = PathBuf::from(&args[1]);
    let max_instructions = args.get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(MAX_INSTRUCTIONS);
    let output_path = args.get(3)
        .cloned()
        .unwrap_or_else(|| "visualization/static/konata_data.json".to_string());

    println!("=== ELF to Konata Export ===");
    println!("Loading ELF: {:?}", elf_path);
    println!("Max instructions: {}", max_instructions);
    println!("Output file: {}", output_path);

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

    // Create visualization configuration (enabled for tracking)
    let viz_config = VisualizationConfig {
        enabled: true,
        port: 3000,
        max_snapshots: 100000,  // Large buffer for all instructions
        animation_speed: 10,
    };

    // Create CPU emulator with visualization
    let config = CPUConfig {
        window_size: 256,
        issue_width: 6,
        commit_width: 6,
        enable_trace_output: false,  // Disable trace output for cleaner output
        ..Default::default()
    };

    let mut cpu = CPUEmulator::with_visualization(config, viz_config.clone())?;

    // Create instruction source from ELF
    let mut source = ElfInstructionSource::new(loader, max_instructions);

    println!("\n--- Running Simulation ---");
    println!("Simulating up to {} instructions...", max_instructions);

    // Run simulation
    let start_time = std::time::Instant::now();
    let result = cpu.run(&mut source);
    let elapsed = start_time.elapsed();

    match result {
        Ok(stats) => {
            println!("\n--- Simulation Complete ---");
            println!("Time: {:.2}s", elapsed.as_secs_f64());
            println!("Total cycles: {}", stats.total_cycles);
            println!("Instructions committed: {}", stats.total_instructions);
            println!("IPC: {:.2}", stats.ipc);
        }
        Err(e) => {
            eprintln!("Simulation error: {:?}", e);
        }
    }

    // Export Konata data
    println!("\n--- Exporting Konata Data ---");
    let cpu_viz = cpu.visualization();
    let konata_snapshots = cpu_viz.konata_snapshots();
    println!("Collected {} Konata snapshots", konata_snapshots.len());

    // Merge all snapshots into one comprehensive export
    let mut all_ops: Vec<KonataOp> = Vec::new();
    let mut last_cycle = 0u64;
    let mut last_committed = 0u64;

    for snapshot in konata_snapshots {
        last_cycle = snapshot.cycle;
        last_committed = snapshot.committed_count;

        for op in &snapshot.ops {
            // Only add if not already present (by id)
            if !all_ops.iter().any(|o| o.id == op.id) {
                all_ops.push(op.clone());
            }
        }
    }

    // Sort by ID
    all_ops.sort_by_key(|op| op.id);

    // Create final export snapshot
    let export = KonataExport {
        version: "1.0".to_string(),
        total_cycles: last_cycle,
        total_instructions: last_committed,
        ops_count: all_ops.len(),
        ops: all_ops,
    };

    // Write to file
    let json = serde_json::to_string_pretty(&export)?;
    let mut file = File::create(&output_path)?;
    file.write_all(json.as_bytes())?;

    println!("Exported {} operations to {}", export.ops_count, output_path);
    println!("\n=== Export Complete ===");
    println!("You can now view the visualization by:");
    println!("1. Starting a simple HTTP server: cd visualization && python3 -m http.server 8080");
    println!("2. Opening http://localhost:8080 in your browser");
    println!("   (The page will load konata_data.json automatically)");

    Ok(())
}

#[cfg(not(feature = "visualization"))]
fn main() {
    eprintln!("This example requires the 'visualization' feature.");
    eprintln!("Run with: cargo run --features visualization --example elf_to_konata -- /path/to/elf");
}

/// Export format for Konata data
#[cfg(feature = "visualization")]
#[derive(serde::Serialize)]
struct KonataExport {
    version: String,
    total_cycles: u64,
    total_instructions: u64,
    ops_count: usize,
    ops: Vec<KonataOp>,
}
