//! Text trace parser for instruction traces.

use crate::types::{BranchInfo, EmulatorError, Instruction, InstructionId, MemAccess, OpcodeType, Reg, Result};
use std::fs::File;
use std::io::{BufRead, BufReader, Seek};
use std::path::Path;

/// Text trace format parser
pub struct TextTraceParser {
    reader: BufReader<File>,
    current_id: u64,
    line_buffer: String,
    file_path: String,
}

impl TextTraceParser {
    /// Create a parser from a file path
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path_str = path.as_ref().to_string_lossy().to_string();
        let file = File::open(&path).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to open file {}: {}", path_str, e))
        })?;

        Ok(Self {
            reader: BufReader::new(file),
            current_id: 0,
            line_buffer: String::new(),
            file_path: path_str,
        })
    }

    /// Parse a single line into an instruction
    fn parse_line(&mut self, line: &str) -> Result<Option<Instruction>> {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            return Ok(None);
        }

        // Parse line format:
        // Format 1: PC OPCODE_TYPE [src_regs] [dst_regs] [mem_addr] [branch_info]
        // Format 2: PC: DISASSEMBLY
        //
        // Example formats:
        // 0x1000 LOAD R0,R1 R2 0x2000
        // 0x1004 ADD R0,R1 R2
        // 0x1008 BRANCH 0x2000 TAKEN
        // 0x100c: ADD X2, X0, X1

        // Try format 2 (with colon and disassembly)
        if let Some(colon_pos) = line.find(':') {
            let pc_str = line[..colon_pos].trim();
            let disasm = line[colon_pos + 1..].trim();

            let pc = Self::parse_hex(pc_str)?;
            let instr = self.parse_disassembly(pc, disasm)?;
            return Ok(Some(instr));
        }

        // Format 1 (structured format)
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(None);
        }

        let pc = Self::parse_hex(parts[0])?;
        let opcode_type = if parts.len() > 1 {
            Self::parse_opcode_type(parts[1])?
        } else {
            OpcodeType::Other
        };

        let mut instr = Instruction::new(
            InstructionId(self.current_id),
            pc,
            0,
            opcode_type,
        );
        self.current_id += 1;

        // Parse remaining fields
        let mut idx = 2;
        while idx < parts.len() {
            let part = parts[idx];

            // Source registers (format: R0,R1,R2 or X0,X1,X2)
            if part.starts_with("R:") || part.starts_with("X:") || part.starts_with("src=") {
                let regs_str = part.split(':').last().unwrap_or(part.split('=').last().unwrap_or(""));
                for reg_str in regs_str.split(',') {
                    if let Some(reg) = Self::parse_reg(reg_str) {
                        instr.src_regs.push(reg);
                    }
                }
            }
            // Destination registers
            else if part.starts_with("W:") || part.starts_with("dst=") {
                let regs_str = part.split(':').last().unwrap_or(part.split('=').last().unwrap_or(""));
                for reg_str in regs_str.split(',') {
                    if let Some(reg) = Self::parse_reg(reg_str) {
                        instr.dst_regs.push(reg);
                    }
                }
            }
            // Memory address
            else if part.starts_with("mem=") || part.starts_with("addr=") {
                let addr_str = part.split('=').last().unwrap_or("0");
                let addr = Self::parse_hex(addr_str)?;
                let size = if idx + 1 < parts.len() && parts[idx + 1].starts_with("size=") {
                    idx += 1;
                    parts[idx].split('=').last().unwrap_or("8").parse().unwrap_or(8)
                } else {
                    8
                };
                instr.mem_access = Some(MemAccess {
                    addr,
                    size,
                    is_load: opcode_type == OpcodeType::Load || opcode_type == OpcodeType::LoadPair,
                });
            }
            // Branch target
            else if part.starts_with("target=") || part.starts_with("br=") {
                let target_str = part.split('=').last().unwrap_or("0");
                let target = Self::parse_hex(target_str)?;
                let is_taken = if idx + 1 < parts.len() {
                    idx += 1;
                    parts[idx].to_lowercase() == "taken"
                } else {
                    true
                };
                instr.branch_info = Some(BranchInfo {
                    is_conditional: opcode_type == OpcodeType::BranchCond,
                    target,
                    is_taken,
                });
            }

            idx += 1;
        }

        Ok(Some(instr))
    }

    /// Parse disassembly into instruction
    fn parse_disassembly(&mut self, pc: u64, disasm: &str) -> Result<Instruction> {
        let parts: Vec<&str> = disasm.split_whitespace().collect();
        if parts.is_empty() {
            return Err(EmulatorError::TraceParseError(
                format!("Empty disassembly at PC {:#x}", pc)
            ));
        }

        let mnemonic = parts[0].to_uppercase();
        let opcode_type = Self::mnemonic_to_opcode(&mnemonic);

        let mut instr = Instruction::new(
            InstructionId(self.current_id),
            pc,
            0,
            opcode_type,
        ).with_disasm(disasm);
        self.current_id += 1;

        // Parse operands (simplified)
        if parts.len() > 1 {
            let operands_str = parts[1..].join("");
            let operands: Vec<&str> = operands_str.split(',').collect();

            for (i, op) in operands.iter().enumerate() {
                let op = op.trim();
                if let Some(reg) = Self::parse_reg(op) {
                    // First operand is usually destination, rest are sources
                    if i == 0 && !opcode_type.is_memory_op() {
                        instr.dst_regs.push(reg);
                    } else {
                        if !instr.src_regs.contains(&reg) {
                            instr.src_regs.push(reg);
                        }
                    }
                }
            }
        }

        // For load/store, try to extract memory address from disassembly
        if opcode_type.is_memory_op() && parts.len() > 2 {
            // Look for [Xn, #offset] pattern
            let rest = parts[1..].join(" ");
            if let Some(mem_str) = Self::extract_memory_operand(&rest) {
                // For simplicity, just mark it as a memory operation
                // The actual address will need to be provided separately
                instr.mem_access = Some(MemAccess {
                    addr: 0, // Will be filled by trace
                    size: 8,
                    is_load: opcode_type == OpcodeType::Load || opcode_type == OpcodeType::LoadPair,
                });
            }
        }

        Ok(instr)
    }

    /// Extract memory operand from disassembly
    fn extract_memory_operand(s: &str) -> Option<String> {
        let start = s.find('[')?;
        let end = s.find(']')?;
        Some(s[start..=end].to_string())
    }

    /// Parse a hexadecimal string
    fn parse_hex(s: &str) -> Result<u64> {
        let s = s.trim();
        let s = s.strip_prefix("0x").unwrap_or(s);
        let s = s.strip_prefix("0X").unwrap_or(s);
        u64::from_str_radix(s, 16)
            .map_err(|_| EmulatorError::TraceParseError(format!("Invalid hex value: {}", s)))
    }

    /// Parse opcode type from string
    fn parse_opcode_type(s: &str) -> Result<OpcodeType> {
        let s = s.to_uppercase();
        Ok(match s.as_str() {
            "ADD" | "ADDS" | "ADC" => OpcodeType::Add,
            "SUB" | "SUBS" | "SBC" => OpcodeType::Sub,
            "MUL" | "SMULL" | "UMULL" => OpcodeType::Mul,
            "DIV" | "SDIV" | "UDIV" => OpcodeType::Div,
            "AND" | "ANDS" => OpcodeType::And,
            "ORR" | "OR" => OpcodeType::Orr,
            "EOR" | "XOR" => OpcodeType::Eor,
            "LSL" => OpcodeType::Lsl,
            "LSR" => OpcodeType::Lsr,
            "ASR" => OpcodeType::Asr,
            "LDR" | "LDUR" | "LDP" | "LDXR" => OpcodeType::Load,
            "STR" | "STUR" | "STP" | "STXR" => OpcodeType::Store,
            "LDPSW" | "LDRSW" => OpcodeType::Load,
            "B" => OpcodeType::Branch,
            "BL" | "BR" | "BLR" | "RET" => OpcodeType::Branch,
            "B.EQ" | "B.NE" | "B.LT" | "B.GT" | "B.LE" | "B.GE" |
            "B.HI" | "B.LS" | "B.CC" | "B.CS" | "B.PL" | "B.MI" => OpcodeType::BranchCond,
            "CBZ" | "CBNZ" | "TBZ" | "TBNZ" => OpcodeType::BranchCond,
            "MSR" => OpcodeType::Msr,
            "MRS" => OpcodeType::Mrs,
            "SYS" | "SYSL" => OpcodeType::Sys,
            "NOP" | "YIELD" | "WFE" | "WFI" | "SEV" => OpcodeType::Nop,
            "FADD" => OpcodeType::Fadd,
            "FSUB" => OpcodeType::Fsub,
            "FMUL" => OpcodeType::Fmul,
            "FDIV" => OpcodeType::Fdiv,
            _ => OpcodeType::Other,
        })
    }

    /// Convert mnemonic to opcode type
    fn mnemonic_to_opcode(mnemonic: &str) -> OpcodeType {
        // Remove conditional suffixes
        let base = mnemonic.trim_end_matches(|c: char| c == '.' || c.is_alphanumeric());

        Self::parse_opcode_type(base).unwrap_or(OpcodeType::Other)
    }

    /// Parse register from string (X0-X30, W0-W30, R0-R30)
    fn parse_reg(s: &str) -> Option<Reg> {
        let s = s.trim().to_uppercase();

        // Handle Xn, Wn, Rn format
        let num_str = if s.starts_with('X') || s.starts_with('W') || s.starts_with('R') {
            &s[1..]
        } else {
            return None;
        };

        let num: u8 = num_str.parse().ok()?;
        if num <= 31 {
            Some(Reg(num))
        } else {
            None
        }
    }
}

impl Iterator for TextTraceParser {
    type Item = Result<Instruction>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            self.line_buffer.clear();
            match self.reader.read_line(&mut self.line_buffer) {
                Ok(0) => return None, // EOF
                Ok(_) => {
                    let line = self.line_buffer.clone();
                    match self.parse_line(&line) {
                        Ok(Some(instr)) => return Some(Ok(instr)),
                        Ok(None) => continue, // Skip empty/comment lines
                        Err(e) => return Some(Err(e)),
                    }
                }
                Err(e) => return Some(Err(EmulatorError::TraceParseError(
                    format!("Error reading file: {}", e)
                ))),
            }
        }
    }
}

impl super::InstructionSource for TextTraceParser {
    fn reset(&mut self) -> Result<()> {
        self.reader.seek(std::io::SeekFrom::Start(0)).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to reset file: {}", e))
        })?;
        self.current_id = 0;
        self.line_buffer.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex() {
        assert_eq!(TextTraceParser::parse_hex("0x1000").unwrap(), 0x1000);
        assert_eq!(TextTraceParser::parse_hex("1000").unwrap(), 0x1000);
        assert_eq!(TextTraceParser::parse_hex("0X1000").unwrap(), 0x1000);
    }

    #[test]
    fn test_parse_reg() {
        assert_eq!(TextTraceParser::parse_reg("X0"), Some(Reg(0)));
        assert_eq!(TextTraceParser::parse_reg("X30"), Some(Reg(30)));
        assert_eq!(TextTraceParser::parse_reg("W15"), Some(Reg(15)));
        assert_eq!(TextTraceParser::parse_reg("X31"), Some(Reg(31)));
        assert_eq!(TextTraceParser::parse_reg("X32"), None);
        assert_eq!(TextTraceParser::parse_reg("invalid"), None);
    }

    #[test]
    fn test_parse_opcode_type() {
        assert_eq!(TextTraceParser::parse_opcode_type("ADD").unwrap(), OpcodeType::Add);
        assert_eq!(TextTraceParser::parse_opcode_type("LDR").unwrap(), OpcodeType::Load);
        assert_eq!(TextTraceParser::parse_opcode_type("B").unwrap(), OpcodeType::Branch);
    }
}
