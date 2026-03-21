//! Binary trace writer for efficient trace output.

use std::collections::HashMap;
use std::io::{self, BufWriter, Seek, SeekFrom, Write};

use crate::trace::binary_format::*;
use crate::types::{Instruction, Reg, VReg};

/// String table for deduplication
#[derive(Default)]
struct StringTable {
    strings: Vec<String>,
    index: HashMap<String, u32>,
}

impl StringTable {
    fn intern(&mut self, s: &str) -> u32 {
        if let Some(&idx) = self.index.get(s) {
            idx
        } else {
            let idx = self.strings.len() as u32;
            self.index.insert(s.to_string(), idx);
            self.strings.push(s.to_string());
            idx
        }
    }

    fn serialized_size(&self) -> u32 {
        let mut size = 0u32;
        for s in &self.strings {
            size += 2; // length prefix (u16)
            size += s.len() as u32;
        }
        size
    }

    fn serialize(&self, writer: &mut impl Write) -> io::Result<()> {
        for s in &self.strings {
            writer.write_all(&(s.len() as u16).to_le_bytes())?;
            writer.write_all(s.as_bytes())?;
        }
        Ok(())
    }
}

/// Binary trace writer
pub struct BinaryTraceWriter<W: Write + Seek> {
    writer: BufWriter<W>,
    string_table: StringTable,
    last_pc: u64,
    instr_count: u64,
    header_position: u64,
    data_start: u64,
    /// Index entries for random access (optional)
    index_entries: Vec<IndexEntry>,
    /// Whether to build index
    build_index: bool,
    /// Current write position for index
    current_offset: u64,
}

impl<W: Write + Seek> BinaryTraceWriter<W> {
    /// Create a new binary trace writer
    pub fn new(writer: W) -> io::Result<Self> {
        Self::with_options(writer, false)
    }

    /// Create a new binary trace writer with index building
    pub fn with_index(writer: W) -> io::Result<Self> {
        Self::with_options(writer, true)
    }

    fn with_options(mut writer: W, build_index: bool) -> io::Result<Self> {
        // Write placeholder header
        let header = FileHeader::default();
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &header as *const FileHeader as *const u8,
                FileHeader::SIZE,
            )
        };
        writer.write_all(header_bytes)?;

        let header_position = 0u64;
        let data_start = FileHeader::SIZE as u64;

        Ok(Self {
            writer: BufWriter::new(writer),
            string_table: StringTable::default(),
            last_pc: 0,
            instr_count: 0,
            header_position,
            data_start,
            index_entries: Vec::new(),
            build_index,
            current_offset: data_start,
        })
    }

    /// Write a single instruction
    pub fn write_instruction(&mut self, instr: &Instruction) -> io::Result<()> {
        if self.build_index {
            self.index_entries.push(IndexEntry {
                instr_id: instr.id.0,
                offset: self.current_offset,
            });
        }

        // Build instruction header
        let pc_delta = if instr.pc >= self.last_pc {
            (instr.pc - self.last_pc) as u16
        } else {
            // Handle backward jumps - use extended PC
            0
        };

        let mut flags = InstrFlags::default();

        if instr.mem_access.is_some() {
            flags.0 |= InstrFlags::HAS_MEM;
        }
        if instr.branch_info.is_some() {
            flags.0 |= InstrFlags::HAS_BRANCH;
        }
        if instr.disasm.is_some() {
            flags.0 |= InstrFlags::HAS_DISASM;
        }
        if pc_delta == 0 && instr.pc != self.last_pc {
            flags.0 |= InstrFlags::EXTENDED_PC;
        }

        // Count operands
        let operand_count = (instr.src_regs.len()
            + instr.dst_regs.len()
            + instr.src_vregs.len()
            + instr.dst_vregs.len()) as u8;

        let header = InstrHeader {
            id: instr.id.0 as u32,
            opcode: encode_opcode(&instr.opcode_type),
            flags,
            pc_delta,
            operand_count,
        };

        // Write header
        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &header as *const InstrHeader as *const u8,
                InstrHeader::SIZE,
            )
        };
        self.writer.write_all(header_bytes)?;
        self.current_offset += InstrHeader::SIZE as u64;

        // Write raw opcode (4 bytes)
        self.writer.write_all(&instr.raw_opcode.to_le_bytes())?;
        self.current_offset += 4;

        // Write extended PC if needed
        if flags.has_extended_pc() {
            self.writer.write_all(&instr.pc.to_le_bytes())?;
            self.current_offset += 8;
        }

        // Write operands (compact encoding)
        self.write_operands(instr)?;

        // Write memory access if present
        if let Some(ref mem) = instr.mem_access {
            self.write_mem_access(mem)?;
        }

        // Write branch info if present
        if let Some(ref branch) = instr.branch_info {
            self.write_branch_info(branch, self.last_pc)?;
        }

        // Write disassembly string index if present
        if let Some(ref disasm) = instr.disasm {
            let idx = self.string_table.intern(disasm);
            self.writer.write_all(&idx.to_le_bytes())?;
            self.current_offset += 4;
        }

        self.last_pc = instr.pc;
        self.instr_count += 1;

        Ok(())
    }

    fn write_operands(&mut self, instr: &Instruction) -> io::Result<()> {
        // Write source registers
        for reg in &instr.src_regs {
            self.writer.write_all(&[OperandType::Reg as u8])?;
            self.writer.write_all(&[reg.0])?;
            self.current_offset += 2;
        }

        // Write destination registers (type | 0x80 to indicate dst)
        for reg in &instr.dst_regs {
            self.writer.write_all(&[OperandType::Reg as u8 | 0x80])?;
            self.writer.write_all(&[reg.0])?;
            self.current_offset += 2;
        }

        // Write source vector registers
        for vreg in &instr.src_vregs {
            self.writer.write_all(&[OperandType::VReg as u8])?;
            self.writer.write_all(&[vreg.0])?;
            self.current_offset += 2;
        }

        // Write destination vector registers (type | 0x80 to indicate dst)
        for vreg in &instr.dst_vregs {
            self.writer.write_all(&[OperandType::VReg as u8 | 0x80])?;
            self.writer.write_all(&[vreg.0])?;
            self.current_offset += 2;
        }

        Ok(())
    }

    fn write_mem_access(&mut self, mem: &crate::types::MemAccess) -> io::Result<()> {
        let mut flags: u8 = if mem.is_load { 1 } else { 0 };
        self.writer.write_all(&[flags])?;
        self.writer.write_all(&[mem.size])?;

        // Write address as varint
        self.write_varint(mem.addr)?;
        self.current_offset += 2;

        Ok(())
    }

    fn write_branch_info(
        &mut self,
        branch: &crate::types::BranchInfo,
        last_pc: u64,
    ) -> io::Result<()> {
        let mut flags: u8 = 0;
        if branch.is_conditional {
            flags |= 1;
        }
        if branch.is_taken {
            flags |= 2;
        }
        self.writer.write_all(&[flags])?;

        // Write target delta as signed varint
        let target_delta = branch.target as i64 - last_pc as i64;
        self.write_signed_varint(target_delta)?;
        self.current_offset += 1;

        Ok(())
    }

    /// Write unsigned varint (variable-length integer)
    fn write_varint(&mut self, mut value: u64) -> io::Result<()> {
        loop {
            let mut byte = (value & 0x7F) as u8;
            value >>= 7;
            if value != 0 {
                byte |= 0x80;
            }
            self.writer.write_all(&[byte])?;
            self.current_offset += 1;
            if value == 0 {
                break;
            }
        }
        Ok(())
    }

    /// Write signed varint using zigzag encoding
    fn write_signed_varint(&mut self, value: i64) -> io::Result<()> {
        // Zigzag encoding: (n << 1) ^ (n >> 63)
        let zigzag = ((value << 1) ^ (value >> 63)) as u64;
        self.write_varint(zigzag)
    }

    /// Flush and finalize the trace file
    pub fn finish(mut self) -> io::Result<()> {
        // Calculate positions
        let string_table_offset = self.current_offset;
        let string_table_size = self.string_table.serialized_size();

        // Write string table
        self.string_table.serialize(&mut self.writer)?;

        // Write index table if enabled
        let (index_table_offset, index_table_size) = if self.build_index {
            let offset = self.current_offset + string_table_size as u64;
            let size = (self.index_entries.len() * IndexEntry::SIZE) as u32;

            for entry in &self.index_entries {
                self.writer.write_all(&entry.instr_id.to_le_bytes())?;
                self.writer.write_all(&entry.offset.to_le_bytes())?;
            }

            (offset, size)
        } else {
            (0, 0)
        };

        // Seek back and write final header
        self.writer.flush()?;
        let header = FileHeader {
            magic: *MAGIC,
            version: VERSION,
            flags: if self.build_index { FileFlags::HAS_INDEX } else { FileFlags::NONE },
            instr_count: self.instr_count,
            string_table_offset,
            string_table_size,
            index_table_offset,
            index_table_size,
            ..Default::default()
        };

        let header_bytes = unsafe {
            std::slice::from_raw_parts(
                &header as *const FileHeader as *const u8,
                FileHeader::SIZE,
            )
        };

        self.writer.get_mut().seek(SeekFrom::Start(0))?;
        self.writer.get_mut().write_all(header_bytes)?;
        self.writer.flush()?;

        Ok(())
    }

    /// Get current instruction count
    pub fn instr_count(&self) -> u64 {
        self.instr_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{InstructionId, OpcodeType};
    use std::io::Cursor;

    #[test]
    fn test_write_single_instruction() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = BinaryTraceWriter::new(&mut cursor).unwrap();
            let instr = Instruction::new(InstructionId(0), 0x1000, 0x12345678, OpcodeType::Add)
                .with_src_reg(Reg(0))
                .with_dst_reg(Reg(1))
                .with_disasm("ADD X1, X0");
            writer.write_instruction(&instr).unwrap();
            writer.finish().unwrap();
        }

        let data = cursor.into_inner();
        assert!(data.len() > FileHeader::SIZE);

        // Verify magic number
        assert_eq!(&data[0..8], MAGIC);
    }

    #[test]
    fn test_write_with_memory() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = BinaryTraceWriter::new(&mut cursor).unwrap();
            let instr = Instruction::new(InstructionId(0), 0x1000, 0x12345678, OpcodeType::Load)
                .with_dst_reg(Reg(0))
                .with_mem_access(0x2000, 8, true);
            writer.write_instruction(&instr).unwrap();
            writer.finish().unwrap();
        }

        let data = cursor.into_inner();
        assert!(data.len() > FileHeader::SIZE);
    }

    #[test]
    fn test_write_with_branch() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = BinaryTraceWriter::new(&mut cursor).unwrap();
            let instr = Instruction::new(InstructionId(0), 0x1000, 0x12345678, OpcodeType::BranchCond)
                .with_branch(0x1100, true, true);
            writer.write_instruction(&instr).unwrap();
            writer.finish().unwrap();
        }

        let data = cursor.into_inner();
        assert!(data.len() > FileHeader::SIZE);
    }
}
