//! AArch64 System Instructions
//!
//! This module decodes system instructions:
//! - MSR, MRS (system register access)
//! - SYS, SYSL (system instruction)
//! - HINT (NOP, YIELD, WFE, WFI, SEV, etc.)
//! - DMB, DSB, ISB (memory barriers)
//! - DC, IC (cache maintenance)
//! - TLBI (TLB invalidate)
//! - ERET, ERETA, ERETAB (exception return)
//! - SMC, HVC (secure monitor/hypervisor call)

use super::encoding::*;
use super::{DecodeResult, DecodedInstruction};
use crate::types::{OpcodeType, Reg};

/// Decode system register access (MRS/MSR)
pub fn decode_sys_reg(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let l = (raw >> 21) & 0x1 == 1;
    let rt = decode_rt(raw);
    let crm = decode_crm(raw);
    let crn = decode_crn(raw);
    let op1 = decode_op1(raw);
    let op2 = decode_op2(raw);

    decoded.opcode = if l {
        OpcodeType::Mrs
    } else {
        OpcodeType::Msr
    };

    if rt != 31 {
        if l {
            decoded.dst_regs.push(Reg(rt));
        } else {
            decoded.src_regs.push(Reg(rt));
        }
    }

    let sysreg = format!("s{}_{}_c{}_c{}", op1, crn, crm, op2);
    let mnemonic = if l { "mrs" } else { "msr" };

    if l {
        decoded.disasm = format!("{} x{}, {}", mnemonic, rt, sysreg);
    } else {
        decoded.disasm = format!("{} {}, x{}", mnemonic, sysreg, rt);
    }

    Ok(decoded)
}

/// Decode system instruction (SYS/SYSL)
pub fn decode_sys_instr(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let l = (raw >> 21) & 0x1 == 1;
    let rt = decode_rt(raw);
    let crm = decode_crm(raw);
    let crn = decode_crn(raw);
    let op1 = decode_op1(raw);
    let op2 = decode_op2(raw);

    decoded.opcode = OpcodeType::Sys;

    let mnemonic = if l { "sysl" } else { "sys" };
    decoded.disasm = format!("{} #{}, #{}, #{}, #{}, x{}", mnemonic, op1, crn, crm, op2, rt);

    Ok(decoded)
}

/// Decode HINT instructions
pub fn decode_hint(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let crm = decode_crm(raw);
    let op2 = decode_op2(raw);
    let imm = (crm << 3) | op2;

    decoded.opcode = match imm {
        0 => OpcodeType::Nop,
        1 => OpcodeType::Nop,
        2 => OpcodeType::Nop,
        3 => OpcodeType::Nop,
        4 => OpcodeType::Nop,
        _ => OpcodeType::Nop,
    };

    let mnemonic = match imm {
        0 => "nop",
        1 => "yield",
        2 => "wfe",
        3 => "wfi",
        4 => "sev",
        5 => "sevl",
        _ => "hint",
    };

    decoded.disasm = mnemonic.to_string();

    Ok(decoded)
}

/// Decode memory barrier instructions
pub fn decode_barrier(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let crm = decode_crm(raw);
    let op2 = decode_op2(raw);
    let rt = decode_rt(raw);

    decoded.opcode = match (crm, op2) {
        (0b0010, 0b100) => OpcodeType::Dmb,
        (0b0010, 0b101) => OpcodeType::Dsb,
        (0b0010, 0b110) => OpcodeType::Isb,
        (0b0011, 0b100) => OpcodeType::Dmb,
        (0b0011, 0b101) => OpcodeType::Dsb,
        _ => OpcodeType::Dmb,
    };

    let domain = if rt == 0b1111 {
        "sy".to_string()
    } else if rt == 0b1110 {
        "ish".to_string()
    } else if rt == 0b1101 {
        "nsh".to_string()
    } else if rt == 0b1011 {
        "osh".to_string()
    } else {
        format!("#{}", rt)
    };

    let mnemonic = match (crm, op2) {
        (0b0010, 0b100) | (0b0011, 0b100) => "dmb",
        (0b0010, 0b101) | (0b0011, 0b101) => "dsb",
        (0b0010, 0b110) => "isb",
        _ => "barrier",
    };

    decoded.disasm = format!("{} {}", mnemonic, domain);

    Ok(decoded)
}

/// Decode data cache maintenance instructions
pub fn decode_dc(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let crm = decode_crm(raw);
    let op2 = decode_op2(raw);
    let rt = decode_rt(raw);

    decoded.opcode = match (crm, op2) {
        (0b0001, 0b000) => OpcodeType::DcCivac,
        (0b0001, 0b001) => OpcodeType::DcCivac,
        (0b0001, 0b010) => OpcodeType::DcCivac,
        (0b0001, 0b011) => OpcodeType::DcCsw,
        (0b0010, 0b000) => OpcodeType::DcCvac,
        (0b0010, 0b001) => OpcodeType::DcCvac,
        (0b0010, 0b010) => OpcodeType::DcCivac,
        (0b0011, 0b000) => OpcodeType::DcZva,
        _ => OpcodeType::DcCvac,
    };

    if rt != 31 {
        decoded.src_regs.push(Reg(rt));
    }

    let mnemonic = match (crm, op2) {
        (0b0001, 0b000) => "dc ivac",
        (0b0001, 0b001) => "dc isw",
        (0b0001, 0b010) => "dc csw",
        (0b0001, 0b011) => "dc cisw",
        (0b0010, 0b000) => "dc cvac",
        (0b0010, 0b001) => "dc cvau",
        (0b0010, 0b010) => "dc civac",
        (0b0011, 0b000) => "dc zva",
        _ => "dc",
    };

    decoded.disasm = format!("{}, x{}", mnemonic, rt);

    Ok(decoded)
}

/// Decode instruction cache maintenance instructions
pub fn decode_ic(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let crm = decode_crm(raw);
    let op2 = decode_op2(raw);
    let rt = decode_rt(raw);

    decoded.opcode = match (crm, op2) {
        (0b0001, 0b000) => OpcodeType::IcIalluis,
        (0b0001, 0b001) => OpcodeType::IcIallu,
        (0b0010, 0b001) => OpcodeType::IcIvau,
        _ => OpcodeType::IcIvau,
    };

    if rt != 31 {
        decoded.src_regs.push(Reg(rt));
    }

    let mnemonic = match (crm, op2) {
        (0b0001, 0b000) => "ic ialluis",
        (0b0001, 0b001) => "ic iallu",
        (0b0010, 0b001) => "ic ivau",
        _ => "ic",
    };

    decoded.disasm = format!("{}, x{}", mnemonic, rt);

    Ok(decoded)
}

/// Decode exception return instructions
pub fn decode_exception_return(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let _opc = (raw >> 21) & 0x3;

    decoded.opcode = OpcodeType::Sys;

    decoded.disasm = "eret".to_string();

    Ok(decoded)
}

/// Decode exception generating instructions
pub fn decode_exception_gen(pc: u64, raw: u32) -> DecodeResult {
    let mut decoded = DecodedInstruction::new(pc, raw);

    let opc = (raw >> 21) & 0x3;
    let imm16 = ((raw >> 5) & 0xFFFF) as u16;

    decoded.opcode = match opc {
        0x0 => OpcodeType::Sys,
        0x1 => OpcodeType::Sys,
        0x2 => OpcodeType::Sys,
        0x3 => OpcodeType::Nop,
        _ => OpcodeType::Sys,
    };

    let mnemonic = match opc {
        0x0 => "svc",
        0x1 => "hvc",
        0x2 => "smc",
        0x3 => "brk",
        _ => "exception",
    };

    decoded.disasm = format!("{} #{}", mnemonic, imm16);

    Ok(decoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_mrs() {
        // MRS X0, S3_0_C15_C0_0
        let raw = 0xD53BE000;
        let result = decode_sys_reg(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Mrs);
    }

    #[test]
    fn test_decode_nop() {
        // NOP
        let raw = 0xD503201F;
        let result = decode_hint(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Nop);
    }

    #[test]
    fn test_decode_dmb() {
        // DMB SY
        let raw = 0xD5033FBF;
        let result = decode_barrier(0x1000, raw).unwrap();
        assert_eq!(result.opcode, OpcodeType::Dmb);
    }
}
