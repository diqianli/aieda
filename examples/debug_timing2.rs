use arm_cpu_emulator::{CPUConfig, CPUEmulator, InstructionId, OpcodeType, Reg, TraceInput};

fn main() {
    let config = CPUConfig {
        window_size: 16,
        issue_width: 8,  // 每周期最多取8条指令
        commit_width: 8,
        ..Default::default()
    };

    let mut cpu = CPUEmulator::new(config).unwrap();

    // 创建依赖链: ADD -> MUL -> SUB
    let mut input = TraceInput::new();

    // 指令 0: ADD X1, X0, X0 (写 X1, 延迟 1)
    input.builder(0x1000, OpcodeType::Add)
        .src_reg(Reg(0)).src_reg(Reg(0)).dst_reg(Reg(1))
        .disasm("ADD X1, X0, X0".to_string()).build();

    // 指令 1: MUL X2, X1, X1 (读 X1, 依赖 0, 延迟 3)
    input.builder(0x1004, OpcodeType::Mul)
        .src_reg(Reg(1)).src_reg(Reg(1)).dst_reg(Reg(2))
        .disasm("MUL X2, X1, X1".to_string()).build();

    // 指令 2: SUB X3, X2, X2 (读 X2, 依赖 1, 延迟 1)
    input.builder(0x1008, OpcodeType::Sub)
        .src_reg(Reg(2)).src_reg(Reg(2)).dst_reg(Reg(3))
        .disasm("SUB X3, X2, X2".to_string()).build();

    // Dispatch 所有指令
    for _ in 0..3 {
        if let Some(Ok(instr)) = input.next() {
            cpu.dispatch(instr).unwrap();
        }
    }

    // 运行直到所有指令 commit
    while cpu.committed_count() < 3 {
        cpu.step();
    }

    // 打印时序
    println!("=== 指令时序 ===");
    let tracker = cpu.pipeline_tracker();

    for i in 0..3 {
        if let Some(timing) = tracker.get_timing(InstructionId(i)) {
            println!("\n指令 {}:", i);
            println!("  Fetch:    {:?} - {:?}", timing.fetch_start, timing.fetch_end);
            println!("  Decode:   {:?} - {:?}", timing.decode_start, timing.decode_end);
            println!("  Rename:   {:?} - {:?}", timing.rename_start, timing.rename_end);
            println!("  Dispatch: {:?} - {:?}", timing.dispatch_start, timing.dispatch_end);
            println!("  Issue:    {:?} - {:?}", timing.issue_start, timing.issue_end);
            println!("  Execute:  {:?} - {:?}", timing.execute_start, timing.execute_end);
            println!("  Complete: {:?}", timing.complete_cycle);
            println!("  Retire:   {:?}", timing.retire_cycle);
            
            println!("  Konata stages:");
            for stage in timing.to_stages() {
                println!("    {} : {} - {}", stage.name, stage.start_cycle, stage.end_cycle);
            }
        }
    }

    // 验证依赖约束
    println!("\n=== 依赖约束验证 ===");
    let t0 = tracker.get_timing(InstructionId(0)).unwrap();
    let t1 = tracker.get_timing(InstructionId(1)).unwrap();
    let t2 = tracker.get_timing(InstructionId(2)).unwrap();

    println!("指令 0 complete: {:?}", t0.complete_cycle);
    println!("指令 1 issue_end: {:?}", t1.issue_end);
    println!("指令 1 complete: {:?}", t1.complete_cycle);
    println!("指令 2 issue_end: {:?}", t2.issue_end);
    println!("指令 2 complete: {:?}", t2.complete_cycle);

    // 检查约束
    let ok1 = t1.issue_end.unwrap_or(0) >= t0.complete_cycle.unwrap_or(0);
    let ok2 = t2.issue_end.unwrap_or(0) >= t1.complete_cycle.unwrap_or(0);
    
    println!("\n约束 1: 指令 1 issue >= 指令 0 complete: {} ({})",
        if ok1 { "✓" } else { "✗" },
        if ok1 { "OK" } else { "FAIL" });
    println!("约束 2: 指令 2 issue >= 指令 1 complete: {} ({})",
        if ok2 { "✓" } else { "✗" },
        if ok2 { "OK" } else { "FAIL" });

    if ok1 && ok2 {
        println!("\n✅ 所有约束满足!");
    } else {
        println!("\n❌ 约束不满足，需要修复!");
    }
}
