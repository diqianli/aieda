//! Core Simulation Engine Module
//!
//! This module provides the core simulation engine with decoupled output
//! support. The simulation engine emits events that can be consumed by
//! various output sinks (Konata, custom formats, etc.)
//!
//! # Architecture
//!
//! - [`engine::SimulationEngine`] - Core simulation engine
//! - [`event::SimulationEvent`] - Events emitted during simulation
//! - [`tracker::PipelineTracker`] - Pipeline stage tracking for visualization

pub mod event;
pub mod engine;
pub mod tracker;

pub use event::{SimulationEvent, SimulationEventSink};
pub use engine::SimulationEngine;
pub use tracker::PipelineTracker as SimulationPipelineTracker;
