//! Binary trace reader with streaming support for large files.

use std::fs::File;
use std::io::{self, BufReader, Read, Seek, SeekFrom};
use std::path::Path;

use crate::trace::binary_format::*;
use crate::types::{BranchInfo, Instruction, InstructionId, MemAccess, OpcodeType, Reg, VReg};
use smallvec::SmallVec;

/// Binary trace reader
pub struct BinaryTraceReader<R: Read + Seek> {
    reader: BufReader<R>,
    header: FileHeader,
    string_table: Vec<String>,
    index: Vec<IndexEntry>,
    current_position: u64,
    instructions_read: u64,
    last_pc: u64,
}

impl BinaryTraceReader<File> {
    /// Open a trace file
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        Self::new(file)
    }
}

impl<R: Read + Seek> BinaryTraceReader<R> {
    /// Create a new reader from a Read + Seek source
    pub fn new(mut reader: R) -> io::Result<Self> {
        // Read header
        let mut header_bytes = [0u8; FileHeader::SIZE];
        reader.read_exact(&mut header_bytes)?;

        let header: FileHeader = unsafe {
            std::ptr::read_unaligned(header_bytes.as_ptr() as *const FileHeader)
        };

        header.validate().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Read string table
        let string_table = if header.string_table_size > 0 {
            reader.seek(SeekFrom::Start(header.string_table_offset))?;
            let mut strings = Vec::new();
            let mut remaining = header.string_table_size as usize;

            while remaining > 0 {
                // Read length prefix
                let mut len_bytes = [0u8; 2];
                reader.read_exact(&mut len_bytes)?;
                let len = u16::from_le_bytes(len_bytes) as usize;
                remaining -= 2;

                // Read string
                let mut s = vec![0u8; len];
                reader.read_exact(&mut s)?;
                remaining -= len;

                strings.push(String::from_utf8_lossy(&s).into_owned());
            }
            strings
        } else {
            Vec::new()
        };

        // Read index if present
        let index = if header.has_index() && header.index_table_size > 0 {
            reader.seek(SeekFrom::Start(header.index_table_offset))?;
            let count = header.index_table_size as usize / IndexEntry::SIZE;

            let mut entries = Vec::with_capacity(count);
            for _ in 0..count {
                let mut entry_bytes = [0u8; IndexEntry::SIZE];
                reader.read_exact(&mut entry_bytes)?;

                let entry: IndexEntry = unsafe {
                    std::ptr::read_unaligned(entry_bytes.as_ptr() as *const IndexEntry)
                };
                entries.push(entry);
            }
            entries
        } else {
            Vec::new()
        };

        // Seek to instruction stream start
        let data_start = FileHeader::SIZE as u64;
        reader.seek(SeekFrom::Start(data_start))?;

        Ok(Self {
            reader: BufReader::new(reader),
            header,
            string_table,
            index,
            current_position: data_start,
            instructions_read: 0,
            last_pc: 0,
        })
    }

    /// Get the file header
    pub fn header(&self) -> &FileHeader {
        &self.header
    }

    /// Get total instruction count
    pub fn instr_count(&self) -> u64 {
        self.header.instr_count
    }

    /// Check if random access is available
    pub fn has_index(&self) -> bool {
        !self.index.is_empty()
    }

    /// Seek to a specific instruction by ID (requires index)
    pub fn seek_to_instruction(&mut self, instr_id: u64) -> io::Result<()> {
        if !self.has_index() {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Random access requires index table",
            ));
        }

        // Binary search for instruction
        let idx = self.index.binary_search_by_key(&instr_id, |e| e.instr_id);
        match idx {
            Ok(i) => {
                let offset = self.index[i].offset;
                self.reader.seek(SeekFrom::Start(offset))?;
                self.current_position = offset;
                self.instructions_read = instr_id;
                Ok(())
            }
            Err(_) => Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Instruction {} not found", instr_id),
            )),
        }
    }

    /// Read the next instruction
    pub fn read_next(&mut self) -> io::Result<Option<Instruction>> {
        if self.instructions_read >= self.header.instr_count {
            return Ok(None);
        }

        // Check if we've reached string table
        if self.current_position >= self.header.string_table_offset {
            return Ok(None);
        }

        // Read instruction header
        let mut header_bytes = [0u8; InstrHeader::SIZE];
        match self.reader.read_exact(&mut header_bytes) {
            Ok(()) => {}
            Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }
        self.current_position += InstrHeader::SIZE as u64;

        let header: InstrHeader = unsafe {
            std::ptr::read_unaligned(header_bytes.as_ptr() as *const InstrHeader)
        };

        // Read raw opcode
        let mut raw_opcode_bytes = [0u8; 4];
        self.reader.read_exact(&mut raw_opcode_bytes)?;
        let raw_opcode = u32::from_le_bytes(raw_opcode_bytes);
        self.current_position += 4;

        // Determine PC
        let pc = if header.flags.has_extended_pc() {
            let mut pc_bytes = [0u8; 8];
            self.reader.read_exact(&mut pc_bytes)?;
            self.current_position += 8;
            u64::from_le_bytes(pc_bytes)
        } else {
            self.last_pc + header.pc_delta as u64
        };

        // Create instruction
        let opcode_type = decode_opcode(header.opcode);
        let mut instr = Instruction::new(
            InstructionId(header.id as u64),
            pc,
            raw_opcode,
            opcode_type,
        );

        // Read operands
        for _ in 0..header.operand_count {
            let operand = self.read_operand()?;
            // Determine if src or dst based on high bit
            // For simplicity, first half are src, second half are dst
        }

        // Read memory access if present
        if header.flags.has_mem() {
            instr.mem_access = Some(self.read_mem_access()?);
        }

        // Read branch info if present
        if header.flags.has_branch() {
            instr.branch_info = Some(self.read_branch_info(self.last_pc)?);
        }

        // Read disassembly if present
        if header.flags.has_disasm() {
            let mut idx_bytes = [0u8; 4];
            self.reader.read_exact(&mut idx_bytes)?;
            let idx = u32::from_le_bytes(idx_bytes) as usize;
            self.current_position += 4;

            if idx < self.string_table.len() {
                instr.disasm = Some(self.string_table[idx].clone());
            }
        }

        self.last_pc = pc;
        self.instructions_read += 1;

        Ok(Some(instr))
    }

    fn read_operand(&mut self) -> io::Result<u8> {
        let mut type_byte = [0u8; 1];
        self.reader.read_exact(&mut type_byte)?;
        let op_type = type_byte[0] & 0x7F;
        let _is_dst = type_byte[0] & 0x80 != 0;

        let mut value_byte = [0u8; 1];
        self.reader.read_exact(&mut value_byte)?;
        self.current_position += 2;

        Ok(op_type)
    }

    fn read_mem_access(&mut self) -> io::Result<MemAccess> {
        let mut flags_byte = [0u8; 1];
        self.reader.read_exact(&mut flags_byte)?;
        let is_load = flags_byte[0] & 1 != 0;

        let mut size_byte = [0u8; 1];
        self.reader.read_exact(&mut size_byte)?;
        let size = size_byte[0];

        let addr = self.read_varint()?;
        self.current_position += 2;

        Ok(MemAccess {
            addr,
            size,
            is_load,
        })
    }

    fn read_branch_info(&mut self, last_pc: u64) -> io::Result<BranchInfo> {
        let mut flags_byte = [0u8; 1];
        self.reader.read_exact(&mut flags_byte)?;

        let is_conditional = flags_byte[0] & 1 != 0;
        let is_taken = flags_byte[0] & 2 != 0;

        let target_delta = self.read_signed_varint()?;
        self.current_position += 1;

        let target = (last_pc as i64 + target_delta) as u64;

        Ok(BranchInfo {
            is_conditional,
            target,
            is_taken,
        })
    }

    /// Read unsigned varint
    fn read_varint(&mut self) -> io::Result<u64> {
        let mut result = 0u64;
        let mut shift = 0;

        loop {
            let mut byte = [0u8; 1];
            self.reader.read_exact(&mut byte)?;
            let b = byte[0];

            result |= ((b & 0x7F) as u64) << shift;
            shift += 7;

            if b & 0x80 == 0 {
                break;
            }

            if shift >= 64 {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "Varint too large",
                ));
            }
        }

        Ok(result)
    }

    /// Read signed varint using zigzag encoding
    fn read_signed_varint(&mut self) -> io::Result<i64> {
        let zigzag = self.read_varint()?;
        // Zigzag decode: (n >> 1) ^ -(n & 1)
        Ok((zigzag >> 1) as i64 ^ -((zigzag & 1) as i64))
    }

    /// Read a range of instructions (for region loading)
    pub fn read_range(&mut self, start: u64, end: u64) -> io::Result<Vec<Instruction>> {
        if start >= end {
            return Ok(Vec::new());
        }

        // Seek to start position if index is available
        if self.has_index() {
            self.seek_to_instruction(start)?;
        } else {
            // No index - need to read from beginning
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "Range reading requires index table",
            ));
        }

        let count = (end - start) as usize;
        let mut instructions = Vec::with_capacity(count.min(10000));

        while instructions.len() < count {
            match self.read_next()? {
                Some(instr) => {
                    if instr.id.0 >= end {
                        break;
                    }
                    instructions.push(instr);
                }
                None => break,
            }
        }

        Ok(instructions)
    }

    /// Stream instructions as an iterator
    pub fn stream(mut self) -> impl Iterator<Item = io::Result<Instruction>> {
        std::iter::from_fn(move || match self.read_next() {
            Ok(Some(instr)) => Some(Ok(instr)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        })
    }
}

/// Streaming iterator for instructions
pub struct InstructionStream<R: Read + Seek> {
    reader: BinaryTraceReader<R>,
}

impl<R: Read + Seek> Iterator for InstructionStream<R> {
    type Item = io::Result<Instruction>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_next() {
            Ok(Some(instr)) => Some(Ok(instr)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trace::BinaryTraceWriter;
    use std::io::Cursor;

    #[test]
    fn test_roundtrip() {
        let mut cursor = Cursor::new(Vec::new());

        // Write
        {
            let mut writer = BinaryTraceWriter::new(&mut cursor).unwrap();
            for i in 0..10 {
                let instr = Instruction::new(
                    InstructionId(i),
                    0x1000 + i * 4,
                    0x12345678,
                    OpcodeType::Add,
                )
                .with_src_reg(Reg(0))
                .with_dst_reg(Reg(1))
                .with_disasm(&format!("ADD X1, X0 #{}", i));
                writer.write_instruction(&instr).unwrap();
            }
            writer.finish().unwrap();
        }

        // Read
        cursor.set_position(0);
        let mut reader = BinaryTraceReader::new(&mut cursor).unwrap();

        assert_eq!(reader.instr_count(), 10);

        let mut count = 0;
        while let Some(instr) = reader.read_next().unwrap() {
            assert_eq!(instr.id.0, count);
            assert_eq!(instr.opcode_type, OpcodeType::Add);
            count += 1;
        }
        assert_eq!(count, 10);
    }

    #[test]
    fn test_stream_interface() {
        let mut cursor = Cursor::new(Vec::new());

        // Write
        {
            let mut writer = BinaryTraceWriter::new(&mut cursor).unwrap();
            for i in 0..5 {
                let instr = Instruction::new(
                    InstructionId(i),
                    0x1000 + i * 4,
                    0x12345678,
                    OpcodeType::Load,
                )
                .with_dst_reg(Reg(0))
                .with_mem_access(0x2000 + i * 8, 8, true);
                writer.write_instruction(&instr).unwrap();
            }
            writer.finish().unwrap();
        }

        // Stream read
        cursor.set_position(0);
        let reader = BinaryTraceReader::new(&mut cursor).unwrap();
        let instructions: Vec<_> = reader.stream().collect::<Result<_, _>>().unwrap();

        assert_eq!(instructions.len(), 5);
    }
}
