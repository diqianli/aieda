use arm_cpu_emulator::{
    CPUConfig, CPUEmulator, ChampSimXzTraceParser, TraceInput,
};

fn main() {
    // Initialize tracing with debug level
    tracing_subscriber::fmt::fmt()
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <trace_file>", args[0]);
        return;
    }

    let trace_path = &args[1];
    let max_instructions = 1000;

    println!("Loading trace: {}", trace_path);

    let parser = match ChampSimXzTraceParser::from_file(trace_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to open trace: {}", e);
            return;
        }
    };

    let config = CPUConfig {
        window_size: 128,
        issue_width: 4,
        commit_width: 4,
        ..CPUConfig::default()
    };

    let mut cpu = CPUEmulator::new(config).unwrap();
    let mut input = TraceInput::with_capacity(max_instructions);

    println!("Parsing {} instructions...", max_instructions);
    for (i, result) in parser.take(max_instructions).enumerate() {
        match result {
            Ok(instr) => {
                // Print more instructions to see instruction 12
                if i < 15 {
                    println!(
                        "  [{}] id={:?} pc=0x{:x} op={:?} srcs={:?} dsts={:?} mem={:?}",
                        i,
                        instr.id,
                        instr.pc,
                        instr.opcode_type,
                        instr.src_regs.as_slice(),
                        instr.dst_regs.as_slice(),
                        instr.mem_access
                    );
                }
                input.push(instr);
            }
            Err(e) => {
                eprintln!("Error at {}: {}", i, e);
                break;
            }
        }
    }

    println!("Input has {} instructions remaining", input.remaining());

    println!("\nRunning simulation...");
    println!("Window size: {}", cpu.ooo_engine().window_size());

    let start = std::time::Instant::now();
    // Use a small cycle limit for debugging
    match cpu.run_with_limit(&mut input, 100000) {
        Ok(metrics) => {
            let elapsed = start.elapsed();
            println!("Completed in {:?}", elapsed);
            println!("Total instructions: {}", metrics.total_instructions);
            println!("Total cycles: {}", metrics.total_cycles);
            println!("IPC: {:.4}", metrics.ipc);
            println!("Committed: {}", cpu.committed_count());
            println!("Input remaining after run: {}", input.remaining());
        }
        Err(e) => {
            eprintln!("Simulation error: {}", e);
        }
    }
}
