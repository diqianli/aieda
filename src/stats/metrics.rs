//! Performance metrics definitions.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// Overall performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PerformanceMetrics {
    /// Total instructions executed
    pub total_instructions: u64,
    /// Total cycles
    pub total_cycles: u64,
    /// Instructions Per Cycle
    pub ipc: f64,
    /// Cycles Per Instruction
    pub cpi: f64,
    /// L1 cache hit rate
    pub l1_hit_rate: f64,
    /// L2 cache hit rate
    pub l2_hit_rate: f64,
    /// L1 MPKI (Misses Per Kilo Instructions)
    pub l1_mpki: f64,
    /// L2 MPKI (Misses Per Kilo Instructions)
    pub l2_mpki: f64,
    /// Memory instruction percentage
    pub memory_instr_pct: f64,
    /// Branch instruction percentage
    pub branch_instr_pct: f64,
    /// Average load latency
    pub avg_load_latency: f64,
    /// Average store latency
    pub avg_store_latency: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            total_instructions: 0,
            total_cycles: 0,
            ipc: 0.0,
            cpi: 0.0,
            l1_hit_rate: 0.0,
            l2_hit_rate: 0.0,
            l1_mpki: 0.0,
            l2_mpki: 0.0,
            memory_instr_pct: 0.0,
            branch_instr_pct: 0.0,
            avg_load_latency: 0.0,
            avg_store_latency: 0.0,
        }
    }
}

impl PerformanceMetrics {
    /// Calculate execution time in nanoseconds (given frequency in MHz)
    pub fn execution_time_ns(&self, frequency_mhz: u64) -> u64 {
        if frequency_mhz == 0 {
            return 0;
        }
        let cycle_ns = 1000.0 / frequency_mhz as f64;
        (self.total_cycles as f64 * cycle_ns) as u64
    }

    /// Calculate throughput in MIPS (Million Instructions Per Second)
    pub fn throughput_mips(&self, frequency_mhz: u64) -> f64 {
        if frequency_mhz == 0 || self.total_cycles == 0 {
            return 0.0;
        }
        let seconds = self.execution_time_ns(frequency_mhz) as f64 / 1e9;
        if seconds == 0.0 {
            return 0.0;
        }
        self.total_instructions as f64 / seconds / 1e6
    }

    /// Format as a summary string
    pub fn summary(&self) -> String {
        format!(
            "Performance Metrics:\n\
             ====================\n\
             Instructions: {}\n\
             Cycles: {}\n\
             IPC: {:.3}\n\
             CPI: {:.3}\n\
             \n\
             L1 Cache:\n\
               Hit Rate: {:.2}%\n\
               MPKI: {:.2}\n\
             \n\
             L2 Cache:\n\
               Hit Rate: {:.2}%\n\
               MPKI: {:.2}\n\
             \n\
             Memory: {:.2}%\n\
             Branch: {:.2}%\n\
             \n\
             Avg Load Latency: {:.2}\n\
             Avg Store Latency: {:.2}",
            self.total_instructions,
            self.total_cycles,
            self.ipc,
            self.cpi,
            self.l1_hit_rate * 100.0,
            self.l1_mpki,
            self.l2_hit_rate * 100.0,
            self.l2_mpki,
            self.memory_instr_pct,
            self.branch_instr_pct,
            self.avg_load_latency,
            self.avg_store_latency
        )
    }
}

/// Cache performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CacheMetrics {
    /// Total accesses
    pub accesses: u64,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Hit rate
    pub hit_rate: f64,
    /// Miss rate
    pub miss_rate: f64,
    /// MPKI
    pub mpki: f64,
}

impl Default for CacheMetrics {
    fn default() -> Self {
        Self {
            accesses: 0,
            hits: 0,
            misses: 0,
            hit_rate: 0.0,
            miss_rate: 0.0,
            mpki: 0.0,
        }
    }
}

/// Execution metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct ExecutionMetrics {
    /// Instructions dispatched
    pub dispatched: u64,
    /// Instructions issued
    pub issued: u64,
    /// Instructions completed
    pub completed: u64,
    /// Instructions committed
    pub committed: u64,
    /// Average dispatch-to-issue latency
    pub avg_dispatch_issue_latency: f64,
    /// Average issue-to-complete latency
    pub avg_issue_complete_latency: f64,
    /// Average complete-to-commit latency
    pub avg_complete_commit_latency: f64,
    /// Window occupancy average
    pub avg_window_occupancy: f64,
    /// Peak window occupancy
    pub peak_window_occupancy: usize,
}

/// Memory performance metrics
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MemoryMetrics {
    /// Total loads
    pub loads: u64,
    /// Total stores
    pub stores: u64,
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Average load latency
    pub avg_load_latency: f64,
    /// Average store latency
    pub avg_store_latency: f64,
    /// Memory bandwidth (bytes per cycle)
    pub bandwidth: f64,
}

impl Default for MemoryMetrics {
    fn default() -> Self {
        Self {
            loads: 0,
            stores: 0,
            bytes_read: 0,
            bytes_written: 0,
            avg_load_latency: 0.0,
            avg_store_latency: 0.0,
            bandwidth: 0.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics {
            total_instructions: 1_000_000,
            total_cycles: 500_000,
            ipc: 2.0,
            cpi: 0.5,
            ..Default::default()
        };

        assert_eq!(metrics.execution_time_ns(2000), 250_000);
        assert!((metrics.throughput_mips(2000) - 4000.0).abs() < 1.0);
    }

    #[test]
    fn test_metrics_summary() {
        let metrics = PerformanceMetrics {
            total_instructions: 1000,
            total_cycles: 500,
            ipc: 2.0,
            cpi: 0.5,
            l1_hit_rate: 0.95,
            l2_hit_rate: 0.80,
            ..Default::default()
        };

        let summary = metrics.summary();
        assert!(summary.contains("IPC: 2.000"));
        assert!(summary.contains("95.00%"));
    }
}
