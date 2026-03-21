//! AArch64 Branch Instructions
//!
//! This module decodes branch instructions:
//! - B, BL (unconditional)
//! - B.cond (conditional)
//! - BR, BLR (register)
//! - RET (return)
//! - CBZ, CBNZ (compare and branch)
//! - TBZ, TBNZ (test bit and branch)

use super::encoding::*;
use super::{DecodeResult, DecodedInstruction};
use crate::types::{OpcodeType, Reg, BranchInfo};

/// Decode unconditional branch (immediate)
pub fn decode_branch_imm(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_call = (raw >> 31) & 0x1 == 1;
    let imm26 = decode_imm26(raw);
    let offset = sign_extend_26(imm26) << 2;
    let target = pc.wrapping_add(offset as u64);

    decoded.opcode = if is_call {
        OpcodeType::Branch  // BL (call)
    } else {
        OpcodeType::Branch  // B
    };

    decoded.branch_info = Some(BranchInfo {
        is_conditional: false,
        target,
        is_taken: true,
    });

    let mnemonic = if is_call { "bl" } else { "b" };
    decoded.disasm = format!("{} {:#x}", mnemonic, target);

    Ok(decoded)
}

/// Decode conditional branch (immediate)
pub fn decode_branch_cond(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let imm19 = decode_imm19(raw);
    let cond = decode_condition(raw);
    let offset = sign_extend_19(imm19) << 2;
    let target = pc.wrapping_add(offset as u64);

    decoded.opcode = OpcodeType::BranchCond;

    decoded.branch_info = Some(BranchInfo {
        is_conditional: true,
        target,
        is_taken: false,  // Will be determined at runtime
    });

    decoded.disasm = format!("b.{} {:#x}", condition_name(cond), target);

    Ok(decoded)
}

/// Decode branch (register)
pub fn decode_branch_reg(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let rn = decode_rn(raw);
    let opc = (raw >> 21) & 0xF;
    let is_call = (raw >> 20) & 0x1 == 1;

    decoded.opcode = match opc {
        0x0 => OpcodeType::BranchReg,   // BR
        0x1 => OpcodeType::BranchReg,   // BLR
        0x2 => OpcodeType::BranchReg,   // RET
        0x4 => OpcodeType::Eret,        // ERET
        0x5 => OpcodeType::Eret,        // DRPS
        _ => OpcodeType::BranchReg,
    };

    if rn != 31 {
        decoded.src_regs.push(Reg(rn));
    }

    let mnemonic = match opc {
        0x0 => "br",
        0x1 => "blr",
        0x2 => "ret",
        0x4 => "eret",
        0x5 => "drps",
        _ => "br",
    };

    if opc == 0x2 {
        decoded.disasm = mnemonic.to_string();
    } else {
        decoded.disasm = format!("{} x{}", mnemonic, rn);
    }

    Ok(decoded)
}

/// Decode compare and branch
pub fn decode_compare_branch(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_64bit = is_64bit(raw);
    let is_nz = (raw >> 24) & 0x1 == 1;  // CBNZ if set
    let rt = decode_rt(raw);
    let imm19 = decode_imm19(raw);
    let offset = sign_extend_19(imm19) << 2;
    let target = pc.wrapping_add(offset as u64);

    decoded.opcode = OpcodeType::BranchCond;

    decoded.branch_info = Some(BranchInfo {
        is_conditional: true,
        target,
        is_taken: false,
    });

    if rt != 31 {
        decoded.src_regs.push(Reg(rt));
    }

    let reg_prefix = if is_64bit { 'x' } else { 'w' };
    let mnemonic = if is_nz { "cbnz" } else { "cbz" };
    decoded.disasm = format!("{} {}{}, {:#x}", mnemonic, reg_prefix, rt, target);

    Ok(decoded)
}

/// Decode test bit and branch
pub fn decode_test_branch(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_64bit = is_64bit(raw);
    let is_nz = (raw >> 24) & 0x1 == 1;  // TBNZ if set
    let rt = decode_rt(raw);
    let imm14 = ((raw >> 5) & 0x3FFF);
    let bit = ((raw >> 19) & 0x1F) as u8;
    let offset = sign_extend(imm14, 14) << 2;
    let target = pc.wrapping_add(offset as u64);

    decoded.opcode = OpcodeType::BranchCond;

    decoded.branch_info = Some(BranchInfo {
        is_conditional: true,
        target,
        is_taken: false,
    });

    if rt != 31 {
        decoded.src_regs.push(Reg(rt));
    }

    let mnemonic = if is_nz { "tbnz" } else { "tbz" };
    let full_bit = if is_64bit { bit } else { bit & 0x1F };
    decoded.disasm = format!("{} {}{}, #{}, {:#x}", mnemonic, if is_64bit { 'x' } else { 'w' }, rt, full_bit, target);

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_branch() {
        // B #0x100
        let raw = 0x14000040;
        let result = decode_branch_imm(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Branch);
        assert!(result.branch_info.is_some());
    }

    #[test]
    fn test_decode_bl() {
        // BL #0x100
        let raw = 0x94000040;
        let result = decode_branch_imm(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Branch);
        let bi = result.branch_info.unwrap();
        assert_eq!(bi.target, 0x1100);
    }

    #[test]
    fn test_decode_bcond() {
        // B.EQ #0x10
        let raw = 0x54000004;
        let result = decode_branch_cond(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::BranchCond);
        assert!(result.branch_info.unwrap().is_conditional);
    }
}
