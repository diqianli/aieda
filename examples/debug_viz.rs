//! Debug viz_server simulation
//!
//! Run with: cargo run --example debug_viz

use arm_cpu_emulator::{CPUConfig, CPUEmulator, OpcodeType, Reg, TraceInput};

fn main() {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let config = CPUConfig {
        window_size: 256,
        issue_width: 6,
        commit_width: 6,
        ..Default::default()
    };

    let mut cpu = CPUEmulator::new(config).unwrap();
    let mut input = TraceInput::new();

    // Create same instructions as viz_server
    for i in 0..100 {
        let pc = 0x1000 + i as u64 * 4;

        match i % 5 {
            0 => {
                input.builder(pc, OpcodeType::Add)
                    .src_reg(Reg((i % 30) as u8))
                    .src_reg(Reg(((i + 1) % 30) as u8))
                    .dst_reg(Reg(((i + 2) % 30) as u8))
                    .disasm(format!("ADD X{}, X{}, X{}", (i + 2) % 30, i % 30, (i + 1) % 30))
                    .build();
            }
            1 => {
                let addr = 0x2000 + (i as u64 * 64);
                input.builder(pc, OpcodeType::Load)
                    .dst_reg(Reg((i % 30) as u8))
                    .mem_access(addr, 8, true)
                    .disasm(format!("LDR X{}, [X{}, #{}]", i % 30, 31, addr))
                    .build();
            }
            2 => {
                input.builder(pc, OpcodeType::Mul)
                    .src_reg(Reg((i % 30) as u8))
                    .src_reg(Reg(((i + 1) % 30) as u8))
                    .dst_reg(Reg(((i + 2) % 30) as u8))
                    .disasm(format!("MUL X{}, X{}, X{}", (i + 2) % 30, i % 30, (i + 1) % 30))
                    .build();
            }
            3 => {
                let addr = 0x3000 + (i as u64 * 64);
                input.builder(pc, OpcodeType::Store)
                    .src_reg(Reg((i % 30) as u8))
                    .mem_access(addr, 8, false)
                    .disasm(format!("STR X{}, [X{}, #{}]", i % 30, 31, addr))
                    .build();
            }
            _ => {
                input.builder(pc, OpcodeType::Sub)
                    .src_reg(Reg((i % 30) as u8))
                    .src_reg(Reg(((i + 1) % 30) as u8))
                    .dst_reg(Reg(((i + 2) % 30) as u8))
                    .disasm(format!("SUB X{}, X{}, X{}", (i + 2) % 30, i % 30, (i + 1) % 30))
                    .build();
            }
        }
    }

    println!("=== Dispatching 100 instructions ===");
    for _ in 0..100 {
        if let Some(Ok(instr)) = input.next() {
            let _ = cpu.dispatch(instr);
        }
    }

    let (waiting, ready, executing, completed) = cpu.ooo_engine().status_counts();
    println!("After dispatch: waiting={}, ready={}, executing={}, completed={}",
        waiting, ready, executing, completed);

    println!("\n=== Running simulation ===");
    let max_cycles = 500;
    let mut prev_committed = 0u64;
    let mut no_progress_count = 0u64;

    // Debug: check window entries
    println!("Window entries after dispatch:");
    for entry in cpu.ooo_engine().get_window_entries().take(5) {
        println!("  Instr {}: status={:?}, complete_cycle={:?}, completion_processed={}",
            entry.instruction.id.0, entry.status, entry.complete_cycle, entry.completion_processed);
    }

    for cycle in 0..max_cycles {
        let (waiting, ready, executing, completed) = cpu.ooo_engine().status_counts();

        if cycle % 50 == 0 || cycle < 10 {
            println!("\n--- Cycle {} ---", cpu.current_cycle());
            println!("Before: waiting={}, ready={}, executing={}, completed={}",
                waiting, ready, executing, completed);

            // Debug: check first 10 instructions by ID
            println!("Instructions 0-9:");
            for i in 0..10 {
                if let Some(entry) = cpu.ooo_engine().get_window_entry(arm_cpu_emulator::InstructionId(i)) {
                    println!("  Instr {}: status={:?}, complete_cycle={:?}, completion_processed={}",
                        i, entry.status, entry.complete_cycle, entry.completion_processed);
                } else {
                    println!("  Instr {}: NOT IN WINDOW (already committed)", i);
                }
            }
        }

        cpu.step();

        let committed = cpu.committed_count();
        let committed_this_cycle = committed - prev_committed;
        prev_committed = committed;

        if cycle % 50 == 0 || cycle < 10 {
            println!("After: committed={} (+{})", committed, committed_this_cycle);
        }

        if committed >= 100 {
            println!("\n✅ All 100 instructions committed at cycle {}", cpu.current_cycle());
            break;
        }

        // Detect deadlock
        if committed_this_cycle == 0 && ready == 0 {
            no_progress_count += 1;
            if no_progress_count > 20 {
                let (w, r, e, c) = cpu.ooo_engine().status_counts();
                println!("\n⚠️ Possible deadlock at cycle {}!", cpu.current_cycle());
                println!("   waiting={}, ready={}, executing={}, completed={}", w, r, e, c);
                println!("   Committed: {}", committed);
                break;
            }
        } else {
            no_progress_count = 0;
        }
    }

    println!("\n=== Final Summary ===");
    println!("Committed: {}", cpu.committed_count());
    println!("Cycles: {}", cpu.current_cycle());
    if cpu.committed_count() > 0 {
        println!("IPC: {:.3}", cpu.committed_count() as f64 / cpu.current_cycle() as f64);
    }
}
