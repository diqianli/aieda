//! Integration tests for the ARM CPU emulator.

use arm_cpu_emulator::{CPUConfig, CPUEmulator, OpcodeType, Reg, TraceInput};

/// Test basic compute instruction execution
#[test]
fn test_compute_instructions() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    // Create a sequence of independent ADD instructions
    for i in 0u8..4 {
        input.builder(0x1000 + (i as u64) * 4, OpcodeType::Add)
            .src_reg(Reg(0))
            .src_reg(Reg(1))
            .dst_reg(Reg(i + 2))
            .disasm(format!("ADD X{}, X0, X1", i + 2))
            .build();
    }

    let metrics = cpu.run(&mut input).unwrap();

    assert_eq!(metrics.total_instructions, 4);
    assert!(metrics.ipc > 0.0);
}

/// Test dependent instruction execution
#[test]
fn test_dependent_instructions() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    // Create a chain of dependent instructions: ADD -> ADD -> ADD
    input.builder(0x1000, OpcodeType::Add)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(2))
        .build();

    input.builder(0x1004, OpcodeType::Add)
        .src_reg(Reg(2))
        .src_reg(Reg(3))
        .dst_reg(Reg(4))
        .build();

    input.builder(0x1008, OpcodeType::Add)
        .src_reg(Reg(4))
        .src_reg(Reg(5))
        .dst_reg(Reg(6))
        .build();

    let metrics = cpu.run(&mut input).unwrap();

    assert_eq!(metrics.total_instructions, 3);
}

/// Test load instruction execution
#[test]
fn test_load_instructions() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    input.builder(0x1000, OpcodeType::Load)
        .dst_reg(Reg(0))
        .mem_access(0x2000, 8, true)
        .disasm("LDR X0, [X1]")
        .build();

    let metrics = cpu.run(&mut input).unwrap();

    assert_eq!(metrics.total_instructions, 1);
}

/// Test store instruction execution
#[test]
fn test_store_instructions() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    input.builder(0x1000, OpcodeType::Store)
        .src_reg(Reg(0))
        .mem_access(0x2000, 8, false)
        .disasm("STR X0, [X1]")
        .build();

    let metrics = cpu.run(&mut input).unwrap();

    assert_eq!(metrics.total_instructions, 1);
}

/// Test mixed compute and memory instructions
#[test]
fn test_mixed_instructions() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    // Load -> Compute -> Store pattern
    input.builder(0x1000, OpcodeType::Load)
        .dst_reg(Reg(0))
        .mem_access(0x2000, 8, true)
        .build();

    input.builder(0x1004, OpcodeType::Add)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(2))
        .build();

    input.builder(0x1008, OpcodeType::Store)
        .src_reg(Reg(2))
        .mem_access(0x2008, 8, false)
        .build();

    let metrics = cpu.run(&mut input).unwrap();

    assert_eq!(metrics.total_instructions, 3);
    assert!(metrics.memory_instr_pct > 0.0);
}

/// Test branch instruction
#[test]
fn test_branch_instructions() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    input.builder(0x1000, OpcodeType::Branch)
        .branch(0x2000, false, true)
        .disasm("B 0x2000")
        .build();

    let metrics = cpu.run(&mut input).unwrap();

    assert_eq!(metrics.total_instructions, 1);
    assert!(metrics.branch_instr_pct > 0.0);
}

/// Test cache behavior
#[test]
fn test_cache_behavior() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    // Multiple loads to the same cache line should show different behavior
    for i in 0..4 {
        input.builder(0x1000 + i * 4, OpcodeType::Load)
            .dst_reg(Reg(i as u8))
            .mem_access(0x2000 + i * 8, 8, true)
            .build();
    }

    let metrics = cpu.run(&mut input).unwrap();

    assert_eq!(metrics.total_instructions, 4);
}

/// Test high performance configuration
#[test]
fn test_high_performance_config() {
    let config = CPUConfig::high_performance();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::with_capacity(100);

    // Many independent instructions should benefit from wider issue
    for i in 0..20 {
        input.builder(0x1000 + i * 4, OpcodeType::Add)
            .src_reg(Reg((i % 10) as u8))
            .dst_reg(Reg(((i + 10) % 20) as u8))
            .build();
    }

    let metrics = cpu.run(&mut input).unwrap();

    assert_eq!(metrics.total_instructions, 20);
}

/// Test emulator reset
#[test]
fn test_reset() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::Nop).build();

    cpu.run(&mut input).unwrap();
    assert!(cpu.committed_count() > 0);

    cpu.reset();

    assert_eq!(cpu.committed_count(), 0);
    assert_eq!(cpu.current_cycle(), 0);
}

/// Test trace output
#[test]
fn test_trace_output() {
    let mut config = CPUConfig::minimal();
    config.enable_trace_output = true;
    config.max_trace_output = 100;

    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::Add)
        .src_reg(Reg(0))
        .dst_reg(Reg(1))
        .disasm("ADD X1, X0")
        .build();

    cpu.run(&mut input).unwrap();

    let trace = cpu.trace();
    assert!(!trace.is_empty());
}

/// Test statistics collection
#[test]
fn test_statistics() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    // Mix of different instruction types
    input.builder(0x1000, OpcodeType::Add).dst_reg(Reg(0)).build();
    input.builder(0x1004, OpcodeType::Load).mem_access(0x2000, 8, true).dst_reg(Reg(1)).build();
    input.builder(0x1008, OpcodeType::Branch).branch(0x1000, false, true).build();

    cpu.run(&mut input).unwrap();

    let stats = cpu.stats();
    assert_eq!(stats.stats().total_instructions, 3);
}

/// Test IPC calculation
#[test]
fn test_ipc_calculation() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    // 4 independent NOPs should achieve good IPC
    for i in 0..4 {
        input.builder(0x1000 + i * 4, OpcodeType::Nop).build();
    }

    let metrics = cpu.run(&mut input).unwrap();

    assert_eq!(metrics.total_instructions, 4);
    assert!(metrics.ipc > 0.0);
    assert!(metrics.cpi > 0.0);
}

/// Test CPU emulator with empty input
#[test]
fn test_empty_input() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    let metrics = cpu.run(&mut input).unwrap();

    assert_eq!(metrics.total_instructions, 0);
    // Empty input completes in 1 cycle (the initial cycle that detects empty window)
    assert!(metrics.total_cycles <= 1);
}

// ============================================================================
// New Instruction Tests
// ============================================================================

/// Test cache maintenance instructions
#[test]
fn test_cache_maintenance() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::DcZva)
        .mem_access(0x2000, 64, false)
        .disasm("DC ZVA, X0")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test DC CIVAC instruction
#[test]
fn test_dc_civac() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::DcCivac)
        .src_reg(Reg(0))
        .mem_access(0x2000, 64, false)
        .disasm("DC CIVAC, X0")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test IC IVAU instruction
#[test]
fn test_ic_ivau() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::IcIvau)
        .src_reg(Reg(0))
        .mem_access(0x2000, 64, false)
        .disasm("IC IVAU, X0")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test SIMD vector add instruction
#[test]
fn test_simd_add() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::Vadd)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(2))
        .disasm("VADD V2.16B, V0.16B, V1.16B")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test SIMD vector multiply instruction
#[test]
fn test_simd_mul() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::Vmul)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(2))
        .disasm("VMUL V2.4S, V0.4S, V1.4S")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test SIMD vector load instruction
#[test]
fn test_simd_load() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::Vld)
        .dst_reg(Reg(0))
        .mem_access(0x2000, 16, true)
        .disasm("LDR Q0, [X1]")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test SIMD vector store instruction
#[test]
fn test_simd_store() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::Vst)
        .src_reg(Reg(0))
        .mem_access(0x2000, 16, false)
        .disasm("STR Q0, [X1]")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test FMA (Fused Multiply-Add) instruction
#[test]
fn test_fma_instructions() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::Fmadd)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .src_reg(Reg(2))
        .dst_reg(Reg(3))
        .disasm("FMADD D3, D0, D1, D2")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test FMSUB (Fused Multiply-Subtract) instruction
#[test]
fn test_fmsub_instruction() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::Fmsub)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .src_reg(Reg(2))
        .dst_reg(Reg(3))
        .disasm("FMSUB D3, D0, D1, D2")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test AES encrypt instruction
#[test]
fn test_aes_encrypt() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::Aese)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(0))
        .disasm("AESE V0.16B, V1.16B")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test AES decrypt instruction
#[test]
fn test_aes_decrypt() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::Aesd)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(0))
        .disasm("AESD V0.16B, V1.16B")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test SHA-256 hash instruction
#[test]
fn test_sha256() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();
    input.builder(0x1000, OpcodeType::Sha256H)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .src_reg(Reg(2))
        .dst_reg(Reg(0))
        .disasm("SHA256H Q0, Q1, V2.4S")
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 1);
}

/// Test mixed new instructions
#[test]
fn test_mixed_new_instructions() {
    let config = CPUConfig::high_performance();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    // Mix of different new instruction types
    input.builder(0x1000, OpcodeType::Vadd)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(2))
        .build();

    input.builder(0x1004, OpcodeType::Fmadd)
        .src_reg(Reg(2))
        .src_reg(Reg(3))
        .src_reg(Reg(4))
        .dst_reg(Reg(5))
        .build();

    input.builder(0x1008, OpcodeType::Aese)
        .src_reg(Reg(5))
        .src_reg(Reg(6))
        .dst_reg(Reg(7))
        .build();

    input.builder(0x100C, OpcodeType::Vst)
        .src_reg(Reg(7))
        .mem_access(0x2000, 16, false)
        .build();

    let metrics = cpu.run(&mut input).unwrap();
    assert_eq!(metrics.total_instructions, 4);
}

/// Test opcode classification methods
#[test]
fn test_opcode_classification() {
    // Cache maintenance
    assert!(OpcodeType::DcZva.is_cache_maintenance());
    assert!(OpcodeType::DcCivac.is_cache_maintenance());
    assert!(OpcodeType::IcIallu.is_cache_maintenance());

    // Crypto
    assert!(OpcodeType::Aese.is_crypto());
    assert!(OpcodeType::Aesd.is_crypto());
    assert!(OpcodeType::Sha256H.is_crypto());
    assert!(OpcodeType::Sha512H.is_crypto());

    // SIMD
    assert!(OpcodeType::Vadd.is_simd());
    assert!(OpcodeType::Vmul.is_simd());
    assert!(OpcodeType::Vld.is_simd());
    assert!(OpcodeType::Vst.is_simd());

    // FMA
    assert!(OpcodeType::Fmadd.is_fma());
    assert!(OpcodeType::Fmsub.is_fma());
    assert!(OpcodeType::Fnmadd.is_fma());
    assert!(OpcodeType::Fnmsub.is_fma());

    // Memory operations
    assert!(OpcodeType::Vld.is_memory_op());
    assert!(OpcodeType::Vst.is_memory_op());

    // Compute operations
    assert!(OpcodeType::Vadd.is_compute());
    assert!(OpcodeType::Fmadd.is_compute());
    assert!(OpcodeType::Aese.is_compute());
}

/// Test latency values for new instructions
#[test]
fn test_new_instruction_latency() {
    // Cache maintenance (high latency)
    assert_eq!(OpcodeType::DcZva.latency(), 20);
    assert_eq!(OpcodeType::DcCivac.latency(), 20);
    assert_eq!(OpcodeType::DcCsw.latency(), 30);
    assert_eq!(OpcodeType::IcIallu.latency(), 15);

    // Crypto
    assert_eq!(OpcodeType::Aese.latency(), 4);
    assert_eq!(OpcodeType::Aesd.latency(), 4);
    assert_eq!(OpcodeType::Sha256H.latency(), 12);
    assert_eq!(OpcodeType::Sha512H.latency(), 16);

    // SIMD
    assert_eq!(OpcodeType::Vadd.latency(), 2);
    assert_eq!(OpcodeType::Vmul.latency(), 4);
    assert_eq!(OpcodeType::Vld.latency(), 3);

    // FMA
    assert_eq!(OpcodeType::Fmadd.latency(), 4);
}
