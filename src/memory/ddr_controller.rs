//! DDR Memory Controller with realistic timing model.
//!
//! This module simulates DDR4 memory access timing including:
//! - Row buffer hits/misses
//! - Bank interleaving
//! - Variable latency based on access patterns

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// DDR row buffer state per bank
#[derive(Debug, Clone)]
struct RowBufferState {
    /// Currently open row address per bank (None if no row open)
    open_rows: Vec<Option<u64>>,
}

impl RowBufferState {
    fn new(num_banks: usize) -> Self {
        Self {
            open_rows: vec![None; num_banks],
        }
    }
}

/// DDR access result
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DdrAccessResult {
    /// Cycle when access completes
    pub complete_cycle: u64,
    /// Bank that was accessed
    pub bank: usize,
    /// Whether this was a row buffer hit
    pub row_hit: bool,
    /// Total latency for this access
    pub latency: u64,
}

/// DDR memory controller statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct DdrStats {
    /// Total memory accesses
    pub total_accesses: u64,
    /// Row buffer hits
    pub row_buffer_hits: u64,
    /// Row buffer misses
    pub row_buffer_misses: u64,
    /// Total access latency (cycles)
    pub total_latency: u64,
}

impl DdrStats {
    /// Calculate row buffer hit rate
    pub fn hit_rate(&self) -> f64 {
        if self.total_accesses == 0 {
            0.0
        } else {
            self.row_buffer_hits as f64 / self.total_accesses as f64
        }
    }

    /// Calculate average access latency
    pub fn avg_latency(&self) -> f64 {
        if self.total_accesses == 0 {
            0.0
        } else {
            self.total_latency as f64 / self.total_accesses as f64
        }
    }
}

/// DDR memory controller with row buffer tracking
#[derive(Debug, Clone)]
pub struct DdrController {
    /// Base latency in cycles (CAS + RAS + AL)
    base_latency: u64,
    /// Latency bonus for row buffer hit
    row_buffer_hit_bonus: u64,
    /// Latency penalty for bank conflicts
    bank_conflict_penalty: u64,
    /// Number of DDR banks
    num_banks: usize,
    /// Row buffer state per bank
    row_buffers: RowBufferState,
    /// Current simulation cycle
    current_cycle: u64,
    /// Access statistics
    stats: DdrStats,
    /// Row size in bytes (typically 8KB for DDR4)
    row_size: u64,
}

impl DdrController {
    /// Create a new DDR controller
    pub fn new(
        base_latency: u64,
        row_buffer_hit_bonus: u64,
        bank_conflict_penalty: u64,
        num_banks: usize,
    ) -> Self {
        Self {
            base_latency,
            row_buffer_hit_bonus,
            bank_conflict_penalty,
            num_banks,
            row_buffers: RowBufferState::new(num_banks),
            current_cycle: 0,
            stats: DdrStats::default(),
            row_size: 8 * 1024, // 8KB rows (typical for DDR4)
        }
    }

    /// Set the current simulation cycle
    pub fn set_cycle(&mut self, cycle: u64) {
        self.current_cycle = cycle;
    }

    /// Get the current simulation cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Perform a memory access and return timing information
    pub fn access(&mut self, addr: u64) -> DdrAccessResult {
        // Bank selection: typically bits [6:8] or [6:9] depending on bank count
        // For 8 banks, use bits [6:8]
        let bank = ((addr >> 6) & ((self.num_banks - 1) as u64)) as usize;

        // Row address: typically upper bits, rows are 8KB
        let row_addr = addr & !(self.row_size - 1);

        // Check for row buffer hit
        let row_hit = self.row_buffers.open_rows[bank]
            .map(|r| r == row_addr)
            .unwrap_or(false);

        // Calculate latency based on row buffer state
        let latency = if row_hit {
            // Row buffer hit: faster access (ACT is skipped)
            self.base_latency.saturating_sub(self.row_buffer_hit_bonus)
        } else {
            // Row buffer miss: need to activate new row
            self.base_latency
        };

        // Update row buffer state
        self.row_buffers.open_rows[bank] = Some(row_addr);

        // Update statistics
        self.stats.total_accesses += 1;
        self.stats.total_latency += latency;
        if row_hit {
            self.stats.row_buffer_hits += 1;
        } else {
            self.stats.row_buffer_misses += 1;
        }

        let complete_cycle = self.current_cycle + latency;

        DdrAccessResult {
            complete_cycle,
            bank,
            row_hit,
            latency,
        }
    }

    /// Get access statistics
    pub fn stats(&self) -> &DdrStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = DdrStats::default();
    }

    /// Reset row buffer state (simulates power-on or refresh)
    pub fn reset_row_buffers(&mut self) {
        self.row_buffers = RowBufferState::new(self.num_banks);
    }

    /// Get the number of banks
    pub fn num_banks(&self) -> usize {
        self.num_banks
    }

    /// Get base latency
    pub fn base_latency(&self) -> u64 {
        self.base_latency
    }

    /// Get row buffer hit bonus
    pub fn row_buffer_hit_bonus(&self) -> u64 {
        self.row_buffer_hit_bonus
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ddr_controller_basic() {
        let mut ctrl = DdrController::new(150, 30, 20, 8);
        ctrl.set_cycle(100);

        // First access to a row - should be a miss
        let result = ctrl.access(0x1000);
        assert!(!result.row_hit);
        assert_eq!(result.latency, 150);
        assert_eq!(result.complete_cycle, 250);

        // Second access to same row - should be a hit
        ctrl.set_cycle(300);
        let result = ctrl.access(0x1040); // Same row, different column
        assert!(result.row_hit);
        assert_eq!(result.latency, 120); // 150 - 30 bonus
        assert_eq!(result.complete_cycle, 420);
    }

    #[test]
    fn test_ddr_bank_interleaving() {
        let mut ctrl = DdrController::new(150, 30, 20, 8);
        ctrl.set_cycle(0);

        // Access different banks
        let result0 = ctrl.access(0x0000); // Bank 0
        let result1 = ctrl.access(0x0040); // Bank 1
        let result2 = ctrl.access(0x0080); // Bank 2

        assert_eq!(result0.bank, 0);
        assert_eq!(result1.bank, 1);
        assert_eq!(result2.bank, 2);
    }

    #[test]
    fn test_ddr_stats() {
        let mut ctrl = DdrController::new(150, 30, 20, 8);
        ctrl.set_cycle(0);

        // Access same address twice
        ctrl.access(0x1000);
        ctrl.access(0x1000);

        let stats = ctrl.stats();
        assert_eq!(stats.total_accesses, 2);
        assert_eq!(stats.row_buffer_hits, 1);
        assert_eq!(stats.row_buffer_misses, 1);
        assert!((stats.hit_rate() - 0.5).abs() < 0.001);
    }
}
