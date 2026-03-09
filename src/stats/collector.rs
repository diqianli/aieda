//! Statistics collector for the CPU emulator.

use super::{PerformanceStats, CacheMetrics, MemoryStats, PerformanceMetrics};
use crate::types::{Instruction, InstructionId, OpcodeType};
use ahash::AHashMap;
use std::collections::VecDeque;

/// Statistics collector
pub struct StatsCollector {
    /// Performance statistics
    stats: PerformanceStats,
    /// Instruction latencies (for averaging)
    latencies: AHashMap<OpcodeType, LatencyTracker>,
    /// Per-instruction timing info
    instr_timing: AHashMap<InstructionId, InstrTiming>,
    /// History of IPC samples
    ipc_history: VecDeque<f64>,
    /// Maximum history length
    max_history: usize,
}

/// Tracks latency statistics
#[derive(Debug, Clone, Default)]
struct LatencyTracker {
    count: u64,
    total: u64,
    min: u64,
    max: u64,
}

impl LatencyTracker {
    fn record(&mut self, latency: u64) {
        self.count += 1;
        self.total += latency;
        self.min = if self.min == 0 { latency } else { self.min.min(latency) };
        self.max = self.max.max(latency);
    }

    fn average(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            self.total as f64 / self.count as f64
        }
    }
}

/// Per-instruction timing information
#[derive(Debug, Clone)]
struct InstrTiming {
    dispatch_cycle: u64,
    issue_cycle: Option<u64>,
    complete_cycle: Option<u64>,
    commit_cycle: Option<u64>,
}

impl InstrTiming {
    fn execution_latency(&self) -> Option<u64> {
        match (self.issue_cycle, self.complete_cycle) {
            (Some(issue), Some(complete)) => Some(complete.saturating_sub(issue)),
            _ => None,
        }
    }

    fn total_latency(&self) -> Option<u64> {
        match (self.dispatch_cycle, self.commit_cycle) {
            (dispatch, Some(commit)) => Some(commit.saturating_sub(dispatch)),
            _ => None,
        }
    }
}

impl StatsCollector {
    /// Create a new statistics collector
    pub fn new() -> Self {
        Self {
            stats: PerformanceStats::new(),
            latencies: AHashMap::new(),
            instr_timing: AHashMap::new(),
            ipc_history: VecDeque::with_capacity(1000),
            max_history: 1000,
        }
    }

    /// Create with custom history length
    pub fn with_history_length(max_history: usize) -> Self {
        Self {
            stats: PerformanceStats::new(),
            latencies: AHashMap::new(),
            instr_timing: AHashMap::new(),
            ipc_history: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    /// Record instruction dispatch
    pub fn record_dispatch(&mut self, id: InstructionId, cycle: u64) {
        self.instr_timing.insert(id, InstrTiming {
            dispatch_cycle: cycle,
            issue_cycle: None,
            complete_cycle: None,
            commit_cycle: None,
        });
    }

    /// Record instruction issue
    pub fn record_issue(&mut self, id: InstructionId, cycle: u64) {
        if let Some(timing) = self.instr_timing.get_mut(&id) {
            timing.issue_cycle = Some(cycle);
        }
    }

    /// Record instruction completion
    pub fn record_complete(&mut self, id: InstructionId, cycle: u64) {
        if let Some(timing) = self.instr_timing.get_mut(&id) {
            timing.complete_cycle = Some(cycle);
        }
    }

    /// Record instruction commit
    pub fn record_commit(&mut self, instr: &Instruction, cycle: u64) {
        let id = instr.id;

        // Update timing
        if let Some(timing) = self.instr_timing.get_mut(&id) {
            timing.commit_cycle = Some(cycle);

            // Record latency
            if let Some(latency) = timing.execution_latency() {
                self.latencies.entry(instr.opcode_type).or_default().record(latency);
            }
        }

        // Update instruction count
        self.stats.record_instruction(instr.opcode_type);

        // Remove from timing map to save memory
        self.instr_timing.remove(&id);
    }

    /// Record cache access
    pub fn record_l1_access(&mut self, hit: bool) {
        self.stats.l1_stats.add_access(hit);
    }

    /// Record L1 eviction
    pub fn record_l1_eviction(&mut self) {
        self.stats.l1_stats.add_eviction();
    }

    /// Record L2 access
    pub fn record_l2_access(&mut self, hit: bool) {
        self.stats.l2_stats.add_access(hit);
    }

    /// Record L2 eviction
    pub fn record_l2_eviction(&mut self) {
        self.stats.l2_stats.add_eviction();
    }

    /// Record memory load
    pub fn record_load(&mut self, bytes: u64, latency: u64) {
        self.stats.memory_stats.record_load(bytes, latency);
    }

    /// Record memory store
    pub fn record_store(&mut self, bytes: u64, latency: u64) {
        self.stats.memory_stats.record_store(bytes, latency);
    }

    /// Record cycle count
    pub fn record_cycles(&mut self, cycles: u64) {
        self.stats.record_cycles(cycles);
    }

    /// Record IPC sample
    pub fn record_ipc_sample(&mut self, ipc: f64) {
        self.ipc_history.push_back(ipc);
        if self.ipc_history.len() > self.max_history {
            self.ipc_history.pop_front();
        }
    }

    /// Get current statistics
    pub fn stats(&self) -> &PerformanceStats {
        &self.stats
    }

    /// Get mutable statistics
    pub fn stats_mut(&mut self) -> &mut PerformanceStats {
        &mut self.stats
    }

    /// Get average latency for an opcode type
    pub fn avg_latency(&self, opcode_type: OpcodeType) -> f64 {
        self.latencies.get(&opcode_type)
            .map(|t| t.average())
            .unwrap_or(0.0)
    }

    /// Get min latency for an opcode type
    pub fn min_latency(&self, opcode_type: OpcodeType) -> u64 {
        self.latencies.get(&opcode_type)
            .map(|t| t.min)
            .unwrap_or(0)
    }

    /// Get max latency for an opcode type
    pub fn max_latency(&self, opcode_type: OpcodeType) -> u64 {
        self.latencies.get(&opcode_type)
            .map(|t| t.max)
            .unwrap_or(0)
    }

    /// Get IPC history
    pub fn ipc_history(&self) -> &VecDeque<f64> {
        &self.ipc_history
    }

    /// Get average IPC from history
    pub fn avg_ipc(&self) -> f64 {
        if self.ipc_history.is_empty() {
            return 0.0;
        }
        let sum: f64 = self.ipc_history.iter().sum();
        sum / self.ipc_history.len() as f64
    }

    /// Get performance metrics summary
    pub fn get_metrics(&self) -> PerformanceMetrics {
        PerformanceMetrics {
            total_instructions: self.stats.total_instructions,
            total_cycles: self.stats.total_cycles,
            ipc: self.stats.ipc(),
            cpi: self.stats.cpi(),
            l1_hit_rate: self.stats.l1_stats.hit_rate(),
            l2_hit_rate: self.stats.l2_stats.hit_rate(),
            l1_mpki: self.stats.l1_stats.mpki(self.stats.total_instructions),
            l2_mpki: self.stats.l2_stats.mpki(self.stats.total_instructions),
            memory_instr_pct: self.stats.memory_instr_percentage(),
            branch_instr_pct: self.stats.branch_instr_percentage(),
            avg_load_latency: self.stats.memory_stats.avg_load_latency,
            avg_store_latency: self.stats.memory_stats.avg_store_latency,
        }
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        self.stats.reset();
        self.latencies.clear();
        self.instr_timing.clear();
        self.ipc_history.clear();
    }
}

impl Default for StatsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_collector() {
        let mut collector = StatsCollector::new();

        let instr = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Add);
        collector.record_dispatch(InstructionId(0), 0);
        collector.record_issue(InstructionId(0), 2);
        collector.record_complete(InstructionId(0), 4);
        collector.record_commit(&instr, 5);

        assert_eq!(collector.stats().total_instructions, 1);
        assert!((collector.avg_latency(OpcodeType::Add) - 2.0).abs() < 0.001);
    }
}
