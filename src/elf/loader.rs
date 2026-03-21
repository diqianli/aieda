//! ELF file loader for ARM64 executables.

use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use std::io::Read;

use crate::types::{EmulatorError, Result};

/// ELF header information
#[derive(Debug, Clone)]
pub struct ElfHeader {
    /// Entry point address
    pub entry_point: u64,
    /// Machine type (should be 0xB7 for ARM64)
    pub machine: u16,
    /// Number of program headers
    pub phnum: u16,
    /// Number of section headers
    pub shnum: u16,
    /// Program header offset
    pub phoff: u64,
    /// Section header offset
    pub shoff: u64,
    /// Section header string table index
    pub shstrndx: u16,
}

/// Program header (segment)
#[derive(Debug, Clone)]
pub struct ProgramHeader {
    /// Segment type
    pub p_type: u32,
    /// Segment flags
    pub p_flags: u32,
    /// Segment offset in file
    pub p_offset: u64,
    /// Segment virtual address
    pub p_vaddr: u64,
    /// Segment size in file
    pub p_filesz: u64,
    /// Segment size in memory
    pub p_memsz: u64,
    /// Segment alignment
    pub p_align: u64,
}

/// Section header
#[derive(Debug, Clone)]
pub struct SectionHeader {
    /// Section name offset in shstrtab
    pub sh_name: u32,
    /// Section type
    pub sh_type: u32,
    /// Section flags
    pub sh_flags: u64,
    /// Section virtual address
    pub sh_addr: u64,
    /// Section offset in file
    pub sh_offset: u64,
    /// Section size
    pub sh_size: u64,
    /// Section name
    pub name: Option<String>,
}

/// Memory segment
#[derive(Debug, Clone)]
pub struct MemorySegment {
    /// Virtual address
    pub vaddr: u64,
    /// Size
    pub size: usize,
    /// Data
    pub data: Vec<u8>,
    /// Is executable
    pub executable: bool,
    /// Is writable
    pub writable: bool,
    /// Is readable
    pub readable: bool,
}

/// ELF loader for ARM64 executables
pub struct ElfLoader {
    /// Raw file data
    data: Vec<u8>,
    /// ELF header
    header: ElfHeader,
    /// Program headers
    program_headers: Vec<ProgramHeader>,
    /// Section headers
    section_headers: Vec<SectionHeader>,
    /// Memory segments
    segments: Vec<MemorySegment>,
    /// Symbol table
    symbols: HashMap<u64, String>,
    /// Address to function mapping
    function_ranges: Vec<(u64, u64, String)>,
}

impl ElfLoader {
    /// Load an ELF file from path
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut file = File::open(path).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to open ELF file: {}", e))
        })?;

        let mut data = Vec::new();
        file.read_to_end(&mut data).map_err(|e| {
            EmulatorError::TraceParseError(format!("Failed to read ELF file: {}", e))
        })?;

        Self::parse(&data)
    }

    /// Parse ELF data
    pub fn parse(data: &[u8]) -> Result<Self> {
        // Verify ELF magic
        if data.len() < 16 {
            return Err(EmulatorError::TraceParseError("File too small".to_string()));
        }

        if &data[0..4] != b"\x7fELF" {
            return Err(EmulatorError::TraceParseError("Invalid ELF magic".to_string()));
        }

        // Check class (64-bit)
        let is_64bit = data[4] == 2;
        if !is_64bit {
            return Err(EmulatorError::TraceParseError(
                "Only 64-bit ELF files are supported".to_string(),
            ));
        }

        // Check endianness (little-endian)
        let is_little = data[5] == 1;
        if !is_little {
            return Err(EmulatorError::TraceParseError(
                "Only little-endian ELF files are supported".to_string(),
            ));
        }

        // Parse ELF64 header
        let header = Self::parse_header(data)?;

        // Verify it's ARM64
        if header.machine != 0xB7 {
            return Err(EmulatorError::TraceParseError(format!(
                "Expected ARM64 (0xB7), got machine type 0x{:X}",
                header.machine
            )));
        }

        // Parse program headers
        let program_headers = Self::parse_program_headers(data, &header)?;

        // Parse section headers
        let section_headers = Self::parse_section_headers(data, &header)?;

        // Load segments
        let segments = Self::load_segments(data, &program_headers)?;

        // Parse symbols
        let (symbols, function_ranges) = Self::parse_symbols(data, &section_headers)?;

        Ok(Self {
            data: data.to_vec(),
            header,
            program_headers,
            section_headers,
            segments,
            symbols,
            function_ranges,
        })
    }

    fn parse_header(data: &[u8]) -> Result<ElfHeader> {
        // ELF64 header layout:
        // 0-15: e_ident
        // 16-17: e_type (u16)
        // 18-19: e_machine (u16)
        // 20-23: e_version (u32)
        // 24-31: e_entry (u64)
        // 32-39: e_phoff (u64)
        // 40-47: e_shoff (u64)
        // 48-49: e_flags (u32)
        // 52-53: e_ehsize (u16)
        // 54-55: e_phentsize (u16)
        // 56-57: e_phnum (u16)
        // 58-59: e_shentsize (u16)
        // 60-61: e_shnum (u16)
        // 62-63: e_shstrndx (u16)

        if data.len() < 64 {
            return Err(EmulatorError::TraceParseError("ELF header too small".to_string()));
        }

        Ok(ElfHeader {
            entry_point: u64::from_le_bytes(data[24..32].try_into().unwrap()),
            machine: u16::from_le_bytes(data[18..20].try_into().unwrap()),
            phnum: u16::from_le_bytes(data[56..58].try_into().unwrap()),
            shnum: u16::from_le_bytes(data[60..62].try_into().unwrap()),
            phoff: u64::from_le_bytes(data[32..40].try_into().unwrap()),
            shoff: u64::from_le_bytes(data[40..48].try_into().unwrap()),
            shstrndx: u16::from_le_bytes(data[62..64].try_into().unwrap()),
        })
    }

    fn parse_program_headers(data: &[u8], header: &ElfHeader) -> Result<Vec<ProgramHeader>> {
        let mut headers = Vec::new();

        for i in 0..header.phnum {
            let offset = header.phoff as usize + i as usize * 56; // phentsize for ELF64

            if offset + 56 > data.len() {
                break;
            }

            headers.push(ProgramHeader {
                p_type: u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()),
                p_flags: u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()),
                p_offset: u64::from_le_bytes(data[offset + 8..offset + 16].try_into().unwrap()),
                p_vaddr: u64::from_le_bytes(data[offset + 16..offset + 24].try_into().unwrap()),
                p_filesz: u64::from_le_bytes(data[offset + 32..offset + 40].try_into().unwrap()),
                p_memsz: u64::from_le_bytes(data[offset + 40..offset + 48].try_into().unwrap()),
                p_align: u64::from_le_bytes(data[offset + 48..offset + 56].try_into().unwrap()),
            });
        }

        Ok(headers)
    }

    fn parse_section_headers(data: &[u8], header: &ElfHeader) -> Result<Vec<SectionHeader>> {
        let mut headers = Vec::new();

        for i in 0..header.shnum {
            let offset = header.shoff as usize + i as usize * 64; // shentsize for ELF64

            if offset + 64 > data.len() {
                break;
            }

            headers.push(SectionHeader {
                sh_name: u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap()),
                sh_type: u32::from_le_bytes(data[offset + 4..offset + 8].try_into().unwrap()),
                sh_flags: u64::from_le_bytes(data[offset + 8..offset + 16].try_into().unwrap()),
                sh_addr: u64::from_le_bytes(data[offset + 16..offset + 24].try_into().unwrap()),
                sh_offset: u64::from_le_bytes(data[offset + 24..offset + 32].try_into().unwrap()),
                sh_size: u64::from_le_bytes(data[offset + 32..offset + 40].try_into().unwrap()),
                name: None,
            });
        }

        // Resolve section names
        if header.shstrndx > 0 && (header.shstrndx as usize) < headers.len() {
            let strtab = &headers[header.shstrndx as usize];
            let strtab_offset = strtab.sh_offset as usize;
            let strtab_size = strtab.sh_size as usize;

            for header in &mut headers {
                if header.sh_name > 0 && (strtab_offset + header.sh_name as usize) < data.len() {
                    let name_start = strtab_offset + header.sh_name as usize;
                    let name_end = data[name_start..strtab_offset + strtab_size]
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(strtab_size - header.sh_name as usize);
                    header.name = Some(
                        String::from_utf8_lossy(&data[name_start..name_start + name_end])
                            .into_owned(),
                    );
                }
            }
        }

        Ok(headers)
    }

    fn load_segments(data: &[u8], program_headers: &[ProgramHeader]) -> Result<Vec<MemorySegment>> {
        let mut segments = Vec::new();

        for ph in program_headers {
            // PT_LOAD = 1
            if ph.p_type != 1 {
                continue;
            }

            let start = ph.p_offset as usize;
            let end = (ph.p_offset + ph.p_filesz) as usize;

            if end > data.len() {
                continue;
            }

            let segment_data = if ph.p_filesz > 0 {
                data[start..end].to_vec()
            } else {
                Vec::new()
            };

            // Pad to p_memsz if needed (BSS segment)
            let mut padded_data = segment_data;
            if padded_data.len() < ph.p_memsz as usize {
                padded_data.resize(ph.p_memsz as usize, 0);
            }

            segments.push(MemorySegment {
                vaddr: ph.p_vaddr,
                size: padded_data.len(),
                data: padded_data,
                executable: (ph.p_flags & 0x1) != 0, // PF_X
                writable: (ph.p_flags & 0x2) != 0,   // PF_W
                readable: (ph.p_flags & 0x4) != 0,   // PF_R
            });
        }

        Ok(segments)
    }

    fn parse_symbols(
        data: &[u8],
        section_headers: &[SectionHeader],
    ) -> Result<(HashMap<u64, String>, Vec<(u64, u64, String)>)> {
        let mut symbols = HashMap::new();
        let mut function_ranges = Vec::new();

        // Find symbol table section
        let symtab = section_headers.iter().find(|s| {
            s.name.as_deref() == Some(".symtab") || s.sh_type == 2 // SHT_SYMTAB
        });

        let strtab = section_headers.iter().find(|s| {
            s.name.as_deref() == Some(".strtab")
        });

        if let (Some(symtab), Some(strtab)) = (symtab, strtab) {
            let symtab_offset = symtab.sh_offset as usize;
            let symtab_size = symtab.sh_size as usize;
            let strtab_offset = strtab.sh_offset as usize;
            let strtab_size = strtab.sh_size as usize;

            // Symbol entry size is 24 bytes for ELF64
            let num_symbols = symtab_size / 24;

            for i in 0..num_symbols {
                let offset = symtab_offset + i * 24;

                if offset + 24 > data.len() {
                    break;
                }

                // ELF64 symbol entry:
                // 0-3: st_name (u32)
                // 4: st_info (u8)
                // 5: st_other (u8)
                // 6-7: st_shndx (u16)
                // 8-15: st_value (u64)
                // 16-23: st_size (u64)

                let st_name = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
                let st_info = data[offset + 4];
                let st_value = u64::from_le_bytes(data[offset + 8..offset + 16].try_into().unwrap());
                let st_size = u64::from_le_bytes(data[offset + 16..offset + 24].try_into().unwrap());

                // Get symbol name
                let name_start = strtab_offset + st_name as usize;
                if name_start < data.len() {
                    let name_end = data[name_start..strtab_offset + strtab_size]
                        .iter()
                        .position(|&b| b == 0)
                        .unwrap_or(0);
                    let name = String::from_utf8_lossy(
                        &data[name_start..name_start + name_end],
                    ).into_owned();

                    if !name.is_empty() {
                        // Symbol type: lower 4 bits of st_info
                        let sym_type = st_info & 0xf;
                        // STT_FUNC = 2
                        if sym_type == 2 && st_value != 0 {
                            symbols.insert(st_value, name.clone());
                            if st_size > 0 {
                                function_ranges.push((st_value, st_value + st_size, name));
                            }
                        }
                    }
                }
            }
        }

        // Sort function ranges by address
        function_ranges.sort_by_key(|(start, _, _)| *start);

        Ok((symbols, function_ranges))
    }

    /// Get entry point address
    pub fn entry_point(&self) -> u64 {
        self.header.entry_point
    }

    /// Get symbol name at address
    pub fn get_symbol(&self, addr: u64) -> Option<&str> {
        self.symbols.get(&addr).map(|s| s.as_str())
    }

    /// Get function containing address
    pub fn get_function_at(&self, addr: u64) -> Option<&str> {
        for (start, end, name) in &self.function_ranges {
            if addr >= *start && addr < *end {
                return Some(name);
            }
        }
        None
    }

    /// Read memory at address
    pub fn read_memory(&self, addr: u64, size: usize) -> Option<Vec<u8>> {
        for segment in &self.segments {
            if addr >= segment.vaddr && addr + size as u64 <= segment.vaddr + segment.size as u64 {
                let offset = (addr - segment.vaddr) as usize;
                return Some(segment.data[offset..offset + size].to_vec());
            }
        }
        None
    }

    /// Read instruction at address (4 bytes for ARM64)
    pub fn read_instruction(&self, addr: u64) -> Option<u32> {
        let bytes = self.read_memory(addr, 4)?;
        Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Get all memory segments
    pub fn segments(&self) -> &[MemorySegment] {
        &self.segments
    }

    /// Get all function symbols
    pub fn functions(&self) -> &[(u64, u64, String)] {
        &self.function_ranges
    }

    /// Get executable segments
    pub fn executable_segments(&self) -> Vec<&MemorySegment> {
        self.segments.iter().filter(|s| s.executable).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_elf_magic_validation() {
        let invalid_data = vec![0x00, 0x01, 0x02, 0x03];
        let result = ElfLoader::parse(&invalid_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_32bit_rejection() {
        // Minimal 32-bit ELF header
        let mut data = vec![0u8; 64];
        data[0..4].copy_from_slice(b"\x7fELF");
        data[4] = 1; // 32-bit
        let result = ElfLoader::parse(&data);
        assert!(result.is_err());
    }
}
