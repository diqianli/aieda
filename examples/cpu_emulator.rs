//! Generate Konata JSON from demo instructions
//!
//! Usage: cargo run --features visualization --example generate_konata [num_instructions] [output.json]

use arm_cpu_emulator::{
    types::{OpcodeType, Reg},
    CPUConfig, CPUEmulator, InstructionSource, TraceInput,
};

#[cfg(feature = "visualization")]
use arm_cpu_emulator::visualization::{KonataLane, KonataOp, KonataStage};

use std::fs::File;
use std::io::{BufWriter, Write};

/// Default number of instructions
const DEFAULT_INSTRUCTIONS: u64 = 500;

#[cfg(feature = "visualization")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use std::env;

    // Parse arguments
    let args: Vec<String> = env::args().collect();
    let num_instructions = args
        .get(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_INSTRUCTIONS);
    let output_path = args
        .get(2)
        .cloned()
        .unwrap_or_else(|| "visualization/static/konata_data.json".to_string());

    println!("=== Konata Data Generator ===");
    println!("Instructions: {}", num_instructions);
    println!("Output file: {}", output_path);

    // Create visualization configuration with minimal snapshots
    let viz_config = arm_cpu_emulator::VisualizationConfig {
        enabled: true,
        port: 3000,
        max_snapshots: 10, // Keep few snapshots - we use pipeline_tracker directly
        animation_speed: 10,
    };

    // Create CPU emulator with visualization
    let config = CPUConfig {
        window_size: 256,
        issue_width: 6,
        commit_width: 6,
        enable_trace_output: false,
        ..Default::default()
    };

    let mut cpu = CPUEmulator::with_visualization(config.clone(), viz_config)?;

    // Create instruction trace
    let mut input = TraceInput::new();

    println!("\n--- Creating Instruction Trace ---");
    for i in 0..num_instructions {
        let pc = 0x1000 + i as u64 * 4;

        // Mix of different instruction types
        match i % 5 {
            0 => {
                // ADD with dependency chain
                input
                    .builder(pc, OpcodeType::Add)
                    .src_reg(Reg((i % 30) as u8))
                    .src_reg(Reg(((i + 1) % 30) as u8))
                    .dst_reg(Reg(((i + 2) % 30) as u8))
                    .disasm(format!(
                        "ADD X{}, X{}, X{}",
                        (i + 2) % 30,
                        i % 30,
                        (i + 1) % 30
                    ))
                    .build();
            }
            1 => {
                // LOAD instruction
                let addr = 0x2000 + (i as u64 * 64);
                input
                    .builder(pc, OpcodeType::Load)
                    .dst_reg(Reg((i % 30) as u8))
                    .mem_access(addr, 8, true)
                    .disasm(format!("LDR X{}, [X{}, #{}]", i % 30, 31, addr))
                    .build();
            }
            2 => {
                // MUL instruction (higher latency)
                input
                    .builder(pc, OpcodeType::Mul)
                    .src_reg(Reg((i % 30) as u8))
                    .src_reg(Reg(((i + 1) % 30) as u8))
                    .dst_reg(Reg(((i + 2) % 30) as u8))
                    .disasm(format!(
                        "MUL X{}, X{}, X{}",
                        (i + 2) % 30,
                        i % 30,
                        (i + 1) % 30
                    ))
                    .build();
            }
            3 => {
                // STORE instruction
                let addr = 0x3000 + (i as u64 * 64);
                input
                    .builder(pc, OpcodeType::Store)
                    .src_reg(Reg((i % 30) as u8))
                    .mem_access(addr, 8, false)
                    .disasm(format!("STR X{}, [X{}, #{}]", i % 30, 31, addr))
                    .build();
            }
            _ => {
                // SUB instruction
                input
                    .builder(pc, OpcodeType::Sub)
                    .src_reg(Reg((i % 30) as u8))
                    .src_reg(Reg(((i + 1) % 30) as u8))
                    .dst_reg(Reg(((i + 2) % 30) as u8))
                    .disasm(format!(
                        "SUB X{}, X{}, X{}",
                        (i + 2) % 30,
                        i % 30,
                        (i + 1) % 30
                    ))
                    .build();
            }
        }
    }

    println!("Created {} instructions", num_instructions);

    // Run simulation
    println!("\n--- Running Simulation ---");
    let start_time = std::time::Instant::now();
    let result = cpu.run(&mut input);
    let elapsed = start_time.elapsed();

    let stats = result?;
    println!("Time: {:.2}s", elapsed.as_secs_f64());
    println!("Total cycles: {}", stats.total_cycles);
    println!("Instructions committed: {}", stats.total_instructions);
    println!("IPC: {:.2}", stats.ipc);

    // Export Konata data directly from pipeline_tracker (memory efficient)
    println!("\n--- Exporting Konata Data ---");
    let tracker = cpu.pipeline_tracker();
    println!("Tracked {} instructions", tracker.len());

    // Build Konata ops directly from pipeline tracker
    let all_ops = build_konata_ops_from_tracker(tracker);
    println!("Generated {} operations", all_ops.len());

    // Generate TopDown analysis report
    println!("\n--- Generating TopDown Analysis ---");
    let topdown_report = generate_topdown_report(tracker, &stats, &config);
    println!("TopDown Analysis Generated");

    // Create final export
    let export = KonataExport {
        version: "1.0".to_string(),
        total_cycles: stats.total_cycles,
        total_instructions: stats.total_instructions,
        ops_count: all_ops.len(),
        ops: all_ops,
    };

    // Write Konata JSON
    let file = File::create(&output_path)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &export)?;
    writer.flush()?;

    println!("Exported {} operations to {}", export.ops_count, output_path);

    // Write TopDown JSON
    let topdown_json_path = output_path.replace(".json", "_topdown.json");
    let topdown_file = File::create(&topdown_json_path)?;
    let mut topdown_writer = BufWriter::new(topdown_file);
    serde_json::to_writer_pretty(&mut topdown_writer, &topdown_report)?;
    topdown_writer.flush()?;
    println!("TopDown report exported to {}", topdown_json_path);

    // Generate HTML visualization
    let html_path = output_path.replace(".json", "_report.html");
    let html_content = generate_html_report(&topdown_report, &output_path, &topdown_json_path);
    let mut html_file = File::create(&html_path)?;
    html_file.write_all(html_content.as_bytes())?;
    println!("HTML report exported to {}", html_path);

    println!("\n=== Export Complete ===");
    println!("To view the visualization:");
    println!("1. Start HTTP server: cd visualization && python3 -m http.server 8080");
    println!("2. Open http://localhost:8080/static/index_static.html");
    println!("3. For TopDown analysis: open http://localhost:8080/static/konata_data_report.html");

    Ok(())
}

/// Build Konata operations directly from the pipeline tracker
#[cfg(feature = "visualization")]
fn build_konata_ops_from_tracker(
    tracker: &arm_cpu_emulator::visualization::PipelineTracker,
) -> Vec<KonataOp> {
    let mut ops: Vec<KonataOp> = Vec::new();
    let timings = tracker.get_all_timings();
    let viz_id_map = tracker.get_all_viz_ids();
    let dependencies = tracker.get_all_dependencies();
    let disasm_map = tracker.get_all_disasm();
    let src_regs_map = tracker.get_all_src_regs();
    let dst_regs_map = tracker.get_all_dst_regs();
    let mem_access_map = tracker.get_all_mem_access();

    for (instr_id, timing) in timings.iter() {
        let viz_id = viz_id_map.get(instr_id).copied().unwrap_or(instr_id.0);

        // Get disassembly or fall back to generic label
        let label = disasm_map
            .get(instr_id)
            .cloned()
            .unwrap_or_else(|| format!("Instr_{}", instr_id.0));

        // Create KonataOp
        let mut op = KonataOp::new(
            viz_id,
            instr_id.0,
            0x1000 + instr_id.0 * 4, // Approximate PC
            label,
        );

        op.fetched_cycle = timing.fetch_start.unwrap_or(0);
        op.retired_cycle = timing.retire_cycle;

        // Add stages from timing
        let stages = timing.to_stages();
        for stage in stages {
            let lane = op
                .lanes
                .entry("main".to_string())
                .or_insert_with(|| KonataLane::new("main"));
            lane.stages
                .push(KonataStage::new(stage.name, stage.start_cycle, stage.end_cycle));
        }

        // Add source registers
        if let Some(src_regs) = src_regs_map.get(instr_id) {
            op.src_regs = src_regs.clone();
        }

        // Add destination registers
        if let Some(dst_regs) = dst_regs_map.get(instr_id) {
            op.dst_regs = dst_regs.clone();
        }

        // Add memory info
        if let Some((addr, _size, is_load)) = mem_access_map.get(instr_id) {
            op.is_memory = true;
            op.mem_addr = Some(*addr);
            let _ = is_load; // Mark as used
        }

        // Add dependencies
        if let Some(deps) = dependencies.get(instr_id) {
            op.prods = deps.clone();
        }

        ops.push(op);
    }

    // Sort by ID
    ops.sort_by_key(|op| op.id);
    ops
}

#[cfg(not(feature = "visualization"))]
fn main() {
    eprintln!("This example requires the 'visualization' feature.");
    eprintln!(
        "Run with: cargo run --features visualization --example generate_konata"
    );
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

/// TopDown Report for export
#[derive(serde::Serialize)]
struct TopDownReportExport {
    version: String,
    summary: SummaryMetrics,
    topdown: TopDownMetricsExport,
    frontend_bound: FrontendBoundExport,
    backend_bound: BackendBoundExport,
    bad_speculation: BadSpeculationExport,
    retiring: RetiringExport,
    stage_utilization: StageUtilizationExport,
    hotspots: Vec<HotspotExport>,
    cycle_distribution: CycleDistributionExport,
}

#[derive(serde::Serialize)]
struct SummaryMetrics {
    total_cycles: u64,
    total_instructions: u64,
    ipc: f64,
    issue_width: u64,
    window_size: u64,
}

#[derive(serde::Serialize)]
struct TopDownMetricsExport {
    retiring_pct: f64,
    bad_speculation_pct: f64,
    frontend_bound_pct: f64,
    backend_bound_pct: f64,
}

#[derive(serde::Serialize)]
struct FrontendBoundExport {
    fetch_latency_pct: f64,
    fetch_bandwidth_pct: f64,
    icache_miss_rate: f64,
    itlb_miss_rate: f64,
}

#[derive(serde::Serialize)]
struct BackendBoundExport {
    memory_bound_pct: f64,
    core_bound_pct: f64,
    l1_dcache_miss_rate: f64,
    l2_cache_miss_rate: f64,
    l3_cache_miss_rate: f64,
    avg_mem_latency: f64,
}

#[derive(serde::Serialize)]
struct BadSpeculationExport {
    branch_mispred_rate: f64,
    wasted_instructions_pct: f64,
}

#[derive(serde::Serialize)]
struct RetiringExport {
    alu_ops_pct: f64,
    memory_ops_pct: f64,
    branch_ops_pct: f64,
    simd_ops_pct: f64,
}

#[derive(serde::Serialize)]
struct StageUtilizationExport {
    fetch_util: f64,
    decode_util: f64,
    rename_util: f64,
    dispatch_util: f64,
    issue_util: f64,
    execute_util: f64,
    memory_util: f64,
    commit_util: f64,
}

#[derive(serde::Serialize, Clone)]
struct HotspotExport {
    name: String,
    start_pc: u64,
    end_pc: u64,
    instruction_count: u64,
    cycle_count: u64,
    cycle_pct: f64,
    ipc: f64,
}

#[derive(serde::Serialize)]
struct CycleDistributionExport {
    full_issue_cycles: u64,
    partial_issue_cycles: u64,
    stall_cycles: u64,
    memory_stall_cycles: u64,
    dependency_stall_cycles: u64,
}

/// Generate TopDown analysis report from pipeline tracker
#[cfg(feature = "visualization")]
fn generate_topdown_report(
    tracker: &arm_cpu_emulator::visualization::PipelineTracker,
    stats: &arm_cpu_emulator::PerformanceMetrics,
    config: &CPUConfig,
) -> TopDownReportExport {
    let timings = tracker.get_all_timings();
    let disasm_map = tracker.get_all_disasm();

    let total_cycles = stats.total_cycles;
    let total_instructions = stats.total_instructions;

    // Analyze instruction types and timing
    let mut alu_count = 0u64;
    let mut memory_count = 0u64;
    let mut branch_count = 0u64;
    let mut simd_count = 0u64;

    // PC-based hotspot analysis
    let mut pc_histogram: std::collections::HashMap<u64, (u64, u64)> = std::collections::HashMap::new();

    // Stage utilization tracking
    let mut fetch_cycles = 0u64;
    let mut decode_cycles = 0u64;
    let mut rename_cycles = 0u64;
    let mut dispatch_cycles = 0u64;
    let mut issue_cycles = 0u64;
    let mut execute_cycles = 0u64;
    let mut memory_cycles = 0u64;
    let mut commit_cycles = 0u64;

    // Issue width analysis
    let mut full_issue_cycles = 0u64;
    let mut partial_issue_cycles = 0u64;
    let mut stall_cycles = 0u64;

    for (instr_id, timing) in timings.iter() {
        // Estimate instruction type from disassembly
        let disasm = disasm_map.get(instr_id).cloned().unwrap_or_default();
        let disasm_upper = disasm.to_uppercase();

        if disasm_upper.contains("LDR") || disasm_upper.contains("STR") ||
           disasm_upper.contains("LD") || disasm_upper.contains("ST") {
            memory_count += 1;
        } else if disasm_upper.contains("B") || disasm_upper.contains("CBZ") ||
                  disasm_upper.contains("CBNZ") || disasm_upper.contains("BL") {
            branch_count += 1;
        } else if disasm_upper.contains("V") || disasm_upper.contains("SIMD") {
            simd_count += 1;
        } else {
            alu_count += 1;
        }

        // Calculate instruction latency
        let fetch = timing.fetch_start.unwrap_or(0);
        let retire = timing.retire_cycle.unwrap_or(fetch);
        let latency = if retire > fetch { retire - fetch } else { 1 };

        // Update PC histogram (use instruction ID as proxy for PC)
        let pc = 0x1000 + instr_id.0 * 4;
        let entry = pc_histogram.entry(pc).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += latency;

        // Accumulate stage durations
        if let Some(start) = timing.fetch_start {
            if let Some(end) = timing.fetch_end {
                fetch_cycles += end.saturating_sub(start).max(1);
            }
        }
        if let Some(start) = timing.decode_start {
            if let Some(end) = timing.decode_end {
                decode_cycles += end.saturating_sub(start).max(1);
            }
        }
        if let Some(start) = timing.rename_start {
            if let Some(end) = timing.rename_end {
                rename_cycles += end.saturating_sub(start).max(1);
            }
        }
        if let Some(start) = timing.dispatch_start {
            if let Some(end) = timing.dispatch_end {
                dispatch_cycles += end.saturating_sub(start).max(1);
            }
        }
        if let Some(start) = timing.issue_start {
            if let Some(end) = timing.issue_end {
                issue_cycles += end.saturating_sub(start).max(1);
            }
        }
        if let Some(start) = timing.execute_start {
            if let Some(end) = timing.execute_end {
                execute_cycles += end.saturating_sub(start).max(1);
            }
        }
        if let Some(start) = timing.memory_start {
            if let Some(end) = timing.memory_end {
                memory_cycles += end.saturating_sub(start).max(1);
            }
        }
        if let Some(complete) = timing.complete_cycle {
            if let Some(end) = timing.execute_end.or(timing.memory_end) {
                if complete > end {
                    commit_cycles += complete.saturating_sub(end);
                }
            }
        }
    }

    // Calculate issue distribution (simplified estimation)
    let issue_width = config.issue_width as u64;
    let total_issue_slots = total_cycles * issue_width;
    let used_issue_slots = total_instructions;

    if total_cycles > 0 {
        let avg_issue_per_cycle = total_instructions as f64 / total_cycles as f64;
        full_issue_cycles = (total_cycles as f64 * (avg_issue_per_cycle / issue_width as f64).min(1.0)) as u64;
        partial_issue_cycles = (total_cycles as f64 * 0.3) as u64; // Estimate
        stall_cycles = total_cycles.saturating_sub(full_issue_cycles).saturating_sub(partial_issue_cycles);
    }

    // Calculate TopDown Level 1 metrics
    let total_instr_f64 = total_instructions as f64;
    let total_cycles_f64 = total_cycles.max(1) as f64;

    // Retiring: fraction of pipeline capacity doing useful work
    let retiring_pct = (total_instr_f64 / (total_cycles_f64 * issue_width as f64)).min(1.0) * 100.0;

    // Estimate backend bound from memory operations
    let memory_ratio = if total_instructions > 0 { memory_count as f64 / total_instr_f64 } else { 0.0 };
    let backend_bound_pct = (memory_ratio * 40.0).min(100.0 - retiring_pct);

    // Frontend bound estimate
    let frontend_bound_pct = (stall_cycles as f64 / total_cycles_f64 * 20.0).min(100.0 - retiring_pct - backend_bound_pct);

    // Bad speculation estimate (simplified)
    let bad_speculation_pct = (100.0 - retiring_pct - frontend_bound_pct - backend_bound_pct).max(0.0);

    // Stage utilization
    let stage_util = StageUtilizationExport {
        fetch_util: if total_cycles > 0 { fetch_cycles as f64 / total_instr_f64 * 100.0 / issue_width as f64 } else { 0.0 },
        decode_util: if total_cycles > 0 { decode_cycles as f64 / total_instr_f64 * 100.0 / issue_width as f64 } else { 0.0 },
        rename_util: if total_cycles > 0 { rename_cycles as f64 / total_instr_f64 * 100.0 / issue_width as f64 } else { 0.0 },
        dispatch_util: if total_cycles > 0 { dispatch_cycles as f64 / total_instr_f64 * 100.0 / issue_width as f64 } else { 0.0 },
        issue_util: if total_cycles > 0 { issue_cycles as f64 / total_instr_f64 * 100.0 / issue_width as f64 } else { 0.0 },
        execute_util: if total_cycles > 0 { execute_cycles as f64 / total_instr_f64 * 100.0 / issue_width as f64 } else { 0.0 },
        memory_util: if total_cycles > 0 { memory_cycles as f64 / total_instr_f64 * 100.0 / issue_width as f64 } else { 0.0 },
        commit_util: if total_cycles > 0 { commit_cycles as f64 / total_instr_f64 * 100.0 / issue_width as f64 } else { 0.0 },
    };

    // Generate hotspots (top 20 by cycle count)
    let mut hotspots: Vec<HotspotExport> = pc_histogram
        .iter()
        .map(|(pc, (count, cycles))| {
            HotspotExport {
                name: format!("PC_{:08X}", pc),
                start_pc: *pc,
                end_pc: pc + 4,
                instruction_count: *count,
                cycle_count: *cycles,
                cycle_pct: (*cycles as f64 / total_cycles_f64) * 100.0,
                ipc: if *cycles > 0 { *count as f64 / *cycles as f64 } else { 0.0 },
            }
        })
        .collect();
    hotspots.sort_by(|a, b| b.cycle_count.cmp(&a.cycle_count));
    hotspots.truncate(20);

    TopDownReportExport {
        version: "1.0".to_string(),
        summary: SummaryMetrics {
            total_cycles,
            total_instructions,
            ipc: stats.ipc,
            issue_width: config.issue_width as u64,
            window_size: config.window_size as u64,
        },
        topdown: TopDownMetricsExport {
            retiring_pct,
            bad_speculation_pct,
            frontend_bound_pct,
            backend_bound_pct,
        },
        frontend_bound: FrontendBoundExport {
            fetch_latency_pct: frontend_bound_pct * 0.6,
            fetch_bandwidth_pct: frontend_bound_pct * 0.4,
            icache_miss_rate: (1.0 - stats.l1_hit_rate) * 100.0,
            itlb_miss_rate: 0.0,
        },
        backend_bound: BackendBoundExport {
            memory_bound_pct: backend_bound_pct * 0.7,
            core_bound_pct: backend_bound_pct * 0.3,
            l1_dcache_miss_rate: (1.0 - stats.l1_hit_rate) * 100.0,
            l2_cache_miss_rate: (1.0 - stats.l2_hit_rate) * 100.0,
            l3_cache_miss_rate: 0.0,
            avg_mem_latency: stats.avg_load_latency,
        },
        bad_speculation: BadSpeculationExport {
            branch_mispred_rate: 0.0, // Not tracked in this simulation
            wasted_instructions_pct: bad_speculation_pct * 0.5,
        },
        retiring: RetiringExport {
            alu_ops_pct: if total_instructions > 0 { alu_count as f64 / total_instr_f64 * 100.0 } else { 0.0 },
            memory_ops_pct: if total_instructions > 0 { memory_count as f64 / total_instr_f64 * 100.0 } else { 0.0 },
            branch_ops_pct: if total_instructions > 0 { branch_count as f64 / total_instr_f64 * 100.0 } else { 0.0 },
            simd_ops_pct: if total_instructions > 0 { simd_count as f64 / total_instr_f64 * 100.0 } else { 0.0 },
        },
        stage_utilization: stage_util,
        hotspots,
        cycle_distribution: CycleDistributionExport {
            full_issue_cycles,
            partial_issue_cycles,
            stall_cycles,
            memory_stall_cycles: (stall_cycles as f64 * 0.4) as u64,
            dependency_stall_cycles: (stall_cycles as f64 * 0.3) as u64,
        },
    }
}

/// Generate HTML visualization for TopDown report
fn generate_html_report(report: &TopDownReportExport, konata_path: &str, topdown_json_path: &str) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="zh-CN">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>TopDown Performance Analysis Report</title>
    <script src="https://cdn.jsdelivr.net/npm/chart.js"></script>
    <style>
        * {{
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }}
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, sans-serif;
            background: linear-gradient(135deg, #1a1a2e 0%, #16213e 100%);
            color: #eee;
            min-height: 100vh;
            padding: 20px;
        }}
        .container {{
            max-width: 1400px;
            margin: 0 auto;
        }}
        h1 {{
            text-align: center;
            margin-bottom: 30px;
            font-size: 2.5em;
            background: linear-gradient(90deg, #00d2ff, #3a7bd5);
            -webkit-background-clip: text;
            -webkit-text-fill-color: transparent;
            background-clip: text;
        }}
        h2 {{
            margin-bottom: 15px;
            color: #00d2ff;
            font-size: 1.5em;
            border-bottom: 2px solid #3a7bd5;
            padding-bottom: 10px;
        }}
        .card {{
            background: rgba(255, 255, 255, 0.05);
            border-radius: 15px;
            padding: 25px;
            margin-bottom: 20px;
            backdrop-filter: blur(10px);
            border: 1px solid rgba(255, 255, 255, 0.1);
        }}
        .summary-grid {{
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin-bottom: 30px;
        }}
        .metric-card {{
            background: rgba(58, 123, 213, 0.2);
            border-radius: 10px;
            padding: 20px;
            text-align: center;
        }}
        .metric-value {{
            font-size: 2.5em;
            font-weight: bold;
            color: #00d2ff;
        }}
        .metric-label {{
            font-size: 0.9em;
            color: #aaa;
            margin-top: 5px;
        }}
        .topdown-container {{
            display: grid;
            grid-template-columns: 1fr 1fr;
            gap: 20px;
        }}
        @media (max-width: 900px) {{
            .topdown-container {{
                grid-template-columns: 1fr;
            }}
        }}
        .chart-container {{
            position: relative;
            height: 300px;
        }}
        .hotspot-table {{
            width: 100%;
            border-collapse: collapse;
            margin-top: 15px;
        }}
        .hotspot-table th,
        .hotspot-table td {{
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid rgba(255, 255, 255, 0.1);
        }}
        .hotspot-table th {{
            background: rgba(0, 210, 255, 0.2);
            color: #00d2ff;
        }}
        .hotspot-table tr:hover {{
            background: rgba(255, 255, 255, 0.05);
        }}
        .progress-bar {{
            height: 20px;
            background: rgba(255, 255, 255, 0.1);
            border-radius: 10px;
            overflow: hidden;
            margin-top: 10px;
        }}
        .progress-fill {{
            height: 100%;
            border-radius: 10px;
            transition: width 0.5s ease;
        }}
        .progress-retiring {{ background: linear-gradient(90deg, #4CAF50, #8BC34A); }}
        .progress-frontend {{ background: linear-gradient(90deg, #2196F3, #03A9F4); }}
        .progress-backend {{ background: linear-gradient(90deg, #FF9800, #FFC107); }}
        .progress-speculation {{ background: linear-gradient(90deg, #F44336, #E91E63); }}
        .legend {{
            display: flex;
            flex-wrap: wrap;
            gap: 15px;
            margin-top: 15px;
        }}
        .legend-item {{
            display: flex;
            align-items: center;
            gap: 8px;
        }}
        .legend-color {{
            width: 20px;
            height: 20px;
            border-radius: 4px;
        }}
        .stage-grid {{
            display: grid;
            grid-template-columns: repeat(4, 1fr);
            gap: 15px;
            margin-top: 20px;
        }}
        .stage-item {{
            background: rgba(255, 255, 255, 0.05);
            padding: 15px;
            border-radius: 8px;
            text-align: center;
        }}
        .stage-name {{
            font-size: 0.9em;
            color: #aaa;
        }}
        .stage-value {{
            font-size: 1.5em;
            font-weight: bold;
            color: #00d2ff;
            margin-top: 5px;
        }}
        .links {{
            margin-top: 20px;
            text-align: center;
        }}
        .links a {{
            color: #00d2ff;
            text-decoration: none;
            margin: 0 15px;
            padding: 10px 20px;
            border: 1px solid #00d2ff;
            border-radius: 5px;
            transition: all 0.3s ease;
        }}
        .links a:hover {{
            background: #00d2ff;
            color: #1a1a2e;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>TopDown Performance Analysis</h1>

        <!-- Summary Metrics -->
        <div class="summary-grid">
            <div class="metric-card">
                <div class="metric-value">{:.2}</div>
                <div class="metric-label">IPC (Instructions Per Cycle)</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Total Cycles</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Total Instructions</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Issue Width</div>
            </div>
        </div>

        <!-- TopDown Level 1 -->
        <div class="card">
            <h2>TopDown Level 1 Analysis</h2>
            <div class="topdown-container">
                <div>
                    <div class="chart-container">
                        <canvas id="topdownChart"></canvas>
                    </div>
                </div>
                <div>
                    <div style="margin-bottom: 15px;">
                        <strong>Retiring: {:.1}%</strong>
                        <div class="progress-bar">
                            <div class="progress-fill progress-retiring" style="width: {:.1}%"></div>
                        </div>
                        <small style="color: #aaa;">Useful work completing successfully</small>
                    </div>
                    <div style="margin-bottom: 15px;">
                        <strong>Frontend Bound: {:.1}%</strong>
                        <div class="progress-bar">
                            <div class="progress-fill progress-frontend" style="width: {:.1}%"></div>
                        </div>
                        <small style="color: #aaa;">Fetch/decode bottlenecks</small>
                    </div>
                    <div style="margin-bottom: 15px;">
                        <strong>Backend Bound: {:.1}%</strong>
                        <div class="progress-bar">
                            <div class="progress-fill progress-backend" style="width: {:.1}%"></div>
                        </div>
                        <small style="color: #aaa;">Execution/memory bottlenecks</small>
                    </div>
                    <div>
                        <strong>Bad Speculation: {:.1}%</strong>
                        <div class="progress-bar">
                            <div class="progress-fill progress-speculation" style="width: {:.1}%"></div>
                        </div>
                        <small style="color: #aaa;">Wasted cycles from branch mispredictions</small>
                    </div>
                </div>
            </div>
        </div>

        <!-- Instruction Mix -->
        <div class="card">
            <h2>Instruction Mix</h2>
            <div class="topdown-container">
                <div class="chart-container">
                    <canvas id="instrMixChart"></canvas>
                </div>
                <div>
                    <table class="hotspot-table">
                        <tr><th>Instruction Type</th><th>Percentage</th></tr>
                        <tr><td>ALU Operations</td><td>{:.1}%</td></tr>
                        <tr><td>Memory Operations</td><td>{:.1}%</td></tr>
                        <tr><td>Branch Operations</td><td>{:.1}%</td></tr>
                        <tr><td>SIMD Operations</td><td>{:.1}%</td></tr>
                    </table>
                </div>
            </div>
        </div>

        <!-- Stage Utilization -->
        <div class="card">
            <h2>Pipeline Stage Utilization</h2>
            <div class="chart-container" style="height: 250px;">
                <canvas id="stageChart"></canvas>
            </div>
            <div class="stage-grid">
                <div class="stage-item">
                    <div class="stage-name">Fetch</div>
                    <div class="stage-value">{:.1}%</div>
                </div>
                <div class="stage-item">
                    <div class="stage-name">Decode</div>
                    <div class="stage-value">{:.1}%</div>
                </div>
                <div class="stage-item">
                    <div class="stage-name">Rename</div>
                    <div class="stage-value">{:.1}%</div>
                </div>
                <div class="stage-item">
                    <div class="stage-name">Dispatch</div>
                    <div class="stage-value">{:.1}%</div>
                </div>
                <div class="stage-item">
                    <div class="stage-name">Issue</div>
                    <div class="stage-value">{:.1}%</div>
                </div>
                <div class="stage-item">
                    <div class="stage-name">Execute</div>
                    <div class="stage-value">{:.1}%</div>
                </div>
                <div class="stage-item">
                    <div class="stage-name">Memory</div>
                    <div class="stage-value">{:.1}%</div>
                </div>
                <div class="stage-item">
                    <div class="stage-name">Commit</div>
                    <div class="stage-value">{:.1}%</div>
                </div>
            </div>
        </div>

        <!-- Cycle Distribution -->
        <div class="card">
            <h2>Cycle Distribution</h2>
            <div class="topdown-container">
                <div class="chart-container">
                    <canvas id="cycleChart"></canvas>
                </div>
                <div>
                    <table class="hotspot-table">
                        <tr><th>Cycle Type</th><th>Count</th><th>Percentage</th></tr>
                        <tr><td>Full Issue</td><td>{}</td><td>{:.1}%</td></tr>
                        <tr><td>Partial Issue</td><td>{}</td><td>{:.1}%</td></tr>
                        <tr><td>Stall Cycles</td><td>{}</td><td>{:.1}%</td></tr>
                        <tr><td>Memory Stalls</td><td>{}</td><td>{:.1}%</td></tr>
                        <tr><td>Dependency Stalls</td><td>{}</td><td>{:.1}%</td></tr>
                    </table>
                </div>
            </div>
        </div>

        <!-- Hotspots -->
        <div class="card">
            <h2>Top 20 Hotspots</h2>
            <table class="hotspot-table">
                <tr>
                    <th>PC Range</th>
                    <th>Instructions</th>
                    <th>Cycles</th>
                    <th>Cycle %</th>
                    <th>Local IPC</th>
                </tr>
                {}
            </table>
        </div>

        <!-- Links -->
        <div class="links">
            <a href="{}">Konata Pipeline View</a>
            <a href="{}">TopDown JSON Data</a>
        </div>
    </div>

    <script>
        // TopDown Chart
        new Chart(document.getElementById('topdownChart'), {{
            type: 'doughnut',
            data: {{
                labels: ['Retiring', 'Frontend Bound', 'Backend Bound', 'Bad Speculation'],
                datasets: [{{
                    data: [{:.1}, {:.1}, {:.1}, {:.1}],
                    backgroundColor: [
                        'rgba(76, 175, 80, 0.8)',
                        'rgba(33, 150, 243, 0.8)',
                        'rgba(255, 152, 0, 0.8)',
                        'rgba(244, 67, 54, 0.8)'
                    ],
                    borderColor: [
                        'rgba(76, 175, 80, 1)',
                        'rgba(33, 150, 243, 1)',
                        'rgba(255, 152, 0, 1)',
                        'rgba(244, 67, 54, 1)'
                    ],
                    borderWidth: 2
                }}]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                plugins: {{
                    legend: {{
                        position: 'bottom',
                        labels: {{ color: '#eee' }}
                    }}
                }}
            }}
        }});

        // Instruction Mix Chart
        new Chart(document.getElementById('instrMixChart'), {{
            type: 'pie',
            data: {{
                labels: ['ALU', 'Memory', 'Branch', 'SIMD'],
                datasets: [{{
                    data: [{:.1}, {:.1}, {:.1}, {:.1}],
                    backgroundColor: [
                        'rgba(0, 210, 255, 0.8)',
                        'rgba(255, 193, 7, 0.8)',
                        'rgba(156, 39, 176, 0.8)',
                        'rgba(0, 150, 136, 0.8)'
                    ],
                    borderWidth: 2
                }}]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                plugins: {{
                    legend: {{
                        position: 'bottom',
                        labels: {{ color: '#eee' }}
                    }}
                }}
            }}
        }});

        // Stage Utilization Chart
        new Chart(document.getElementById('stageChart'), {{
            type: 'bar',
            data: {{
                labels: ['Fetch', 'Decode', 'Rename', 'Dispatch', 'Issue', 'Execute', 'Memory', 'Commit'],
                datasets: [{{
                    label: 'Utilization %',
                    data: [{:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}, {:.1}],
                    backgroundColor: 'rgba(0, 210, 255, 0.6)',
                    borderColor: 'rgba(0, 210, 255, 1)',
                    borderWidth: 1
                }}]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                scales: {{
                    y: {{
                        beginAtZero: true,
                        ticks: {{ color: '#aaa' }},
                        grid: {{ color: 'rgba(255,255,255,0.1)' }}
                    }},
                    x: {{
                        ticks: {{ color: '#aaa' }},
                        grid: {{ color: 'rgba(255,255,255,0.1)' }}
                    }}
                }},
                plugins: {{
                    legend: {{ display: false }}
                }}
            }}
        }});

        // Cycle Distribution Chart
        new Chart(document.getElementById('cycleChart'), {{
            type: 'bar',
            data: {{
                labels: ['Full Issue', 'Partial Issue', 'Stall', 'Memory Stall', 'Dep Stall'],
                datasets: [{{
                    label: 'Cycles',
                    data: [{}, {}, {}, {}, {}],
                    backgroundColor: [
                        'rgba(76, 175, 80, 0.8)',
                        'rgba(33, 150, 243, 0.8)',
                        'rgba(244, 67, 54, 0.8)',
                        'rgba(255, 152, 0, 0.8)',
                        'rgba(156, 39, 176, 0.8)'
                    ],
                    borderWidth: 1
                }}]
            }},
            options: {{
                responsive: true,
                maintainAspectRatio: false,
                scales: {{
                    y: {{
                        beginAtZero: true,
                        ticks: {{ color: '#aaa' }},
                        grid: {{ color: 'rgba(255,255,255,0.1)' }}
                    }},
                    x: {{
                        ticks: {{ color: '#aaa' }},
                        grid: {{ color: 'rgba(255,255,255,0.1)' }}
                    }}
                }},
                plugins: {{
                    legend: {{ display: false }}
                }}
            }}
        }});
    </script>
</body>
</html>"#,
        report.summary.ipc,
        report.summary.total_cycles,
        report.summary.total_instructions,
        report.summary.issue_width,
        report.topdown.retiring_pct,
        report.topdown.retiring_pct,
        report.topdown.frontend_bound_pct,
        report.topdown.frontend_bound_pct,
        report.topdown.backend_bound_pct,
        report.topdown.backend_bound_pct,
        report.topdown.bad_speculation_pct,
        report.topdown.bad_speculation_pct,
        report.retiring.alu_ops_pct,
        report.retiring.memory_ops_pct,
        report.retiring.branch_ops_pct,
        report.retiring.simd_ops_pct,
        report.stage_utilization.fetch_util,
        report.stage_utilization.decode_util,
        report.stage_utilization.rename_util,
        report.stage_utilization.dispatch_util,
        report.stage_utilization.issue_util,
        report.stage_utilization.execute_util,
        report.stage_utilization.memory_util,
        report.stage_utilization.commit_util,
        report.cycle_distribution.full_issue_cycles,
        report.cycle_distribution.full_issue_cycles as f64 / report.summary.total_cycles as f64 * 100.0,
        report.cycle_distribution.partial_issue_cycles,
        report.cycle_distribution.partial_issue_cycles as f64 / report.summary.total_cycles as f64 * 100.0,
        report.cycle_distribution.stall_cycles,
        report.cycle_distribution.stall_cycles as f64 / report.summary.total_cycles as f64 * 100.0,
        report.cycle_distribution.memory_stall_cycles,
        report.cycle_distribution.memory_stall_cycles as f64 / report.summary.total_cycles as f64 * 100.0,
        report.cycle_distribution.dependency_stall_cycles,
        report.cycle_distribution.dependency_stall_cycles as f64 / report.summary.total_cycles as f64 * 100.0,
        report.hotspots.iter().map(|h| format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{:.2}%</td><td>{:.2}</td></tr>",
            h.name, h.instruction_count, h.cycle_count, h.cycle_pct, h.ipc
        )).collect::<Vec<_>>().join("\n"),
        // Use relative paths - extract just the filename
        "index_static.html",
        "konata_data_topdown.json",
        report.topdown.retiring_pct,
        report.topdown.frontend_bound_pct,
        report.topdown.backend_bound_pct,
        report.topdown.bad_speculation_pct,
        report.retiring.alu_ops_pct,
        report.retiring.memory_ops_pct,
        report.retiring.branch_ops_pct,
        report.retiring.simd_ops_pct,
        report.stage_utilization.fetch_util,
        report.stage_utilization.decode_util,
        report.stage_utilization.rename_util,
        report.stage_utilization.dispatch_util,
        report.stage_utilization.issue_util,
        report.stage_utilization.execute_util,
        report.stage_utilization.memory_util,
        report.stage_utilization.commit_util,
        report.cycle_distribution.full_issue_cycles,
        report.cycle_distribution.partial_issue_cycles,
        report.cycle_distribution.stall_cycles,
        report.cycle_distribution.memory_stall_cycles,
        report.cycle_distribution.dependency_stall_cycles,
    )
}
