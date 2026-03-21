//! Simulation Instance
//!
//! This module provides an isolated simulation instance that can run
//! independently from other instances.

use std::sync::atomic::{AtomicU64, Ordering};

use crate::config::CPUConfig;
use crate::cpu::CPUEmulator;
use crate::stats::PerformanceMetrics;
use crate::types::Result;

/// Unique identifier for a simulation instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct InstanceId(pub u64);

impl Default for InstanceId {
    fn default() -> Self {
        Self(0)
    }
}

impl std::fmt::Display for InstanceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "instance-{}", self.0)
    }
}

/// State of a simulation instance
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceState {
    /// Instance is idle and ready to run
    Idle,
    /// Instance is currently running
    Running,
    /// Instance is paused
    Paused,
    /// Instance has completed execution
    Completed,
    /// Instance encountered an error
    Error,
}

/// Statistics for a simulation instance
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InstanceStats {
    /// Total cycles executed
    pub cycles: u64,
    /// Total instructions retired
    pub instructions_retired: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Branch mispredictions
    pub branch_mispredictions: u64,
}

/// Instance-specific metrics with execution time
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InstanceMetrics {
    /// Base performance metrics from the emulator
    #[serde(flatten)]
    pub perf: PerformanceMetrics,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl InstanceMetrics {
    /// Calculate cache hit rate
    pub fn cache_hit_rate(&self) -> f64 {
        // Use L1 hit rate from base metrics
        self.perf.l1_hit_rate
    }
}

/// Result of a completed simulation instance
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InstanceResult {
    /// Instance ID
    pub instance_id: InstanceId,
    /// Performance metrics
    pub metrics: InstanceMetrics,
    /// Final statistics
    pub stats: InstanceStats,
    /// Optional trace file path (if output was saved)
    pub trace_path: Option<String>,
    /// Error message if instance failed
    pub error: Option<String>,
}

/// A single simulation instance
pub struct SimulationInstance {
    /// Unique instance identifier
    pub id: InstanceId,
    /// CPU configuration
    config: CPUConfig,
    /// CPU emulator
    emulator: CPUEmulator,
    /// Current state
    state: InstanceState,
    /// Instance statistics
    stats: InstanceStats,
    /// Instruction counter for generating unique IDs
    instruction_counter: AtomicU64,
}

impl SimulationInstance {
    /// Create a new simulation instance
    pub fn new(id: InstanceId, config: CPUConfig) -> Self {
        let emulator = CPUEmulator::new(config.clone()).expect("Failed to create CPU emulator");

        Self {
            id,
            config,
            emulator,
            state: InstanceState::Idle,
            stats: InstanceStats::default(),
            instruction_counter: AtomicU64::new(0),
        }
    }

    /// Get the current state
    pub fn state(&self) -> InstanceState {
        self.state
    }

    /// Get the current statistics
    pub fn stats(&self) -> &InstanceStats {
        &self.stats
    }

    /// Get mutable statistics (for updating)
    pub fn stats_mut(&mut self) -> &mut InstanceStats {
        &mut self.stats
    }

    /// Get the next instruction ID
    pub fn next_instruction_id(&self) -> u64 {
        self.instruction_counter.fetch_add(1, Ordering::SeqCst)
    }

    /// Run the simulation for a specified number of cycles
    pub fn run_cycles(&mut self, max_cycles: u64) -> Result<InstanceResult> {
        self.state = InstanceState::Running;

        let start_time = std::time::Instant::now();

        // Run for the specified number of cycles
        self.emulator.run_cycles(max_cycles);

        // Get metrics from emulator
        let perf = self.emulator.get_metrics();
        self.stats.cycles = self.emulator.current_cycle();
        self.stats.instructions_retired = self.emulator.committed_count();

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        let result = InstanceResult {
            instance_id: self.id,
            metrics: InstanceMetrics {
                perf,
                execution_time_ms,
            },
            stats: self.stats.clone(),
            trace_path: None,
            error: None,
        };

        self.state = InstanceState::Completed;
        Ok(result)
    }

    /// Reset the instance to initial state
    pub fn reset(&mut self) {
        self.state = InstanceState::Idle;
        self.stats = InstanceStats::default();
        self.emulator.reset();
        self.instruction_counter.store(0, Ordering::SeqCst);
    }
}

impl Clone for SimulationInstance {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            config: self.config.clone(),
            emulator: CPUEmulator::new(self.config.clone()).expect("Failed to clone emulator"),
            state: self.state,
            stats: self.stats.clone(),
            instruction_counter: AtomicU64::new(self.instruction_counter.load(Ordering::SeqCst)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_creation() {
        let config = CPUConfig::default();
        let instance = SimulationInstance::new(InstanceId(1), config);
        assert_eq!(instance.state(), InstanceState::Idle);
    }

    #[test]
    fn test_instance_reset() {
        let config = CPUConfig::default();
        let mut instance = SimulationInstance::new(InstanceId(1), config);
        instance.stats.instructions_retired = 100;
        instance.reset();
        assert_eq!(instance.stats().instructions_retired, 0);
    }

    #[test]
    fn test_instance_id_display() {
        let id = InstanceId(42);
        assert_eq!(format!("{}", id), "instance-42");
    }
}
