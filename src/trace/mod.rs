//! Binary trace format for high-performance storage and streaming.
//!
//! This module provides a compact binary format for CPU simulation traces,
//! designed to reduce storage by 10-100x compared to JSON while enabling
//! efficient streaming for large-scale simulations (1M-10M instructions).

pub mod binary_format;
pub mod binary_writer;
pub mod binary_reader;

pub use binary_format::*;
pub use binary_writer::BinaryTraceWriter;
pub use binary_reader::BinaryTraceReader;
