//! Output Format Module
//!
//! This module provides output format generators for simulation data.
//! The primary output format is Konata-compatible JSON for pipeline visualization.
//!
//! # Features
//!
//! - [`sink::OutputSink`] - Trait for output sinks
//! - [`konata::KonataWriter`] - Konata-compatible JSON format generator
//!
//! # Usage
//!
//! ```rust,ignore
//! use arm_cpu_emulator::output::{KonataWriter, OutputSink};
//! use std::fs::File;
//!
//! let writer = KonataWriter::new();
//! // ... feed events to writer ...
//! writer.write_to_file("output.json")?;
//! ```

pub mod sink;
pub mod konata;

pub use sink::OutputSink;
pub use konata::{KonataWriter, KonataConfig, KonataOp, KonataStage, KonataDependency};
