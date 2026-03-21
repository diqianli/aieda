//! ARM64 (AArch64) instruction decoder.
//!
//! Provides basic instruction decoding without external dependencies.

use crate::types::{OpcodeType, Reg, VReg};

/// Decoded instruction information
#[derive(Debug, Clone)]
pub struct DecodedInstruction {
    /// Program counter
    pub pc: u64,
    /// Raw instruction encoding
    pub raw: u32,
    /// Decoded opcode type
    pub opcode: OpcodeType,
    /// Source registers
    pub src_regs: Vec<Reg>,
    /// Destination registers
    pub dst_regs: Vec<Reg>,
    /// Source vector registers
    pub src_vregs: Vec<VReg>,
    /// Destination vector registers
    pub dst_vregs: Vec<VReg>,
    /// Immediate value (if any)
    pub immediate: Option<i64>,
    /// Memory address (for load/store)
    pub mem_addr: Option<u64>,
    /// Memory size (for load/store)
    pub mem_size: Option<u8>,
    /// Is load operation
    pub is_load: bool,
    /// Branch target (for branches)
    pub branch_target: Option<u64>,
    /// Is conditional branch
    pub is_conditional: bool,
    /// Disassembly text
    pub disasm: String,
}

impl DecodedInstruction {
    /// Create a new decoded instruction
    pub fn new(pc: u64, raw: u32) -> Self {
        Self {
            pc,
            raw,
            opcode: OpcodeType::Other,
            src_regs: Vec::new(),
            dst_regs: Vec::new(),
            src_vregs: Vec::new(),
            dst_vregs: Vec::new(),
            immediate: None,
            mem_addr: None,
            mem_size: None,
            is_load: false,
            branch_target: None,
            is_conditional: false,
            disasm: String::new(),
        }
    }
}

/// ARM64 instruction decoder
pub struct Arm64Decoder {
    /// Base address for PC-relative calculations
    base_addr: u64,
}

impl Arm64Decoder {
    /// Create a new decoder
    pub fn new() -> Self {
        Self { base_addr: 0 }
    }

    /// Set base address for PC-relative calculations
    pub fn with_base_addr(mut self, addr: u64) -> Self {
        self.base_addr = addr;
        self
    }

    /// Decode a single instruction
    pub fn decode(&self, pc: u64, raw: u32) -> DecodedInstruction {
        let mut instr = DecodedInstruction::new(pc, raw);

        // ARM64 instruction encoding uses bits[28:25] as primary class indicator:
        // - 0000: Reserved, PC-rel addressing, system
        // - 100x: Data processing - immediate
        // - 101x: Load/store
        // - 1101: Data processing - register
        // - 111x: Data processing - SIMD/FP

        let bits_28_25 = (raw >> 25) & 0xF;  // bits[28:25]
        let bits_31_30 = (raw >> 30) & 0x3;  // bits[31:30] for size
        let bits_31_24 = (raw >> 24) & 0xFF;

        // Special case: System instructions (NOP, etc.) - bits[31:24] = 0xD4 or 0xD5
        // Must check BEFORE load/store since they share bits[28:27] = 10
        if bits_31_24 == 0xD4 || bits_31_24 == 0xD5 {
            self.decode_branch(&mut instr, pc, raw);
        }
        // Data processing - immediate: bits[28:25] = 100x (1000 or 1001)
        else if bits_28_25 == 0b1000 || bits_28_25 == 0b1001 {
            self.decode_data_imm(&mut instr, raw);
        }
        // Branch, exception generation, system: bits[28:25] = 000x with specific patterns
        // Unconditional branch: bits[31:30] = 00, bits[28:25] = 0101 (actually bits[31:25] = 000101x)
        else if bits_31_24 == 0x14 || bits_31_24 == 0x15 ||  // B (unconditional)
                bits_31_24 == 0x94 || bits_31_24 == 0x95 ||  // BL (unconditional with link)
                (bits_31_24 & 0xFE) == 0x54 {  // Conditional branch (B.cond)
            self.decode_branch(&mut instr, pc, raw);
        }
        // Load/store: bits[28:25] = 101x (1010 or 1011) OR bits[31:27] = 111xx
        // This covers: pairs, exclusive, unsigned immediate, etc.
        else if bits_28_25 == 0b1010 || bits_28_25 == 0b1011 ||
                (raw >> 27) & 0x1F == 0b11111 ||  // 64-bit unsigned immediate
                (raw >> 27) & 0x1F == 0b11101 ||  // 32-bit unsigned immediate
                (raw >> 27) & 0x1F == 0b11100 ||  // Unscaled immediate
                (raw >> 27) & 0x1F == 0b11110 {   // Prefetch
            self.decode_load_store(&mut instr, raw);
        }
        // Data processing - register: bits[28:24] = 11010
        else if (raw >> 24) & 0x1F == 0b11010 {
            self.decode_data_reg(&mut instr, raw);
        }
        // SIMD/FP: bits[28:24] = 01110 or 11110
        else if ((raw >> 24) & 0x1F) == 0b01110 || ((raw >> 24) & 0x1F) == 0b11110 {
            self.decode_simd_fp(&mut instr, raw);
        }
        else {
            self.decode_reserved(&mut instr);
        }

        instr
    }

    fn decode_reserved(&self, instr: &mut DecodedInstruction) {
        instr.opcode = OpcodeType::Nop;
        instr.disasm = "nop".to_string();
    }

    fn decode_data_imm(&self, instr: &mut DecodedInstruction, raw: u32) {
        let op = (raw >> 23) & 0x3;
        let is_64bit = (raw >> 31) & 1 == 1;

        match op {
            // PC-relative addressing (ADR, ADRP)
            0b00 => {
                let rd = (raw & 0x1F) as u8;
                let is_adrp = (raw >> 31) & 1 == 1;
                let immlo = ((raw >> 29) & 0x3) as u64;
                let immhi = ((raw >> 5) & 0x7FFFF) as u64;
                let mut imm = (immhi << 2) | immlo;
                if is_adrp {
                    imm <<= 12;
                }
                // Sign extend
                let imm_signed = if is_adrp {
                    ((imm as i64) << 23 >> 23) as i64
                } else {
                    ((imm as i64) << 43 >> 43) as i64
                };

                instr.dst_regs.push(Reg(rd));
                instr.immediate = Some(imm_signed);
                instr.opcode = OpcodeType::Add;
                let target = instr.pc.wrapping_add(imm_signed as u64);
                if is_adrp {
                    instr.disasm = format!("ADRP X{}, {:#X}", rd, target & !0xFFF);
                } else {
                    instr.disasm = format!("ADR X{}, {:#X}", rd, target);
                }
            }

            // Add/subtract immediate
            0b10 => {
                let is_sub = (raw >> 30) & 1 == 1;
                let sets_flags = (raw >> 29) & 1 == 1;
                let rd = (raw & 0x1F) as u8;
                let rn = ((raw >> 5) & 0x1F) as u8;
                let imm = ((raw >> 10) & 0xFFF) as u64;
                let shift = ((raw >> 22) & 0x3);

                instr.dst_regs.push(Reg(rd));
                instr.src_regs.push(Reg(rn));
                let shifted_imm = if shift == 0 { imm } else { imm << 12 };
                instr.immediate = Some(shifted_imm as i64);

                if is_sub && sets_flags {
                    instr.opcode = OpcodeType::Cmp;
                } else {
                    instr.opcode = if is_sub { OpcodeType::Sub } else { OpcodeType::Add };
                }

                let reg_prefix = if is_64bit { "X" } else { "W" };
                let rd_str = if rd == 31 && !sets_flags { "SP".to_string() } else { format!("{}{}", reg_prefix, rd) };
                let rn_str = if rn == 31 { "SP".to_string() } else { format!("{}{}", reg_prefix, rn) };

                let op_name = if is_sub {
                    if sets_flags { "SUBS" } else { "SUB" }
                } else {
                    if sets_flags { "ADDS" } else { "ADD" }
                };

                if shift == 0 {
                    instr.disasm = format!("{} {}, {}, #{:#X}", op_name, rd_str, rn_str, imm);
                } else {
                    instr.disasm = format!("{} {}, {}, #{:#X}, LSL #12", op_name, rd_str, rn_str, imm);
                }
            }

            // Logical immediate
            0b00 => {
                let rd = (raw & 0x1F) as u8;
                let rn = ((raw >> 5) & 0x1F) as u8;
                let opc = (raw >> 29) & 0x3;
                let immr = ((raw >> 16) & 0x3F) as u8;
                let imms = ((raw >> 10) & 0x3F) as u8;
                let n = (raw >> 22) & 1;

                // Check for MOV (alias for ORR with XZR)
                if opc == 0b01 && rn == 31 {
                    // MOV (wide immediate from logical)
                    instr.dst_regs.push(Reg(rd));
                    instr.opcode = OpcodeType::Mov;
                    // Decode the bitfield immediate (simplified)
                    let imm = Self::decode_logical_immediate(n, immr, imms, is_64bit);
                    instr.disasm = format!("MOV X{}, #{:#X}", rd, imm);
                    return;
                }

                instr.dst_regs.push(Reg(rd));
                instr.src_regs.push(Reg(rn));

                instr.opcode = match opc {
                    0b00 => OpcodeType::And,
                    0b01 => OpcodeType::Orr,
                    0b10 => OpcodeType::Eor,
                    _ => OpcodeType::And,
                };

                let op_name = match opc {
                    0b00 => "AND",
                    0b01 => "ORR",
                    0b10 => "EOR",
                    _ => "ANDS",
                };

                let reg_prefix = if is_64bit { "X" } else { "W" };
                let imm = Self::decode_logical_immediate(n, immr, imms, is_64bit);
                instr.disasm = format!("{} {}{}, {}{}, #{:#X}", op_name, reg_prefix, rd, reg_prefix, rn, imm);
            }

            // Move wide immediate (MOVZ, MOVN, MOVK)
            0b11 => {
                let opc = (raw >> 29) & 0x3;
                let hw = ((raw >> 21) & 0x3) as u8;
                let imm16 = ((raw >> 5) & 0xFFFF) as u64;
                let rd = (raw & 0x1F) as u8;

                instr.dst_regs.push(Reg(rd));
                instr.immediate = Some((imm16 << (hw * 16)) as i64);

                let (op_name, opcode) = match opc {
                    0b00 => ("MOVN", OpcodeType::Mov),
                    0b10 => ("MOVZ", OpcodeType::Mov),
                    0b11 => ("MOVK", OpcodeType::Mov),
                    _ => ("MOV", OpcodeType::Mov),
                };
                instr.opcode = opcode;

                let reg_prefix = if is_64bit { "X" } else { "W" };

                if hw == 0 {
                    instr.disasm = format!("{} {}{}, #{:#X}", op_name, reg_prefix, rd, imm16);
                } else {
                    instr.disasm = format!("{} {}{}, #{:#X}, LSL #{}", op_name, reg_prefix, rd, imm16, hw * 16);
                }
            }

            // Bitfield operations
            0b01 => {
                let opc = (raw >> 29) & 0x3;
                let rd = (raw & 0x1F) as u8;
                let rn = ((raw >> 5) & 0x1F) as u8;
                let immr = ((raw >> 16) & 0x3F) as u8;
                let imms = ((raw >> 10) & 0x3F) as u8;

                instr.dst_regs.push(Reg(rd));
                instr.src_regs.push(Reg(rn));

                let (op_name, opcode) = match opc {
                    0b00 => ("SBFM", OpcodeType::Other),
                    0b01 => ("BFM", OpcodeType::Other),
                    0b10 => ("UBFM", OpcodeType::Other),
                    _ => ("BFM", OpcodeType::Other),
                };
                instr.opcode = opcode;

                let reg_prefix = if is_64bit { "X" } else { "W" };

                // Check for aliases (LSL, LSR, ASR, etc.)
                if opc == 0b10 && imms == 0x3F - immr {
                    // LSR alias
                    instr.disasm = format!("LSR {}{}, {}{}, #{}", reg_prefix, rd, reg_prefix, rn, immr);
                } else if opc == 0b10 && imms == 0x3F && immr != 0 {
                    // LSL alias
                    instr.disasm = format!("LSL {}{}, {}{}, #{}", reg_prefix, rd, reg_prefix, rn, 64 - immr);
                } else if opc == 0b00 && imms == 0x3F {
                    // ASR alias
                    instr.disasm = format!("ASR {}{}, {}{}, #{}", reg_prefix, rd, reg_prefix, rn, immr);
                } else {
                    instr.disasm = format!("{} {}{}, {}{}, #{}, #{}", op_name, reg_prefix, rd, reg_prefix, rn, immr, imms);
                }
            }

            _ => {
                instr.opcode = OpcodeType::Other;
                instr.disasm = format!(".word 0x{:08X}", raw);
            }
        }
    }

    /// Decode logical immediate value
    fn decode_logical_immediate(n: u32, immr: u8, imms: u8, is_64bit: bool) -> u64 {
        // This is a simplified decoder for common cases
        // Full decoder requires rotating bit patterns
        let len = if n == 1 { 64 } else if is_64bit {
            if imms >= 32 { 32 } else if imms >= 16 { 16 } else if imms >= 8 { 8 } else if imms >= 4 { 4 } else if imms >= 2 { 2 } else { 1 }
        } else {
            if imms >= 16 { 16 } else if imms >= 8 { 8 } else if imms >= 4 { 4 } else if imms >= 2 { 2 } else { 1 }
        };

        // For simple patterns like 0, -1, etc.
        if imms == 0x3F && n == 1 {
            return !0; // All ones
        }
        if imms == 0 && n == 0 {
            return 0; // All zeros (unlikely in practice)
        }

        // Return a placeholder for complex bit patterns
        // In a real implementation, this would compute the actual rotated pattern
        0xDEAD_BEEF
    }

    fn decode_branch(&self, instr: &mut DecodedInstruction, pc: u64, raw: u32) {
        let op = (raw >> 29) & 0x7;
        let op2 = (raw >> 22) & 0xF;
        let is_64bit = (raw >> 31) & 1 == 1;
        let reg_prefix = if is_64bit { "X" } else { "W" };

        // Check for system instructions first (bits[31:24] = 0xD4 or 0xD5)
        let sys_op = (raw >> 24) & 0xFF;
        if sys_op == 0xD4 || sys_op == 0xD5 {
            // System instructions
            let crn = ((raw >> 12) & 0xF) as u8;
            let crm = ((raw >> 8) & 0xF) as u8;
            let op2_sys = ((raw >> 5) & 0x7) as u8;
            let rt = (raw & 0x1F) as u8;
            let op1_sys = ((raw >> 16) & 0x7) as u8;

            // NOP and other hint instructions
            // Standard NOP encoding: D503201F (op1=3, CRn=2, CRm=0, op2=0, Rt=31)
            // This is the most common NOP encoding used by assemblers
            if raw == 0xD503201F {
                instr.opcode = OpcodeType::Nop;
                instr.disasm = "NOP".to_string();
                return;
            }

            // HINT instructions (op1=3, CRn=3, CRm=2, op2=2)
            if op1_sys == 3 && crn == 3 && crm == 2 && op2_sys == 2 {
                match rt {
                    0 | 31 => {  // 0 or XZR (31)
                        instr.opcode = OpcodeType::Nop;
                        instr.disasm = "NOP".to_string();
                    }
                    1 => {
                        instr.opcode = OpcodeType::Nop;
                        instr.disasm = "YIELD".to_string();
                    }
                    2 => {
                        instr.opcode = OpcodeType::Nop;
                        instr.disasm = "WFE".to_string();
                    }
                    3 => {
                        instr.opcode = OpcodeType::Nop;
                        instr.disasm = "WFI".to_string();
                    }
                    4 => {
                        instr.opcode = OpcodeType::Nop;
                        instr.disasm = "SEV".to_string();
                    }
                    5 => {
                        instr.opcode = OpcodeType::Nop;
                        instr.disasm = "SEVL".to_string();
                    }
                    _ => {
                        instr.opcode = OpcodeType::Nop;
                        instr.disasm = format!("HINT #{}", rt);
                    }
                }
                return;
            }

            // Other system instructions
            instr.opcode = OpcodeType::Sys;
            instr.disasm = format!(".word 0x{:08X} (system)", raw);
            return;
        }

        match (op, op2) {
            // Unconditional branch (immediate)
            (0b000, 0b0000) => {
                let imm26 = (raw & 0x3FFFFFF) as i32;
                let imm26 = ((imm26 << 6) >> 6) << 2; // Sign extend and shift

                instr.branch_target = Some(pc.wrapping_add(imm26 as u64));
                instr.opcode = OpcodeType::Branch;
                instr.disasm = format!("B {:#X}", pc.wrapping_add(imm26 as u64));
            }

            // Conditional branch (immediate)
            (0b010, 0b0000) => {
                let imm19 = ((raw >> 5) & 0x7FFFF) as i32;
                let imm19 = ((imm19 << 13) >> 13) << 2; // Sign extend and shift
                let cond = (raw & 0xF) as u8;

                instr.branch_target = Some(pc.wrapping_add(imm19 as u64));
                instr.is_conditional = true;
                instr.opcode = OpcodeType::BranchCond;

                let cond_names = [
                    "EQ", "NE", "CS", "CC", "MI", "PL", "VS", "VC",
                    "HI", "LS", "GE", "LT", "GT", "LE", "AL", "NV",
                ];
                instr.disasm = format!(
                    "B.{} {:#X}",
                    cond_names.get(cond as usize).unwrap_or(&"?"),
                    pc.wrapping_add(imm19 as u64)
                );
            }

            // Unconditional branch with link
            (0b100, 0b0000) | (0b101, 0b0000) => {
                let imm26 = (raw & 0x3FFFFFF) as i32;
                let imm26 = ((imm26 << 6) >> 6) << 2;

                instr.branch_target = Some(pc.wrapping_add(imm26 as u64));
                instr.dst_regs.push(Reg(30)); // X30 = LR
                instr.opcode = OpcodeType::Branch;
                instr.disasm = format!("BL {:#X}", pc.wrapping_add(imm26 as u64));
            }

            // Compare and branch (CBZ, CBNZ)
            (0b011, 0b0000) | (0b011, 0b0001) | (0b011, 0b0010) | (0b011, 0b0011) => {
                let rt = (raw & 0x1F) as u8;
                let imm19 = ((raw >> 5) & 0x7FFFF) as i32;
                let imm19 = ((imm19 << 13) >> 13) << 2;

                instr.branch_target = Some(pc.wrapping_add(imm19 as u64));
                instr.is_conditional = true;
                instr.src_regs.push(Reg(rt));
                instr.opcode = OpcodeType::BranchCond;

                let op_names = ["CBZ", "CBNZ", "CBZ", "CBNZ"];
                let idx = ((op2 >> 1) & 1) as usize;
                instr.disasm = format!(
                    "{} {}{}, {:#X}",
                    op_names[idx],
                    reg_prefix,
                    rt,
                    pc.wrapping_add(imm19 as u64)
                );
            }

            // Test and branch (TBZ, TBNZ)
            (0b011, 0b0100) | (0b011, 0b0101) => {
                let rt = (raw & 0x1F) as u8;
                let imm14 = ((raw >> 5) & 0x3FFF) as i32;
                let imm14 = ((imm14 << 18) >> 18) << 2;
                let bit = ((raw >> 19) & 0x1F) as u8 | (((raw >> 31) & 1) << 5) as u8;

                instr.branch_target = Some(pc.wrapping_add(imm14 as u64));
                instr.is_conditional = true;
                instr.src_regs.push(Reg(rt));
                instr.opcode = OpcodeType::BranchCond;

                let op_name = if (op2 & 1) == 0 { "TBZ" } else { "TBNZ" };
                instr.disasm = format!(
                    "{} {}{}, #{}, {:#X}",
                    op_name,
                    reg_prefix,
                    rt,
                    bit,
                    pc.wrapping_add(imm14 as u64)
                );
            }

            // RET
            (0b010, 0b1111) => {
                let rn = ((raw >> 5) & 0x1F) as u8;
                instr.src_regs.push(Reg(rn));
                instr.opcode = OpcodeType::BranchReg;
                instr.disasm = format!("RET X{}", rn);
            }

            // BR
            (0b000, 0b1111) => {
                let rn = ((raw >> 5) & 0x1F) as u8;
                instr.src_regs.push(Reg(rn));
                instr.opcode = OpcodeType::BranchReg;
                instr.disasm = format!("BR X{}", rn);
            }

            // BLR
            (0b001, 0b1111) => {
                let rn = ((raw >> 5) & 0x1F) as u8;
                instr.src_regs.push(Reg(rn));
                instr.dst_regs.push(Reg(30));
                instr.opcode = OpcodeType::BranchReg;
                instr.disasm = format!("BLR X{}", rn);
            }

            // System instructions (HINT, NOP, etc.)
            (0b110, 0b0000) => {
                let crm = ((raw >> 8) & 0xF) as u8;
                let op2 = ((raw >> 5) & 0x7) as u8;

                // Check for HINT instructions
                if crm == 0 && op2 == 0 {
                    instr.opcode = OpcodeType::Nop;
                    instr.disasm = "NOP".to_string();
                } else {
                    instr.opcode = OpcodeType::Sys;
                    instr.disasm = format!("HINT #{}", crm);
                }
            }

            // System register access (MRS, MSR)
            (0b110, 0b0010) | (0b110, 0b0011) => {
                instr.opcode = if (raw >> 21) & 1 == 0 {
                    OpcodeType::Mrs
                } else {
                    OpcodeType::Msr
                };
                instr.disasm = if (raw >> 21) & 1 == 0 {
                    "MRS".to_string()
                } else {
                    "MSR".to_string()
                };
            }

            _ => {
                // More system instructions
                if op == 0b110 {
                    instr.opcode = OpcodeType::Sys;
                    instr.disasm = format!(".word 0x{:08X} (system)", raw);
                } else {
                    instr.opcode = OpcodeType::Other;
                    instr.disasm = format!(".word 0x{:08X}", raw);
                }
            }
        }
    }

    fn decode_load_store(&self, instr: &mut DecodedInstruction, raw: u32) {
        let size = (raw >> 30) & 0x3;
        let is_64bit = (raw >> 31) & 1 == 1;

        // Load/store register pair (offset, pre-indexed, post-indexed)
        // bits[28:27] = 10 for pair instructions
        // bits[26] = 0 for pair, 1 for single
        if (raw >> 27) & 0x1E == 0b10100 {
            let is_load = (raw >> 22) & 1 == 1;
            let rt = (raw & 0x1F) as u8;
            let rt2 = ((raw >> 10) & 0x1F) as u8;
            let rn = ((raw >> 5) & 0x1F) as u8;
            let imm7 = ((raw >> 15) & 0x7F) as i8 as i64;
            let imm = imm7 << (2 + size);

            instr.src_regs.push(Reg(rn));
            if is_load {
                instr.dst_regs.push(Reg(rt));
                instr.dst_regs.push(Reg(rt2));
            } else {
                instr.src_regs.push(Reg(rt));
                instr.src_regs.push(Reg(rt2));
            }
            instr.is_load = is_load;
            instr.mem_size = Some(8 << size);
            instr.opcode = if is_load {
                OpcodeType::LoadPair
            } else {
                OpcodeType::StorePair
            };

            // Determine addressing mode
            let opc = (raw >> 23) & 0x7;
            let (op, suffix) = if is_load { ("LDP", "") } else { ("STP", "") };

            let addr_mode = match opc {
                0b010 => "!",  // Pre-indexed
                0b011 => "!",  // Pre-indexed
                _ => "",
            };

            let rn_str = if rn == 31 { "SP".to_string() } else { format!("X{}", rn) };

            if imm == 0 && suffix.is_empty() {
                instr.disasm = format!("{} X{}, X{}, [{}]", op, rt, rt2, rn_str);
            } else if !addr_mode.is_empty() {
                instr.disasm = format!("{} X{}, X{}, [{}, {:#X}]{}", op, rt, rt2, rn_str, imm, addr_mode);
            } else {
                instr.disasm = format!("{} X{}, X{}, [{}, {:#X}]", op, rt, rt2, rn_str, imm);
            }
            return;
        }

        // Load/store register (unsigned immediate)
        // bits[31:27] = 11111 for unsigned immediate (LDR/STR with 64-bit)
        if (raw >> 27) & 0x1F == 0b11111 {
            let rt = (raw & 0x1F) as u8;
            let rn = ((raw >> 5) & 0x1F) as u8;
            let imm12 = ((raw >> 10) & 0xFFF) as u64;
            let is_load = (raw >> 22) & 1 == 1;

            instr.src_regs.push(Reg(rn));
            if is_load {
                instr.dst_regs.push(Reg(rt));
            } else {
                instr.src_regs.push(Reg(rt));
            }

            instr.is_load = is_load;
            instr.mem_size = Some(1 << size);
            instr.opcode = if is_load {
                OpcodeType::Load
            } else {
                OpcodeType::Store
            };

            let op = if is_load { "LDR" } else { "STR" };
            let size_name = match size {
                0 => "B",
                1 => "H",
                2 => "W",
                3 => "",
                _ => "?",
            };
            let rn_str = if rn == 31 { "SP".to_string() } else { format!("X{}", rn) };
            let rt_str = if size == 3 || is_load { format!("X{}", rt) } else { format!("W{}", rt) };

            if imm12 == 0 {
                instr.disasm = format!("{}{} {}, [{}]", op, size_name, rt_str, rn_str);
            } else {
                instr.disasm = format!("{}{} {}, [{}, {:#X}]", op, size_name, rt_str, rn_str, imm12 << size);
            }
            return;
        }

        // Load/store register (register offset)
        // bits[29:21] = 0b111000000 for register offset
        if (raw >> 21) & 0x1FF == 0b111000000 {
            let rt = (raw & 0x1F) as u8;
            let rn = ((raw >> 5) & 0x1F) as u8;
            let rm = ((raw >> 16) & 0x1F) as u8;
            let is_load = (raw >> 22) & 1 == 1;
            let option = (raw >> 13) & 0x7;
            let s = (raw >> 12) & 1;

            instr.src_regs.push(Reg(rn));
            instr.src_regs.push(Reg(rm));
            if is_load {
                instr.dst_regs.push(Reg(rt));
            } else {
                instr.src_regs.push(Reg(rt));
            }

            instr.is_load = is_load;
            instr.mem_size = Some(1 << size);
            instr.opcode = if is_load {
                OpcodeType::Load
            } else {
                OpcodeType::Store
            };

            let op = if is_load { "LDR" } else { "STR" };
            let extend_name = match option {
                0b010 => "UXTW",
                0b011 => "LSL",
                0b110 => "SXTW",
                0b111 => "SXTX",
                _ => "",
            };

            let rn_str = if rn == 31 { "SP".to_string() } else { format!("X{}", rn) };
            let rm_str = if rm == 31 { "SP".to_string() } else { format!("X{}", rm) };

            if s == 1 && option == 0b011 {
                instr.disasm = format!("{} X{}, [{}, X{}, LSL #{}]", op, rt, rn_str, rm_str, size);
            } else if !extend_name.is_empty() {
                instr.disasm = format!("{} X{}, [{}, {}, {}]", op, rt, rn_str, rm_str, extend_name);
            } else {
                instr.disasm = format!("{} X{}, [{}, X{}]", op, rt, rn_str, rm_str);
            }
            return;
        }

        // Load/store register (immediate post-indexed)
        // bits[29:22] = 0b11100000
        if (raw >> 22) & 0xFF == 0b11100000 {
            let rt = (raw & 0x1F) as u8;
            let rn = ((raw >> 5) & 0x1F) as u8;
            let imm9_raw = ((raw >> 12) & 0x1FF) as i16;
            let imm9 = (imm9_raw << 7) >> 7; // Sign extend 9-bit
            let is_load = (raw >> 22) & 1 == 0; // Check bit 22

            instr.src_regs.push(Reg(rn));
            if is_load {
                instr.dst_regs.push(Reg(rt));
            } else {
                instr.src_regs.push(Reg(rt));
            }

            instr.is_load = is_load;
            instr.mem_size = Some(1 << size);
            instr.opcode = if is_load {
                OpcodeType::Load
            } else {
                OpcodeType::Store
            };

            let op = if is_load { "LDR" } else { "STR" };
            let rn_str = if rn == 31 { "SP".to_string() } else { format!("X{}", rn) };

            instr.disasm = format!("{} X{}, [{}], {:#X}", op, rt, rn_str, imm9);
            return;
        }

        // Load/store register (immediate pre-indexed)
        // bits[29:22] = 0b11100010
        if (raw >> 22) & 0xFF == 0b11100010 {
            let rt = (raw & 0x1F) as u8;
            let rn = ((raw >> 5) & 0x1F) as u8;
            let imm9_raw = ((raw >> 12) & 0x1FF) as i16;
            let imm9 = (imm9_raw << 7) >> 7; // Sign extend 9-bit
            let is_load = (raw >> 22) & 1 == 0; // Check bit 22

            instr.src_regs.push(Reg(rn));
            if is_load {
                instr.dst_regs.push(Reg(rt));
            } else {
                instr.src_regs.push(Reg(rt));
            }

            instr.is_load = is_load;
            instr.mem_size = Some(1 << size);
            instr.opcode = if is_load {
                OpcodeType::Load
            } else {
                OpcodeType::Store
            };

            let op = if is_load { "LDR" } else { "STR" };
            let rn_str = if rn == 31 { "SP".to_string() } else { format!("X{}", rn) };

            instr.disasm = format!("{} X{}, [{}, {:#X}]!", op, rt, rn_str, imm9);
            return;
        }

        // Load register (literal) - PC-relative
        // bits[31:27] = 0x0_0110 or 0x1_0110
        if (raw >> 27) & 0x1F == 0b00011 || (raw >> 27) & 0x1F == 0b10011 {
            let rt = (raw & 0x1F) as u8;
            let imm19 = ((raw >> 5) & 0x7FFFF) as i32;
            let imm19 = ((imm19 << 13) >> 13) << 2;

            instr.dst_regs.push(Reg(rt));
            instr.is_load = true;
            instr.mem_size = Some(if is_64bit { 8 } else { 4 });
            instr.mem_addr = Some(instr.pc.wrapping_add(imm19 as u64));
            instr.opcode = OpcodeType::Load;

            instr.disasm = format!("LDR X{}, {:#X}", rt, instr.pc.wrapping_add(imm19 as u64));
            return;
        }

        // Default: unhandled load/store
        instr.opcode = OpcodeType::Other;
        instr.disasm = format!(".word 0x{:08X} (load/store)", raw);
    }

    fn decode_data_reg(&self, instr: &mut DecodedInstruction, raw: u32) {
        let is_64bit = (raw >> 31) & 1 == 1;
        let reg_prefix = if is_64bit { "X" } else { "W" };
        let rd = (raw & 0x1F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;
        let rm = ((raw >> 16) & 0x1F) as u8;

        instr.dst_regs.push(Reg(rd));
        instr.src_regs.push(Reg(rn));

        // Check for Data-processing (3 source) - MADD, MSUB, etc.
        if (raw >> 24) & 0x1F == 0b00011 {
            let ra = ((raw >> 10) & 0x1F) as u8;
            let op = (raw >> 29) & 0x3;
            instr.src_regs.push(Reg(rm));
            if ra != 31 {
                instr.src_regs.push(Reg(ra));
            }

            let op_name = match op {
                0b00 => "MADD",
                0b01 => "MSUB",
                _ => "MADD",
            };
            instr.opcode = OpcodeType::Mul;
            instr.disasm = format!("{} {}{}, {}{}, {}{}, {}{}", op_name, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm, reg_prefix, ra);
            return;
        }

        // Check for Data-processing (2 source) - DIV, LSL, LSR, ASR, etc.
        if (raw >> 24) & 0x1F == 0b00010 {
            let opcode = (raw >> 10) & 0x3F;
            instr.src_regs.push(Reg(rm));

            let (op_name, opcode_type) = match opcode {
                0b000000 => ("UDIV", OpcodeType::Div),
                0b000001 => ("SDIV", OpcodeType::Div),
                0b001000 => ("LSLV", OpcodeType::Shift),
                0b001001 => ("LSRV", OpcodeType::Shift),
                0b001010 => ("ASRV", OpcodeType::Shift),
                0b001011 => ("RORV", OpcodeType::Shift),
                _ => ("DATA2SRC", OpcodeType::Other),
            };
            instr.opcode = opcode_type;
            instr.disasm = format!("{} {}{}, {}{}, {}{}", op_name, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm);
            return;
        }

        // Check for Data-processing (1 source) - RBIT, REV, CLZ, etc.
        if (raw >> 24) & 0x1F == 0b00001 {
            let opcode = (raw >> 10) & 0x3F;
            let rm = ((raw >> 16) & 0x1F) as u8;

            // For 1-source, there's usually only one source register
            let (op_name, opcode_type) = match opcode {
                0b000000 => ("RBIT", OpcodeType::Other),
                0b000001 => ("REV16", OpcodeType::Other),
                0b000010 => if is_64bit { ("REV32", OpcodeType::Other) } else { ("REV", OpcodeType::Other) },
                0b000011 => if is_64bit { ("REV", OpcodeType::Other) } else { ("REV16", OpcodeType::Other) },
                0b000100 => ("CLZ", OpcodeType::Other),
                0b000101 => ("CLS", OpcodeType::Other),
                _ => ("DATA1SRC", OpcodeType::Other),
            };
            instr.opcode = opcode_type;

            if rm != 31 {
                instr.disasm = format!("{} {}{}, {}{}", op_name, reg_prefix, rd, reg_prefix, rm);
            } else {
                instr.disasm = format!("{} {}{}, {}{}", op_name, reg_prefix, rd, reg_prefix, rn);
            }
            return;
        }

        // Add/subtract shifted register
        if (raw >> 24) & 0x7 == 0b010 {
            let is_sub = (raw >> 30) & 1 == 1;
            let sets_flags = (raw >> 29) & 1 == 1;
            let shift = ((raw >> 22) & 0x3) as u8;
            let amount = ((raw >> 10) & 0x3F) as u8;

            instr.src_regs.push(Reg(rm));

            if is_sub && sets_flags {
                instr.opcode = OpcodeType::Cmp;
            } else {
                instr.opcode = if is_sub { OpcodeType::Sub } else { OpcodeType::Add };
            }

            let op_name = if is_sub {
                if sets_flags { "SUBS" } else { "SUB" }
            } else {
                if sets_flags { "ADDS" } else { "ADD" }
            };

            let shift_name = match shift {
                0 => "LSL",
                1 => "LSR",
                2 => "ASR",
                _ => "ROR",
            };

            instr.disasm = if amount > 0 {
                format!("{} {}{}, {}{}, {}{}, {} #{}", op_name, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm, shift_name, amount)
            } else {
                format!("{} {}{}, {}{}, {}{}", op_name, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm)
            };
            return;
        }

        // Add/subtract extended register
        if (raw >> 24) & 0x7 == 0b011 {
            let is_sub = (raw >> 30) & 1 == 1;
            let sets_flags = (raw >> 29) & 1 == 1;
            let option = ((raw >> 13) & 0x7) as u8;
            let imm3 = ((raw >> 10) & 0x7) as u8;

            instr.src_regs.push(Reg(rm));

            if is_sub && sets_flags {
                instr.opcode = OpcodeType::Cmp;
            } else {
                instr.opcode = if is_sub { OpcodeType::Sub } else { OpcodeType::Add };
            }

            let op_name = if is_sub {
                if sets_flags { "SUBS" } else { "SUB" }
            } else {
                if sets_flags { "ADDS" } else { "ADD" }
            };

            let extend_name = match option {
                0b000 => "UXTB",
                0b001 => "UXTH",
                0b010 => "UXTW",
                0b011 => "UXTX",
                0b100 => "SXTB",
                0b101 => "SXTH",
                0b110 => "SXTW",
                0b111 => "SXTX",
                _ => "???",
            };

            instr.disasm = if imm3 > 0 {
                format!("{} {}{}, {}{}, {}{}, {} #{}", op_name, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm, extend_name, imm3)
            } else {
                format!("{} {}{}, {}{}, {}{}, {}", op_name, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm, extend_name)
            };
            return;
        }

        // Logical shifted register
        if (raw >> 24) & 0x7 == 0b000 {
            let opc = (raw >> 29) & 0x3;
            let shift = ((raw >> 22) & 0x3) as u8;
            let amount = ((raw >> 10) & 0x3F) as u8;

            instr.src_regs.push(Reg(rm));

            // Check for MOV (register) - alias for ORR Xd, XZR, Xm
            if opc == 0b01 && rn == 31 && amount == 0 {
                instr.opcode = OpcodeType::Mov;
                instr.disasm = format!("MOV {}{}, {}{}", reg_prefix, rd, reg_prefix, rm);
                return;
            }

            instr.opcode = match opc {
                0b00 => OpcodeType::And,
                0b01 => OpcodeType::Orr,
                0b10 => OpcodeType::Eor,
                _ => OpcodeType::And,
            };

            let op_name = match opc {
                0b00 => "AND",
                0b01 => "ORR",
                0b10 => "EOR",
                _ => "ANDS",
            };

            let shift_name = match shift {
                0 => "LSL",
                1 => "LSR",
                2 => "ASR",
                _ => "ROR",
            };

            if amount > 0 {
                instr.disasm = format!("{} {}{}, {}{}, {}{}, {} #{}", op_name, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm, shift_name, amount);
            } else {
                instr.disasm = format!("{} {}{}, {}{}, {}{}", op_name, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm);
            }
            return;
        }

        // Conditional select (CSEL, CSINC, CSINV, CSNEG)
        if (raw >> 24) & 0x1F == 0b01101 {
            let rm = ((raw >> 16) & 0x1F) as u8;
            let cond = (raw & 0xF) as u8;
            let op2 = ((raw >> 10) & 0x3) as u8;

            instr.src_regs.push(Reg(rm));

            let cond_names = [
                "EQ", "NE", "CS", "CC", "MI", "PL", "VS", "VC",
                "HI", "LS", "GE", "LT", "GT", "LE", "AL", "NV",
            ];

            let (op_name, opcode_type) = match op2 {
                0b00 => ("CSEL", OpcodeType::Other),
                0b01 => ("CSINC", OpcodeType::Other),
                0b10 => ("CSINV", OpcodeType::Other),
                _ => ("CSNEG", OpcodeType::Other),
            };
            instr.opcode = opcode_type;

            instr.disasm = format!("{} {}{}, {}{}, {}{}, {}",
                op_name, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm, cond_names[cond as usize]);
            return;
        }

        // Default
        instr.opcode = OpcodeType::Other;
        instr.disasm = format!(".word 0x{:08X} (data reg)", raw);
    }

    fn decode_simd_fp(&self, instr: &mut DecodedInstruction, raw: u32) {
        let rd = (raw & 0x1F) as u8;
        let rn = ((raw >> 5) & 0x1F) as u8;

        // FP data processing - check bits[31:24] for the instruction class
        let bits_31_24 = (raw >> 24) & 0xFF;
        let ftype = (raw >> 22) & 0x3;

        // Scalar FP arithmetic (FADD, FSUB, FMUL, FDIV)
        // Encoding: 0001 1110 00 1 m opcode 10 n d
        // bits[31:24] = 0x1E
        if bits_31_24 == 0x1E {
            let rm = ((raw >> 16) & 0x1F) as u8;
            let opcode = (raw >> 12) & 0xF;

            instr.dst_vregs.push(VReg(rd));
            instr.src_vregs.push(VReg(rn));
            instr.src_vregs.push(VReg(rm));

            instr.opcode = match opcode {
                0b0000 => OpcodeType::Fadd,
                0b0001 => OpcodeType::Fsub,
                0b0010 => OpcodeType::Fmul,
                0b0011 => OpcodeType::Fdiv,
                _ => OpcodeType::Other,
            };

            let op_name = match opcode {
                0b0000 => "FADD",
                0b0001 => "FSUB",
                0b0010 => "FMUL",
                0b0011 => "FDIV",
                _ => "F???",
            };

            let size_name = match ftype {
                0 => "S",
                1 => "D",
                _ => "?",
            };

            instr.disasm = format!("{} {}{}, {}{}, {}{}",
                op_name, size_name, rd, size_name, rn, size_name, rm);
            return;
        }

        // FMOV, FABS, FNEG (register-to-register)
        // Encoding: 0001 1110 00 1 00000 000 n d
        // bits[31:24] = 0x1E, bits[15:10] = 000000
        if bits_31_24 == 0x1E && (raw >> 10) & 0x3F == 0 {
            instr.dst_vregs.push(VReg(rd));
            instr.src_vregs.push(VReg(rn));

            let opcode = (raw >> 12) & 0xF;

            instr.opcode = match opcode {
                0b0000 => OpcodeType::Vmov,
                _ => OpcodeType::Other,
            };

            instr.disasm = format!("FMOV D{}, D{}", rd, rn);
            return;
        }

        // SIMD vector operations
        if (raw >> 24) & 0xFF == 0b00001110 {
            let rm = ((raw >> 16) & 0x1F) as u8;
            let opcode = (raw >> 12) & 0x3;

            instr.dst_vregs.push(VReg(rd));
            instr.src_vregs.push(VReg(rn));
            instr.src_vregs.push(VReg(rm));

            instr.opcode = match opcode {
                0b00 => OpcodeType::Vadd,
                0b10 => OpcodeType::Vsub,
                0b11 => OpcodeType::Vmul,
                _ => OpcodeType::Other,
            };

            let op_name = match opcode {
                0b00 => "ADD",
                0b10 => "SUB",
                0b11 => "MUL",
                _ => "???",
            };

            instr.disasm = format!("V{} V{}.16B, V{}.16B, V{}.16B", op_name, rd, rn, rm);
            return;
        }

        // Default
        instr.opcode = OpcodeType::Other;
        instr.disasm = format!(".word 0x{:08X} (simd/fp)", raw);
    }
}

impl Default for Arm64Decoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_raw(raw: u32) -> DecodedInstruction {
        let decoder = Arm64Decoder::new();
        decoder.decode(0x1000, raw)
    }

    #[test]
    fn test_decode_nop() {
        // NOP (actually HINT #0)
        let instr = decode_raw(0xD503201F);
        assert_eq!(instr.opcode, OpcodeType::Nop);
    }

    #[test]
    fn test_decode_add() {
        // ADD X0, X1, #0x100
        let instr = decode_raw(0x91004020);
        assert_eq!(instr.opcode, OpcodeType::Add);
        assert_eq!(instr.dst_regs.len(), 1);
        assert_eq!(instr.src_regs.len(), 1);
    }

    #[test]
    fn test_decode_branch() {
        // B #0x1000 (from PC 0x1000, target 0x2000)
        // imm26 = 0x1000 >> 2 = 0x400
        // encoding = 0x14000000 | 0x400 = 0x14000400
        let decoder = Arm64Decoder::new();
        let instr = decoder.decode(0x1000, 0x14000400);
        assert_eq!(instr.opcode, OpcodeType::Branch);
        assert_eq!(instr.branch_target, Some(0x2000));
    }

    #[test]
    fn test_decode_load() {
        // LDR X0, [X1]
        let instr = decode_raw(0xF9400020);
        assert_eq!(instr.opcode, OpcodeType::Load);
        assert!(instr.is_load);
    }

    #[test]
    fn test_decode_store() {
        // STR X0, [X1]
        let instr = decode_raw(0xF9000020);
        assert_eq!(instr.opcode, OpcodeType::Store);
        assert!(!instr.is_load);
    }

    #[test]
    fn test_decode_fadd() {
        // FADD D0, D1, D2
        // Encoding: 0001 1110 01 1 m 0000 10 n d
        // bits[31:24] = 0x1E, ftype=1 (D), m=2, opcode=0000, n=1, d=0
        // = 0x1E620820
        let instr = decode_raw(0x1E620820);
        assert_eq!(instr.opcode, OpcodeType::Fadd);
    }
}
