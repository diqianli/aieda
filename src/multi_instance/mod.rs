//! Multi-Instance Parallelism
//!
//! This module provides support for running multiple independent simulation
//! instances in parallel, useful for:
//! - Parameter sweeps
//! - Benchmarking multiple programs
//! - Statistical analysis of simulation behavior
//!
//! # Example
//!
//! ```rust,ignore
//! use arm_cpu_emulator::multi_instance::{InstanceManager, MultiRunConfig};
//! use arm_cpu_emulator::config::CPUConfig;
//!
//! let config = CPUConfig::default();
//! let run_config = MultiRunConfig {
//!     max_cycles: 1_000_000,
//!     parallel: true,
//!     ..Default::default()
//! };
//!
//! let manager = InstanceManager::with_run_config(config, run_config);
//!
//! // Create instances for each program
//! for _ in 0..10 {
//!     manager.create_instance();
//! }
//!
//! // Run all instances in parallel
//! let results = manager.run_all_parallel()?;
//!
//! println!("Average IPC: {:.3}", results.avg_ipc);
//! ```

pub mod instance;
pub mod manager;

pub use instance::{
    InstanceId, InstanceMetrics, InstanceResult, InstanceState, InstanceStats,
    SimulationInstance,
};
pub use manager::{generate_instance_id, AggregatedResults, InstanceManager, MultiRunConfig};

// Re-export PerformanceMetrics from stats for convenience
pub use crate::stats::PerformanceMetrics;
