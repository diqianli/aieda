//! Instruction input module for the ARM CPU emulator.
//!
//! This module provides various ways to input instruction traces:
//! - Text format trace files
//! - Binary format trace files
//! - ChampSim trace files (for SPEC CPU 2017 validation)
//! - Direct API calls

mod text_trace;
mod binary_trace;
mod champsim_trace;
mod api;

pub use text_trace::TextTraceParser;
pub use binary_trace::BinaryTraceParser;
pub use champsim_trace::{ChampSimTraceParser, ChampSimXzTraceParser};
pub use api::TraceInput;

use crate::config::TraceFormat;
use crate::types::{EmulatorError, Instruction, Result};

/// Instruction trace source trait
pub trait InstructionSource: Iterator<Item = Result<Instruction>> {
    /// Get the total number of instructions (if known)
    fn total_count(&self) -> Option<usize> {
        None
    }

    /// Reset the source to the beginning
    fn reset(&mut self) -> Result<()>;
}

/// Create an instruction source based on the trace format
pub fn create_source(config: &crate::config::TraceInputConfig) -> Result<Box<dyn InstructionSource>> {
    match config.format {
        TraceFormat::Text => {
            let parser = TextTraceParser::from_file(&config.file_path)?;
            Ok(Box::new(parser))
        }
        TraceFormat::Binary => {
            let parser = BinaryTraceParser::from_file(&config.file_path)?;
            Ok(Box::new(parser))
        }
        TraceFormat::Json => {
            // For now, treat JSON as text
            let parser = TextTraceParser::from_file(&config.file_path)?;
            Ok(Box::new(parser))
        }
        TraceFormat::ChampSim => {
            let parser = ChampSimTraceParser::from_file(&config.file_path)?;
            Ok(Box::new(parser))
        }
        TraceFormat::ChampSimXz => {
            let parser = ChampSimXzTraceParser::from_file(&config.file_path)?;
            Ok(Box::new(parser))
        }
    }
}
