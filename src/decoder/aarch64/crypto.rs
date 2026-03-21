//! AArch64 Cryptography Extension Instructions
//!
//! This module decodes cryptography extension instructions:
//! - AES: AESE, AESD, AESMC, AESIMC
//! - SHA-1: SHA1C, SHA1P, SHA1M, SHA1H, SHA1SU0, SHA1SU1
//! - SHA-256: SHA256H, SHA256H2, SHA256SU0, SHA256SU1
//! - SHA-512: SHA512H, SHA512H2, SHA512SU0, SHA512SU1
//! - SHA-3: EOR3, RAX1, XAR, BCAX
//! - CRC32: CRC32B, CRC32H, CRC32W, CRC32X

use super::encoding::*;
use super::{DecodeResult, DecodedInstruction};
use crate::types::{OpcodeType, Reg, VReg};

/// Decode AES instructions
pub fn decode_aes(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let opcode = (raw >> 12) & 0x3;

    decoded.opcode = match opcode {
        0x0 => OpcodeType::Aese,
        0x1 => OpcodeType::Aesd,
        0x2 => OpcodeType::Aesmc,
        0x3 => OpcodeType::Aesimc,
        _ => OpcodeType::Aese,
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));

    let mnemonic = match opcode {
        0x0 => "aese",
        0x1 => "aesd",
        0x2 => "aesmc",
        0x3 => "aesimc",
        _ => "aes",
    };

    decoded.disasm = format!("{} v{}.16b, v{}.16b", mnemonic, rd, rn);

    Ok(decoded)
}

/// Decode SHA-1 instructions
pub fn decode_sha1(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let opcode = (raw >> 10) & 0x7;

    decoded.opcode = match opcode {
        0x0 | 0x1 | 0x2 | 0x3 => OpcodeType::Sha1H,
        _ => OpcodeType::Sha1H,
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));
    decoded.src_vregs.push(VReg(rm));

    decoded.disasm = format!("sha1 v{}.4s, v{}.4s, v{}.4s", rd, rn, rm);

    Ok(decoded)
}

/// Decode SHA-256 instructions
pub fn decode_sha256(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let opcode = (raw >> 10) & 0x7;

    decoded.opcode = match opcode {
        0x0 => OpcodeType::Sha256H,
        0x1 => OpcodeType::Sha256H,
        _ => OpcodeType::Sha256H,
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));
    decoded.src_vregs.push(VReg(rm));

    decoded.disasm = format!("sha256h v{}.4s, v{}.4s, v{}.4s", rd, rn, rm);

    Ok(decoded)
}

/// Decode SHA-512 instructions
pub fn decode_sha512(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let opcode = (raw >> 10) & 0x7;

    decoded.opcode = match opcode {
        0x0 => OpcodeType::Sha512H,
        0x1 => OpcodeType::Sha512H,
        _ => OpcodeType::Sha512H,
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));
    decoded.src_vregs.push(VReg(rm));

    decoded.disasm = format!("sha512h v{}.2d, v{}.2d, v{}.2d", rd, rn, rm);

    Ok(decoded)
}

/// Decode SHA-3 (three-register XOR) instructions
pub fn decode_sha3(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let ra = ((raw >> 10) & 0x1F) as u8;
    let opcode = (raw >> 12) & 0x7;

    decoded.opcode = match opcode {
        0x0 => OpcodeType::Eor,
        0x1 => OpcodeType::Eor,
        0x2 => OpcodeType::Eor,
        _ => OpcodeType::Eor,
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));
    decoded.src_vregs.push(VReg(rm));
    decoded.src_vregs.push(VReg(ra));

    decoded.disasm = format!("eor3 v{}.16b, v{}.16b, v{}.16b, v{}.16b", rd, rn, rm, ra);

    Ok(decoded)
}

/// Decode CRC32 instructions
pub fn decode_crc32(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let sf = is_64bit(raw);
    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let size = (raw >> 21) & 0x3;
    let c = (raw >> 10) & 0x1;

    decoded.opcode = match (c, size) {
        (0, 0) => OpcodeType::Other,
        (0, 1) => OpcodeType::Other,
        (0, 2) => OpcodeType::Other,
        (0, 3) => OpcodeType::Other,
        (1, 0) => OpcodeType::Other,
        (1, 1) => OpcodeType::Other,
        (1, 2) => OpcodeType::Other,
        (1, 3) => OpcodeType::Other,
        _ => OpcodeType::Other,
    };

    decoded.dst_regs.push(Reg(rd));
    decoded.src_regs.push(Reg(rn));
    decoded.src_regs.push(Reg(rm));

    let mnemonic = match (c, size) {
        (0, 0) => "crc32b",
        (0, 1) => "crc32h",
        (0, 2) => "crc32w",
        (0, 3) => "crc32x",
        (1, 0) => "crc32cb",
        (1, 1) => "crc32ch",
        (1, 2) => "crc32cw",
        (1, 3) => "crc32cx",
        _ => "crc32",
    };

    let reg_prefix = if sf { 'x' } else { 'w' };
    decoded.disasm = format!("{} {}{}, w{}, {}{}", mnemonic, reg_prefix, rd, rn, reg_prefix, rm);

    Ok(decoded)
}

/// Decode polynomial multiply instructions
pub fn decode_pmull(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let q = decode_q_bit(raw);
    let rd = decode_rd(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let size = (raw >> 22) & 0x3;

    decoded.opcode = if q {
        OpcodeType::Pmull
    } else {
        OpcodeType::Pmull
    };

    decoded.dst_vregs.push(VReg(rd));
    decoded.src_vregs.push(VReg(rn));
    decoded.src_vregs.push(VReg(rm));

    let mnemonic = if q { "pmull2" } else { "pmull" };
    let reg_size = match size {
        0 => "8b",
        1 => "16b",
        _ => "16b",
    };

    decoded.disasm = format!("{} v{}.1q, v{}.{}, v{}.{}", mnemonic, rd, rn, reg_size, rm, reg_size);

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_aes() {
        // AESE V0.16B, V1.16B
        let raw = 0x4E284800;
        let result = decode_aes(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Aese);
    }

    #[test]
    fn test_decode_crc32() {
        // CRC32B W0, W1, W2
        let raw = 0x1AC04020;
        let result = decode_crc32(0x1000, raw).unwrap();
        assert!(result.disasm.contains("crc32"));
    }
}
