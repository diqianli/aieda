//! ChampSim trace format parser.
//!
//! ChampSim trace format is a binary format used by the ChampSim simulator.
//! Each instruction is stored as:
//! - 8 bytes: instruction pointer (PC)
//! - 1 byte: is_branch
//! - 1 byte: branch_taken
//! - 2 bytes: destination_registers[2]
//! - 4 bytes: source_registers[4]
//! - 16 bytes: destination_memory[2] (8 bytes each)
//! - 32 bytes: source_memory[4] (8 bytes each)
//! Total: 64 bytes per instruction

use crate::types::{BranchInfo, EmulatorError, Instruction, InstructionId, MemAccess, OpcodeType, Reg, Result};
use std::fs::File;
use std::io::{BufReader, Read, Seek};
use std::path::Path;

const INSTR_SIZE: usize = 64; // 8 + 1 + 1 + 2 + 4 + 16 + 32

/// Parsed ChampSim instruction (unpacked for safe access)
#[derive(Debug, Clone, Copy, Default)]
struct ChampSimInstr {
    ip: u64,
    is_branch: bool,
    branch_taken: bool,
    destination_registers: [u8; 2],
    source_registers: [u8; 4],
    destination_memory: [u64; 2],
    source_memory: [u64; 4],
}

impl ChampSimInstr {
    /// Parse from raw bytes
    fn from_bytes(buf: &[u8; INSTR_SIZE]) -> Self {
        Self {
            ip: u64::from_le_bytes(buf[0..8].try_into().unwrap()),
            is_branch: buf[8] != 0,
            branch_taken: buf[9] != 0,
            destination_registers: [buf[10], buf[11]],
            source_registers: [buf[12], buf[13], buf[14], buf[15]],
            destination_memory: [
                u64::from_le_bytes(buf[16..24].try_into().unwrap()),
                u64::from_le_bytes(buf[24..32].try_into().unwrap()),
            ],
            source_memory: [
                u64::from_le_bytes(buf[32..40].try_into().unwrap()),
                u64::from_le_bytes(buf[40..48].try_into().unwrap()),
                u64::from_le_bytes(buf[48..56].try_into().unwrap()),
                u64::from_le_bytes(buf[56..64].try_into().unwrap()),
            ],
        }
    }

    /// Check if instruction has memory access
    fn has_memory_access(&self) -> bool {
        self.source_memory.iter().any(|&x| x != 0)
            || self.destination_memory.iter().any(|&x| x != 0)
    }
}

/// ChampSim trace parser
pub struct ChampSimTraceParser {
    reader: BufReader<File>,
    current_id: u64,
    instructions_read: u64,
    file_path: String,
    buffer: [u8; INSTR_SIZE],
}

impl ChampSimTraceParser {
    /// Create a parser from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::open(&path).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to open file {}: {}", path_str, e))
        })?;

        Ok(Self {
            reader: BufReader::new(file),
            current_id: 0,
            instructions_read: 0,
            file_path: path_str,
            buffer: [0u8; INSTR_SIZE],
        })
    }

    /// Read a single instruction
    fn read_instruction(&mut self) -> Result<Option<ChampSimInstr>> {
        match self.reader.read_exact(&mut self.buffer) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(EmulatorError::TraceParseError(format!("Read error: {}", e))),
        }

        let instr = ChampSimInstr::from_bytes(&self.buffer);
        self.instructions_read += 1;
        Ok(Some(instr))
    }

    /// Convert ChampSim instruction to our Instruction type
    fn convert_instruction(&mut self, cs_instr: ChampSimInstr) -> Instruction {
        // Determine opcode type
        let opcode_type = if cs_instr.is_branch {
            OpcodeType::Branch
        } else if cs_instr.has_memory_access() {
            if cs_instr.destination_memory[0] != 0 || cs_instr.destination_memory[1] != 0 {
                OpcodeType::Store
            } else {
                OpcodeType::Load
            }
        } else {
            OpcodeType::Other
        };

        let mut instr = Instruction::new(
            InstructionId(self.current_id),
            cs_instr.ip,
            0,
            opcode_type,
        );
        self.current_id += 1;

        // Add source registers (filter out 0s which mean "no register")
        for &reg in &cs_instr.source_registers {
            if reg != 0 {
                instr.src_regs.push(Reg(reg));
            }
        }

        // Add destination registers
        for &reg in &cs_instr.destination_registers {
            if reg != 0 {
                instr.dst_regs.push(Reg(reg));
            }
        }

        // Add memory accesses
        // Source memory = loads
        for &addr in &cs_instr.source_memory {
            if addr != 0 {
                instr.mem_access = Some(MemAccess {
                    addr,
                    size: 8,
                    is_load: true,
                });
                break;
            }
        }

        // Destination memory = stores
        for &addr in &cs_instr.destination_memory {
            if addr != 0 {
                instr.mem_access = Some(MemAccess {
                    addr,
                    size: 8,
                    is_load: false,
                });
                break;
            }
        }

        // Add branch info
        if cs_instr.is_branch {
            instr.branch_info = Some(BranchInfo {
                is_conditional: true,
                target: 0,
                is_taken: cs_instr.branch_taken,
            });
        }

        instr
    }

    /// Get the number of instructions read so far
    pub fn instructions_read(&self) -> u64 {
        self.instructions_read
    }
}

impl Iterator for ChampSimTraceParser {
    type Item = Result<Instruction>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_instruction() {
            Ok(Some(cs_instr)) => {
                let instr = self.convert_instruction(cs_instr);
                Some(Ok(instr))
            }
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl super::InstructionSource for ChampSimTraceParser {
    fn total_count(&self) -> Option<usize> {
        None
    }

    fn reset(&mut self) -> Result<()> {
        self.reader.seek(std::io::SeekFrom::Start(0)).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to reset file: {}", e))
        })?;
        self.current_id = 0;
        self.instructions_read = 0;
        Ok(())
    }
}

/// XZ-compressed ChampSim trace parser
pub struct ChampSimXzTraceParser {
    decoder: xz2::read::XzDecoder<BufReader<File>>,
    current_id: u64,
    instructions_read: u64,
    file_path: String,
    buffer: [u8; INSTR_SIZE],
}

impl ChampSimXzTraceParser {
    /// Create a parser from an XZ-compressed file
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::open(&path).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to open file {}: {}", path_str, e))
        })?;

        Ok(Self {
            decoder: xz2::read::XzDecoder::new(BufReader::new(file)),
            current_id: 0,
            instructions_read: 0,
            file_path: path_str,
            buffer: [0u8; INSTR_SIZE],
        })
    }

    /// Read a single instruction
    fn read_instruction(&mut self) -> Result<Option<ChampSimInstr>> {
        let mut total_read = 0;
        while total_read < INSTR_SIZE {
            match std::io::Read::read(&mut self.decoder, &mut self.buffer[total_read..]) {
                Ok(0) => {
                    if total_read == 0 {
                        return Ok(None);
                    } else {
                        return Err(EmulatorError::TraceParseError(
                            "Unexpected end of compressed file".to_string()
                        ));
                    }
                }
                Ok(n) => total_read += n,
                Err(e) => return Err(EmulatorError::TraceParseError(format!("Read error: {}", e))),
            }
        }

        let instr = ChampSimInstr::from_bytes(&self.buffer);
        self.instructions_read += 1;
        Ok(Some(instr))
    }

    /// Convert ChampSim instruction to our Instruction type
    fn convert_instruction(&mut self, cs_instr: ChampSimInstr) -> Instruction {
        // Same logic as ChampSimTraceParser
        let opcode_type = if cs_instr.is_branch {
            OpcodeType::Branch
        } else if cs_instr.has_memory_access() {
            if cs_instr.destination_memory[0] != 0 || cs_instr.destination_memory[1] != 0 {
                OpcodeType::Store
            } else {
                OpcodeType::Load
            }
        } else {
            OpcodeType::Other
        };

        let mut instr = Instruction::new(
            InstructionId(self.current_id),
            cs_instr.ip,
            0,
            opcode_type,
        );
        self.current_id += 1;

        for &reg in &cs_instr.source_registers {
            if reg != 0 {
                instr.src_regs.push(Reg(reg));
            }
        }

        for &reg in &cs_instr.destination_registers {
            if reg != 0 {
                instr.dst_regs.push(Reg(reg));
            }
        }

        for &addr in &cs_instr.source_memory {
            if addr != 0 {
                instr.mem_access = Some(MemAccess { addr, size: 8, is_load: true });
                break;
            }
        }

        for &addr in &cs_instr.destination_memory {
            if addr != 0 {
                instr.mem_access = Some(MemAccess { addr, size: 8, is_load: false });
                break;
            }
        }

        if cs_instr.is_branch {
            instr.branch_info = Some(BranchInfo {
                is_conditional: true,
                target: 0,
                is_taken: cs_instr.branch_taken,
            });
        }

        instr
    }

    /// Get the number of instructions read so far
    pub fn instructions_read(&self) -> u64 {
        self.instructions_read
    }
}

impl Iterator for ChampSimXzTraceParser {
    type Item = Result<Instruction>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.read_instruction() {
            Ok(Some(cs_instr)) => {
                let instr = self.convert_instruction(cs_instr);
                Some(Ok(instr))
            }
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

impl super::InstructionSource for ChampSimXzTraceParser {
    fn total_count(&self) -> Option<usize> {
        None
    }

    fn reset(&mut self) -> Result<()> {
        // For XZ compressed files, we need to re-open the file
        let file = File::open(&self.file_path).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to reopen file: {}", e))
        })?;
        self.decoder = xz2::read::XzDecoder::new(BufReader::new(file));
        self.current_id = 0;
        self.instructions_read = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_champsim_instr_size() {
        // Verify the buffer size matches the expected binary format
        assert_eq!(INSTR_SIZE, 64);
    }
}
