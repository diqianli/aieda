//! AArch64 Instruction Decoder Module
//!
//! This module provides comprehensive instruction decoding for AArch64 (ARMv8-A)
//! including SIMD/NEON, cryptography extensions, atomic operations, and more.
//!
//! # Module Structure
//!
//! - [`aarch64::ArithmeticDecoder`] - Integer arithmetic instructions
//! - [`aarch64::LogicalDecoder`] - Logical and bitwise instructions
//! - [`aarch64::LoadStoreDecoder`] - Load/store instructions
//! - [`aarch64::BranchDecoder`] - Branch and control flow instructions
//! - [`aarch64::SimdDecoder`] - SIMD/NEON vector instructions
//! - [`aarch64::FpDecoder`] - Floating-point instructions
//! - [`aarch64::CryptoDecoder`] - Cryptography extension instructions
//! - [`aarch64::SystemDecoder`] - System and cache maintenance instructions

pub mod aarch64;

use crate::types::{OpcodeType, Reg, VReg, Instruction, InstructionId, MemAccess, BranchInfo};
use smallvec::SmallVec;

/// Decoding result
pub type DecodeResult = Result<DecodedInstruction, DecodeError>;

/// Decoding error
#[derive(Debug, Clone, thiserror::Error)]
pub enum DecodeError {
    #[error("Invalid instruction encoding at PC {pc:#x}: {raw:#x}")]
    InvalidEncoding { pc: u64, raw: u32 },

    #[error("Unsupported instruction at PC {pc:#x}: {message}")]
    Unsupported { pc: u64, message: String },

    #[error("Decoding failed: {0}")]
    Other(String),
}

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
    pub src_regs: SmallVec<[Reg; 4]>,
    /// Destination registers
    pub dst_regs: SmallVec<[Reg; 2]>,
    /// Source vector registers
    pub src_vregs: SmallVec<[VReg; 4]>,
    /// Destination vector registers
    pub dst_vregs: SmallVec<[VReg; 2]>,
    /// Immediate value
    pub immediate: Option<i64>,
    /// Memory access info
    pub mem_access: Option<MemAccess>,
    /// Branch info
    pub branch_info: Option<BranchInfo>,
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
            src_regs: SmallVec::new(),
            dst_regs: SmallVec::new(),
            src_vregs: SmallVec::new(),
            dst_vregs: SmallVec::new(),
            immediate: None,
            mem_access: None,
            branch_info: None,
            disasm: String::new(),
        }
    }

    /// Convert to an Instruction for the emulator
    pub fn to_instruction(&self, id: InstructionId) -> Instruction {
        let mut instr = Instruction::new(id, self.pc, self.raw, self.opcode);
        instr.src_regs = self.src_regs.clone();
        instr.dst_regs = self.dst_regs.clone();
        instr.src_vregs = self.src_vregs.clone();
        instr.dst_vregs = self.dst_vregs.clone();
        instr.mem_access = self.mem_access.clone();
        instr.branch_info = self.branch_info.clone();
        instr.disasm = Some(self.disasm.clone());
        instr
    }
}

/// Main AArch64 decoder
pub struct AArch64Decoder {
    /// Whether to decode all instructions in detail
    detailed: bool,
}

impl AArch64Decoder {
    /// Create a new decoder
    pub fn new() -> Self {
        Self { detailed: true }
    }

    /// Create a fast decoder that only extracts essential info
    pub fn fast() -> Self {
        Self { detailed: false }
    }

    /// Decode a single instruction
    pub fn decode(&self, pc: u64, raw: u32) -> DecodeResult {
        // Try each decoder category in order of frequency

        // Data processing (immediate)
        if let Some(op) = self.try_data_proc_imm(pc, raw) {
            return Ok(op);
        }

        // Branches
        if let Some(op) = self.try_branch(pc, raw) {
            return Ok(op);
        }

        // Loads and stores
        if let Some(op) = self.try_load_store(pc, raw) {
            return Ok(op);
        }

        // Data processing (register)
        if let Some(op) = self.try_data_proc_reg(pc, raw) {
            return Ok(op);
        }

        // SIMD/FP
        if let Some(op) = self.try_simd_fp(pc, raw) {
            return Ok(op);
        }

        // System instructions
        if let Some(op) = self.try_system(pc, raw) {
            return Ok(op);
        }

        // Fallback: unknown instruction
        let mut decoded = DecodedInstruction::new(pc, raw);
        decoded.disasm = format!("unknown {:08x}", raw);
        Ok(decoded)
    }

    /// Try to decode as data processing (immediate)
    fn try_data_proc_imm(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let op = (raw >> 25) & 0x1;

        if op == 0 {
            // PC-relative addressing
            self.decode_pc_rel(pc, raw)
        } else {
            // Add/subtract (immediate)
            self.decode_add_sub_imm(pc, raw)
                .or_else(|| self.decode_logical_imm(pc, raw))
                .or_else(|| self.decode_move_wide(pc, raw))
                .or_else(|| self.decode_bitfield(pc, raw))
                .or_else(|| self.decode_extract(pc, raw))
        }
    }

    /// Decode PC-relative addressing
    fn decode_pc_rel(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let op = (raw >> 24) & 0x1;

        let mut decoded = DecodedInstruction::new(pc, raw);

        if op == 0 {
            // ADRP
            decoded.opcode = OpcodeType::Adr;
            let immlo = (raw >> 29) & 0x3;
            let immhi = (raw >> 5) & 0x7FFFF;
            let imm = ((immhi << 2) | immlo) as i64;
            let target = (pc & 0xFFFF_FFFF_FFFF_F000) + (imm << 12) as u64;
            decoded.immediate = Some(target as i64);
            decoded.disasm = format!("adrp x{}, {:#x}", (raw >> 0) & 0x1F, target);
        } else {
            // ADR or ADRP variant
            decoded.opcode = OpcodeType::Adr;
            decoded.disasm = format!("adr");
        }

        decoded.dst_regs.push(Reg(((raw >> 0) & 0x1F) as u8));
        Some(decoded)
    }

    /// Decode add/subtract immediate
    fn decode_add_sub_imm(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        // Check for add/sub immediate encoding (0x11000000 - 0x17FFFFFF)
        let top_bits = (raw >> 24) & 0x1F;
        if top_bits != 0x11 && top_bits != 0x12 && top_bits != 0x13 {
            return None;
        }

        let mut decoded = DecodedInstruction::new(pc, raw);

        let is_sub = (raw >> 30) & 0x1 == 1;
        let is_64bit = (raw >> 31) & 0x1 == 1;
        let rd = (raw >> 0) & 0x1F;
        let rn = (raw >> 5) & 0x1F;
        let imm = ((raw >> 10) & 0xFFF) as u16;
        let shift = (raw >> 22) & 0x3;

        decoded.opcode = if is_sub { OpcodeType::Sub } else { OpcodeType::Add };

        if rd != 31 {
            decoded.dst_regs.push(Reg(rd as u8));
        }
        if rn != 31 {
            decoded.src_regs.push(Reg(rn as u8));
        }

        let reg_prefix = if is_64bit { 'x' } else { 'w' };
        let shift_str = if shift == 1 { ", lsl #12" } else { "" };
        decoded.disasm = format!(
            "{} {}{}, {}{}{}, #{}{}",
            if is_sub { "sub" } else { "add" },
            reg_prefix, rd,
            reg_prefix, rn,
            if rd == 31 { "sp" } else { "" },
            imm,
            shift_str
        );

        Some(decoded)
    }

    /// Decode logical immediate
    fn decode_logical_imm(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let top_bits = (raw >> 23) & 0x3F;
        if (top_bits >> 1) != 0x10 {
            return None;
        }

        let mut decoded = DecodedInstruction::new(pc, raw);

        let opc = (raw >> 29) & 0x3;
        let rn = (raw >> 5) & 0x1F;
        let rd = (raw >> 0) & 0x1F;

        decoded.opcode = match opc {
            0 => OpcodeType::And,
            1 => OpcodeType::Orr,
            2 => OpcodeType::Eor,
            3 => OpcodeType::And, // ANDS
            _ => OpcodeType::Other,
        };

        if rd != 31 {
            decoded.dst_regs.push(Reg(rd as u8));
        }
        if rn != 31 {
            decoded.src_regs.push(Reg(rn as u8));
        }

        Some(decoded)
    }

    /// Decode move wide
    fn decode_move_wide(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let top_bits = (raw >> 23) & 0x3F;
        if top_bits != 0x12 && top_bits != 0x13 {
            return None;
        }

        let mut decoded = DecodedInstruction::new(pc, raw);
        let opc = (raw >> 29) & 0x3;
        let hw = (raw >> 21) & 0x3;
        let imm = ((raw >> 5) & 0xFFFF) as u16;
        let rd = (raw >> 0) & 0x1F;

        decoded.opcode = match opc {
            0 => OpcodeType::Nop, // MOVN
            2 => OpcodeType::Mov, // MOVZ
            3 => OpcodeType::Mov, // MOVK
            _ => OpcodeType::Mov,
        };

        if rd != 31 {
            decoded.dst_regs.push(Reg(rd as u8));
        }

        decoded.immediate = Some((imm as i64) << (hw * 16));
        decoded.disasm = format!("mov x{}, #{}", rd, imm);

        Some(decoded)
    }

    /// Decode bitfield
    fn decode_bitfield(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let top_bits = (raw >> 23) & 0x3F;
        if top_bits != 0x14 {
            return None;
        }

        let mut decoded = DecodedInstruction::new(pc, raw);
        let opc = (raw >> 29) & 0x3;

        decoded.opcode = match opc {
            0 => OpcodeType::Lsl, // SBFM
            1 => OpcodeType::Lsr, // BFM
            2 => OpcodeType::Asr, // UBFM
            _ => OpcodeType::Shift,
        };

        let rn = (raw >> 5) & 0x1F;
        let rd = (raw >> 0) & 0x1F;

        if rd != 31 {
            decoded.dst_regs.push(Reg(rd as u8));
        }
        if rn != 31 {
            decoded.src_regs.push(Reg(rn as u8));
        }

        Some(decoded)
    }

    /// Decode extract
    fn decode_extract(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let top_bits = (raw >> 23) & 0x3F;
        if top_bits != 0x15 {
            return None;
        }

        let mut decoded = DecodedInstruction::new(pc, raw);
        let op21 = (raw >> 21) & 0x3;

        decoded.opcode = if op21 == 0 {
            OpcodeType::Shift // EXTR
        } else {
            OpcodeType::Shift // DEPR (deprecated)
        };

        let rm = (raw >> 16) & 0x1F;
        let rn = (raw >> 5) & 0x1F;
        let rd = (raw >> 0) & 0x1F;

        if rd != 31 {
            decoded.dst_regs.push(Reg(rd as u8));
        }
        if rn != 31 {
            decoded.src_regs.push(Reg(rn as u8));
        }
        if rm != 31 {
            decoded.src_regs.push(Reg(rm as u8));
        }

        Some(decoded)
    }

    /// Try to decode as branch
    fn try_branch(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let top_bits = (raw >> 26) & 0x3F;

        match top_bits {
            0x05 => {
                // Conditional branch (immediate)
                let mut decoded = DecodedInstruction::new(pc, raw);
                decoded.opcode = OpcodeType::BranchCond;

                let imm19 = ((raw >> 5) & 0x7FFFF) as i32;
                let offset = (imm19 << 13) >> 11; // Sign extend
                let target = pc.wrapping_add((offset as i64 as u64) * 4);

                decoded.branch_info = Some(BranchInfo {
                    is_conditional: true,
                    target,
                    is_taken: true, // Assume taken for now
                });
                decoded.disasm = format!("b.{:#x}", target);
                Some(decoded)
            }
            0x04 => {
                // Unconditional branch (immediate)
                let mut decoded = DecodedInstruction::new(pc, raw);
                decoded.opcode = OpcodeType::Branch;

                let imm26 = (raw & 0x3FFFFFF) as i32;
                let offset = (imm26 << 6) >> 4; // Sign extend
                let target = pc.wrapping_add((offset as i64 as u64) * 4);

                decoded.branch_info = Some(BranchInfo {
                    is_conditional: false,
                    target,
                    is_taken: true,
                });
                decoded.disasm = format!("b {:#x}", target);
                Some(decoded)
            }
            0x06 | 0x07 => {
                // Unconditional branch (register)
                let mut decoded = DecodedInstruction::new(pc, raw);
                decoded.opcode = OpcodeType::BranchReg;

                let rn = (raw >> 5) & 0x1F;
                if rn != 31 {
                    decoded.src_regs.push(Reg(rn as u8));
                }
                decoded.disasm = format!("br x{}", rn);
                Some(decoded)
            }
            0x25 | 0x27 => {
                // Compare and branch
                let mut decoded = DecodedInstruction::new(pc, raw);
                decoded.opcode = OpcodeType::BranchCond;

                let rt = (raw >> 0) & 0x1F;
                let sf = (raw >> 31) & 0x1;

                if rt != 31 {
                    decoded.src_regs.push(Reg(rt as u8));
                }

                let imm19 = ((raw >> 5) & 0x7FFFF) as i32;
                let offset = (imm19 << 13) >> 11;
                let target = pc.wrapping_add((offset as i64 as u64) * 4);

                decoded.branch_info = Some(BranchInfo {
                    is_conditional: true,
                    target,
                    is_taken: true,
                });
                decoded.disasm = if sf == 1 {
                    format!("cbz x{}, {:#x}", rt, target)
                } else {
                    format!("cbz w{}, {:#x}", rt, target)
                };
                Some(decoded)
            }
            _ => None,
        }
    }

    /// Try to decode as load/store
    fn try_load_store(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let op0 = (raw >> 28) & 0x7;

        match op0 {
            0x4 | 0x5 | 0x6 | 0x7 => {
                // Load/store register
                self.decode_load_store_reg(pc, raw)
            }
            0x0 | 0x1 | 0x2 | 0x3 => {
                // Load/store exclusive or pair
                self.decode_load_store_pair_or_excl(pc, raw)
            }
            _ => None,
        }
    }

    /// Decode load/store register
    fn decode_load_store_reg(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let mut decoded = DecodedInstruction::new(pc, raw);

        let size = (raw >> 30) & 0x3;
        let is_64bit = (raw >> 31) & 0x1 == 1;
        let is_load = (raw >> 22) & 0x1 == 1;

        let rt = (raw >> 0) & 0x1F;
        let rn = (raw >> 5) & 0x1F;

        decoded.opcode = if is_load {
            OpcodeType::Load
        } else {
            OpcodeType::Store
        };

        if rt != 31 {
            if is_64bit || size == 3 {
                decoded.dst_regs.push(Reg(rt as u8));
            } else {
                decoded.dst_regs.push(Reg(rt as u8));
            }
        }
        if rn != 31 {
            decoded.src_regs.push(Reg(rn as u8));
        }

        // Simple immediate offset
        let imm = ((raw >> 10) & 0xFFF) as u16;
        let access_size = 1 << size;

        decoded.mem_access = Some(MemAccess {
            addr: 0, // Will be computed at runtime
            size: access_size,
            is_load,
        });

        decoded.disasm = if is_load {
            format!("ldr x{}, [x{}, #{}]", rt, rn, imm)
        } else {
            format!("str x{}, [x{}, #{}]", rt, rn, imm)
        };

        Some(decoded)
    }

    /// Decode load/store pair or exclusive
    fn decode_load_store_pair_or_excl(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let op1 = (raw >> 23) & 0x1;

        if op1 == 0 {
            // Load/store pair
            let mut decoded = DecodedInstruction::new(pc, raw);

            let is_load = (raw >> 22) & 0x1 == 1;
            let l = (raw >> 22) & 0x1;

            decoded.opcode = if l == 1 {
                OpcodeType::LoadPair
            } else {
                OpcodeType::StorePair
            };

            let rt = (raw >> 0) & 0x1F;
            let rt2 = (raw >> 10) & 0x1F;
            let rn = (raw >> 5) & 0x1F;

            if rt != 31 {
                decoded.dst_regs.push(Reg(rt as u8));
            }
            if rt2 != 31 && is_load {
                decoded.dst_regs.push(Reg(rt2 as u8));
            }
            if rn != 31 {
                decoded.src_regs.push(Reg(rn as u8));
            }

            decoded.mem_access = Some(MemAccess {
                addr: 0,
                size: 16, // Pair is typically 16 bytes
                is_load,
            });

            decoded.disasm = if is_load {
                format!("ldp x{}, x{}, [x{}]", rt, rt2, rn)
            } else {
                format!("stp x{}, x{}, [x{}]", rt, rt2, rn)
            };

            Some(decoded)
        } else {
            // Load/store exclusive
            None // Simplified for now
        }
    }

    /// Try to decode as data processing (register)
    fn try_data_proc_reg(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let top_bits = (raw >> 24) & 0x1F;

        match top_bits {
            0x0A | 0x0B => {
                // Data processing - register
                self.decode_data_proc_reg_op(pc, raw)
            }
            0x08 | 0x09 => {
                // Data processing - add/sub (extended register)
                self.decode_add_sub_ext(pc, raw)
            }
            _ => None,
        }
    }

    /// Decode data processing register operation
    fn decode_data_proc_reg_op(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let mut decoded = DecodedInstruction::new(pc, raw);

        let opc = (raw >> 29) & 0x7;
        let is_64bit = (raw >> 31) & 0x1 == 1;

        let rd = (raw >> 0) & 0x1F;
        let rn = (raw >> 5) & 0x1F;
        let rm = (raw >> 16) & 0x1F;

        decoded.opcode = match opc {
            0x0 | 0x4 => OpcodeType::Add,
            0x1 | 0x5 => OpcodeType::Mul,
            0x2 | 0x6 => OpcodeType::Sub,
            0x3 | 0x7 => OpcodeType::Div,
            _ => OpcodeType::Other,
        };

        if rd != 31 {
            decoded.dst_regs.push(Reg(rd as u8));
        }
        if rn != 31 {
            decoded.src_regs.push(Reg(rn as u8));
        }
        if rm != 31 {
            decoded.src_regs.push(Reg(rm as u8));
        }

        let reg_prefix = if is_64bit { 'x' } else { 'w' };
        decoded.disasm = format!(
            "{:?} {}{}, {}{}, {}{}",
            decoded.opcode, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm
        );

        Some(decoded)
    }

    /// Decode add/sub extended register
    fn decode_add_sub_ext(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let mut decoded = DecodedInstruction::new(pc, raw);

        let is_sub = (raw >> 30) & 0x1 == 1;
        let is_64bit = (raw >> 31) & 0x1 == 1;

        decoded.opcode = if is_sub {
            OpcodeType::Sub
        } else {
            OpcodeType::Add
        };

        let rd = (raw >> 0) & 0x1F;
        let rn = (raw >> 5) & 0x1F;
        let rm = (raw >> 16) & 0x1F;

        if rd != 31 {
            decoded.dst_regs.push(Reg(rd as u8));
        }
        if rn != 31 {
            decoded.src_regs.push(Reg(rn as u8));
        }
        if rm != 31 {
            decoded.src_regs.push(Reg(rm as u8));
        }

        let reg_prefix = if is_64bit { 'x' } else { 'w' };
        decoded.disasm = format!(
            "{} {}{}, {}{}, {}{}",
            if is_sub { "sub" } else { "add" },
            reg_prefix, rd, reg_prefix, rn, reg_prefix, rm
        );

        Some(decoded)
    }

    /// Try to decode as SIMD/FP
    fn try_simd_fp(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let top_bits = (raw >> 24) & 0x1F;

        if top_bits == 0x0E || top_bits == 0x0F {
            // SIMD/FP instruction
            let mut decoded = DecodedInstruction::new(pc, raw);

            let is_fp = (raw >> 28) & 0x1 == 1;
            let size = (raw >> 22) & 0x3;

            if is_fp {
                // Floating-point
                let opc = (raw >> 12) & 0xF;

                decoded.opcode = match opc {
                    0x0 => OpcodeType::Fadd,
                    0x2 => OpcodeType::Fsub,
                    0x8 => OpcodeType::Fmul,
                    0xA => OpcodeType::Fdiv,
                    0x1 | 0x5 => OpcodeType::Fmadd,
                    _ => OpcodeType::Fadd,
                };
            } else {
                // SIMD
                let opc = (raw >> 12) & 0xF;

                decoded.opcode = match opc {
                    0x0 => OpcodeType::Vadd,
                    0x2 => OpcodeType::Vsub,
                    0x8 => OpcodeType::Vmul,
                    0xA => OpcodeType::Vmla,
                    _ => OpcodeType::Vadd,
                };
            }

            let rd = (raw >> 0) & 0x1F;
            let rn = (raw >> 5) & 0x1F;

            decoded.dst_vregs.push(VReg(rd as u8));
            decoded.src_vregs.push(VReg(rn as u8));

            decoded.disasm = format!("{:?} v{}, v{}", decoded.opcode, rd, rn);

            Some(decoded)
        } else {
            None
        }
    }

    /// Try to decode as system instruction
    fn try_system(&self, pc: u64, raw: u32) -> Option<DecodedInstruction> {
        let top_bits = (raw >> 24) & 0x1F;

        if top_bits == 0x14 || top_bits == 0x15 || top_bits == 0x16 || top_bits == 0x17 {
            let mut decoded = DecodedInstruction::new(pc, raw);

            let crm = (raw >> 8) & 0xF;
            let op1 = (raw >> 16) & 0x7;
            let op2 = (raw >> 5) & 0x7;

            // HINT instructions (NOP, YIELD, etc.)
            if op1 == 0 && op2 == 0 {
                match crm {
                    0 => decoded.opcode = OpcodeType::Nop,
                    1 => decoded.opcode = OpcodeType::Yield,
                    _ => decoded.opcode = OpcodeType::Nop,
                }
                decoded.disasm = format!("{:?}", decoded.opcode);
                return Some(decoded);
            }

            // Cache maintenance
            if op1 == 3 {
                decoded.opcode = match crm {
                    1 => OpcodeType::IcIvau,
                    2 => OpcodeType::IcIallu,
                    _ => OpcodeType::Sys,
                };
                decoded.disasm = format!("{:?}", decoded.opcode);
                return Some(decoded);
            }

            if op1 == 7 {
                decoded.opcode = match crm {
                    1 => OpcodeType::DcCivac,
                    2 => OpcodeType::DcCvac,
                    4 => OpcodeType::DcZva,
                    _ => OpcodeType::Sys,
                };
                decoded.disasm = format!("{:?}", decoded.opcode);
                return Some(decoded);
            }

            decoded.opcode = OpcodeType::Sys;
            decoded.disasm = "sys".to_string();
            Some(decoded)
        } else if top_bits == 0x18 || top_bits == 0x19 {
            // MSR/MRS
            let mut decoded = DecodedInstruction::new(pc, raw);
            let l = (raw >> 21) & 0x1;

            decoded.opcode = if l == 1 {
                OpcodeType::Mrs
            } else {
                OpcodeType::Msr
            };

            let rt = (raw >> 0) & 0x1F;
            if rt != 31 {
                decoded.dst_regs.push(Reg(rt as u8));
            }

            decoded.disasm = format!("{:?}", decoded.opcode);
            Some(decoded)
        } else {
            None
        }
    }
}

impl Default for AArch64Decoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_add() {
        let decoder = AArch64Decoder::new();
        // ADD X0, X1, X2
        let raw = 0x8B020020;
        let result = decoder.decode(0x1000, raw).unwrap();

        assert_eq!(result.opcode, OpcodeType::Add);
        assert_eq!(result.dst_regs.len(), 1);
        assert_eq!(result.src_regs.len(), 2);
    }

    #[test]
    fn test_decode_branch() {
        let decoder = AArch64Decoder::new();
        // B #0x100
        let raw = 0x14000040;
        let result = decoder.decode(0x1000, raw).unwrap();

        assert_eq!(result.opcode, OpcodeType::Branch);
        assert!(result.branch_info.is_some());
    }

    #[test]
    fn test_decode_load() {
        let decoder = AArch64Decoder::new();
        // LDR X0, [X1]
        let raw = 0xF9400020;
        let result = decoder.decode(0x1000, raw).unwrap();

        assert_eq!(result.opcode, OpcodeType::Load);
        assert!(result.mem_access.is_some());
    }
}
