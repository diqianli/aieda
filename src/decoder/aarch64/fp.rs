//! AArch64 Floating-Point Instructions
//!
//! This module decodes floating-point instructions:
//! - FADD, FSUB, FMUL, FDIV
//! - FMA, FMS, FNMA, FNMS (fused multiply-add)
//! - FCMP, FCCMP, FCMPE
//! - FMOV, FABS, FNEG, FSQRT
//! - FCVT, FCVTZS, FCVTZU
//! - FRINTN, FRINTP, FRINTM, FRINTZ, etc.

use super::encoding::*;
use super::{DecodeResult, DecodedInstruction};
use crate::types::{OpcodeType, VReg};

/// Decode floating-point arithmetic instructions
pub fn decode_fp_arith(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let ftype = (raw >> 22) & 0x3;
    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let opcode = (raw >> 12) & 0xF;

    decoded.opcode = match opcode {
        0x0 => OpcodeType::Fadd,
        0x1 => OpcodeType::Fsub,
        0x2 => OpcodeType::Fmul,
        0x3 => OpcodeType::Fdiv,
        0x4 => OpcodeType::Fmadd,
        0x5 => OpcodeType::Fmsub,
        0x6 => OpcodeType::Fnmadd,
        0x7 => OpcodeType::Fnmsub,
        0x8 => OpcodeType::Fadd,
        _ => OpcodeType::Fadd,
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));
    decoded.src_vregs.push(VReg(rm));

    let mnemonic = match opcode {
        0x0 => "fadd",
        0x1 => "fsub",
        0x2 => "fmul",
        0x3 => "fdiv",
        0x4 => "fmadd",
        0x5 => "fmsub",
        0x6 => "fnmadd",
        0x7 => "fnmsub",
        0x8 => "faddp",
        _ => "fp",
    };

    let type_suffix = match ftype {
        0 => "s",
        1 => "d",
        2 => "h",
        _ => "s",
    };

    decoded.disasm = format!("{} {}{}, {}{}, {}{}", mnemonic, type_suffix, rd, type_suffix, rn, type_suffix, rm);

    Ok(decoded)
}

/// Decode floating-point compare instructions
pub fn decode_fp_compare(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let ftype = (raw >> 22) & 0x3;
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let opcode = (raw >> 3) & 0x3;

    decoded.opcode = OpcodeType::Cmp;

    decoded.src_vregs.push(VReg(rn));
    decoded.src_vregs.push(VReg(rm));

    let mnemonic = match opcode {
        0 => "fcmp",
        1 => "fcmpe",
        2 => "fcmp",
        3 => "fcmpe",
        _ => "fcmp",
    };

    let type_suffix = match ftype {
        0 => "s",
        1 => "d",
        2 => "h",
        _ => "s",
    };

    decoded.disasm = format!("{} {}{}, {}{}", mnemonic, type_suffix, rn, type_suffix, rm);

    Ok(decoded)
}

/// Decode floating-point data processing (1 source)
pub fn decode_fp_1src(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let ftype = (raw >> 22) & 0x3;
    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let opcode = (raw >> 15) & 0x3F;

    decoded.opcode = match opcode {
        0x00 => OpcodeType::Mov,
        0x01 => OpcodeType::And,
        0x02 => OpcodeType::Eor,
        0x03 => OpcodeType::Shift,
        0x04 => OpcodeType::Fcvt,
        0x05 => OpcodeType::Fcvt,
        0x06 => OpcodeType::Fcvt,
        0x07 => OpcodeType::Fcvt,
        0x08..=0x0F => OpcodeType::Fcvt,
        _ => OpcodeType::Fcvt,
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));

    let mnemonic = match opcode {
        0x00 => "fmov",
        0x01 => "fabs",
        0x02 => "fneg",
        0x03 => "fsqrt",
        0x04 => "fcvt",
        0x05 => "fcvt",
        0x06 => "fcvt",
        0x07 => "fcvt",
        _ => "fp",
    };

    let type_suffix = match ftype {
        0 => "s",
        1 => "d",
        2 => "h",
        _ => "s",
    };

    decoded.disasm = format!("{} {}{}, {}{}", mnemonic, type_suffix, rd, type_suffix, rn);

    Ok(decoded)
}

/// Decode floating-point conditional compare
pub fn decode_fp_cond_compare(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let ftype = (raw >> 22) & 0x3;
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let cond = decode_condition(raw);

    decoded.opcode = OpcodeType::Cmp;

    decoded.src_vregs.push(VReg(rn));
    decoded.src_vregs.push(VReg(rm));

    let type_suffix = match ftype {
        0 => "s",
        1 => "d",
        2 => "h",
        _ => "s",
    };

    decoded.disasm = format!("fccmp {}{}, {}{}, #{}", type_suffix, rn, type_suffix, rm, cond);

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_fadd() {
        // FADD S0, S1, S2
        let raw = 0x1E222820;
        let result = decode_fp_arith(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Fadd);
    }
}
