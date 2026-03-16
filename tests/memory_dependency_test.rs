//! Test to verify memory dependency tracking in visualization

use arm_cpu_emulator::{
    CPUConfig, CPUEmulator, OpcodeType, Reg, TraceInput,
};
use arm_cpu_emulator::visualization::{KonataDependencyType, PipelineTracker};
use arm_cpu_emulator::ooo::DependencyTracker;
use arm_cpu_emulator::types::{Instruction, InstructionId, MemAccess};

#[test]
fn test_memory_dependency_tracking() {
    // Create dependency tracker
    let mut tracker = DependencyTracker::new();

    // Instruction 0: STORE to address 0x1000
    let store0 = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Store)
        .with_src_reg(Reg(0))
        .with_mem_access(0x1000, 8, false);

    // Instruction 1: LOAD from address 0x1000 (should depend on STORE)
    let load1 = Instruction::new(InstructionId(1), 0x1004, 0, OpcodeType::Load)
        .with_dst_reg(Reg(1))
        .with_mem_access(0x1000, 8, true);

    // Instruction 2: STORE to address 0x1000 (should depend on STORE 0)
    let store2 = Instruction::new(InstructionId(2), 0x1008, 0, OpcodeType::Store)
        .with_src_reg(Reg(2))
        .with_mem_access(0x1000, 8, false);

    // Instruction 3: LOAD from address 0x2000 (independent - different address)
    let load3 = Instruction::new(InstructionId(3), 0x100C, 0, OpcodeType::Load)
        .with_dst_reg(Reg(3))
        .with_mem_access(0x2000, 8, true);

    // Register instructions
    let deps0 = tracker.register_instruction(&store0, InstructionId(0), 0);
    println!("Store0 deps: {:?}", deps0);

    let deps1 = tracker.register_instruction(&load1, InstructionId(1), 0);
    println!("Load1 deps (should have 1 memory dep on Store0): {:?}", deps1);
    assert_eq!(deps1.len(), 1, "Load1 should depend on Store0");
    assert!(deps1[0].is_memory, "Load1 dependency should be memory type");
    assert_eq!(deps1[0].producer, InstructionId(0));

    let deps2 = tracker.register_instruction(&store2, InstructionId(2), 0);
    println!("Store2 deps (should have 1 memory dep on Store0): {:?}", deps2);
    assert_eq!(deps2.len(), 1, "Store2 should depend on Store0");
    assert!(deps2[0].is_memory, "Store2 dependency should be memory type");

    let deps3 = tracker.register_instruction(&load3, InstructionId(3), 0);
    println!("Load3 deps (should be empty - different address): {:?}", deps3);
    assert_eq!(deps3.len(), 0, "Load3 should have no dependencies (different address)");

    // Check ready status
    assert!(tracker.is_ready(InstructionId(0)), "Store0 should be ready");
    assert!(!tracker.is_ready(InstructionId(1)), "Load1 should wait for Store0");
    assert!(!tracker.is_ready(InstructionId(2)), "Store2 should wait for Store0");
    assert!(tracker.is_ready(InstructionId(3)), "Load3 should be ready");

    // Complete Store0
    tracker.release_dependencies(&store0, InstructionId(0));

    // Now dependent instructions should be ready
    assert!(tracker.is_ready(InstructionId(1)), "Load1 should be ready after Store0 completes");
    assert!(tracker.is_ready(InstructionId(2)), "Store2 should be ready after Store0 completes");
}

#[test]
fn test_register_and_memory_dependencies() {
    let mut tracker = DependencyTracker::new();

    // Instruction 0: LOAD R1 <- [0x1000]
    let load0 = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Load)
        .with_dst_reg(Reg(1))
        .with_mem_access(0x1000, 8, true);

    // Instruction 1: ADD R2 = R1 + R0 (depends on R1 from Load0)
    let add1 = Instruction::new(InstructionId(1), 0x1004, 0, OpcodeType::Add)
        .with_src_reg(Reg(0))
        .with_src_reg(Reg(1))
        .with_dst_reg(Reg(2));

    // Instruction 2: STORE [0x2000] <- R2 (depends on R2 from Add1)
    let store2 = Instruction::new(InstructionId(2), 0x1008, 0, OpcodeType::Store)
        .with_src_reg(Reg(2))
        .with_mem_access(0x2000, 8, false);

    let deps0 = tracker.register_instruction(&load0, InstructionId(0), 0);
    println!("Load0 deps: {:?}", deps0);
    assert_eq!(deps0.len(), 0, "Load0 should have no dependencies");

    let deps1 = tracker.register_instruction(&add1, InstructionId(1), 0);
    println!("Add1 deps (should have register dep on R1 from Load0): {:?}", deps1);
    assert_eq!(deps1.len(), 1, "Add1 should depend on Load0 (register)");
    assert!(!deps1[0].is_memory, "Add1 dependency should be register type");

    let deps2 = tracker.register_instruction(&store2, InstructionId(2), 0);
    println!("Store2 deps (should have register dep on R2 from Add1): {:?}", deps2);
    assert_eq!(deps2.len(), 1, "Store2 should depend on Add1 (register)");
    assert!(!deps2[0].is_memory, "Store2 dependency should be register type");

    // Check dependency chain
    assert!(tracker.is_ready(InstructionId(0)), "Load0 should be ready");
    assert!(!tracker.is_ready(InstructionId(1)), "Add1 should wait for Load0");
    assert!(!tracker.is_ready(InstructionId(2)), "Store2 should wait for Add1");

    // Complete Load0
    tracker.release_dependencies(&load0, InstructionId(0));
    assert!(tracker.is_ready(InstructionId(1)), "Add1 should be ready after Load0");
    assert!(!tracker.is_ready(InstructionId(2)), "Store2 should still wait for Add1");

    // Complete Add1
    tracker.release_dependencies(&add1, InstructionId(1));
    assert!(tracker.is_ready(InstructionId(2)), "Store2 should be ready after Add1");
}

#[test]
fn test_load_after_store_dependency() {
    // Test the critical Store-to-Load forwarding case
    let mut tracker = DependencyTracker::new();

    // STORE [R0] <- R1
    let store = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Store)
        .with_src_reg(Reg(1))
        .with_mem_access(0x1000, 8, false);

    // LOAD R2 <- [R0] (depends on STORE - RAW memory dependency)
    let load = Instruction::new(InstructionId(1), 0x1004, 0, OpcodeType::Load)
        .with_dst_reg(Reg(2))
        .with_mem_access(0x1000, 8, true);

    tracker.register_instruction(&store, InstructionId(0), 0);
    let deps = tracker.register_instruction(&load, InstructionId(1), 0);

    println!("Load after Store - deps: {:?}", deps);
    assert_eq!(deps.len(), 1, "Load should depend on Store");
    assert!(deps[0].is_memory, "Load-Store dependency should be memory type");
    assert_eq!(deps[0].producer, InstructionId(0));
}

#[test]
fn test_full_simulation_with_memory_deps() {
    let config = CPUConfig::minimal();
    let mut cpu = CPUEmulator::new(config).unwrap();

    let mut input = TraceInput::new();

    // Create a dependency chain: Store -> Load -> Add -> Store
    // Store [0x1000] <- X0
    input.builder(0x1000, OpcodeType::Store)
        .src_reg(Reg(0))
        .mem_access(0x1000, 8, false)
        .disasm("STR X0, [X1]")
        .build();

    // Load X2 <- [0x1000] (depends on Store)
    input.builder(0x1004, OpcodeType::Load)
        .dst_reg(Reg(2))
        .mem_access(0x1000, 8, true)
        .disasm("LDR X2, [X1]")
        .build();

    // Add X3 = X2 + X0 (depends on Load for X2)
    input.builder(0x1008, OpcodeType::Add)
        .src_reg(Reg(2))
        .src_reg(Reg(0))
        .dst_reg(Reg(3))
        .disasm("ADD X3, X2, X0")
        .build();

    // Store [0x2000] <- X3 (depends on Add for X3)
    input.builder(0x100C, OpcodeType::Store)
        .src_reg(Reg(3))
        .mem_access(0x2000, 8, false)
        .disasm("STR X3, [X4]")
        .build();

    let metrics = cpu.run(&mut input).unwrap();

    println!("Memory dependency chain simulation:");
    println!("  Instructions: {}", metrics.total_instructions);
    println!("  Cycles: {}", metrics.total_cycles);
    println!("  IPC: {:.3}", metrics.ipc);

    // Get the trace to check dependencies
    let trace = cpu.trace();
    println!("  Trace entries: {}", trace.len());

    for (i, entry) in trace.entries().iter().take(4).enumerate() {
        println!("  [{}] {} @ {:#x}: disp={}, issue={:?}, commit={:?}",
            i, entry.opcode, entry.pc, entry.dispatch_cycle,
            entry.issue_cycle, entry.commit_cycle);
    }

    assert_eq!(metrics.total_instructions, 4);
}
