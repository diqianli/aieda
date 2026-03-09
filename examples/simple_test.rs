use arm_cpu_emulator::{CPUConfig, CPUEmulator, TraceInput, OpcodeType, Reg};

fn main() {
    let config = CPUConfig {
        window_size: 16,
        issue_width: 4,
        commit_width: 4,
        ..CPUConfig::default()
    };
    let mut cpu = CPUEmulator::new(config).unwrap();
    
    let mut input = TraceInput::with_capacity(100);
    
    // Create 20 simple compute instructions (no memory, no dependencies)
    for i in 0..20 {
        input.builder(0x1000 + i * 4, OpcodeType::Add)
            .dst_reg(Reg((i % 10) as u8))
            .build();
    }
    
    println!("Running 20 compute instructions...");
    let start = std::time::Instant::now();
    
    // Add debug output
    println!("Window size: {}", cpu.ooo_engine().window_size());
    println!("Free slots: {}", cpu.ooo_engine().free_slots());
    
    let metrics = cpu.run(&mut input).unwrap();
    let elapsed = start.elapsed();
    
    println!("Completed in {:?}", elapsed);
    println!("Total instructions: {}", metrics.total_instructions);
    println!("Total cycles: {}", metrics.total_cycles);
    println!("IPC: {:.4}", metrics.ipc);
}
