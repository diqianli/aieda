//! ARM CPU Emulator - Run ELF executable and export Konata visualization
//!
//! Usage: cpu_emulator <elf_file> [max_instructions] [output.json]
//!
//! This program:
//! 1. Loads an ELF executable (AArch64)
//! 2. Runs CPU simulation with out-of-order execution
//! 3. Exports Konata JSON for pipeline visualization
//! 4. Generates TopDown performance analysis report

use arm_cpu_emulator::{
    elf::{ElfLoader, Arm64Decoder},
    types::{Instruction, InstructionId},
    CPUConfig, CPUEmulator, InstructionSource, PerformanceMetrics,
};

#[cfg(feature = "visualization")]
use arm_cpu_emulator::visualization::KonataOp;

use std::path::PathBuf;
use std::fs::File;
use std::io::Write;

/// Default maximum instructions to simulate
const DEFAULT_MAX_INSTRUCTIONS: u64 = 10000;

/// Instruction source from ELF file
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
                // Check if this is a load/store by checking if mem_size is set
                if decoded.mem_size.is_some() {
                    // Use a synthetic address since we don't simulate actual memory state
                    let addr = 0x10000 + self.count * 64;
                    let size = decoded.mem_size.unwrap_or(8);
                    instr = instr.with_mem_access(addr, size, decoded.is_load);
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
    use arm_cpu_emulator::visualization::KonataOp;

    // Parse arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("============================================");
        println!("  ARM CPU Emulator - AArch64");
        println!("============================================");
        println!();
        println!("Usage: cpu_emulator <elf_file> [max_instructions] [output.json]");
        println!();
        println!("Arguments:");
        println!("  elf_file         Path to ELF executable (AArch64)");
        println!("  max_instructions Maximum instructions to simulate (default: {})", DEFAULT_MAX_INSTRUCTIONS);
        println!("  output.json      Output Konata JSON path (default: konata_data.json)");
        println!();
        println!("Examples:");
        println!("  cpu_emulator program.elf");
        println!("  cpu_emulator program.elf 50000");
        println!("  cpu_emulator program.elf 100000 output.json");
        println!();
        std::process::exit(0);
    }

    let elf_path = PathBuf::from(&args[1]);
    let max_instructions = args.get(2)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_MAX_INSTRUCTIONS);

    // Determine output path
    let output_path = args.get(3)
        .cloned()
        .unwrap_or_else(|| {
            // Default: same directory as ELF, with .json extension
            let mut path = elf_path.clone();
            path.set_extension("json");
            path.to_string_lossy().to_string()
        });

    // Derive additional output paths
    let topdown_path = output_path.replace(".json", "_topdown.json");
    let report_path = output_path.replace(".json", "_report.html");

    println!("============================================");
    println!("  ARM CPU Emulator");
    println!("============================================");
    println!();
    println!("ELF file: {:?}", elf_path);
    println!("Max instructions: {}", max_instructions);
    println!("Output: {}", output_path);
    println!();

    // Load ELF file
    let loader = match ElfLoader::load(&elf_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[ERROR] Failed to load ELF: {:?}", e);
            std::process::exit(1);
        }
    };

    // Show ELF info
    println!("--- ELF Info ---");
    println!("Entry point: {:#X}", loader.entry_point());
    println!("Segments: {}", loader.segments().len());

    for (i, seg) in loader.segments().iter().enumerate() {
        let flags = match (seg.executable, seg.writable) {
            (true, true) => "RWX",
            (true, false) => "R-X",
            (false, true) => "RW-",
            (false, false) => "R--",
        };
        println!("  Segment {}: {:#X}-{:#X} ({})", i, seg.vaddr, seg.vaddr + seg.size as u64, flags);
    }

    // Show first few instructions
    println!();
    println!("--- Instructions (first 10) ---");
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
    let viz_config = arm_cpu_emulator::VisualizationConfig {
        enabled: true,
        port: 3000,
        max_snapshots: 100000,
        animation_speed: 10,
    };

    // Create CPU emulator
    let config = CPUConfig {
        window_size: 256,
        issue_width: 6,
        commit_width: 6,
        enable_trace_output: false,
        ..Default::default()
    };

    let mut cpu = CPUEmulator::with_visualization(config, viz_config.clone())?;

    // Create instruction source from ELF
    let mut source = ElfInstructionSource::new(loader, max_instructions);

    println!();
    println!("--- Running Simulation ---");

    // Run simulation
    let start_time = std::time::Instant::now();
    let result = cpu.run(&mut source);
    let elapsed = start_time.elapsed();

    let stats = match result {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[ERROR] Simulation failed: {:?}", e);
            std::process::exit(1);
        }
    };

    println!();
    println!("--- Results ---");
    println!("Time: {:.2}s", elapsed.as_secs_f64());
    println!("Total cycles: {}", stats.total_cycles);
    println!("Instructions: {}", stats.total_instructions);
    println!("IPC: {:.2}", stats.ipc);

    // Export Konata JSON (including retired instructions)
    println!();
    println!("--- Exporting Konata Data ---");

    let ops_count = cpu.visualization().export_all_konata_to_file(&output_path)?;
    println!("Konata JSON: {} ({} ops)", output_path, ops_count);

    // Generate TopDown analysis
    println!();
    println!("--- TopDown Analysis ---");

    let topdown = generate_topdown_analysis(&stats, stats.total_cycles);

    // Export TopDown JSON
    let topdown_json = serde_json::to_string_pretty(&topdown)?;
    let mut file = File::create(&topdown_path)?;
    file.write_all(topdown_json.as_bytes())?;
    println!("TopDown JSON: {}", topdown_path);

    // Generate HTML report
    let html = generate_html_report(&topdown, &stats);
    let mut file = File::create(&report_path)?;
    file.write_all(html.as_bytes())?;
    println!("HTML Report: {}", report_path);

    println!();
    println!("============================================");
    println!("  Complete!");
    println!("============================================");
    println!();
    println!("To view visualization:");
    println!("  1. cd {}", PathBuf::from(&output_path).parent().unwrap_or(&PathBuf::from(".")).display());
    println!("  2. python -m http.server 8080");
    println!("  3. Open http://localhost:8080/{}_report.html", PathBuf::from(&output_path).file_stem().unwrap_or_default().to_string_lossy());

    Ok(())
}

#[cfg(not(feature = "visualization"))]
fn main() {
    eprintln!("[ERROR] This program requires the 'visualization' feature.");
    eprintln!("Build with: cargo build --release --features visualization --example cpu_emulator");
}

/// Konata export format
#[cfg(feature = "visualization")]
#[derive(serde::Serialize)]
struct KonataExport {
    version: String,
    total_cycles: u64,
    total_instructions: u64,
    ops_count: usize,
    ops: Vec<KonataOp>,
}

/// TopDown analysis result
#[derive(serde::Serialize)]
struct TopDownAnalysis {
    pub summary: SummaryMetrics,
    pub topdown: TopDownMetrics,
    pub pipeline: PipelineMetrics,
}

#[derive(serde::Serialize)]
struct SummaryMetrics {
    pub total_cycles: u64,
    pub total_instructions: u64,
    pub ipc: f64,
}

#[derive(serde::Serialize)]
struct TopDownMetrics {
    pub retiring: f64,
    pub frontend_bound: f64,
    pub backend_bound: f64,
    pub bad_speculation: f64,
}

#[derive(serde::Serialize)]
struct PipelineMetrics {
    pub issue_width: u64,
    pub commit_width: u64,
    pub window_size: u64,
}

#[cfg(feature = "visualization")]
fn generate_topdown_analysis(stats: &PerformanceMetrics, total_cycles: u64) -> TopDownAnalysis {
    let total_instructions = stats.total_instructions as f64;
    let cycles = total_cycles as f64;

    // Calculate TopDown metrics
    let retiring = if cycles > 0.0 { total_instructions / cycles } else { 0.0 };
    let frontend_bound = 0.1; // Placeholder
    let backend_bound = ((1.0_f64 - stats.l1_hit_rate).max(0.0)) * 0.3;
    let bad_speculation = 0.05; // Placeholder

    // Normalize to 100%
    let total = retiring + frontend_bound + backend_bound + bad_speculation;
    let normalize = |v: f64| if total > 0.0 { (v / total * 100.0).min(100.0) } else { 0.0 };

    TopDownAnalysis {
        summary: SummaryMetrics {
            total_cycles,
            total_instructions: stats.total_instructions,
            ipc: stats.ipc,
        },
        topdown: TopDownMetrics {
            retiring: normalize(retiring),
            frontend_bound: normalize(frontend_bound),
            backend_bound: normalize(backend_bound),
            bad_speculation: normalize(bad_speculation),
        },
        pipeline: PipelineMetrics {
            issue_width: 6,
            commit_width: 6,
            window_size: 256,
        },
    }
}

#[cfg(feature = "visualization")]
fn generate_html_report(topdown: &TopDownAnalysis, stats: &PerformanceMetrics) -> String {
    format!(r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <title>ARM CPU Emulator - Performance Report</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 40px; background: #1a1a2e; color: #eee; }}
        h1 {{ color: #00d4ff; }}
        h2 {{ color: #00d4ff; border-bottom: 1px solid #333; padding-bottom: 10px; }}
        .metric {{ background: #16213e; padding: 20px; margin: 10px 0; border-radius: 8px; }}
        .metric-label {{ color: #888; font-size: 14px; }}
        .metric-value {{ font-size: 32px; font-weight: bold; color: #00d4ff; }}
        .bar {{ height: 30px; background: #333; border-radius: 4px; margin: 10px 0; }}
        .bar-fill {{ height: 100%; border-radius: 4px; }}
        .retiring {{ background: #4CAF50; }}
        .frontend {{ background: #2196F3; }}
        .backend {{ background: #FF9800; }}
        .speculation {{ background: #f44336; }}
        table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
        th, td {{ padding: 12px; text-align: left; border-bottom: 1px solid #333; }}
        th {{ color: #00d4ff; }}
    </style>
</head>
<body>
    <h1>ARM CPU Emulator - Performance Report</h1>

    <h2>Summary</h2>
    <div class="metric">
        <div class="metric-label">Total Cycles</div>
        <div class="metric-value">{}</div>
    </div>
    <div class="metric">
        <div class="metric-label">Instructions</div>
        <div class="metric-value">{}</div>
    </div>
    <div class="metric">
        <div class="metric-label">IPC</div>
        <div class="metric-value">{:.2}</div>
    </div>

    <h2>TopDown Analysis</h2>
    <table>
        <tr><th>Metric</th><th>Value</th><th>Bar</th></tr>
        <tr>
            <td>Retiring</td>
            <td>{:.1}%</td>
            <td><div class="bar"><div class="bar-fill retiring" style="width: {:.1}%"></div></div></td>
        </tr>
        <tr>
            <td>Frontend Bound</td>
            <td>{:.1}%</td>
            <td><div class="bar"><div class="bar-fill frontend" style="width: {:.1}%"></div></div></td>
        </tr>
        <tr>
            <td>Backend Bound</td>
            <td>{:.1}%</td>
            <td><div class="bar"><div class="bar-fill backend" style="width: {:.1}%"></div></div></td>
        </tr>
        <tr>
            <td>Bad Speculation</td>
            <td>{:.1}%</td>
            <td><div class="bar"><div class="bar-fill speculation" style="width: {:.1}%"></div></div></td>
        </tr>
    </table>

    <h2>Pipeline Configuration</h2>
    <table>
        <tr><th>Parameter</th><th>Value</th></tr>
        <tr><td>Issue Width</td><td>{}</td></tr>
        <tr><td>Commit Width</td><td>{}</td></tr>
        <tr><td>Window Size</td><td>{}</td></tr>
        <tr><td>L1 Hit Rate</td><td>{:.1}%</td></tr>
    </table>
</body>
</html>"#,
        topdown.summary.total_cycles,
        topdown.summary.total_instructions,
        topdown.summary.ipc,
        topdown.topdown.retiring, topdown.topdown.retiring,
        topdown.topdown.frontend_bound, topdown.topdown.frontend_bound,
        topdown.topdown.backend_bound, topdown.topdown.backend_bound,
        topdown.topdown.bad_speculation, topdown.topdown.bad_speculation,
        topdown.pipeline.issue_width,
        topdown.pipeline.commit_width,
        topdown.pipeline.window_size,
        stats.l1_hit_rate * 100.0,
    )
}
