//! Memory controller for managing memory requests.

use crate::types::InstructionId;
use std::collections::VecDeque;

/// Memory request tracking
#[derive(Debug, Clone)]
pub struct MemoryRequest {
    /// Instruction ID that made this request
    pub instruction_id: InstructionId,
    /// Address being accessed
    pub addr: u64,
    /// Size of access
    pub size: u8,
    /// Whether this is a read
    pub is_read: bool,
    /// Cycle when request was issued
    pub issue_cycle: u64,
    /// Cycle when request will complete
    pub complete_cycle: u64,
}

/// Memory controller
pub struct MemoryController {
    /// Memory latency in cycles
    latency: u64,
    /// Maximum outstanding requests
    max_outstanding: usize,
    /// Pending requests
    pending: VecDeque<MemoryRequest>,
    /// Current cycle
    current_cycle: u64,
    /// Total requests served
    total_requests: u64,
    /// Total bytes transferred
    total_bytes: u64,
}

impl MemoryController {
    /// Create a new memory controller
    pub fn new(latency: u64, max_outstanding: usize) -> Self {
        Self {
            latency,
            max_outstanding,
            pending: VecDeque::with_capacity(max_outstanding),
            current_cycle: 0,
            total_requests: 0,
            total_bytes: 0,
        }
    }

    /// Check if the controller can accept more requests
    pub fn can_accept(&self) -> bool {
        self.pending.len() < self.max_outstanding
    }

    /// Submit a read request
    pub fn read(&mut self, id: InstructionId, addr: u64, size: u8) -> Option<u64> {
        if !self.can_accept() {
            return None;
        }

        let complete_cycle = self.current_cycle + self.latency;

        self.pending.push_back(MemoryRequest {
            instruction_id: id,
            addr,
            size,
            is_read: true,
            issue_cycle: self.current_cycle,
            complete_cycle,
        });

        self.total_requests += 1;
        self.total_bytes += size as u64;

        Some(complete_cycle)
    }

    /// Submit a write request
    pub fn write(&mut self, id: InstructionId, addr: u64, size: u8) -> Option<u64> {
        if !self.can_accept() {
            return None;
        }

        let complete_cycle = self.current_cycle + self.latency;

        self.pending.push_back(MemoryRequest {
            instruction_id: id,
            addr,
            size,
            is_read: false,
            issue_cycle: self.current_cycle,
            complete_cycle,
        });

        self.total_requests += 1;
        self.total_bytes += size as u64;

        Some(complete_cycle)
    }

    /// Poll for completed requests
    pub fn poll_completed(&mut self, cycle: u64) -> Vec<MemoryRequest> {
        let mut completed = Vec::new();

        while let Some(req) = self.pending.front() {
            if req.complete_cycle <= cycle {
                completed.push(self.pending.pop_front().unwrap());
            } else {
                break;
            }
        }

        completed
    }

    /// Get the number of pending requests
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Advance the current cycle
    pub fn advance_cycle(&mut self) {
        self.current_cycle += 1;
    }

    /// Get the current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get statistics
    pub fn get_stats(&self) -> MemoryControllerStats {
        MemoryControllerStats {
            pending_requests: self.pending.len(),
            total_requests: self.total_requests,
            total_bytes: self.total_bytes,
            average_latency: if self.total_requests > 0 {
                self.latency
            } else {
                0
            },
        }
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.total_requests = 0;
        self.total_bytes = 0;
    }

    /// Clear all pending requests
    pub fn clear(&mut self) {
        self.pending.clear();
    }
}

/// Memory controller statistics
#[derive(Debug, Clone, Copy)]
pub struct MemoryControllerStats {
    pub pending_requests: usize,
    pub total_requests: u64,
    pub total_bytes: u64,
    pub average_latency: u64,
}

/// Bandwidth tracker for memory controller
pub struct BandwidthTracker {
    /// Samples per window
    samples_per_window: usize,
    /// Bytes transferred in current window
    current_bytes: u64,
    /// Bytes transferred in previous windows
    history: VecDeque<u64>,
    /// Maximum history length
    max_history: usize,
}

impl BandwidthTracker {
    /// Create a new bandwidth tracker
    pub fn new(samples_per_window: usize, max_history: usize) -> Self {
        Self {
            samples_per_window,
            current_bytes: 0,
            history: VecDeque::with_capacity(max_history),
            max_history,
        }
    }

    /// Record bytes transferred
    pub fn record(&mut self, bytes: u64) {
        self.current_bytes += bytes;
    }

    /// End current window and start a new one
    pub fn advance_window(&mut self) {
        self.history.push_back(self.current_bytes);
        if self.history.len() > self.max_history {
            self.history.pop_front();
        }
        self.current_bytes = 0;
    }

    /// Get average bandwidth (bytes per window)
    pub fn average_bandwidth(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        let total: u64 = self.history.iter().sum();
        total as f64 / self.history.len() as f64
    }

    /// Get peak bandwidth
    pub fn peak_bandwidth(&self) -> u64 {
        self.history.iter().copied().max().unwrap_or(0)
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.current_bytes = 0;
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_controller_basic() {
        let mut controller = MemoryController::new(100, 16);

        assert!(controller.can_accept());

        let complete = controller.read(InstructionId(0), 0x1000, 8);
        assert!(complete.is_some());
        assert_eq!(complete.unwrap(), 100);

        assert_eq!(controller.pending_count(), 1);
    }

    #[test]
    fn test_poll_completed() {
        let mut controller = MemoryController::new(100, 16);

        controller.read(InstructionId(0), 0x1000, 8);

        // Not complete at cycle 50
        let completed = controller.poll_completed(50);
        assert!(completed.is_empty());

        // Complete at cycle 100
        let completed = controller.poll_completed(100);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].instruction_id, InstructionId(0));
    }

    #[test]
    fn test_max_outstanding() {
        let mut controller = MemoryController::new(100, 2);

        assert!(controller.read(InstructionId(0), 0x1000, 8).is_some());
        assert!(controller.read(InstructionId(1), 0x1008, 8).is_some());
        assert!(controller.read(InstructionId(2), 0x1010, 8).is_none()); // Should fail
    }

    #[test]
    fn test_bandwidth_tracker() {
        let mut tracker = BandwidthTracker::new(100, 10);

        tracker.record(1000);
        tracker.record(500);
        tracker.advance_window();

        tracker.record(2000);
        tracker.advance_window();

        assert!((tracker.average_bandwidth() - 1750.0).abs() < 0.1);
        assert_eq!(tracker.peak_bandwidth(), 2000);
    }
}
