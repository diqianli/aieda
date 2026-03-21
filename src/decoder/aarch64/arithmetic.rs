//! AArch64 Integer Arithmetic Instructions
//!
//! This module decodes integer arithmetic instructions including:
//! - ADD, ADDS, SUB, SUBS
//! - MUL, MNEG, SMULL, UMULL
//! - SDIV, UDIV
//! - MAD, MSB, MADD, MSUB
//! - ADC, ADCS, SBC, SBCS
//! - Shifted and extended register variants

use super::encoding::*;
use super::{DecodeResult, DecodedInstruction};
use crate::types::{OpcodeType, Reg};

/// Decode add/subtract (immediate) instructions
///
/// Encoding: sf opc shift imm12 Rn Rd
pub fn decode_add_sub_imm(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_64bit = is_64bit(raw);
    let opc = (raw >> 29) & 0x3;
    let shift = decode_shift(raw);
    let imm12 = decode_imm12(raw);
    let rn = decode_rn(raw);
    let rd = decode_rd(raw);

    // Determine opcode
    decoded.opcode = match opc {
        0 => OpcodeType::Add,        // ADD
        1 => OpcodeType::Add,        // ADDS
        2 => OpcodeType::Sub,        // SUB
        3 => OpcodeType::Sub,        // SUBS
        _ => OpcodeType::Add,
    };

    // Set registers
    if rd != 31 {
        decoded.dst_regs.push(Reg(rd));
    }
    if rn != 31 {
        decoded.src_regs.push(Reg(rn));
    }

    // Immediate value with optional shift
    let imm = if shift == 1 {
        (imm12 as u64) << 12
    } else {
        imm12 as u64
    };
    decoded.immediate = Some(imm as i64);

    // Disassembly
    let reg_prefix = if is_64bit { 'x' } else { 'w' };
    let rd_name = if rd == 31 { "sp" } else { &format!("{}{}", reg_prefix, rd) };
    let rn_name = if rn == 31 { "sp" } else { &format!("{}{}", reg_prefix, rn) };
    let shift_str = if shift == 1 { ", lsl #12" } else { "" };
    let mnemonic = match opc {
        0 => "add",
        1 => "adds",
        2 => "sub",
        3 => "subs",
        _ => "add",
    };
    decoded.disasm = format!("{} {}, {}, #{}{}", mnemonic, rd_name, rn_name, imm12, shift_str);

    Ok(decoded)
}

/// Decode add/subtract (shifted register) instructions
///
/// Encoding: sf opc shift Rm imm6 Rn Rd
pub fn decode_add_sub_shifted(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_64bit = is_64bit(raw);
    let opc = (raw >> 29) & 0x3;
    let shift = ((raw >> 22) & 0x3) as u8;
    let rm = decode_rm(raw);
    let imm6 = ((raw >> 10) & 0x3F) as u8;
    let rn = decode_rn(raw);
    let rd = decode_rd(raw);

    decoded.opcode = match opc {
        0 => OpcodeType::Add,
        1 => OpcodeType::Add,
        2 => OpcodeType::Sub,
        3 => OpcodeType::Sub,
        _ => OpcodeType::Add,
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
        0 => "add",
        1 => "adds",
        2 => "sub",
        3 => "subs",
        _ => "add",
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

/// Decode add/subtract (extended register) instructions
///
/// Encoding: sf opc opt Rm option imm3 Rn Rd
pub fn decode_add_sub_extended(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_64bit = is_64bit(raw);
    let opc = (raw >> 29) & 0x3;
    let option = decode_option(raw);
    let imm3 = ((raw >> 10) & 0x7) as u8;
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let rd = decode_rd(raw);

    decoded.opcode = match opc {
        0 => OpcodeType::Add,
        1 => OpcodeType::Add,
        2 => OpcodeType::Sub,
        3 => OpcodeType::Sub,
        _ => OpcodeType::Add,
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
        0 => "add",
        1 => "adds",
        2 => "sub",
        3 => "subs",
        _ => "add",
    };
    let extend_str = if imm3 > 0 {
        format!("{}, #{}", extend_name(option), imm3)
    } else {
        extend_name(option).to_string()
    };
    decoded.disasm = format!(
        "{} {}{}, {}{}, {}{} {}",
        mnemonic, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm, extend_str
    );

    Ok(decoded)
}

/// Decode multiply/divide instructions
pub fn decode_multiply_divide(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_64bit = is_64bit(raw);
    let rm = decode_rm(raw);
    let ra = ((raw >> 10) & 0x1F) as u8;
    let rn = decode_rn(raw);
    let rd = decode_rd(raw);
    let op = (raw >> 15) & 0x7;

    decoded.opcode = match op {
        0 => OpcodeType::Mul,     // MADD
        1 => OpcodeType::Mul,     // MSUB
        2 => OpcodeType::Div,     // SMULH
        3 => OpcodeType::Div,     // UMULH
        4 => OpcodeType::Div,     // SMULL
        5 => OpcodeType::Div,     // UMULL
        6 => OpcodeType::Mul,     // SMULH (vector)
        7 => OpcodeType::Div,     // SDIV/UDIV
        _ => OpcodeType::Mul,
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
    if ra != 31 && (op == 0 || op == 1) {
        decoded.src_regs.push(Reg(ra));
    }

    let reg_prefix = if is_64bit { 'x' } else { 'w' };
    let mnemonic = match op {
        0 => "madd",
        1 => "msub",
        2 => "smulh",
        3 => "umulh",
        4 => "smull",
        5 => "umull",
        6 => "smull",
        7 => if (raw >> 10) & 0x1 == 1 { "udiv" } else { "sdiv" },
        _ => "mul",
    };

    decoded.disasm = format!(
        "{} {}{}, {}{}, {}{}",
        mnemonic, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm
    );

    Ok(decoded)
}

/// Decode conditional compare instructions
pub fn decode_cond_compare(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let cond = decode_condition(raw);

    decoded.opcode = OpcodeType::Cmp;
    decoded.src_regs.push(Reg(rn));
    if rm != 31 {
        decoded.src_regs.push(Reg(rm));
    }

    decoded.disasm = format!("ccmp x{}, x{}, #{}", rn, rm, condition_name(cond));

    Ok(decoded)
}

/// Decode data processing (2 source) instructions
pub fn decode_data_proc_2src(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_64bit = is_64bit(raw);
    let rm = decode_rm(raw);
    let opcode = (raw >> 10) & 0x3F;
    let rn = decode_rn(raw);
    let rd = decode_rd(raw);

    decoded.opcode = match opcode {
        0x00 => OpcodeType::Div,     // SDIV
        0x01 => OpcodeType::Div,     // UDIV
        0x02 => OpcodeType::Lsl,     // LSLV
        0x03 => OpcodeType::Lsr,     // LSRV
        0x04 => OpcodeType::Asr,     // ASRV
        0x05 => OpcodeType::Asr,     // RORV
        0x06 => OpcodeType::Shift,   // CRC32X
        0x07 => OpcodeType::Shift,   // CRC32W
        _ => OpcodeType::Other,
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
    decoded.disasm = format!(
        "{:?} {}{}, {}{}, {}{}",
        decoded.opcode, reg_prefix, rd, reg_prefix, rn, reg_prefix, rm
    );

    Ok(decoded)
}

/// Decode data processing (1 source) instructions
pub fn decode_data_proc_1src(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_64bit = is_64bit(raw);
    let opcode = (raw >> 10) & 0x3F;
    let rn = decode_rn(raw);
    let rd = decode_rd(raw);

    decoded.opcode = match opcode {
        0x00 => OpcodeType::Mov,     // RBIT
        0x01 => OpcodeType::Shift,   // REV16
        0x02 => OpcodeType::Shift,   // REV32
        0x03 => OpcodeType::Shift,   // REV64
        0x04 => OpcodeType::Mov,     // CLZ
        0x05 => OpcodeType::Cmp,     // CLS
        0x06 => OpcodeType::Cmp,     // CTZ
        _ => OpcodeType::Other,
    };

    if rd != 31 {
        decoded.dst_regs.push(Reg(rd));
    }
    if rn != 31 {
        decoded.src_regs.push(Reg(rn));
    }

    let reg_prefix = if is_64bit { 'x' } else { 'w' };
    decoded.disasm = format!(
        "{:?} {}{}, {}{}",
        decoded.opcode, reg_prefix, rd, reg_prefix, rn
    );

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_add_imm() {
        // ADD X0, X1, #42
        let raw = 0x9100A820;
        let result = decode_add_sub_imm(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Add);
    }

    #[test]
    fn test_decode_sub() {
        // SUB X0, X1, X2
        let raw = 0xCB020020;
        let result = decode_add_sub_shifted(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Sub);
    }
}
