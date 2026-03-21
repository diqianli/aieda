//! Binary trace format definitions.
//!
//! File Structure:
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           File Header (64 bytes)        │
//! ├─────────────────────────────────────────┤
//! │  Magic: "ARMTRACE" (8 bytes)            │
//! │  Version: u16                            │
//! │  Flags: u16                              │
//! │  Instruction Count: u64                  │
//! │  String Table Offset: u64                │
//! │  String Table Size: u32                  │
//! │  Index Table Offset: u64                 │
//! │  Index Table Size: u32                   │
//! │  Reserved: 24 bytes                      │
//! ├─────────────────────────────────────────┤
//! │           Instruction Stream            │
//! ├─────────────────────────────────────────┤
//! │  [Instr Header | Operands | Deps]*      │
//! ├─────────────────────────────────────────┤
//! │           Index Table (optional)         │
//! ├─────────────────────────────────────────┤
//! │           String Table                   │
//! └─────────────────────────────────────────┘
//! ```

use serde::{Deserialize, Serialize};

/// Magic number for trace file identification
pub const MAGIC: &[u8; 8] = b"ARMTRACE";

/// Current format version
pub const VERSION: u16 = 1;

/// File header flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFlags(pub u16);

impl FileFlags {
    /// No special flags
    pub const NONE: u16 = 0;
    /// Contains index table for random access
    pub const HAS_INDEX: u16 = 1 << 0;
    /// Contains extended timing information
    pub const EXTENDED_TIMING: u16 = 1 << 1;
    /// Compressed with zstd
    pub const COMPRESSED: u16 = 1 << 2;

    pub fn has_index(&self) -> bool {
        self.0 & Self::HAS_INDEX != 0
    }

    pub fn has_extended_timing(&self) -> bool {
        self.0 & Self::EXTENDED_TIMING != 0
    }

    pub fn is_compressed(&self) -> bool {
        self.0 & Self::COMPRESSED != 0
    }
}

/// File header structure (64 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct FileHeader {
    /// Magic number "ARMTRACE"
    pub magic: [u8; 8],
    /// Format version
    pub version: u16,
    /// Flags (see FileFlags)
    pub flags: u16,
    /// Total number of instructions
    pub instr_count: u64,
    /// Offset to string table from file start
    pub string_table_offset: u64,
    /// Size of string table in bytes
    pub string_table_size: u32,
    /// Offset to index table (for random access)
    pub index_table_offset: u64,
    /// Size of index table in bytes
    pub index_table_size: u32,
    /// Reserved for future use
    pub reserved: [u8; 24],
}

impl Default for FileHeader {
    fn default() -> Self {
        Self {
            magic: *MAGIC,
            version: VERSION,
            flags: FileFlags::NONE,
            instr_count: 0,
            string_table_offset: 0,
            string_table_size: 0,
            index_table_offset: 0,
            index_table_size: 0,
            reserved: [0; 24],
        }
    }
}

impl FileHeader {
    /// Size of the file header in bytes
    pub const SIZE: usize = 64;

    /// Create a new header with the given instruction count
    pub fn new(instr_count: u64) -> Self {
        Self {
            magic: *MAGIC,
            version: VERSION,
            flags: 0,
            instr_count,
            string_table_offset: 0,
            string_table_size: 0,
            index_table_offset: 0,
            index_table_size: 0,
            reserved: [0; 24],
        }
    }

    /// Validate the header
    pub fn validate(&self) -> Result<(), String> {
        // Copy magic to avoid unaligned reference
        let magic = self.magic;
        if &magic != MAGIC {
            return Err(format!(
                "Invalid magic number: expected {:?}, got {:?}",
                MAGIC, magic
            ));
        }
        // Copy version to avoid unaligned reference
        let version = self.version;
        if version > VERSION {
            return Err(format!(
                "Unsupported version: {} > {}",
                version, VERSION
            ));
        }
        Ok(())
    }

    /// Check if flags has index
    pub fn has_index(&self) -> bool {
        self.flags & FileFlags::HAS_INDEX != 0
    }
}

/// Instruction header (8 bytes)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct InstrHeader {
    /// Instruction ID (lower 32 bits, can extend with varint for >4B)
    pub id: u32,
    /// Opcode type (see OpcodeType encoding)
    pub opcode: u8,
    /// Flags (memory op, branch, etc.)
    pub flags: InstrFlags,
    /// PC delta from previous instruction (for compression)
    pub pc_delta: u16,
    /// Number of operands
    pub operand_count: u8,
}

impl InstrHeader {
    /// Size of instruction header in bytes
    pub const SIZE: usize = 9;
}

/// Instruction flags
#[derive(Debug, Clone, Copy, Default)]
pub struct InstrFlags(pub u8);

impl InstrFlags {
    /// No special flags
    pub const NONE: u8 = 0;
    /// Has memory access
    pub const HAS_MEM: u8 = 1 << 0;
    /// Has branch info
    pub const HAS_BRANCH: u8 = 1 << 1;
    /// Has extended PC (for large PC values)
    pub const EXTENDED_PC: u8 = 1 << 2;
    /// Has disassembly string
    pub const HAS_DISASM: u8 = 1 << 3;
    /// Has dependencies
    pub const HAS_DEPS: u8 = 1 << 4;

    pub fn has_mem(&self) -> bool {
        self.0 & Self::HAS_MEM != 0
    }

    pub fn has_branch(&self) -> bool {
        self.0 & Self::HAS_BRANCH != 0
    }

    pub fn has_extended_pc(&self) -> bool {
        self.0 & Self::EXTENDED_PC != 0
    }

    pub fn has_disasm(&self) -> bool {
        self.0 & Self::HAS_DISASM != 0
    }

    pub fn has_deps(&self) -> bool {
        self.0 & Self::HAS_DEPS != 0
    }
}

/// Operand type encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OperandType {
    /// General purpose register (value is register number)
    Reg = 0,
    /// SIMD/FP register (value is register number)
    VReg = 1,
    /// Immediate value (signed varint)
    Imm = 2,
    /// Memory operand (base + offset)
    Mem = 3,
    /// Shifted register
    ShiftedReg = 4,
    /// Extended register
    ExtendedReg = 5,
}

impl TryFrom<u8> for OperandType {
    type Error = String;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Reg),
            1 => Ok(Self::VReg),
            2 => Ok(Self::Imm),
            3 => Ok(Self::Mem),
            4 => Ok(Self::ShiftedReg),
            5 => Ok(Self::ExtendedReg),
            _ => Err(format!("Invalid operand type: {}", value)),
        }
    }
}

/// Memory access encoding
#[derive(Debug, Clone, Copy)]
pub struct MemAccessEncoding {
    /// Base register
    pub base_reg: u8,
    /// Access size in bytes (1, 2, 4, 8, 16)
    pub size: u8,
    /// Is load (bit 0), is signed (bit 1)
    pub flags: u8,
    /// Offset (varint)
    pub offset: i64,
}

impl MemAccessEncoding {
    pub fn is_load(&self) -> bool {
        self.flags & 1 != 0
    }

    pub fn is_signed(&self) -> bool {
        self.flags & 2 != 0
    }
}

/// Branch info encoding
#[derive(Debug, Clone, Copy)]
pub struct BranchInfoEncoding {
    /// Branch flags: is_conditional (bit 0), is_taken (bit 1)
    pub flags: u8,
    /// Target address delta (varint for compression)
    pub target_delta: i64,
}

impl BranchInfoEncoding {
    pub fn is_conditional(&self) -> bool {
        self.flags & 1 != 0
    }

    pub fn is_taken(&self) -> bool {
        self.flags & 2 != 0
    }
}

/// Index entry for random access (16 bytes each)
#[repr(C, packed)]
#[derive(Debug, Clone, Copy, Default)]
pub struct IndexEntry {
    /// Instruction ID
    pub instr_id: u64,
    /// File offset to instruction data
    pub offset: u64,
}

impl IndexEntry {
    pub const SIZE: usize = 16;
}

/// Encode opcode type to u8
pub fn encode_opcode(opcode: &crate::types::OpcodeType) -> u8 {
    use crate::types::OpcodeType::*;
    match opcode {
        Add => 0,
        Sub => 1,
        Mul => 2,
        Div => 3,
        And => 4,
        Orr => 5,
        Eor => 6,
        Lsl => 7,
        Lsr => 8,
        Asr => 9,
        Load => 10,
        Store => 11,
        LoadPair => 12,
        StorePair => 13,
        Branch => 14,
        BranchCond => 15,
        BranchReg => 16,
        Msr => 17,
        Mrs => 18,
        Sys => 19,
        Nop => 20,
        Fadd => 21,
        Fsub => 22,
        Fmul => 23,
        Fdiv => 24,
        DcZva => 25,
        DcCivac => 26,
        DcCvac => 27,
        DcCsw => 28,
        IcIvau => 29,
        IcIallu => 30,
        IcIalluis => 31,
        Aesd => 32,
        Aese => 33,
        Aesimc => 34,
        Aesmc => 35,
        Sha1H => 36,
        Sha256H => 37,
        Sha512H => 38,
        Vadd => 39,
        Vsub => 40,
        Vmul => 41,
        Vmla => 42,
        Vmls => 43,
        Vld => 44,
        Vst => 45,
        Vdup => 46,
        Vmov => 47,
        Fmadd => 48,
        Fmsub => 49,
        Fnmadd => 50,
        Fnmsub => 51,
        Mov => 52,
        Cmp => 53,
        Shift => 54,
        Load => 55,
        Store => 56,
        LoadPair => 57,
        StorePair => 58,
        BranchCond => 59,
        BranchReg => 60,
        Vadd => 61,
        Vsub => 62,
        Vmul => 63,
        Vmla => 64,
        Vmls => 65,
        Vld => 66,
        Vst => 67,
        Vdup => 68,
        Vmov => 69,
        Fmadd => 70,
        Fmsub => 71,
        Fnmadd => 72,
        Fnmsub => 73,
        Fcvt => 74,
        Dmb => 75,
        Dsb => 76,
        Isb => 77,
        Eret => 78,
        Yield => 79,
        Adr => 80,
        Pmull => 81,
        Other => 255,
    }
}

/// Decode opcode type from u8
pub fn decode_opcode(code: u8) -> crate::types::OpcodeType {
    use crate::types::OpcodeType::*;
    match code {
        0 => Add,
        1 => Sub,
        2 => Mul,
        3 => Div,
        4 => And,
        5 => Orr,
        6 => Eor,
        7 => Lsl,
        8 => Lsr,
        9 => Asr,
        10 => Load,
        11 => Store,
        12 => LoadPair,
        13 => StorePair,
        14 => Branch,
        15 => BranchCond,
        16 => BranchReg,
        17 => Msr,
        18 => Mrs,
        19 => Sys,
        20 => Nop,
        21 => Fadd,
        22 => Fsub,
        23 => Fmul,
        24 => Fdiv,
        25 => DcZva,
        26 => DcCivac,
        27 => DcCvac,
        28 => DcCsw,
        29 => IcIvau,
        30 => IcIallu,
        31 => IcIalluis,
        32 => Aesd,
        33 => Aese,
        34 => Aesimc,
        35 => Aesmc,
        36 => Sha1H,
        37 => Sha256H,
        38 => Sha512H,
        39 => Vadd,
        40 => Vsub,
        41 => Vmul,
        42 => Vmla,
        43 => Vmls,
        44 => Vld,
        45 => Vst,
        46 => Vdup,
        47 => Vmov,
        48 => Fmadd,
        49 => Fmsub,
        50 => Fnmadd,
        51 => Fnmsub,
        52 => Mov,
        53 => Cmp,
        54 => Shift,
        55 => Load,
        56 => Store,
        57 => LoadPair,
        58 => StorePair,
        59 => BranchCond,
        60 => BranchReg,
        61 => Vadd,
        62 => Vsub,
        63 => Vmul,
        64 => Vmla,
        65 => Vmls,
        66 => Vld,
        67 => Vst,
        68 => Vdup,
        69 => Vmov,
        70 => Fmadd,
        71 => Fmsub,
        72 => Fnmadd,
        73 => Fnmsub,
        74 => Fcvt,
        75 => Dmb,
        76 => Dsb,
        77 => Isb,
        78 => Eret,
        79 => Yield,
        80 => Adr,
        81 => Pmull,
        _ => Other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_header_size() {
        assert_eq!(std::mem::size_of::<FileHeader>(), FileHeader::SIZE);
    }

    #[test]
    fn test_instr_header_size() {
        assert_eq!(std::mem::size_of::<InstrHeader>(), InstrHeader::SIZE);
    }

    #[test]
    fn test_index_entry_size() {
        assert_eq!(std::mem::size_of::<IndexEntry>(), IndexEntry::SIZE);
    }

    #[test]
    fn test_opcode_roundtrip() {
        use crate::types::OpcodeType::*;

        let opcodes = [Add, Sub, Load, Store, Branch, Fadd, Vadd, Other];
        for opcode in opcodes {
            assert_eq!(decode_opcode(encode_opcode(&opcode)), opcode);
        }
    }
}
