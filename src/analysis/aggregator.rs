//! Statistics aggregator for large-scale simulation data.
//!
//! Provides binned timeline statistics for visualization and analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A point on the timeline
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePoint {
    /// Cycle number
    pub cycle: u64,
    /// Instruction number
    pub instr: u64,
    /// Value at this point
    pub value: f64,
}

impl TimelinePoint {
    pub fn new(cycle: u64, instr: u64, value: f64) -> Self {
        Self { cycle, instr, value }
    }
}

/// Statistics bin for aggregated data
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StatsBin {
    /// Starting instruction ID
    pub start_instr: u64,
    /// Ending instruction ID
    pub end_instr: u64,
    /// Starting cycle
    pub start_cycle: u64,
    /// Ending cycle
    pub end_cycle: u64,
    /// Instructions in this bin
    pub instr_count: u64,
    /// Cycles in this bin
    pub cycle_count: u64,
    /// IPC for this bin
    pub ipc: f64,
    /// Memory operations count
    pub mem_ops: u64,
    /// Branch operations count
    pub branch_ops: u64,
    /// L1 cache misses
    pub l1_misses: u64,
    /// L2 cache misses
    pub l2_misses: u64,
    /// Pipeline bubbles (no instructions issued)
    pub bubbles: u64,
    /// Average instruction latency
    pub avg_latency: f64,
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStatistics {
    /// L1 data cache hits
    pub l1_d_hits: u64,
    /// L1 data cache misses
    pub l1_d_misses: u64,
    /// L1 instruction cache hits
    pub l1_i_hits: u64,
    /// L1 instruction cache misses
    pub l1_i_misses: u64,
    /// L2 cache hits
    pub l2_hits: u64,
    /// L2 cache misses
    pub l2_misses: u64,
    /// L1 miss rate
    pub l1_miss_rate: f64,
    /// L2 miss rate
    pub l2_miss_rate: f64,
}

/// Pipeline utilization statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineUtilization {
    /// Average instruction window utilization
    pub window_utilization: f64,
    /// Peak window utilization
    pub peak_window_utilization: f64,
    /// Average LSQ utilization
    pub lsq_utilization: f64,
    /// Peak LSQ utilization
    pub peak_lsq_utilization: f64,
    /// Issue width utilization
    pub issue_utilization: f64,
    /// Commit width utilization
    pub commit_utilization: f64,
}

/// Aggregated statistics for large-scale visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedStatistics {
    /// Total instructions executed
    pub total_instructions: u64,
    /// Total cycles
    pub total_cycles: u64,
    /// Overall IPC
    pub ipc: f64,
    /// Bin size (instructions per bin)
    pub bin_size: u64,
    /// Number of bins
    pub bin_count: u64,

    // Timeline data (one point per bin)
    /// IPC timeline
    pub ipc_timeline: Vec<TimelinePoint>,
    /// Throughput timeline (instructions per cycle window)
    pub throughput_timeline: Vec<TimelinePoint>,
    /// L1 miss rate timeline
    pub l1_miss_timeline: Vec<TimelinePoint>,
    /// L2 miss rate timeline
    pub l2_miss_timeline: Vec<TimelinePoint>,

    // Detailed bins
    /// Statistics bins
    pub bins: Vec<StatsBin>,

    // Function-level statistics
    /// Function statistics
    pub function_stats: Vec<super::function_profiler::FunctionStats>,

    // Cache statistics
    /// Cache statistics
    pub cache_stats: CacheStatistics,

    // Pipeline statistics
    /// Pipeline utilization
    pub pipeline_utilization: PipelineUtilization,

    // Anomalies detected
    /// Detected anomalies
    pub anomalies: Vec<super::anomaly_detector::Anomaly>,
}

impl Default for AggregatedStatistics {
    fn default() -> Self {
        Self {
            total_instructions: 0,
            total_cycles: 0,
            ipc: 0.0,
            bin_size: 10000,
            bin_count: 0,
            ipc_timeline: Vec::new(),
            throughput_timeline: Vec::new(),
            l1_miss_timeline: Vec::new(),
            l2_miss_timeline: Vec::new(),
            bins: Vec::new(),
            function_stats: Vec::new(),
            cache_stats: CacheStatistics::default(),
            pipeline_utilization: PipelineUtilization::default(),
            anomalies: Vec::new(),
        }
    }
}

/// Statistics aggregator for binning and aggregating large-scale data
pub struct StatsAggregator {
    /// Bin size (instructions per bin)
    bin_size: u64,
    /// Current bin being built
    current_bin: StatsBin,
    /// Completed bins
    bins: Vec<StatsBin>,
    /// Function statistics
    function_stats: HashMap<u64, super::function_profiler::FunctionStats>,
    /// Function map (PC -> function ID)
    function_map: HashMap<u64, u64>,
    /// Total statistics
    total_instructions: u64,
    total_cycles: u64,
    total_l1_misses: u64,
    total_l2_misses: u64,
    total_mem_ops: u64,
    total_branch_ops: u64,
    total_latency: f64,
    latency_count: u64,
}

impl StatsAggregator {
    /// Create a new aggregator with the given bin size
    pub fn new(bin_size: u64) -> Self {
        Self {
            bin_size,
            current_bin: StatsBin::default(),
            bins: Vec::new(),
            function_stats: HashMap::new(),
            function_map: HashMap::new(),
            total_instructions: 0,
            total_cycles: 0,
            total_l1_misses: 0,
            total_l2_misses: 0,
            total_mem_ops: 0,
            total_branch_ops: 0,
            total_latency: 0.0,
            latency_count: 0,
        }
    }

    /// Record an instruction completion
    pub fn record_instruction(
        &mut self,
        instr_id: u64,
        start_cycle: u64,
        end_cycle: u64,
        is_memory: bool,
        is_branch: bool,
        l1_miss: bool,
        l2_miss: bool,
    ) {
        // Initialize bin if needed
        if self.current_bin.instr_count == 0 {
            self.current_bin.start_instr = instr_id;
            self.current_bin.start_cycle = start_cycle;
        }

        // Update current bin
        self.current_bin.end_instr = instr_id;
        self.current_bin.end_cycle = end_cycle;
        self.current_bin.instr_count += 1;
        self.current_bin.cycle_count = end_cycle.saturating_sub(self.current_bin.start_cycle) + 1;

        if is_memory {
            self.current_bin.mem_ops += 1;
            self.total_mem_ops += 1;
        }
        if is_branch {
            self.current_bin.branch_ops += 1;
            self.total_branch_ops += 1;
        }
        if l1_miss {
            self.current_bin.l1_misses += 1;
            self.total_l1_misses += 1;
        }
        if l2_miss {
            self.current_bin.l2_misses += 1;
            self.total_l2_misses += 1;
        }

        let latency = end_cycle.saturating_sub(start_cycle) + 1;
        self.total_latency += latency as f64;
        self.latency_count += 1;
        self.current_bin.avg_latency = self.total_latency / self.latency_count as f64;

        self.total_instructions += 1;
        self.total_cycles = self.total_cycles.max(end_cycle + 1);

        // Check if bin is complete
        if self.current_bin.instr_count >= self.bin_size {
            self.finalize_current_bin();
        }
    }

    /// Record a pipeline bubble (no instructions issued)
    pub fn record_bubble(&mut self, cycle: u64) {
        self.current_bin.bubbles += 1;
        self.total_cycles = self.total_cycles.max(cycle + 1);
    }

    /// Register a function
    pub fn register_function(&mut self, start_pc: u64, end_pc: u64, name: &str) {
        let func_id = start_pc; // Use start PC as function ID
        for pc in start_pc..=end_pc {
            self.function_map.insert(pc, func_id);
        }
        self.function_stats.insert(
            func_id,
            super::function_profiler::FunctionStats {
                name: name.to_string(),
                start_pc,
                end_pc,
                instruction_count: 0,
                cycle_count: 0,
                ipc: 0.0,
                cache_miss_rate: 0.0,
            },
        );
    }

    /// Record instruction for function profiling
    pub fn record_function_instr(&mut self, pc: u64, cycles: u64) {
        if let Some(&func_id) = self.function_map.get(&pc) {
            if let Some(stats) = self.function_stats.get_mut(&func_id) {
                stats.instruction_count += 1;
                stats.cycle_count += cycles;
            }
        }
    }

    /// Finalize the current bin and start a new one
    fn finalize_current_bin(&mut self) {
        if self.current_bin.instr_count == 0 {
            return;
        }

        // Calculate IPC for bin
        if self.current_bin.cycle_count > 0 {
            self.current_bin.ipc = self.current_bin.instr_count as f64
                / self.current_bin.cycle_count as f64;
        }

        self.bins.push(self.current_bin.clone());
        self.current_bin = StatsBin::default();
    }

    /// Finalize aggregation and return statistics
    pub fn finalize(mut self) -> AggregatedStatistics {
        // Finalize the last bin
        self.finalize_current_bin();

        // Build timeline data
        let ipc_timeline: Vec<_> = self
            .bins
            .iter()
            .map(|b| TimelinePoint::new(b.start_cycle, b.start_instr, b.ipc))
            .collect();

        let throughput_timeline: Vec<_> = self
            .bins
            .iter()
            .map(|b| {
                TimelinePoint::new(
                    b.start_cycle,
                    b.start_instr,
                    b.instr_count as f64 / (b.cycle_count.max(1) as f64),
                )
            })
            .collect();

        let l1_miss_timeline: Vec<_> = self
            .bins
            .iter()
            .map(|b| {
                let rate = if b.mem_ops > 0 {
                    b.l1_misses as f64 / b.mem_ops as f64
                } else {
                    0.0
                };
                TimelinePoint::new(b.start_cycle, b.start_instr, rate)
            })
            .collect();

        let l2_miss_timeline: Vec<_> = self
            .bins
            .iter()
            .map(|b| {
                let rate = if b.l1_misses > 0 {
                    b.l2_misses as f64 / b.l1_misses as f64
                } else {
                    0.0
                };
                TimelinePoint::new(b.start_cycle, b.start_instr, rate)
            })
            .collect();

        // Calculate overall IPC
        let ipc = if self.total_cycles > 0 {
            self.total_instructions as f64 / self.total_cycles as f64
        } else {
            0.0
        };

        // Finalize function stats
        let mut function_stats: Vec<_> = self.function_stats.into_values().collect();
        for stats in &mut function_stats {
            if stats.cycle_count > 0 {
                stats.ipc = stats.instruction_count as f64 / stats.cycle_count as f64;
            }
        }

        // Calculate cache statistics
        let cache_stats = CacheStatistics {
            l1_d_misses: self.total_l1_misses,
            l1_d_hits: self.total_mem_ops.saturating_sub(self.total_l1_misses),
            l2_misses: self.total_l2_misses,
            l2_hits: self.total_l1_misses.saturating_sub(self.total_l2_misses),
            l1_miss_rate: if self.total_mem_ops > 0 {
                self.total_l1_misses as f64 / self.total_mem_ops as f64
            } else {
                0.0
            },
            l2_miss_rate: if self.total_l1_misses > 0 {
                self.total_l2_misses as f64 / self.total_l1_misses as f64
            } else {
                0.0
            },
            ..Default::default()
        };

        AggregatedStatistics {
            total_instructions: self.total_instructions,
            total_cycles: self.total_cycles,
            ipc,
            bin_size: self.bin_size,
            bin_count: self.bins.len() as u64,
            ipc_timeline,
            throughput_timeline,
            l1_miss_timeline,
            l2_miss_timeline,
            bins: self.bins,
            function_stats,
            cache_stats,
            pipeline_utilization: PipelineUtilization::default(),
            anomalies: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aggregator_basic() {
        let mut aggregator = StatsAggregator::new(100);

        for i in 0..250 {
            aggregator.record_instruction(i, i * 2, i * 2 + 1, false, false, false, false);
        }

        let stats = aggregator.finalize();

        assert_eq!(stats.total_instructions, 250);
        assert_eq!(stats.bin_count, 3); // 100 + 100 + 50
    }

    #[test]
    fn test_aggregator_with_memory() {
        let mut aggregator = StatsAggregator::new(10);

        for i in 0..20 {
            let is_mem = i % 3 == 0;
            let l1_miss = is_mem && i % 6 == 0;
            aggregator.record_instruction(i, i, i + 5, is_mem, false, l1_miss, false);
        }

        let stats = aggregator.finalize();

        assert!(stats.cache_stats.l1_miss_rate > 0.0);
    }

    #[test]
    fn test_timeline_points() {
        let mut aggregator = StatsAggregator::new(50);

        for i in 0..100 {
            aggregator.record_instruction(i, i, i + 2, false, false, false, false);
        }

        let stats = aggregator.finalize();

        assert_eq!(stats.ipc_timeline.len(), 2);
        assert!(stats.ipc_timeline[0].value > 0.0);
    }
}
