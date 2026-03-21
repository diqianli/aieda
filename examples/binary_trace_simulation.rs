//! Full simulation example with binary trace format
//!
//! This demonstrates:
//! 1. Running a simulation with 1M+ instructions
//! 2. Saving results to binary trace format
//! 3. Loading and analyzing the binary trace
//! 4. Performance comparison

use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::time::Instant;

use arm_cpu_emulator::{
    config::CPUConfig,
    input::TraceInput,
    ooo::{BatchSimulator, InstructionBatch, ParallelConfig},
    trace::{BinaryTraceReader, BinaryTraceWriter},
    types::{Instruction, InstructionId, OpcodeType, Reg},
    CPUEmulator,
};

/// Generate synthetic instructions for testing
fn generate_instructions(count: usize) -> Vec<Instruction> {
    let mut instructions = Vec::with_capacity(count);

    for i in 0..count {
        let pc = 0x1000 + (i as u64 * 4);

        // Vary instruction types
        let opcode = match i % 10 {
            0 | 1 | 2 => OpcodeType::Add,
            3 | 4 => OpcodeType::Sub,
            5 | 6 => OpcodeType::Mul,
            7 => OpcodeType::Load,
            8 => OpcodeType::Store,
            9 => OpcodeType::BranchCond,
            _ => OpcodeType::Nop,
        };

        let mut instr = Instruction::new(InstructionId(i as u64), pc, 0, opcode);

        // Add register dependencies
        let src_reg = Reg((i % 28) as u8);
        let dst_reg = Reg(((i + 1) % 28) as u8);

        instr = instr
            .with_src_reg(src_reg)
            .with_dst_reg(dst_reg);

        // Add memory access for load/store
        if opcode == OpcodeType::Load {
            instr = instr.with_mem_access(0x2000 + (i as u64 * 8), 8, true);
        } else if opcode == OpcodeType::Store {
            instr = instr.with_mem_access(0x2000 + (i as u64 * 8), 8, false);
        }

        // Add branch info
        if opcode == OpcodeType::BranchCond {
            instr = instr.with_branch(pc + 100, true, i % 3 == 0);
        }

        instructions.push(instr);
    }

    instructions
}

fn main() {
    println!("==========================================================");
    println!("       Full CPU Simulation with Binary Trace Format        ");
    println!("==========================================================");

    let instruction_count = 100_000; // 100K instructions for faster demo

    // === Phase 1: Generate Instructions ===
    println!("\n=== Phase 1: Generating {} Instructions ===", instruction_count);
    let start_gen = Instant::now();
    let instructions = generate_instructions(instruction_count);
    let gen_time = start_gen.elapsed();
    println!("Generated in: {:.2?}", gen_time);

    println!("  Sample instructions:");
    for i in 0..5 {
        let instr = &instructions[i];
        if instr.opcode_type.is_memory_op() {
            println!("    {:#010X}: {:?} (mem)", instr.pc, instr.opcode_type);
        } else if instr.opcode_type.is_branch() {
            println!("    {:#010X}: {:?} (branch)", instr.pc, instr.opcode_type);
        } else {
            println!("    {:#010X}: {:?}", instr.pc, instr.opcode_type);
        }
    }

    // === Phase 2: Run Standard Simulation ===
    println!("\n=== Phase 2: Standard Simulation ===");
    let config = CPUConfig {
        window_size: 256,
        issue_width: 6,
        commit_width: 6,
        fetch_width: 4,
        ..Default::default()
    };

    // Clone instructions for later verification
    let instructions_for_verify: Vec<Instruction> = instructions.iter()
        .take(100)
        .map(|i| Instruction {
            id: i.id,
            pc: i.pc,
            raw_opcode: i.raw_opcode,
            opcode_type: i.opcode_type,
            src_regs: i.src_regs.clone(),
            dst_regs: i.dst_regs.clone(),
            src_vregs: i.src_vregs.clone(),
            dst_vregs: i.dst_vregs.clone(),
            mem_access: i.mem_access.clone(),
            branch_info: i.branch_info.clone(),
            disasm: i.disasm.clone(),
        })
        .collect();

    let mut input = TraceInput::from_vec(instructions.clone());

    let start_sim = Instant::now();
    let mut cpu = CPUEmulator::new(config.clone()).unwrap();
    let metrics = cpu.run(&mut input).unwrap();
    let sim_time = start_sim.elapsed();

    println!("Simulation completed in: {:.2?}", sim_time);
    println!("  Total instructions: {}", metrics.total_instructions);
    println!("  Total cycles: {}", metrics.total_cycles);
    println!("  IPC: {:.3}", metrics.ipc);

    // === Phase 3: Save to Binary Trace Format ===
    println!("\n=== Phase 3: Saving to Binary Trace Format ===");
    let trace_path = PathBuf::from("/tmp/simulation_trace.bin");

    let start_write = Instant::now();
    {
        let file = File::create(&trace_path).unwrap();
        let mut writer = BinaryTraceWriter::new(BufWriter::new(file)).unwrap();

        for instr in &instructions {
            writer.write_instruction(instr).unwrap();
        }
        writer.finish().unwrap();
    }
    let write_time = start_write.elapsed();
    println!("Binary trace saved in: {:.2?}", write_time);

    // Check file size
    let file_metadata = std::fs::metadata(&trace_path).unwrap();
    let json_size = instruction_count * 200; // Rough estimate for JSON
    println!("  Binary file size: {} bytes ({:.2} MB)", file_metadata.len(), file_metadata.len() as f64 / 1024.0 / 1024.0);
    println!("  Estimated JSON size: {} bytes ({:.2} MB)", json_size, json_size as f64 / 1024.0 / 1024.0);
    println!("  Compression ratio: {:.1}x", json_size as f64 / file_metadata.len() as f64);

    // === Phase 4: Run Batch Simulation ===
    println!("\n=== Phase 4: Batch Simulation (Parallel) ===");
    let batch_size = 10_000;
    let parallel_config = ParallelConfig {
        num_workers: 4,
        batch_size,
        parallel_deps: true,
        stats_interval: 100_000,
    };

    let mut batch_sim = BatchSimulator::new(config.clone())
        .with_parallel_config(parallel_config);

    let start_batch = Instant::now();

    // Create batches from instructions
    let batches: Vec<InstructionBatch> = instructions
        .chunks(batch_size)
        .enumerate()
        .map(|(i, chunk)| InstructionBatch {
            instructions: chunk.to_vec(),
            batch_id: i as u64,
            start_id: (i * batch_size) as u64,
        })
        .collect();
    println!("Created {} batches", batches.len());

    let batch_results = batch_sim.process_batches(&batches);
    let batch_time = start_batch.elapsed();

    println!("Batch simulation completed in: {:.2?}", batch_time);

    // Aggregate results
    let total_batch_instr: u64 = batch_results.iter().map(|r| r.instr_count as u64).sum();
    let total_batch_cycles: u64 = batch_results.iter().map(|r| r.cycles as u64).sum();
    let avg_batch_ipc = if total_batch_cycles > 0 {
        total_batch_instr as f64 / total_batch_cycles as f64
    } else {
        0.0
    };

    println!("  Total instructions: {}", total_batch_instr);
    println!("  Total cycles: {}", total_batch_cycles);
    println!("  Average IPC: {:.3}", avg_batch_ipc);
    if batch_time.as_millis() > 0 {
        println!("  Throughput: {:.2} instructions/ms",
            (total_batch_instr as f64 * 1000.0) / batch_time.as_millis() as f64);
    }

    // === Phase 5: Load and Verify Binary Trace ===
    println!("\n=== Phase 5: Loading and Verifying Binary Trace ===");
    let start_load = Instant::now();
    let reader = BinaryTraceReader::open(&trace_path).unwrap();
    println!("Loaded binary trace:");
    println!("  Header: {} instructions", reader.instr_count());
    println!("  Has index: {}", reader.has_index());

    // Read some instructions
    let mut loaded_count = 0;
    let mut loaded_instructions = Vec::new();
    for result in reader.stream().take(100) {
        match result {
            Ok(instr) => {
                loaded_instructions.push(instr);
                loaded_count += 1;
            }
            Err(e) => {
                println!("Error reading instruction: {}", e);
                break;
            }
        }
    }
    let load_time = start_load.elapsed();
    println!("Loaded {} instructions in: {:.2?}", loaded_count, load_time);

    // Verify instructions
    let mut all_match = true;
    for (i, loaded) in loaded_instructions.iter().enumerate() {
        if loaded.id.0 != instructions_for_verify[i].id.0 {
            all_match = false;
            println!("  Mismatch at index {}: expected {}, got {}",
                i, instructions_for_verify[i].id.0, loaded.id.0);
        }
    }
    if all_match {
        println!("  [OK] All loaded instructions match original");
    } else {
        println!("  [FAIL] Some instructions don't match");
    }

    // === Phase 6: Statistics Summary ===
    println!("\n=== Phase 6: Statistics Summary ===");
    println!("\n+----------------------------------------------+");
    println!("|           Performance Comparison             |");
    println!("+----------------------------------------------+");
    println!("| Metric          | Standard    | Batch        |");
    println!("+----------------------------------------------+");
    println!("| Instructions    | {:>10} | {:>10} |",
        metrics.total_instructions, total_batch_instr);
    println!("| Cycles          | {:>10} | {:>10} |",
        metrics.total_cycles, total_batch_cycles);
    println!("| IPC             | {:>10.3} | {:>10.3} |",
        metrics.ipc, avg_batch_ipc);
    println!("| Execution Time  | {:>7} ms | {:>7} ms |",
        sim_time.as_millis(), batch_time.as_millis());
    println!("+----------------------------------------------+");

    // Cleanup
    std::fs::remove_file(&trace_path).ok();

    println!("\n[SUCCESS] All phases completed successfully!");
}
