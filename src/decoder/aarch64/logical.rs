//! AArch64 Logical Instructions
//!
//! This module decodes logical and bitwise instructions:
//! - AND, ANDS, ORR, EOR, EON
//! - BIC, BICS, ORN
//! - MOV (from ORR alias)
//! - MVN (from ORN alias)
//! - TST (from ANDS alias)

use super::encoding::*;
use super::{DecodeResult, DecodedInstruction};
use crate::types::{OpcodeType, Reg};

/// Decode logical (immediate) instructions
pub fn decode_logical_imm(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_64bit = is_64bit(raw);
    let opc = (raw >> 29) & 0x3;
    let rn = decode_rn(raw);
    let rd = decode_rd(raw);
    let immr = ((raw >> 16) & 0x3F) as u8;
    let imms = ((raw >> 10) & 0x3F) as u8;

    decoded.opcode = match opc {
        0 => OpcodeType::And,  // AND
        1 => OpcodeType::Orr,  // ORR
        2 => OpcodeType::Eor,  // EOR
        3 => OpcodeType::And,  // ANDS
        _ => OpcodeType::And,
    };

    if rd != 31 {
        decoded.dst_regs.push(Reg(rd));
    }
    if rn != 31 {
        decoded.src_regs.push(Reg(rn));
    }

    let reg_prefix = if is_64bit { 'x' } else { 'w' };
    let mnemonic = match opc {
        0 => "and",
        1 => "orr",
        2 => "eor",
        3 => "ands",
        _ => "and",
    };
    decoded.disasm = format!(
        "{} {}{}, {}{}, #0x{:02x}",
        mnemonic, reg_prefix, rd, reg_prefix, rn, ((immr as u16) << 8) | (imms as u16)
    );

    Ok(decoded)
}

/// Decode logical (shifted register) instructions
pub fn decode_logical_shifted(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_64bit = is_64bit(raw);
    let opc = (raw >> 29) & 0x3;
    let shift = decode_shift(raw);
    let rm = decode_rm(raw);
    let imm6 = ((raw >> 10) & 0x3F) as u8;
    let rn = decode_rn(raw);
    let rd = decode_rd(raw);

    decoded.opcode = match opc {
        0 => OpcodeType::And,
        1 => OpcodeType::Orr,
        2 => OpcodeType::Eor,
        3 => OpcodeType::And,  // ANDS/TST
        _ => OpcodeType::And,
    };

    if rd != 31 {
        decoded.dst_regs.push(Reg(rd));
    }
    if rn != 31 {
        decoded.src_regs.push(Reg(rn));
    }
    if rm != 31 {
        decoded.src_regs.push(Reg(rm));
    }

    let reg_prefix = if is_64bit { 'x' } else { 'w' };
    let mnemonic = match opc {
        0 => "and",
        1 => "orr",
        2 => "eor",
        3 => "ands",
        _ => "and",
    };
    let shift_str = if imm6 > 0 {
        format!(", {} #{}", shift_name(shift), imm6)
    } else {
        String::new()
    };
    decoded.disasm = format!(
        "{} {}{}, {}{}, {}{}{}",
        mnemonic, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm, shift_str
    );

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_and() {
        // AND X0, X1, X2
        let raw = 0x8A020020;
        let result = decode_logical_shifted(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::And);
    }
}
