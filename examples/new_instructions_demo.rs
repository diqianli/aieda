//! Demonstration of new ARM instruction support
//!
//! This example shows how the new instruction types work:
//! - Cache Maintenance Instructions
//! - Cryptography Instructions (AES/SHA)
//! - SIMD/Vector Instructions
//! - FMA Instructions

use arm_cpu_emulator::{CPUConfig, CPUEmulator, OpcodeType, Reg, TraceInput};

fn main() {
    println!("=== ARM CPU Emulator - New Instructions Demo ===\n");

    // Use high performance config for better IPC
    let config = CPUConfig::high_performance();
    let mut cpu = CPUEmulator::new(config).unwrap();

    // Demo 1: SIMD Vector Instructions
    demo_simd_instructions(&mut cpu);

    // Demo 2: FMA Instructions
    demo_fma_instructions(&mut cpu);

    // Demo 3: Cache Maintenance Instructions
    demo_cache_maintenance(&mut cpu);

    // Demo 4: Cryptography Instructions
    demo_crypto_instructions(&mut cpu);

    // Demo 5: Mixed instruction types
    demo_mixed_instructions(&mut cpu);

    println!("\n=== All demos completed successfully! ===");
}

fn demo_simd_instructions(cpu: &mut CPUEmulator) {
    println!("--- SIMD/Vector Instructions Demo ---");

    let mut input = TraceInput::new();

    // VADD V2.16B, V0.16B, V1.16B
    input.builder(0x1000, OpcodeType::Vadd)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(2))
        .disasm("VADD V2.16B, V0.16B, V1.16B")
        .build();

    // VSUB V4.4S, V2.4S, V3.4S
    input.builder(0x1004, OpcodeType::Vsub)
        .src_reg(Reg(2))
        .src_reg(Reg(3))
        .dst_reg(Reg(4))
        .disasm("VSUB V4.4S, V2.4S, V3.4S")
        .build();

    // VMUL V5.2D, V0.2D, V1.2D
    input.builder(0x1008, OpcodeType::Vmul)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(5))
        .disasm("VMUL V5.2D, V0.2D, V1.2D")
        .build();

    // VMLA V6.4S, V4.4S, V5.4S (Multiply-Accumulate)
    input.builder(0x100C, OpcodeType::Vmla)
        .src_reg(Reg(4))
        .src_reg(Reg(5))
        .src_reg(Reg(6))
        .dst_reg(Reg(6))
        .disasm("VMLA V6.4S, V4.4S, V5.4S")
        .build();

    let metrics = cpu.run(&mut input).unwrap();

    println!("  Instructions executed: {}", metrics.total_instructions);
    println!("  Total cycles: {}", metrics.total_cycles);
    println!("  IPC: {:.3}", metrics.ipc);
    println!("  CPI: {:.3}", metrics.cpi);

    // Verify instruction classification
    assert!(OpcodeType::Vadd.is_simd(), "VADD should be SIMD");
    assert!(OpcodeType::Vmul.is_simd(), "VMUL should be SIMD");

    // Verify latencies
    assert_eq!(OpcodeType::Vadd.latency(), 2, "VADD latency should be 2");
    assert_eq!(OpcodeType::Vmul.latency(), 4, "VMUL latency should be 4");
    assert_eq!(OpcodeType::Vmla.latency(), 4, "VMLA latency should be 4");

    println!("  ✓ SIMD instruction classification verified");
    println!("  ✓ SIMD instruction latencies verified\n");

    cpu.reset();
}

fn demo_fma_instructions(cpu: &mut CPUEmulator) {
    println!("--- FMA (Fused Multiply-Add) Instructions Demo ---");

    let mut input = TraceInput::new();

    // FMADD D3, D0, D1, D2 (a * b + c)
    input.builder(0x2000, OpcodeType::Fmadd)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .src_reg(Reg(2))
        .dst_reg(Reg(3))
        .disasm("FMADD D3, D0, D1, D2")
        .build();

    // FMSUB D4, D0, D1, D2 (a * b - c)
    input.builder(0x2004, OpcodeType::Fmsub)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .src_reg(Reg(2))
        .dst_reg(Reg(4))
        .disasm("FMSUB D4, D0, D1, D2")
        .build();

    // FNMADD D5, D0, D1, D2 (-(a * b) + c)
    input.builder(0x2008, OpcodeType::Fnmadd)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .src_reg(Reg(2))
        .dst_reg(Reg(5))
        .disasm("FNMADD D5, D0, D1, D2")
        .build();

    // FNMSUB D6, D0, D1, D2 (-(a * b) - c)
    input.builder(0x200C, OpcodeType::Fnmsub)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .src_reg(Reg(2))
        .dst_reg(Reg(6))
        .disasm("FNMSUB D6, D0, D1, D2")
        .build();

    let metrics = cpu.run(&mut input).unwrap();

    println!("  Instructions executed: {}", metrics.total_instructions);
    println!("  Total cycles: {}", metrics.total_cycles);
    println!("  IPC: {:.3}", metrics.ipc);

    // Verify FMA classification
    assert!(OpcodeType::Fmadd.is_fma(), "FMADD should be FMA");
    assert!(OpcodeType::Fmsub.is_fma(), "FMSUB should be FMA");
    assert!(OpcodeType::Fnmadd.is_fma(), "FNMADD should be FMA");
    assert!(OpcodeType::Fnmsub.is_fma(), "FNMSUB should be FMA");

    // Verify FMA latency
    assert_eq!(OpcodeType::Fmadd.latency(), 4, "FMADD latency should be 4");
    assert_eq!(OpcodeType::Fmsub.latency(), 4, "FMSUB latency should be 4");

    println!("  ✓ FMA instruction classification verified");
    println!("  ✓ FMA instruction latencies verified\n");

    cpu.reset();
}

fn demo_cache_maintenance(cpu: &mut CPUEmulator) {
    println!("--- Cache Maintenance Instructions Demo ---");

    let mut input = TraceInput::new();

    // DC ZVA, X0 (Zero cache line by VA)
    input.builder(0x3000, OpcodeType::DcZva)
        .mem_access(0x8000, 64, false)
        .disasm("DC ZVA, X0")
        .build();

    // DC CIVAC, X1 (Clean & Invalidate by VA to PoC)
    input.builder(0x3004, OpcodeType::DcCivac)
        .src_reg(Reg(1))
        .mem_access(0x8100, 64, false)
        .disasm("DC CIVAC, X1")
        .build();

    // IC IVAU, X2 (Invalidate instruction cache by VA to PoU)
    input.builder(0x3008, OpcodeType::IcIvau)
        .src_reg(Reg(2))
        .mem_access(0x9000, 64, false)
        .disasm("IC IVAU, X2")
        .build();

    // IC IALLU (Invalidate all instruction cache)
    input.builder(0x300C, OpcodeType::IcIallu)
        .disasm("IC IALLU")
        .build();

    let metrics = cpu.run(&mut input).unwrap();

    println!("  Instructions executed: {}", metrics.total_instructions);
    println!("  Total cycles: {}", metrics.total_cycles);
    println!("  IPC: {:.3}", metrics.ipc);

    // Verify cache maintenance classification
    assert!(OpcodeType::DcZva.is_cache_maintenance(), "DC ZVA should be cache maintenance");
    assert!(OpcodeType::DcCivac.is_cache_maintenance(), "DC CIVAC should be cache maintenance");
    assert!(OpcodeType::IcIvau.is_cache_maintenance(), "IC IVAU should be cache maintenance");
    assert!(OpcodeType::IcIallu.is_cache_maintenance(), "IC IALLU should be cache maintenance");

    // Verify high latencies for cache operations
    assert_eq!(OpcodeType::DcZva.latency(), 20, "DC ZVA latency should be 20");
    assert_eq!(OpcodeType::DcCivac.latency(), 20, "DC CIVAC latency should be 20");
    assert_eq!(OpcodeType::IcIallu.latency(), 15, "IC IALLU latency should be 15");
    assert_eq!(OpcodeType::DcCsw.latency(), 30, "DC CSW latency should be 30");

    println!("  ✓ Cache maintenance classification verified");
    println!("  ✓ Cache maintenance latencies verified (high latency ops)\n");

    cpu.reset();
}

fn demo_crypto_instructions(cpu: &mut CPUEmulator) {
    println!("--- Cryptography Instructions Demo ---");

    let mut input = TraceInput::new();

    // AESE V0.16B, V1.16B (AES Encrypt)
    input.builder(0x4000, OpcodeType::Aese)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(0))
        .disasm("AESE V0.16B, V1.16B")
        .build();

    // AESD V0.16B, V1.16B (AES Decrypt)
    input.builder(0x4004, OpcodeType::Aesd)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .dst_reg(Reg(0))
        .disasm("AESD V0.16B, V1.16B")
        .build();

    // AESMC V2.16B, V0.16B (AES Mix Columns)
    input.builder(0x4008, OpcodeType::Aesmc)
        .src_reg(Reg(0))
        .dst_reg(Reg(2))
        .disasm("AESMC V2.16B, V0.16B")
        .build();

    // SHA256H Q0, Q1, V2.4S
    input.builder(0x400C, OpcodeType::Sha256H)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .src_reg(Reg(2))
        .dst_reg(Reg(0))
        .disasm("SHA256H Q0, Q1, V2.4S")
        .build();

    // SHA512H Q0, Q1, V2.2D
    input.builder(0x4010, OpcodeType::Sha512H)
        .src_reg(Reg(0))
        .src_reg(Reg(1))
        .src_reg(Reg(2))
        .dst_reg(Reg(0))
        .disasm("SHA512H Q0, Q1, V2.2D")
        .build();

    let metrics = cpu.run(&mut input).unwrap();

    println!("  Instructions executed: {}", metrics.total_instructions);
    println!("  Total cycles: {}", metrics.total_cycles);
    println!("  IPC: {:.3}", metrics.ipc);

    // Verify crypto classification
    assert!(OpcodeType::Aese.is_crypto(), "AESE should be crypto");
    assert!(OpcodeType::Aesd.is_crypto(), "AESD should be crypto");
    assert!(OpcodeType::Aesmc.is_crypto(), "AESMC should be crypto");
    assert!(OpcodeType::Sha256H.is_crypto(), "SHA256H should be crypto");
    assert!(OpcodeType::Sha512H.is_crypto(), "SHA512H should be crypto");

    // Verify crypto latencies
    assert_eq!(OpcodeType::Aese.latency(), 4, "AESE latency should be 4");
    assert_eq!(OpcodeType::Aesd.latency(), 4, "AESD latency should be 4");
    assert_eq!(OpcodeType::Sha256H.latency(), 12, "SHA256H latency should be 12");
    assert_eq!(OpcodeType::Sha512H.latency(), 16, "SHA512H latency should be 16");

    println!("  ✓ Crypto instruction classification verified");
    println!("  ✓ Crypto instruction latencies verified\n");

    cpu.reset();
}

fn demo_mixed_instructions(cpu: &mut CPUEmulator) {
    println!("--- Mixed Instruction Types Demo ---");

    let mut input = TraceInput::with_capacity(30);

    // Regular ALU operations
    input.builder(0x5000, OpcodeType::Add)
        .src_reg(Reg(0)).src_reg(Reg(1)).dst_reg(Reg(10))
        .disasm("ADD X10, X0, X1").build();
    input.builder(0x5004, OpcodeType::Add)
        .src_reg(Reg(1)).src_reg(Reg(2)).dst_reg(Reg(11))
        .disasm("ADD X11, X1, X2").build();
    input.builder(0x5008, OpcodeType::Add)
        .src_reg(Reg(2)).src_reg(Reg(3)).dst_reg(Reg(12))
        .disasm("ADD X12, X2, X3").build();
    input.builder(0x500C, OpcodeType::Add)
        .src_reg(Reg(3)).src_reg(Reg(4)).dst_reg(Reg(13))
        .disasm("ADD X13, X3, X4").build();

    // SIMD operations
    input.builder(0x5010, OpcodeType::Vadd)
        .src_reg(Reg(0)).src_reg(Reg(4)).dst_reg(Reg(8))
        .disasm("VADD V8.16B, V0.16B, V4.16B").build();
    input.builder(0x5014, OpcodeType::Vadd)
        .src_reg(Reg(1)).src_reg(Reg(5)).dst_reg(Reg(9))
        .disasm("VADD V9.16B, V1.16B, V5.16B").build();
    input.builder(0x5018, OpcodeType::Vadd)
        .src_reg(Reg(2)).src_reg(Reg(6)).dst_reg(Reg(10))
        .disasm("VADD V10.16B, V2.16B, V6.16B").build();
    input.builder(0x501C, OpcodeType::Vadd)
        .src_reg(Reg(3)).src_reg(Reg(7)).dst_reg(Reg(11))
        .disasm("VADD V11.16B, V3.16B, V7.16B").build();

    // FMA operations
    input.builder(0x5020, OpcodeType::Fmadd)
        .src_reg(Reg(0)).src_reg(Reg(1)).src_reg(Reg(2)).dst_reg(Reg(16))
        .disasm("FMADD D16, D0, D1, D2").build();
    input.builder(0x5024, OpcodeType::Fmadd)
        .src_reg(Reg(1)).src_reg(Reg(2)).src_reg(Reg(3)).dst_reg(Reg(17))
        .disasm("FMADD D17, D1, D2, D3").build();
    input.builder(0x5028, OpcodeType::Fmadd)
        .src_reg(Reg(2)).src_reg(Reg(3)).src_reg(Reg(4)).dst_reg(Reg(18))
        .disasm("FMADD D18, D2, D3, D4").build();
    input.builder(0x502C, OpcodeType::Fmadd)
        .src_reg(Reg(3)).src_reg(Reg(4)).src_reg(Reg(5)).dst_reg(Reg(19))
        .disasm("FMADD D19, D3, D4, D5").build();

    // Memory operations
    input.builder(0x5030, OpcodeType::Load)
        .dst_reg(Reg(20))
        .mem_access(0xA000, 8, true)
        .disasm("LDR X20, [X0]").build();

    input.builder(0x5034, OpcodeType::Vld)
        .dst_reg(Reg(21))
        .mem_access(0xA100, 16, true)
        .disasm("LDR Q21, [X1]").build();

    input.builder(0x5038, OpcodeType::Store)
        .src_reg(Reg(22))
        .mem_access(0xA200, 8, false)
        .disasm("STR X22, [X2]").build();

    input.builder(0x503C, OpcodeType::Vst)
        .src_reg(Reg(23))
        .mem_access(0xA300, 16, false)
        .disasm("STR Q23, [X3]").build();

    // Crypto operations
    input.builder(0x5040, OpcodeType::Aese)
        .src_reg(Reg(24)).src_reg(Reg(25)).dst_reg(Reg(24))
        .disasm("AESE V24.16B, V25.16B").build();

    input.builder(0x5044, OpcodeType::Sha256H)
        .src_reg(Reg(26)).src_reg(Reg(27)).src_reg(Reg(28)).dst_reg(Reg(26))
        .disasm("SHA256H Q26, Q27, V28.4S").build();

    let metrics = cpu.run(&mut input).unwrap();

    println!("  Total instructions: {}", metrics.total_instructions);
    println!("  Total cycles: {}", metrics.total_cycles);
    println!("  IPC: {:.3}", metrics.ipc);
    println!("  CPI: {:.3}", metrics.cpi);
    println!("  Memory instruction %: {:.1}%", metrics.memory_instr_pct);
    println!("  Branch instruction %: {:.1}%", metrics.branch_instr_pct);

    // Verify counts: 4 ALU + 4 SIMD + 4 FMA + 4 Memory + 2 Crypto = 18
    assert_eq!(metrics.total_instructions, 18, "Should have 18 instructions");

    println!("  ✓ Mixed instruction execution verified\n");

    cpu.reset();
}
