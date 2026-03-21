//! AArch64 SIMD/NEON Instructions
//!
//! This module decodes SIMD and NEON vector instructions:
//! - Vector arithmetic (ADD, SUB, MUL, etc.)
//! - Vector multiply-accumulate (MLA, MLS, etc.)
//! - Vector load/store (LD1, ST1, LD2, ST2, etc.)
//! - Vector permutation (ZIP, UZP, TRN, EXT)
//! - Vector compare (CMEQ, CMGT, etc.)
//! - Saturating operations (SQADD, SQSUB, etc.)

use super::encoding::*;
use super::{DecodeResult, DecodedInstruction};
use crate::types::{OpcodeType, VReg};

/// Decode SIMD vector arithmetic instructions
pub fn decode_simd_arith(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let q = decode_q_bit(raw);
    let _u = (raw >> 29) & 0x1 == 1;
    let _size = decode_size(raw);
    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let opcode = (raw >> 11) & 0x1F;

    decoded.opcode = match opcode {
        0x00 | 0x01 => OpcodeType::Vadd,
        0x02 => OpcodeType::Vmul,
        0x03 => OpcodeType::Vmul,
        0x04 | 0x05 => OpcodeType::Vmla,
        0x06 | 0x07 => OpcodeType::Vmls,
        0x08 | 0x09 => OpcodeType::Vsub,
        0x0C | 0x0D => OpcodeType::Vmla,
        0x0E | 0x0F => OpcodeType::Vadd,
        _ => OpcodeType::Vadd,
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));
    decoded.src_vregs.push(VReg(rm));

    let reg_size = if q { "16b" } else { "8b" };
    decoded.disasm = format!(
        "{:?} v{}.{}, v{}.{}, v{}.{}",
        decoded.opcode, rd, reg_size, rn, reg_size, rm, reg_size
    );

    Ok(decoded)
}

/// Decode SIMD load/store multiple structures
pub fn decode_simd_load_store(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let _q = decode_q_bit(raw);
    let l = (raw >> 22) & 0x1 == 1;
    let rt = decode_rt(raw);
    let rn = decode_rn(raw);
    let opcode = (raw >> 12) & 0xF;

    decoded.opcode = if l {
        OpcodeType::Vld
    } else {
        OpcodeType::Vst
    };

    decoded.dst_vregs.push(VReg(rt));
    if rn != 31 {
        // decoded.src_regs.push(crate::types::Reg(rn));
    }

    let mnemonic = match opcode {
        0x0 | 0x2 | 0x4 | 0x6 | 0x7 => if l { "ld1" } else { "st1" },
        0x1 | 0x3 | 0x5 => if l { "ld2" } else { "st2" },
        0x8 => if l { "ld3" } else { "st3" },
        0x9 => if l { "ld4" } else { "st4" },
        _ => if l { "ld1" } else { "st1" },
    };

    decoded.disasm = format!("{} {{ v{}.16b }}, [x{}]", mnemonic, rt, rn);

    Ok(decoded)
}

/// Decode SIMD permutation instructions
pub fn decode_simd_permute(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let q = decode_q_bit(raw);
    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let opcode = (raw >> 12) & 0x3F;

    decoded.opcode = match opcode {
        0x00 | 0x01 => OpcodeType::Vmov,     // ZIP1, ZIP2
        0x02 | 0x03 => OpcodeType::Vmov,     // UZP1, UZP2
        0x04 | 0x05 => OpcodeType::Vmov,     // TRN1, TRN2
        0x08 | 0x09 => OpcodeType::Vmov,     // EXT
        _ => OpcodeType::Vmov,
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));
    decoded.src_vregs.push(VReg(rm));

    let reg_size = if q { "16b" } else { "8b" };
    decoded.disasm = format!(
        "perm v{}.{}, v{}.{}, v{}.{}",
        rd, reg_size, rn, reg_size, rm, reg_size
    );

    Ok(decoded)
}

/// Decode SIMD saturating arithmetic
pub fn decode_simd_saturating(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let q = decode_q_bit(raw);
    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let opcode = (raw >> 11) & 0x1F;

    decoded.opcode = match opcode {
        0x00 | 0x01 => OpcodeType::Vadd,
        0x02 | 0x03 => OpcodeType::Vsub,
        0x04 => OpcodeType::Vsub,
        0x05 => OpcodeType::Vadd,
        _ => OpcodeType::Vadd,
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));
    decoded.src_vregs.push(VReg(rm));

    let reg_size = if q { "16b" } else { "8b" };
    decoded.disasm = format!(
        "qop v{}.{}, v{}.{}, v{}.{}",
        rd, reg_size, rn, reg_size, rm, reg_size
    );

    Ok(decoded)
}

/// Decode SIMD pairwise operations
pub fn decode_simd_pairwise(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let q = decode_q_bit(raw);
    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let opcode = (raw >> 12) & 0x1F;

    decoded.opcode = match opcode {
        0x00 | 0x01 => OpcodeType::Vadd,
        0x0A | 0x0B => OpcodeType::Vmla,
        _ => OpcodeType::Vadd,
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));

    let reg_size = if q { "16b" } else { "8b" };
    decoded.disasm = format!("addp v{}.{}, v{}.{}", rd, reg_size, rn, reg_size);

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_simd_add() {
        // ADD V0.16B, V1.16B, V2.16B
        let raw = 0x4E208420;
        let result = decode_simd_arith(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Vadd);
    }
}
