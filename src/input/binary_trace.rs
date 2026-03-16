//! Binary trace parser for instruction traces.
//!
//! Binary format specification:
//! - Header: 8 bytes magic + 8 bytes version + 8 bytes count
//! - Each instruction:
//!   - 8 bytes: PC (u64)
//!   - 4 bytes: raw opcode (u32)
//!   - 4 bytes: opcode type (u32)
//!   - 1 byte: src_reg_count
//!   - src_reg_count bytes: src registers
//!   - 1 byte: dst_reg_count
//!   - dst_reg_count bytes: dst registers
//!   - 1 byte: flags (bit 0: has_mem, bit 1: has_branch)
//!   - if has_mem: 8 bytes addr + 1 byte size + 1 byte is_load
//!   - if has_branch: 8 bytes target + 1 byte is_conditional + 1 byte is_taken

use crate::types::{BranchInfo, EmulatorError, Instruction, InstructionId, MemAccess, OpcodeType, Reg, Result};
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

const MAGIC: u64 = 0x41524D5F54524143; // "ARM_TRAC" in hex
const VERSION: u64 = 1;

/// Binary trace file header
#[derive(Debug, Clone)]
struct TraceHeader {
    magic: u64,
    version: u64,
    instruction_count: u64,
}

/// Binary trace parser
pub struct BinaryTraceParser {
    reader: BufReader<File>,
    header: TraceHeader,
    current_id: u64,
    instructions_read: u64,
    file_path: String,
}

impl BinaryTraceParser {
    /// Create a parser from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::open(&path).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to open file {}: {}", path_str, e))
        })?;

        let mut reader = BufReader::new(file);

        // Read header
        let header = Self::read_header(&mut reader)?;

        if header.magic != MAGIC {
            return Err(EmulatorError::TraceParseError(
                format!("Invalid magic number: expected {:#x}, got {:#x}", MAGIC, header.magic)
            ));
        }

        if header.version > VERSION {
            return Err(EmulatorError::TraceParseError(
                format!("Unsupported version: {}", header.version)
            ));
        }

        Ok(Self {
            reader,
            header,
            current_id: 0,
            instructions_read: 0,
            file_path: path_str,
        })
    }

    /// Read and validate the trace header
    fn read_header(reader: &mut BufReader<File>) -> Result<TraceHeader> {
        let mut buf = [0u8; 24];

        reader.read_exact(&mut buf).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to read header: {}", e))
        })?;

        let magic = u64::from_le_bytes(buf[0..8].try_into().unwrap());
        let version = u64::from_le_bytes(buf[8..16].try_into().unwrap());
        let instruction_count = u64::from_le_bytes(buf[16..24].try_into().unwrap());

        Ok(TraceHeader {
            magic,
            version,
            instruction_count,
        })
    }

    /// Read a single instruction from the binary stream
    fn read_instruction(&mut self) -> Result<Option<Instruction>> {
        if self.header.instruction_count > 0 && self.instructions_read >= self.header.instruction_count {
            return Ok(None);
        }

        // Read fixed fields
        let mut buf = [0u8; 18]; // PC(8) + opcode(4) + opcode_type(4) + src_count(1) + dst_count(1)

        match self.reader.read_exact(&mut buf) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(EmulatorError::TraceParseError(format!("Read error: {}", e))),
        }

        let pc = u64::from_le_bytes(buf[0..8].try_into().unwrap());
        let raw_opcode = u32::from_le_bytes(buf[8..12].try_into().unwrap());
        let opcode_type_u32 = u32::from_le_bytes(buf[12..16].try_into().unwrap());
        let src_count = buf[16];
        let dst_count = buf[17];

        let opcode_type = Self::u32_to_opcode_type(opcode_type_u32);

        // Read source registers
        let mut src_regs = Vec::with_capacity(src_count as usize);
        if src_count > 0 {
            let mut src_buf = vec![0u8; src_count as usize];
            self.reader.read_exact(&mut src_buf).map_err(|e| {
                EmulatorError::TraceParseError(format!("Failed to read src regs: {}", e))
            })?;
            for &r in &src_buf {
                src_regs.push(Reg(r));
            }
        }

        // Read destination registers
        let mut dst_regs = Vec::with_capacity(dst_count as usize);
        if dst_count > 0 {
            let mut dst_buf = vec![0u8; dst_count as usize];
            self.reader.read_exact(&mut dst_buf).map_err(|e| {
                EmulatorError::TraceParseError(format!("Failed to read dst regs: {}", e))
            })?;
            for &r in &dst_buf {
                dst_regs.push(Reg(r));
            }
        }

        // Read flags
        let mut flags_buf = [0u8; 1];
        self.reader.read_exact(&mut flags_buf).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to read flags: {}", e))
        })?;
        let flags = flags_buf[0];

        let has_mem = (flags & 0x01) != 0;
        let has_branch = (flags & 0x02) != 0;

        // Read memory access info
        let mem_access = if has_mem {
            let mut mem_buf = [0u8; 10]; // addr(8) + size(1) + is_load(1)
            self.reader.read_exact(&mut mem_buf).map_err(|e| {
                EmulatorError::TraceParseError(format!("Failed to read mem info: {}", e))
            })?;
            let addr = u64::from_le_bytes(mem_buf[0..8].try_into().unwrap());
            let size = mem_buf[8];
            let is_load = mem_buf[9] != 0;

            Some(MemAccess { addr, size, is_load })
        } else {
            None
        };

        // Read branch info
        let branch_info = if has_branch {
            let mut branch_buf = [0u8; 10]; // target(8) + is_cond(1) + is_taken(1)
            self.reader.read_exact(&mut branch_buf).map_err(|e| {
                EmulatorError::TraceParseError(format!("Failed to read branch info: {}", e))
            })?;
            let target = u64::from_le_bytes(branch_buf[0..8].try_into().unwrap());
            let is_conditional = branch_buf[8] != 0;
            let is_taken = branch_buf[9] != 0;

            Some(BranchInfo {
                is_conditional,
                target,
                is_taken,
            })
        } else {
            None
        };

        let mut instr = Instruction::new(InstructionId(self.current_id), pc, raw_opcode, opcode_type);
        self.current_id += 1;
        self.instructions_read += 1;

        instr.src_regs = src_regs.into();
        instr.dst_regs = dst_regs.into();
        instr.mem_access = mem_access;
        instr.branch_info = branch_info;

        Ok(Some(instr))
    }

    /// Convert u32 to OpcodeType
    fn u32_to_opcode_type(v: u32) -> OpcodeType {
        match v {
            // Computational
            0 => OpcodeType::Add,
            1 => OpcodeType::Sub,
            2 => OpcodeType::Mul,
            3 => OpcodeType::Div,
            4 => OpcodeType::And,
            5 => OpcodeType::Orr,
            6 => OpcodeType::Eor,
            7 => OpcodeType::Lsl,
            8 => OpcodeType::Lsr,
            9 => OpcodeType::Asr,
            // Load/Store
            10 => OpcodeType::Load,
            11 => OpcodeType::Store,
            12 => OpcodeType::LoadPair,
            13 => OpcodeType::StorePair,
            // Branch
            14 => OpcodeType::Branch,
            15 => OpcodeType::BranchCond,
            16 => OpcodeType::BranchReg,
            // System
            17 => OpcodeType::Msr,
            18 => OpcodeType::Mrs,
            19 => OpcodeType::Sys,
            20 => OpcodeType::Nop,
            // Floating-point
            21 => OpcodeType::Fadd,
            22 => OpcodeType::Fsub,
            23 => OpcodeType::Fmul,
            24 => OpcodeType::Fdiv,
            // Cache Maintenance (25-31)
            25 => OpcodeType::DcZva,
            26 => OpcodeType::DcCivac,
            27 => OpcodeType::DcCvac,
            28 => OpcodeType::DcCsw,
            29 => OpcodeType::IcIvau,
            30 => OpcodeType::IcIallu,
            31 => OpcodeType::IcIalluis,
            // Cryptography (32-38)
            32 => OpcodeType::Aesd,
            33 => OpcodeType::Aese,
            34 => OpcodeType::Aesimc,
            35 => OpcodeType::Aesmc,
            36 => OpcodeType::Sha1H,
            37 => OpcodeType::Sha256H,
            38 => OpcodeType::Sha512H,
            // SIMD/Vector (39-47)
            39 => OpcodeType::Vadd,
            40 => OpcodeType::Vsub,
            41 => OpcodeType::Vmul,
            42 => OpcodeType::Vmla,
            43 => OpcodeType::Vmls,
            44 => OpcodeType::Vld,
            45 => OpcodeType::Vst,
            46 => OpcodeType::Vdup,
            47 => OpcodeType::Vmov,
            // FMA (48-51)
            48 => OpcodeType::Fmadd,
            49 => OpcodeType::Fmsub,
            50 => OpcodeType::Fnmadd,
            51 => OpcodeType::Fnmsub,
            // Other
            255 => OpcodeType::Other,
            _ => OpcodeType::Other,
        }
    }

    /// Get the total instruction count from the header
    pub fn instruction_count(&self) -> u64 {
        self.header.instruction_count
    }
}

impl Iterator for BinaryTraceParser {
    type Item = Result<Instruction>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_instruction() {
            Ok(Some(instr)) => Some(Ok(instr)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl super::InstructionSource for BinaryTraceParser {
    fn total_count(&self) -> Option<usize> {
        Some(self.header.instruction_count as usize)
    }

    fn reset(&mut self) -> Result<()> {
        self.reader.seek(std::io::SeekFrom::Start(24)).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to reset file: {}", e))
        })?;
        self.current_id = 0;
        self.instructions_read = 0;
        Ok(())
    }
}

/// Binary trace writer for creating test files
#[cfg(test)]
pub struct BinaryTraceWriter {
    buffer: Vec<u8>,
}

#[cfg(test)]
impl BinaryTraceWriter {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
        }
    }

    /// Write header (call this first)
    pub fn write_header(&mut self, instruction_count: u64) {
        self.buffer.extend_from_slice(&MAGIC.to_le_bytes());
        self.buffer.extend_from_slice(&VERSION.to_le_bytes());
        self.buffer.extend_from_slice(&instruction_count.to_le_bytes());
    }

    /// Write an instruction
    pub fn write_instruction(&mut self, instr: &Instruction) {
        // PC
        self.buffer.extend_from_slice(&instr.pc.to_le_bytes());
        // Raw opcode
        self.buffer.extend_from_slice(&instr.raw_opcode.to_le_bytes());
        // Opcode type
        let opcode_u32 = Self::opcode_type_to_u32(&instr.opcode_type);
        self.buffer.extend_from_slice(&opcode_u32.to_le_bytes());
        // Source registers
        self.buffer.push(instr.src_regs.len() as u8);
        for reg in &instr.src_regs {
            self.buffer.push(reg.0);
        }
        // Destination registers
        self.buffer.push(instr.dst_regs.len() as u8);
        for reg in &instr.dst_regs {
            self.buffer.push(reg.0);
        }
        // Flags
        let mut flags: u8 = 0;
        if instr.mem_access.is_some() {
            flags |= 0x01;
        }
        if instr.branch_info.is_some() {
            flags |= 0x02;
        }
        self.buffer.push(flags);
        // Memory access
        if let Some(ref mem) = instr.mem_access {
            self.buffer.extend_from_slice(&mem.addr.to_le_bytes());
            self.buffer.push(mem.size);
            self.buffer.push(if mem.is_load { 1 } else { 0 });
        }
        // Branch info
        if let Some(ref br) = instr.branch_info {
            self.buffer.extend_from_slice(&br.target.to_le_bytes());
            self.buffer.push(if br.is_conditional { 1 } else { 0 });
            self.buffer.push(if br.is_taken { 1 } else { 0 });
        }
    }

    fn opcode_type_to_u32(opcode: &OpcodeType) -> u32 {
        match opcode {
            // Computational
            OpcodeType::Add => 0,
            OpcodeType::Sub => 1,
            OpcodeType::Mul => 2,
            OpcodeType::Div => 3,
            OpcodeType::And => 4,
            OpcodeType::Orr => 5,
            OpcodeType::Eor => 6,
            OpcodeType::Lsl => 7,
            OpcodeType::Lsr => 8,
            OpcodeType::Asr => 9,
            // Load/Store
            OpcodeType::Load => 10,
            OpcodeType::Store => 11,
            OpcodeType::LoadPair => 12,
            OpcodeType::StorePair => 13,
            // Branch
            OpcodeType::Branch => 14,
            OpcodeType::BranchCond => 15,
            OpcodeType::BranchReg => 16,
            // System
            OpcodeType::Msr => 17,
            OpcodeType::Mrs => 18,
            OpcodeType::Sys => 19,
            OpcodeType::Nop => 20,
            // Floating-point
            OpcodeType::Fadd => 21,
            OpcodeType::Fsub => 22,
            OpcodeType::Fmul => 23,
            OpcodeType::Fdiv => 24,
            // Cache Maintenance (25-31)
            OpcodeType::DcZva => 25,
            OpcodeType::DcCivac => 26,
            OpcodeType::DcCvac => 27,
            OpcodeType::DcCsw => 28,
            OpcodeType::IcIvau => 29,
            OpcodeType::IcIallu => 30,
            OpcodeType::IcIalluis => 31,
            // Cryptography (32-38)
            OpcodeType::Aesd => 32,
            OpcodeType::Aese => 33,
            OpcodeType::Aesimc => 34,
            OpcodeType::Aesmc => 35,
            OpcodeType::Sha1H => 36,
            OpcodeType::Sha256H => 37,
            OpcodeType::Sha512H => 38,
            // SIMD/Vector (39-47)
            OpcodeType::Vadd => 39,
            OpcodeType::Vsub => 40,
            OpcodeType::Vmul => 41,
            OpcodeType::Vmla => 42,
            OpcodeType::Vmls => 43,
            OpcodeType::Vld => 44,
            OpcodeType::Vst => 45,
            OpcodeType::Vdup => 46,
            OpcodeType::Vmov => 47,
            // FMA (48-51)
            OpcodeType::Fmadd => 48,
            OpcodeType::Fmsub => 49,
            OpcodeType::Fnmadd => 50,
            OpcodeType::Fnmsub => 51,
            // Other
            OpcodeType::Other => 255,
        }
    }

    pub fn to_bytes(&self) -> &[u8] {
        &self.buffer
    }

    pub fn save_to_file(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, &self.buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use smallvec::smallvec;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_binary_roundtrip() {
        let mut writer = BinaryTraceWriter::new();

        let instr1 = Instruction::new(InstructionId(0), 0x1000, 0x8B000000, OpcodeType::Add)
            .with_src_reg(Reg(0))
            .with_src_reg(Reg(1))
            .with_dst_reg(Reg(2));

        let instr2 = Instruction::new(InstructionId(1), 0x1004, 0xF9400000, OpcodeType::Load)
            .with_src_reg(Reg(0))
            .with_dst_reg(Reg(1))
            .with_mem_access(0x2000, 8, true);

        writer.write_header(2);
        writer.write_instruction(&instr1);
        writer.write_instruction(&instr2);

        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(writer.to_bytes()).unwrap();
        temp_file.flush().unwrap();

        let path = temp_file.path();
        let mut parser = BinaryTraceParser::from_file(path).unwrap();

        let read_instr1 = parser.next().unwrap().unwrap();
        assert_eq!(read_instr1.pc, 0x1000);
        assert_eq!(read_instr1.opcode_type, OpcodeType::Add);

        let read_instr2 = parser.next().unwrap().unwrap();
        assert_eq!(read_instr2.pc, 0x1004);
        assert_eq!(read_instr2.opcode_type, OpcodeType::Load);
        assert!(read_instr2.mem_access.is_some());
    }
}
