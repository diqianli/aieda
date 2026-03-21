//! AArch64 Instruction Decoding Submodules
//!
//! This module provides specialized decoders for different instruction categories.

pub mod arithmetic;
pub mod logical;
pub mod load_store;
pub mod branch;
pub mod simd_neon;
pub mod fp;
pub mod crypto;
pub mod system;
pub mod encoding;

// Re-export key types
pub use super::{DecodeError, DecodeResult, DecodedInstruction};
