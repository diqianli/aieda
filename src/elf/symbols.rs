//! Symbol table management for ELF files.

use std::collections::HashMap;

/// Symbol information
#[derive(Debug, Clone)]
pub struct Symbol {
    /// Symbol name
    pub name: String,
    /// Symbol address
    pub address: u64,
    /// Symbol size
    pub size: u64,
    /// Symbol type
    pub sym_type: SymbolType,
    /// Symbol binding
    pub binding: SymbolBinding,
    /// Section index
    pub section_index: u16,
}

/// Symbol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    /// Not specified
    NoType,
    /// Object (data)
    Object,
    /// Function
    Func,
    /// Section
    Section,
    /// File name
    File,
    /// Common
    Common,
    /// TLS
    Tls,
    /// Other
    Other(u8),
}

impl From<u8> for SymbolType {
    fn from(value: u8) -> Self {
        match value & 0xF {
            0 => Self::NoType,
            1 => Self::Object,
            2 => Self::Func,
            3 => Self::Section,
            4 => Self::File,
            5 => Self::Common,
            6 => Self::Tls,
            other => Self::Other(other),
        }
    }
}

/// Symbol binding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolBinding {
    /// Local symbol
    Local,
    /// Global symbol
    Global,
    /// Weak symbol
    Weak,
    /// Other
    Other(u8),
}

impl From<u8> for SymbolBinding {
    fn from(value: u8) -> Self {
        match value >> 4 {
            0 => Self::Local,
            1 => Self::Global,
            2 => Self::Weak,
            other => Self::Other(other),
        }
    }
}

/// Symbol table for address-to-name resolution
#[derive(Debug, Default)]
pub struct SymbolTable {
    /// Symbols by address
    by_address: HashMap<u64, Symbol>,
    /// Function symbols sorted by address
    functions: Vec<(u64, u64, String)>, // (start, end, name)
    /// Demangled names cache
    demangled: HashMap<String, String>,
}

impl SymbolTable {
    /// Create a new empty symbol table
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a symbol to the table
    pub fn add(&mut self, symbol: Symbol) {
        // Add to by_address map
        self.by_address.insert(symbol.address, symbol.clone());

        // Add function to sorted list
        if symbol.sym_type == SymbolType::Func && symbol.size > 0 {
            let end = symbol.address + symbol.size;
            let name = symbol.name.clone();
            self.functions.push((symbol.address, end, name));

            // Keep functions sorted
            self.functions.sort_by_key(|(addr, _, _)| *addr);
        }
    }

    /// Look up symbol by exact address
    pub fn lookup(&self, address: u64) -> Option<&Symbol> {
        self.by_address.get(&address)
    }

    /// Find function containing an address
    pub fn find_function(&self, address: u64) -> Option<&str> {
        // Binary search for function containing address
        let idx = self.functions.partition_point(|(start, _, _)| *start <= address);

        if idx > 0 {
            let (start, end, name) = &self.functions[idx - 1];
            if address >= *start && address < *end {
                return Some(name);
            }
        }

        None
    }

    /// Find function by name
    pub fn find_by_name(&self, name: &str) -> Option<&Symbol> {
        self.by_address.values().find(|s| s.name == name)
    }

    /// Get all functions
    pub fn functions(&self) -> &[(u64, u64, String)] {
        &self.functions
    }

    /// Get all symbols
    pub fn symbols(&self) -> impl Iterator<Item = &Symbol> {
        self.by_address.values()
    }

    /// Get symbol count
    pub fn len(&self) -> usize {
        self.by_address.len()
    }

    /// Check if table is empty
    pub fn is_empty(&self) -> bool {
        self.by_address.is_empty()
    }

    /// Get demangled name for a symbol
    pub fn demangle(&self, name: &str) -> String {
        // Check cache
        if let Some(demangled) = self.demangled.get(name) {
            return demangled.clone();
        }

        // Basic C++ demangling for common patterns
        let demangled = if name.starts_with("_Z") {
            self.demangle_cpp(name)
        } else {
            name.to_string()
        };

        demangled
    }

    fn demangle_cpp(&self, mangled: &str) -> String {
        // Very basic C++ demangling - just handle simple cases
        // For full demangling, use cpp_demangle crate

        if !mangled.starts_with("_Z") {
            return mangled.to_string();
        }

        let rest = &mangled[2..];

        // Handle simple function names like _ZN3foo3barEv
        if rest.starts_with('N') {
            let mut parts = Vec::new();
            let mut pos = 1; // Skip 'N'

            while pos < rest.len() {
                // Read length prefix
                let start = pos;
                while pos < rest.len() && rest[pos..pos + 1].chars().next().unwrap().is_ascii_digit() {
                    pos += 1;
                }

                if pos > start {
                    if let Ok(len) = rest[start..pos].parse::<usize>() {
                        if pos + len <= rest.len() {
                            parts.push(&rest[pos..pos + len]);
                            pos += len;
                            continue;
                        }
                    }
                }
                break;
            }

            if !parts.is_empty() {
                return parts.join("::");
            }
        }

        // Handle simple names like _Z3foov -> foo
        let mut pos = 0;
        let start = pos;
        while pos < rest.len() && rest[pos..pos + 1].chars().next().unwrap().is_ascii_digit() {
            pos += 1;
        }

        if pos > start {
            if let Ok(len) = rest[start..pos].parse::<usize>() {
                if pos + len <= rest.len() {
                    return rest[pos..pos + len].to_string();
                }
            }
        }

        // Give up, return original
        mangled.to_string()
    }

    /// Find symbols near an address (for debugging)
    pub fn find_nearby(&self, address: u64, range: u64) -> Vec<&Symbol> {
        let min_addr = address.saturating_sub(range);
        let max_addr = address.saturating_add(range);

        self.by_address
            .values()
            .filter(|s| s.address >= min_addr && s.address <= max_addr)
            .collect()
    }

    /// Get statistics about the symbol table
    pub fn stats(&self) -> SymbolTableStats {
        let mut func_count = 0;
        let mut obj_count = 0;
        let mut other_count = 0;

        for sym in self.by_address.values() {
            match sym.sym_type {
                SymbolType::Func => func_count += 1,
                SymbolType::Object => obj_count += 1,
                _ => other_count += 1,
            }
        }

        SymbolTableStats {
            total_symbols: self.by_address.len(),
            functions: func_count,
            objects: obj_count,
            other: other_count,
        }
    }
}

/// Statistics about symbol table
#[derive(Debug, Clone)]
pub struct SymbolTableStats {
    /// Total symbol count
    pub total_symbols: usize,
    /// Function count
    pub functions: usize,
    /// Object count
    pub objects: usize,
    /// Other symbols
    pub other: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table_basic() {
        let mut table = SymbolTable::new();

        table.add(Symbol {
            name: "main".to_string(),
            address: 0x1000,
            size: 0x100,
            sym_type: SymbolType::Func,
            binding: SymbolBinding::Global,
            section_index: 1,
        });

        table.add(Symbol {
            name: "helper".to_string(),
            address: 0x1100,
            size: 0x50,
            sym_type: SymbolType::Func,
            binding: SymbolBinding::Local,
            section_index: 1,
        });

        assert_eq!(table.len(), 2);
        assert!(table.lookup(0x1000).is_some());
        assert_eq!(table.find_function(0x1050), Some("main"));
        assert_eq!(table.find_function(0x1150), None);
    }

    #[test]
    fn test_find_function() {
        let mut table = SymbolTable::new();

        table.add(Symbol {
            name: "func1".to_string(),
            address: 0x1000,
            size: 0x100,
            sym_type: SymbolType::Func,
            binding: SymbolBinding::Global,
            section_index: 1,
        });

        table.add(Symbol {
            name: "func2".to_string(),
            address: 0x1100,
            size: 0x200,
            sym_type: SymbolType::Func,
            binding: SymbolBinding::Global,
            section_index: 1,
        });

        // Test boundary cases
        assert_eq!(table.find_function(0x0FFF), None);
        assert_eq!(table.find_function(0x1000), Some("func1"));
        assert_eq!(table.find_function(0x10FF), Some("func1"));
        assert_eq!(table.find_function(0x1100), Some("func2"));
        assert_eq!(table.find_function(0x12FF), Some("func2"));
        assert_eq!(table.find_function(0x1300), None);
    }

    #[test]
    fn test_stats() {
        let mut table = SymbolTable::new();

        table.add(Symbol {
            name: "func1".to_string(),
            address: 0x1000,
            size: 0x100,
            sym_type: SymbolType::Func,
            binding: SymbolBinding::Global,
            section_index: 1,
        });

        table.add(Symbol {
            name: "data1".to_string(),
            address: 0x2000,
            size: 0x10,
            sym_type: SymbolType::Object,
            binding: SymbolBinding::Global,
            section_index: 2,
        });

        let stats = table.stats();
        assert_eq!(stats.functions, 1);
        assert_eq!(stats.objects, 1);
    }
}
