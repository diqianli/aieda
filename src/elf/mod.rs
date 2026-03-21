//! ELF executable file support.
//!
//! This module provides:
//! - ELF file loading and parsing
//! - ARM64 instruction decoding
//! - Symbol table resolution
//! - Memory mapping

pub mod loader;
pub mod decoder;
pub mod symbols;

pub use loader::ElfLoader;
pub use decoder::{Arm64Decoder, DecodedInstruction};
pub use symbols::SymbolTable;
