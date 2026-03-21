//! AArch64 Load/Store Instructions
//!
//! This module decodes load and store instructions:
//! - LDR, STR (register, immediate, pair)
//! - LDUR, STUR (unscaled immediate)
//! - LDP, STP (pair)
//! - LDTR, STTR (unprivileged)
//! - LDAXR, STLXR (exclusive)
//! - LDRSW (sign-extended load)
//! - Atomic operations (LDADD, LDCLR, LDEOR, LDSET, LDSWP, CAS)

use super::encoding::*;
use super::{DecodeResult, DecodedInstruction};
use crate::types::{OpcodeType, Reg, MemAccess};

/// Decode load/store register (unsigned immediate)
pub fn decode_load_store_imm(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let size = decode_size(raw);
    let is_64bit = is_64bit(raw);
    let is_load = (raw >> 22) & 0x1 == 1;
    let rt = decode_rt(raw);
    let rn = decode_rn(raw);
    let imm12 = decode_imm12(raw);

    let access_size = if size == 0 { 1 } else if size == 1 { 2 } else if size == 2 { 4 } else { 8 };

    decoded.opcode = if is_load {
        OpcodeType::Load
    } else {
        OpcodeType::Store
    };

    if rt != 31 {
        decoded.dst_regs.push(Reg(rt));
    }
    if rn != 31 {
        decoded.src_regs.push(Reg(rn));
    }

    decoded.mem_access = Some(MemAccess {
        addr: 0,
        size: access_size,
        is_load,
    });

    let reg_prefix = if is_64bit { 'x' } else { 'w' };
    let mnemonic = if is_load { "ldr" } else { "str" };
    let offset = (imm12 as u64) * access_size as u64;
    decoded.disasm = format!(
        "{} {}{}, [x{}, #{}]",
        mnemonic, reg_prefix, rt, rn, offset
    );

    Ok(decoded)
}

/// Decode load/store register (register offset)
pub fn decode_load_store_reg(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let size = decode_size(raw);
    let is_64bit = is_64bit(raw);
    let is_load = (raw >> 22) & 0x1 == 1;
    let rt = decode_rt(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);

    let access_size = 1 << size;

    decoded.opcode = if is_load {
        OpcodeType::Load
    } else {
        OpcodeType::Store
    };

    if rt != 31 {
        decoded.dst_regs.push(Reg(rt));
    }
    if rn != 31 {
        decoded.src_regs.push(Reg(rn));
    }
    if rm != 31 {
        decoded.src_regs.push(Reg(rm));
    }

    decoded.mem_access = Some(MemAccess {
        addr: 0,
        size: access_size,
        is_load,
    });

    let reg_prefix = if is_64bit { 'x' } else { 'w' };
    let mnemonic = if is_load { "ldr" } else { "str" };
    decoded.disasm = format!(
        "{} {}{}, [x{}, x{}]",
        mnemonic, reg_prefix, rt, rn, rm
    );

    Ok(decoded)
}

/// Decode load/store pair
pub fn decode_load_store_pair(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let is_64bit = is_64bit(raw);
    let l = (raw >> 22) & 0x1 == 1;
    let rt = decode_rt(raw);
    let rt2 = ((raw >> 10) & 0x1F) as u8;
    let rn = decode_rn(raw);
    let imm7 = ((raw >> 15) & 0x7F) as i8;

    decoded.opcode = if l {
        OpcodeType::LoadPair
    } else {
        OpcodeType::StorePair
    };

    if rt != 31 {
        decoded.dst_regs.push(Reg(rt));
    }
    if l && rt2 != 31 {
        decoded.dst_regs.push(Reg(rt2));
    }
    if rn != 31 {
        decoded.src_regs.push(Reg(rn));
    }

    let access_size = if is_64bit { 8 } else { 4 };
    decoded.mem_access = Some(MemAccess {
        addr: 0,
        size: (access_size * 2) as u8,
        is_load: l,
    });

    let reg_prefix = if is_64bit { 'x' } else { 'w' };
    let mnemonic = if l { "ldp" } else { "stp" };
    let offset = (imm7 as i64) * (access_size as i64);
    decoded.disasm = format!(
        "{} {}{}, {}{}, [x{}, #{}]",
        mnemonic, reg_prefix, rt, reg_prefix, rt2, rn, offset
    );

    Ok(decoded)
}

/// Decode atomic memory operations
pub fn decode_atomic(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let size = decode_size(raw);
    let is_64bit = is_64bit(raw);
    let rt = decode_rt(raw);
    let rn = decode_rn(raw);
    let rm = decode_rm(raw);
    let _opc = (raw >> 12) & 0xF;

    decoded.opcode = OpcodeType::Load;

    if rt != 31 {
        decoded.dst_regs.push(Reg(rt));
    }
    if rn != 31 {
        decoded.src_regs.push(Reg(rn));
    }
    if rm != 31 {
        decoded.src_regs.push(Reg(rm));
    }

    let access_size = 1 << size;
    decoded.mem_access = Some(MemAccess {
        addr: 0,
        size: access_size,
        is_load: true,
    });

    let reg_prefix = if is_64bit { 'x' } else { 'w' };
    decoded.disasm = format!(
        "atomic {}{}, {}{}, [x{}]",
        reg_prefix, rt, reg_prefix, rm, rn
    );

    Ok(decoded)
}

/// Decode exclusive load/store
pub fn decode_exclusive(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let size = decode_size(raw);
    let is_64bit = is_64bit(raw);
    let is_load = (raw >> 22) & 0x1 == 1;
    let rt = decode_rt(raw);
    let rn = decode_rn(raw);
    let _rt2 = ((raw >> 10) & 0x1F) as u8;
    let rs = ((raw >> 16) & 0x1F) as u8;

    decoded.opcode = if is_load {
        OpcodeType::Load
    } else {
        OpcodeType::Store
    };

    if rt != 31 {
        decoded.dst_regs.push(Reg(rt));
    }
    if rn != 31 {
        decoded.src_regs.push(Reg(rn));
    }
    if !is_load && rs != 31 {
        decoded.src_regs.push(Reg(rs));
    }

    let access_size = 1 << size;
    decoded.mem_access = Some(MemAccess {
        addr: 0,
        size: access_size,
        is_load,
    });

    let reg_prefix = if is_64bit { 'x' } else { 'w' };
    let mnemonic = if is_load { "ldaxr" } else { "stlxr" };
    if is_load {
        decoded.disasm = format!("{} {}{}, [x{}]", mnemonic, reg_prefix, rt, rn);
    } else {
        decoded.disasm = format!("{} w{}, {}{}, [x{}]", mnemonic, rs, reg_prefix, rt, rn);
    }

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_load() {
        // LDR X0, [X1]
        let raw = 0xF9400020;
        let result = decode_load_store_imm(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Load);
    }

    #[test]
    fn test_decode_store() {
        // STR X0, [X1]
        let raw = 0xF9000020;
        let result = decode_load_store_imm(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Store);
    }
}
