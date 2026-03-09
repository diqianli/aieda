//! Performance statistics module for the ARM CPU emulator.

mod collector;
mod metrics;
mod trace_output;

pub use collector::StatsCollector;
pub use metrics::{PerformanceMetrics, CacheMetrics, ExecutionMetrics};
pub use trace_output::{TraceOutput, TraceEntry};

use crate::types::OpcodeType;
use ahash::AHashMap;

/// Internal cache statistics (for tracking during simulation)
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total accesses
    pub accesses: u64,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Evictions
    pub evictions: u64,
}

impl CacheStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        if self.accesses == 0 {
            0.0
        } else {
            self.hits as f64 / self.accesses as f64
        }
    }

    /// Calculate miss rate
    pub fn miss_rate(&self) -> f64 {
        if self.accesses == 0 {
            0.0
        } else {
            self.misses as f64 / self.accesses as f64
        }
    }

    /// Calculate MPKI (Misses Per Kilo Instructions)
    pub fn mpki(&self, total_instructions: u64) -> f64 {
        if total_instructions == 0 {
            0.0
        } else {
            (self.misses as f64 / total_instructions as f64) * 1000.0
        }
    }

    /// Add an access
    pub fn add_access(&mut self, hit: bool) {
        self.accesses += 1;
        if hit {
            self.hits += 1;
        } else {
            self.misses += 1;
        }
    }

    /// Add an eviction
    pub fn add_eviction(&mut self) {
        self.evictions += 1;
    }
}

/// Performance statistics for the CPU emulator
#[derive(Debug, Clone, Default)]
pub struct PerformanceStats {
    /// Total instructions executed
    pub total_instructions: u64,
    /// Total cycles
    pub total_cycles: u64,
    /// Instructions by type
    pub instr_by_type: AHashMap<OpcodeType, u64>,
    /// L1 cache statistics
    pub l1_stats: CacheStats,
    /// L2 cache statistics
    pub l2_stats: CacheStats,
    /// Memory access statistics
    pub memory_stats: MemoryStats,
    /// Execution statistics
    pub exec_stats: ExecutionMetrics,
}

/// Memory access statistics
#[derive(Debug, Clone, Default)]
pub struct MemoryStats {
    /// Total load instructions
    pub loads: u64,
    /// Total store instructions
    pub stores: u64,
    /// Total bytes read
    pub bytes_read: u64,
    /// Total bytes written
    pub bytes_written: u64,
    /// Average load latency
    pub avg_load_latency: f64,
    /// Average store latency
    pub avg_store_latency: f64,
}

impl MemoryStats {
    /// Record a load
    pub fn record_load(&mut self, bytes: u64, latency: u64) {
        self.loads += 1;
        self.bytes_read += bytes;

        // Running average
        let n = self.loads as f64;
        self.avg_load_latency = self.avg_load_latency * (n - 1.0) / n + latency as f64 / n;
    }

    /// Record a store
    pub fn record_store(&mut self, bytes: u64, latency: u64) {
        self.stores += 1;
        self.bytes_written += bytes;

        // Running average
        let n = self.stores as f64;
        self.avg_store_latency = self.avg_store_latency * (n - 1.0) / n + latency as f64 / n;
    }

    /// Total memory operations
    pub fn total_ops(&self) -> u64 {
        self.loads + self.stores
    }
}

impl PerformanceStats {
    /// Create new empty statistics
    pub fn new() -> Self {
        Self::default()
    }

    /// Calculate IPC (Instructions Per Cycle)
    pub fn ipc(&self) -> f64 {
        if self.total_cycles == 0 {
            0.0
        } else {
            self.total_instructions as f64 / self.total_cycles as f64
        }
    }

    /// Calculate CPI (Cycles Per Instruction)
    pub fn cpi(&self) -> f64 {
        if self.total_instructions == 0 {
            0.0
        } else {
            self.total_cycles as f64 / self.total_instructions as f64
        }
    }

    /// Record an executed instruction
    pub fn record_instruction(&mut self, opcode_type: OpcodeType) {
        self.total_instructions += 1;
        *self.instr_by_type.entry(opcode_type).or_insert(0) += 1;
    }

    /// Record cycles
    pub fn record_cycles(&mut self, cycles: u64) {
        self.total_cycles += cycles;
    }

    /// Get instruction count by type
    pub fn instr_count(&self, opcode_type: OpcodeType) -> u64 {
        self.instr_by_type.get(&opcode_type).copied().unwrap_or(0)
    }

    /// Get percentage of instructions by type
    pub fn instr_percentage(&self, opcode_type: OpcodeType) -> f64 {
        if self.total_instructions == 0 {
            0.0
        } else {
            self.instr_count(opcode_type) as f64 / self.total_instructions as f64 * 100.0
        }
    }

    /// Calculate memory instruction percentage
    pub fn memory_instr_percentage(&self) -> f64 {
        let mem_instrs = self.instr_count(OpcodeType::Load)
            + self.instr_count(OpcodeType::Store)
            + self.instr_count(OpcodeType::LoadPair)
            + self.instr_count(OpcodeType::StorePair);

        if self.total_instructions == 0 {
            0.0
        } else {
            mem_instrs as f64 / self.total_instructions as f64 * 100.0
        }
    }

    /// Calculate branch instruction percentage
    pub fn branch_instr_percentage(&self) -> f64 {
        let branch_instrs = self.instr_count(OpcodeType::Branch)
            + self.instr_count(OpcodeType::BranchCond)
            + self.instr_count(OpcodeType::BranchReg);

        if self.total_instructions == 0 {
            0.0
        } else {
            branch_instrs as f64 / self.total_instructions as f64 * 100.0
        }
    }

    /// Reset all statistics
    pub fn reset(&mut self) {
        self.total_instructions = 0;
        self.total_cycles = 0;
        self.instr_by_type.clear();
        self.l1_stats = CacheStats::default();
        self.l2_stats = CacheStats::default();
        self.memory_stats = MemoryStats::default();
        self.exec_stats = ExecutionMetrics::default();
    }

    /// Merge statistics from another instance
    pub fn merge(&mut self, other: &PerformanceStats) {
        self.total_instructions += other.total_instructions;
        self.total_cycles += other.total_cycles;

        for (opcode, count) in &other.instr_by_type {
            *self.instr_by_type.entry(*opcode).or_insert(0) += count;
        }

        self.l1_stats.accesses += other.l1_stats.accesses;
        self.l1_stats.hits += other.l1_stats.hits;
        self.l1_stats.misses += other.l1_stats.misses;
        self.l1_stats.evictions += other.l1_stats.evictions;

        self.l2_stats.accesses += other.l2_stats.accesses;
        self.l2_stats.hits += other.l2_stats.hits;
        self.l2_stats.misses += other.l2_stats.misses;
        self.l2_stats.evictions += other.l2_stats.evictions;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_perf_stats() {
        let mut stats = PerformanceStats::new();

        stats.record_instruction(OpcodeType::Add);
        stats.record_instruction(OpcodeType::Add);
        stats.record_instruction(OpcodeType::Load);

        stats.record_cycles(10);

        assert_eq!(stats.total_instructions, 3);
        assert_eq!(stats.total_cycles, 10);
        assert!((stats.ipc() - 0.3).abs() < 0.001);
        assert_eq!(stats.instr_count(OpcodeType::Add), 2);
    }

    #[test]
    fn test_cache_stats() {
        let mut stats = CacheStats::default();

        stats.add_access(true);
        stats.add_access(true);
        stats.add_access(false);

        assert_eq!(stats.accesses, 3);
        assert_eq!(stats.hits, 2);
        assert_eq!(stats.misses, 1);
        assert!((stats.hit_rate() - 0.666).abs() < 0.01);
    }

    #[test]
    fn test_memory_stats() {
        let mut stats = MemoryStats::default();

        stats.record_load(8, 10);
        stats.record_load(8, 20);
        stats.record_store(8, 5);

        assert_eq!(stats.loads, 2);
        assert_eq!(stats.stores, 1);
        assert_eq!(stats.bytes_read, 16);
        assert_eq!(stats.bytes_written, 8);
        assert!((stats.avg_load_latency - 15.0).abs() < 0.01);
    }
}
