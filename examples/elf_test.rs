//! Test ELF loader with a minimal ARM64 ELF binary

use std::fs::File;
use std::io::Write;
use std::path::Path;

use arm_cpu_emulator::elf::{ElfLoader, Arm64Decoder};
use arm_cpu_emulator::types::OpcodeType;

/// Create a minimal ARM64 ELF executable for testing
fn create_minimal_arm64_elf(path: &Path) -> std::io::Result<()> {
    // Create ELF file with separate header and code
    let mut elf = Vec::new();

    // Code section - will be at offset 0x1000 in file
    // ARM64 Code - Simple program
    let mut code = Vec::new();

    // mov x0, #1        (stdout)
    code.extend_from_slice(&0xD2800020u32.to_le_bytes());

    // ldr x1, =message  (PC-relative load)
    code.extend_from_slice(&0x58000041u32.to_le_bytes());

    // mov x2, #13       (message length)
    code.extend_from_slice(&0xD28001A2u32.to_le_bytes());

    // mov x8, #64       (write syscall)
    code.extend_from_slice(&0xD2800808u32.to_le_bytes());

    // svc #0            (syscall)
    code.extend_from_slice(&0xD4000001u32.to_le_bytes());

    // mov x0, #0        (exit code)
    code.extend_from_slice(&0xD2800000u32.to_le_bytes());

    // mov x8, #93       (exit syscall)
    code.extend_from_slice(&0xD2800BA8u32.to_le_bytes());

    // svc #0            (syscall)
    code.extend_from_slice(&0xD4000001u32.to_le_bytes());

    // NOP instructions for padding
    for _ in 0..5 {
        code.extend_from_slice(&0xD503201Fu32.to_le_bytes()); // NOP
    }

    // message: "Hello, ARM64"
    code.extend_from_slice(b"Hello, ARM64\n");

    // ELF Header (64 bytes)
    // e_ident (16 bytes)
    elf.extend_from_slice(b"\x7fELF");     // Magic
    elf.push(2);                            // 64-bit
    elf.push(1);                            // Little endian
    elf.push(1);                            // ELF version
    elf.push(0);                            // OS/ABI
    elf.extend_from_slice(&[0; 8]);         // Padding

    // e_type (2 bytes) - ET_EXEC = 2
    elf.extend_from_slice(&2u16.to_le_bytes());

    // e_machine (2 bytes) - EM_AARCH64 = 0xB7
    elf.extend_from_slice(&0xB7u16.to_le_bytes());

    // e_version (4 bytes)
    elf.extend_from_slice(&1u32.to_le_bytes());

    // e_entry (8 bytes) - Entry point at 0x1000 (offset to code)
    elf.extend_from_slice(&0x1000u64.to_le_bytes());

    // e_phoff (8 bytes) - Program header offset (right after ELF header)
    elf.extend_from_slice(&64u64.to_le_bytes());

    // e_shoff (8 bytes) - Section header offset (no sections for minimal)
    elf.extend_from_slice(&0u64.to_le_bytes());

    // e_flags (4 bytes)
    elf.extend_from_slice(&0u32.to_le_bytes());

    // e_ehsize (2 bytes) - ELF header size = 64
    elf.extend_from_slice(&64u16.to_le_bytes());

    // e_phentsize (2 bytes) - Program header entry size = 56
    elf.extend_from_slice(&56u16.to_le_bytes());

    // e_phnum (2 bytes) - Number of program headers = 1
    elf.extend_from_slice(&1u16.to_le_bytes());

    // e_shentsize (2 bytes)
    elf.extend_from_slice(&64u16.to_le_bytes());

    // e_shnum (2 bytes) - No sections
    elf.extend_from_slice(&0u16.to_le_bytes());

    // e_shstrndx (2 bytes)
    elf.extend_from_slice(&0u16.to_le_bytes());

    assert_eq!(elf.len(), 64, "ELF header should be 64 bytes");

    // Program Header (56 bytes)
    // p_type (4 bytes) - PT_LOAD = 1
    elf.extend_from_slice(&1u32.to_le_bytes());

    // p_flags (4 bytes) - PF_X | PF_R = 5
    elf.extend_from_slice(&5u32.to_le_bytes());

    // p_offset (8 bytes) - Code starts at file offset 0x1000
    elf.extend_from_slice(&0x1000u64.to_le_bytes());

    // p_vaddr (8 bytes) - Load at 0x1000
    elf.extend_from_slice(&0x1000u64.to_le_bytes());

    // p_paddr (8 bytes)
    elf.extend_from_slice(&0x1000u64.to_le_bytes());

    // p_filesz (8 bytes) - Size of code
    elf.extend_from_slice(&(code.len() as u64).to_le_bytes());

    // p_memsz (8 bytes)
    elf.extend_from_slice(&(code.len() as u64).to_le_bytes());

    // p_align (8 bytes)
    elf.extend_from_slice(&0x1000u64.to_le_bytes());

    assert_eq!(elf.len(), 120, "ELF header + PH should be 120 bytes");

    // Pad to offset 0x1000
    while elf.len() < 0x1000 {
        elf.push(0);
    }

    // Add code
    elf.extend_from_slice(&code);

    // Write to file
    let mut file = File::create(path)?;
    file.write_all(&elf)?;

    Ok(())
}

fn main() {
    println!("=== ARM64 ELF Loader Test ===\n");

    // Create test ELF file
    let elf_path = std::path::PathBuf::from("/tmp/test_arm64.elf");

    match create_minimal_arm64_elf(&elf_path) {
        Ok(()) => println!("Created test ELF file: {:?}", elf_path),
        Err(e) => {
            eprintln!("Failed to create ELF file: {}", e);
            return;
        }
    }

    // Load the ELF file
    println!("\n--- Loading ELF file ---");
    let loader = match ElfLoader::load(&elf_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to load ELF: {:?}", e);
            return;
        }
    };

    // Print ELF info
    println!("Entry point: {:#X}", loader.entry_point());
    println!("Number of segments: {}", loader.segments().len());
    println!("Number of functions: {}", loader.functions().len());

    // Print segment info
    println!("\n--- Segments ---");
    for (i, seg) in loader.segments().iter().enumerate() {
        println!(
            "Segment {}: vaddr={:#X}, size={:#X}, exec={}, write={}, read={}",
            i, seg.vaddr, seg.size, seg.executable, seg.writable, seg.readable
        );
    }

    // Decode and display instructions
    println!("\n--- Decoding Instructions ---");
    let decoder = Arm64Decoder::new();

    let entry = loader.entry_point();
    let mut pc = entry;
    let mut count = 0;

    println!("Disassembly from entry point {:#X}:\n", entry);

    while count < 20 {
        match loader.read_instruction(pc) {
            Some(raw) => {
                let decoded = decoder.decode(pc, raw);

                println!(
                    "{:#010X}: {:08X}  {}",
                    pc,
                    raw,
                    decoded.disasm
                );

                // Show opcode type
                if decoded.opcode != OpcodeType::Other {
                    println!("           └─ Opcode: {:?}, Latency: {} cycles",
                        decoded.opcode,
                        decoded.opcode.latency()
                    );
                }

                // Show registers
                if !decoded.src_regs.is_empty() {
                    println!("           └─ Src regs: {:?}", decoded.src_regs);
                }
                if !decoded.dst_regs.is_empty() {
                    println!("           └─ Dst regs: {:?}", decoded.dst_regs);
                }

                pc += 4;
                count += 1;
            }
            None => break,
        }
    }

    // Test symbol lookup (if any)
    println!("\n--- Symbol Lookup ---");
    if let Some(sym) = loader.get_symbol(entry) {
        println!("Symbol at entry point: {}", sym);
    } else {
        println!("No symbol at entry point (expected for minimal ELF)");
    }

    // Show functions
    if !loader.functions().is_empty() {
        println!("\n--- Functions ---");
        for (start, end, name) in loader.functions() {
            println!("{}: {:#X} - {:#X}", name, start, end);
        }
    }

    // Clean up
    std::fs::remove_file(&elf_path).ok();

    println!("\n=== Test Complete ===");
}
