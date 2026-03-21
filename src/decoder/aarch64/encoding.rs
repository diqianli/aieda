//! AArch64 Instruction Encoding Helpers
//!
//! This module provides helper functions for extracting fields from AArch64 instruction encodings.

/// Extract bits from a 32-bit word
#[inline]
pub fn extract_bits(raw: u32, msb: u32, lsb: u32) -> u32 {
    let mask = if msb == 31 {
        0xFFFFFFFF
    } else {
        (1 << (msb + 1)) - 1
    };
    (raw >> lsb) & (mask >> lsb)
}

/// Sign-extend a value
#[inline]
pub fn sign_extend(value: u32, bits: u32) -> i64 {
    let shift = 64 - bits;
    ((value as u64) << shift) as i64 >> shift
}

/// Sign-extend an immediate for PC-relative addressing
#[inline]
pub fn sign_extend_19(value: u32) -> i64 {
    sign_extend(value, 19)
}

/// Sign-extend an immediate for branch instructions
#[inline]
pub fn sign_extend_26(value: u32) -> i64 {
    sign_extend(value, 26)
}

/// Sign-extend an immediate for conditional branches
#[inline]
pub fn sign_extend_19_shift(value: u32, shift: u32) -> i64 {
    sign_extend(value, 19) << (shift as i64)
}

/// Decode the register at bits [4:0]
#[inline]
pub fn decode_rd(raw: u32) -> u8 {
    (raw & 0x1F) as u8
}

/// Decode the register at bits [4:0] (alias for decode_rd, used for Rt)
#[inline]
pub fn decode_rt(raw: u32) -> u8 {
    (raw & 0x1F) as u8
}

/// Decode the register at bits [9:5]
#[inline]
pub fn decode_rn(raw: u32) -> u8 {
    ((raw >> 5) & 0x1F) as u8
}

/// Decode the register at bits [20:16]
#[inline]
pub fn decode_rm(raw: u32) -> u8 {
    ((raw >> 16) & 0x1F) as u8
}

/// Decode the register at bits [15:10]
#[inline]
pub fn decode_ra(raw: u32) -> u8 {
    ((raw >> 10) & 0x1F) as u8
}

/// Check if the instruction is 64-bit
#[inline]
pub fn is_64bit(raw: u32) -> bool {
    (raw >> 31) & 0x1 == 1
}

/// Decode the immediate at bits [21:10]
#[inline]
pub fn decode_imm12(raw: u32) -> u16 {
    ((raw >> 10) & 0xFFF) as u16
}

/// Decode the immediate at bits [23:5]
#[inline]
pub fn decode_imm19(raw: u32) -> u32 {
    ((raw >> 5) & 0x7FFFF)
}

/// Decode the immediate at bits [25:0]
#[inline]
pub fn decode_imm26(raw: u32) -> u32 {
    raw & 0x3FFFFFF
}

/// Decode the shift amount at bits [23:22]
#[inline]
pub fn decode_shift(raw: u32) -> u8 {
    ((raw >> 22) & 0x3) as u8
}

/// Decode the option field at bits [23:22]
#[inline]
pub fn decode_option(raw: u32) -> u8 {
    ((raw >> 22) & 0x3) as u8
}

/// Decode the size field at bits [31:30]
#[inline]
pub fn decode_size(raw: u32) -> u8 {
    ((raw >> 30) & 0x3) as u8
}

/// Decode the size field at bits [23:22]
#[inline]
pub fn decode_size2(raw: u32) -> u8 {
    ((raw >> 22) & 0x3) as u8
}

/// Check if 32-bit operation
#[inline]
pub fn is_32bit(raw: u32) -> bool {
    !is_64bit(raw)
}

/// Decode the condition code at bits [3:0]
#[inline]
pub fn decode_condition(raw: u32) -> u8 {
    (raw & 0xF) as u8
}

/// Decode the extend type at bits [22:20]
#[inline]
pub fn decode_extend(raw: u32) -> u8 {
    ((raw >> 20) & 0x7) as u8
}

/// Decode the vector element size
#[inline]
pub fn decode_esize(raw: u32) -> u32 {
    let size = decode_size(raw);
    1 << size
}

/// Decode the vector register
#[inline]
pub fn decode_vreg(raw: u32, offset: u32) -> u8 {
    ((raw >> offset) & 0x1F) as u8
}

/// Decode Q bit for SIMD instructions
#[inline]
pub fn decode_q_bit(raw: u32) -> bool {
    (raw >> 30) & 0x1 == 1
}

/// Decode scalar bit for SIMD instructions
#[inline]
pub fn decode_scalar(raw: u32) -> bool {
    (raw >> 28) & 0x1 == 1
}

/// Decode the CRm field for system instructions
#[inline]
pub fn decode_crm(raw: u32) -> u8 {
    ((raw >> 8) & 0xF) as u8
}

/// Decode the CRn field for system instructions
#[inline]
pub fn decode_crn(raw: u32) -> u8 {
    ((raw >> 12) & 0xF) as u8
}

/// Decode op1 for system instructions
#[inline]
pub fn decode_op1(raw: u32) -> u8 {
    ((raw >> 16) & 0x7) as u8
}

/// Decode op2 for system instructions
#[inline]
pub fn decode_op2(raw: u32) -> u8 {
    ((raw >> 5) & 0x7) as u8
}

/// Condition code names
pub const CONDITION_NAMES: [&str; 16] = [
    "eq", "ne", "cs", "cc", "mi", "pl", "vs", "vc",
    "hi", "ls", "ge", "lt", "gt", "le", "al", "nv",
];

/// Get condition name
pub fn condition_name(cond: u8) -> &'static str {
    CONDITION_NAMES[cond as usize]
}

/// Extension type names
pub const EXTEND_NAMES: [&str; 8] = [
    "uxtb", "uxth", "uxtw", "uxtx", "sxtb", "sxth", "sxtw", "sxtx",
];

/// Get extension name
pub fn extend_name(ext: u8) -> &'static str {
    EXTEND_NAMES[ext as usize]
}

/// Shift type names
pub const SHIFT_NAMES: [&str; 4] = ["lsl", "lsr", "asr", "ror"];

/// Get shift type name
pub fn shift_name(shift: u8) -> &'static str {
    SHIFT_NAMES[shift as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bits() {
        let raw = 0b11110000_00000000_00000000_00000000u32;
        assert_eq!(extract_bits(raw, 31, 28), 0b1111);
    }

    #[test]
    fn test_decode_registers() {
        let raw = 0b00000_00001_00010_000000000000_00011u32;
        assert_eq!(decode_rd(raw), 3);
        assert_eq!(decode_rn(raw), 2);
        assert_eq!(decode_rm(raw), 1);
    }

    #[test]
    fn test_sign_extend() {
        assert_eq!(sign_extend(0x7FFFF, 19), 0x7FFFFi64);
        assert_eq!(sign_extend(0x80000, 19), -0x80000i64);
    }
}
