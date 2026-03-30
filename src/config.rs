//! Configuration management for the ARM CPU emulator.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::time::Duration;

/// Main CPU configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CPUConfig {
    // Out-of-Order Engine
    /// Instruction window size (128-256)
    pub window_size: usize,
    /// Fetch width (instructions fetched per cycle)
    pub fetch_width: usize,
    /// Issue width (instructions per cycle)
    pub issue_width: usize,
    /// Commit width (instructions per cycle)
    pub commit_width: usize,

    // Memory Subsystem
    /// Load/Store Queue size
    pub lsq_size: usize,
    /// Number of load pipelines
    pub load_pipeline_count: usize,
    /// Number of store pipelines
    pub store_pipeline_count: usize,
    /// Maximum outstanding memory requests
    pub outstanding_requests: usize,

    // L1 Data Cache
    /// L1 cache size in bytes
    pub l1_size: usize,
    /// L1 cache associativity
    pub l1_associativity: usize,
    /// L1 cache line size in bytes
    pub l1_line_size: usize,
    /// L1 cache hit latency in cycles
    pub l1_hit_latency: u64,

    // L2 Cache
    /// L2 cache size in bytes
    pub l2_size: usize,
    /// L2 cache associativity
    pub l2_associativity: usize,
    /// L2 cache line size in bytes
    pub l2_line_size: usize,
    /// L2 cache hit latency in cycles
    pub l2_hit_latency: u64,

    // L3 Cache
    /// L3 cache size in bytes (8-32 MB typical)
    pub l3_size: usize,
    /// L3 cache associativity (16-32 way typical)
    pub l3_associativity: usize,
    /// L3 cache line size in bytes
    pub l3_line_size: usize,
    /// L3 cache hit latency in cycles (30-50 typical)
    pub l3_hit_latency: u64,

    // DDR Memory Controller
    /// DDR base latency in cycles (CAS + RAS, ~100-200)
    pub ddr_base_latency: u64,
    /// DDR row buffer hit bonus (cycles saved if row open)
    pub ddr_row_buffer_hit_bonus: u64,
    /// DDR bank conflict penalty (cycles added)
    pub ddr_bank_conflict_penalty: u64,
    /// Number of DDR banks
    pub ddr_num_banks: usize,

    // External Memory (deprecated - use DDR config)
    /// L2 miss latency (memory access) in cycles
    pub l2_miss_latency: u64,

    // CPU
    /// CPU frequency in MHz
    pub frequency_mhz: u64,

    // CHI Interface
    /// Enable CHI protocol modeling
    pub enable_chi: bool,
    /// CHI request channel latency in cycles
    pub chi_request_latency: u64,
    /// CHI response channel latency in cycles
    pub chi_response_latency: u64,
    /// CHI data channel latency in cycles
    pub chi_data_latency: u64,
    /// CHI snoop channel latency in cycles
    pub chi_snoop_latency: u64,

    // CHI Node Configuration
    /// RN-F node ID
    pub chi_rnf_node_id: u8,
    /// HN-F node ID
    pub chi_hnf_node_id: u8,
    /// SN-F node ID
    pub chi_snf_node_id: u8,

    // CHI QoS Configuration
    /// Maximum PCrd credits
    pub chi_max_pcrd_credits: u16,
    /// Maximum outstanding DBIDs
    pub chi_max_outstanding_dbid: u16,
    /// Maximum retry queue size
    pub chi_max_retry_queue_size: usize,

    // CHI Directory Configuration
    /// Directory size (number of entries)
    pub chi_directory_size: usize,

    // Statistics
    /// Enable detailed instruction trace output
    pub enable_trace_output: bool,
    /// Maximum trace output instructions (0 = unlimited)
    pub max_trace_output: usize,
}

impl Default for CPUConfig {
    fn default() -> Self {
        Self {
            // Out-of-Order Engine
            window_size: 128,
            fetch_width: 8,
            issue_width: 4,
            commit_width: 4,

            // Memory Subsystem
            lsq_size: 64,
            load_pipeline_count: 2,
            store_pipeline_count: 1,
            outstanding_requests: 16,

            // L1 Data Cache (64KB, 4-way, 64B line)
            l1_size: 64 * 1024,
            l1_associativity: 4,
            l1_line_size: 64,
            l1_hit_latency: 4,

            // L2 Cache (512KB, 8-way, 64B line)
            l2_size: 512 * 1024,
            l2_associativity: 8,
            l2_line_size: 64,
            l2_hit_latency: 12,

            // L3 Cache (8MB, 16-way, 64B line, 40 cycle hit)
            l3_size: 8 * 1024 * 1024,
            l3_associativity: 16,
            l3_line_size: 64,
            l3_hit_latency: 40,

            // DDR Memory Controller (DDR4-3200 typical)
            ddr_base_latency: 150,
            ddr_row_buffer_hit_bonus: 30,
            ddr_bank_conflict_penalty: 20,
            ddr_num_banks: 8,

            // External Memory
            l2_miss_latency: 100,

            // CPU
            frequency_mhz: 2000,

            // CHI Interface
            enable_chi: false,
            chi_request_latency: 2,
            chi_response_latency: 2,
            chi_data_latency: 4,
            chi_snoop_latency: 2,

            // CHI Node Configuration
            chi_rnf_node_id: 0,
            chi_hnf_node_id: 1,
            chi_snf_node_id: 2,

            // CHI QoS Configuration
            chi_max_pcrd_credits: 16,
            chi_max_outstanding_dbid: 32,
            chi_max_retry_queue_size: 64,

            // CHI Directory Configuration
            chi_directory_size: 4096,

            // Statistics
            enable_trace_output: false,
            max_trace_output: 0,
        }
    }
}

impl CPUConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a high-performance configuration (larger buffers, more parallelism)
    pub fn high_performance() -> Self {
        Self {
            window_size: 256,
            issue_width: 6,
            commit_width: 6,
            lsq_size: 128,
            load_pipeline_count: 4,
            store_pipeline_count: 2,
            outstanding_requests: 32,
            ..Self::default()
        }
    }

    /// Create a minimal configuration (for testing)
    pub fn minimal() -> Self {
        Self {
            window_size: 16,
            issue_width: 2,
            commit_width: 2,
            lsq_size: 8,
            load_pipeline_count: 1,
            store_pipeline_count: 1,
            outstanding_requests: 4,
            l1_size: 4 * 1024,
            l1_associativity: 2,
            l2_size: 16 * 1024,
            l2_associativity: 2,
            ..Self::default()
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> crate::types::Result<()> {
        if self.window_size < 4 || self.window_size > 512 {
            return Err(crate::types::EmulatorError::ConfigError(
                "window_size must be between 4 and 512".to_string(),
            ));
        }

        if self.fetch_width < 1 || self.fetch_width > 16 {
            return Err(crate::types::EmulatorError::ConfigError(
                "fetch_width must be between 1 and 16".to_string(),
            ));
        }

        if self.issue_width < 1 || self.issue_width > 16 {
            return Err(crate::types::EmulatorError::ConfigError(
                "issue_width must be between 1 and 16".to_string(),
            ));
        }

        if self.commit_width < 1 || self.commit_width > 16 {
            return Err(crate::types::EmulatorError::ConfigError(
                "commit_width must be between 1 and 16".to_string(),
            ));
        }

        if self.l1_line_size != 32 && self.l1_line_size != 64 && self.l1_line_size != 128 {
            return Err(crate::types::EmulatorError::ConfigError(
                "l1_line_size must be 32, 64, or 128 bytes".to_string(),
            ));
        }

        if self.l2_line_size != 32 && self.l2_line_size != 64 && self.l2_line_size != 128 {
            return Err(crate::types::EmulatorError::ConfigError(
                "l2_line_size must be 32, 64, or 128 bytes".to_string(),
            ));
        }

        if self.l3_line_size != 64 && self.l3_line_size != 128 {
            return Err(crate::types::EmulatorError::ConfigError(
                "l3_line_size must be 64 or 128 bytes".to_string(),
            ));
        }

        // Check that cache sizes are powers of 2 and properly divisible
        if !is_power_of_two(self.l1_size) {
            return Err(crate::types::EmulatorError::ConfigError(
                "l1_size must be a power of 2".to_string(),
            ));
        }

        if !is_power_of_two(self.l2_size) {
            return Err(crate::types::EmulatorError::ConfigError(
                "l2_size must be a power of 2".to_string(),
            ));
        }

        if !is_power_of_two(self.l3_size) {
            return Err(crate::types::EmulatorError::ConfigError(
                "l3_size must be a power of 2".to_string(),
            ));
        }

        let l1_sets = self.l1_size / (self.l1_associativity * self.l1_line_size);
        let l2_sets = self.l2_size / (self.l2_associativity * self.l2_line_size);
        let l3_sets = self.l3_size / (self.l3_associativity * self.l3_line_size);

        if !is_power_of_two(l1_sets) {
            return Err(crate::types::EmulatorError::ConfigError(
                "L1 cache configuration results in non-power-of-2 sets".to_string(),
            ));
        }

        if !is_power_of_two(l2_sets) {
            return Err(crate::types::EmulatorError::ConfigError(
                "L2 cache configuration results in non-power-of-2 sets".to_string(),
            ));
        }

        if !is_power_of_two(l3_sets) {
            return Err(crate::types::EmulatorError::ConfigError(
                "L3 cache configuration results in non-power-of-2 sets".to_string(),
            ));
        }

        Ok(())
    }

    /// Get L1 cache number of sets
    pub fn l1_sets(&self) -> usize {
        self.l1_size / (self.l1_associativity * self.l1_line_size)
    }

    /// Get L2 cache number of sets
    pub fn l2_sets(&self) -> usize {
        self.l2_size / (self.l2_associativity * self.l2_line_size)
    }

    /// Get L3 cache number of sets
    pub fn l3_sets(&self) -> usize {
        self.l3_size / (self.l3_associativity * self.l3_line_size)
    }

    /// Get the period of one cycle in nanoseconds
    pub fn cycle_period_ns(&self) -> f64 {
        1000.0 / self.frequency_mhz as f64
    }

    /// Convert cycles to Duration
    pub fn cycles_to_duration(&self, cycles: u64) -> Duration {
        let ns = (cycles as f64 * self.cycle_period_ns()) as u64;
        Duration::from_nanos(ns)
    }
}

/// Check if a number is a power of 2
fn is_power_of_two(n: usize) -> bool {
    n > 0 && (n & (n - 1)) == 0
}

/// Configuration for trace input
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TraceInputConfig {
    /// Path to the trace file
    pub file_path: String,
    /// Trace format
    pub format: TraceFormat,
    /// Maximum number of instructions to read (0 = unlimited)
    pub max_instructions: usize,
    /// Skip first N instructions
    pub skip_instructions: usize,
}

impl Default for TraceInputConfig {
    fn default() -> Self {
        Self {
            file_path: String::new(),
            format: TraceFormat::Text,
            max_instructions: 0,
            skip_instructions: 0,
        }
    }
}

/// Supported trace file formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TraceFormat {
    /// Text format trace
    Text,
    /// Binary format trace
    Binary,
    /// JSON format trace
    Json,
    /// ChampSim trace format (uncompressed)
    ChampSim,
    /// ChampSim trace format (XZ compressed, .champsimtrace.xz)
    ChampSimXz,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CPUConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.window_size, 128);
        assert_eq!(config.l1_size, 64 * 1024);
    }

    #[test]
    fn test_config_validation() {
        let mut config = CPUConfig::default();
        config.window_size = 1024; // Too large
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_l1_sets() {
        let config = CPUConfig::default();
        // 64KB / (4 * 64) = 256 sets
        assert_eq!(config.l1_sets(), 256);
    }

    #[test]
    fn test_cycle_period() {
        let config = CPUConfig {
            frequency_mhz: 2000,
            ..Default::default()
        };
        assert!((config.cycle_period_ns() - 0.5).abs() < 0.001);
    }
}
