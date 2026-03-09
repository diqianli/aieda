//! CHI timing model.

use super::interface::ChiTransaction;

/// CHI timing configuration
#[derive(Debug, Clone, Copy)]
pub struct ChiTimingConfig {
    /// Request channel latency (cycles)
    pub request_latency: u64,
    /// Response channel latency (cycles)
    pub response_latency: u64,
    /// Data channel latency (cycles)
    pub data_latency: u64,
    /// Snoop channel latency (cycles)
    pub snoop_latency: u64,
}

impl Default for ChiTimingConfig {
    fn default() -> Self {
        Self {
            request_latency: 2,
            response_latency: 2,
            data_latency: 2,
            snoop_latency: 2,
        }
    }
}

/// CHI timing model
pub struct ChiTimingModel {
    /// Configuration
    config: ChiTimingConfig,
    /// Current cycle
    current_cycle: u64,
    /// Pending events
    events: Vec<TimingEvent>,
}

/// Timing event
#[derive(Debug, Clone)]
struct TimingEvent {
    /// Transaction ID
    txn_id: super::protocol::ChiTxnId,
    /// Event type
    event_type: TimingEventType,
    /// Cycle when event fires
    cycle: u64,
}

/// Timing event type
#[derive(Debug, Clone, Copy)]
enum TimingEventType {
    RequestSent,
    ResponseReceived,
    DataReceived,
    SnoopSent,
    SnoopResponseReceived,
}

impl ChiTimingModel {
    /// Create a new timing model
    pub fn new(config: ChiTimingConfig) -> Self {
        Self {
            config,
            current_cycle: 0,
            events: Vec::new(),
        }
    }

    /// Calculate completion cycle for a transaction
    pub fn calculate_completion(&mut self, txn: &ChiTransaction) -> u64 {
        let request_sent = self.current_cycle + self.config.request_latency;

        // Assume immediate response from memory (simplified model)
        let response_received = request_sent + self.config.response_latency;

        let complete_cycle = if txn.request_type.requires_data() {
            response_received + self.config.data_latency
        } else {
            response_received
        };

        // Schedule events
        self.events.push(TimingEvent {
            txn_id: txn.txn_id,
            event_type: TimingEventType::RequestSent,
            cycle: request_sent,
        });

        self.events.push(TimingEvent {
            txn_id: txn.txn_id,
            event_type: TimingEventType::ResponseReceived,
            cycle: response_received,
        });

        if txn.request_type.requires_data() {
            self.events.push(TimingEvent {
                txn_id: txn.txn_id,
                event_type: TimingEventType::DataReceived,
                cycle: complete_cycle,
            });
        }

        complete_cycle
    }

    /// Process events for the current cycle
    pub fn process_events(&mut self) -> Vec<(super::protocol::ChiTxnId, TimingEventType)> {
        let current = self.current_cycle;

        let (ready, pending): (Vec<_>, Vec<_>) = self.events
            .drain(..)
            .partition(|e| e.cycle <= current);

        self.events = pending;

        ready.into_iter()
            .map(|e| (e.txn_id, e.event_type))
            .collect()
    }

    /// Advance simulation by one cycle
    pub fn advance_cycle(&mut self) {
        self.current_cycle += 1;
    }

    /// Get current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get pending event count
    pub fn pending_event_count(&self) -> usize {
        self.events.len()
    }

    /// Clear all events
    pub fn clear(&mut self) {
        self.events.clear();
    }
}

/// CHI latency calculator
pub struct ChiLatencyCalculator {
    /// Base memory latency
    base_memory_latency: u64,
    /// Interconnect hop latency
    hop_latency: u64,
    /// Maximum hops
    max_hops: u64,
}

impl ChiLatencyCalculator {
    /// Create a new latency calculator
    pub fn new(base_memory_latency: u64, hop_latency: u64, max_hops: u64) -> Self {
        Self {
            base_memory_latency,
            hop_latency,
            max_hops,
        }
    }

    /// Calculate read latency
    pub fn read_latency(&self, hops: u64) -> u64 {
        let hops = hops.min(self.max_hops);
        self.base_memory_latency + hops * self.hop_latency
    }

    /// Calculate write latency
    pub fn write_latency(&self, hops: u64) -> u64 {
        let hops = hops.min(self.max_hops);
        self.base_memory_latency + hops * self.hop_latency
    }

    /// Calculate snoop latency
    pub fn snoop_latency(&self, hops: u64) -> u64 {
        let hops = hops.min(self.max_hops);
        hops * self.hop_latency
    }
}

impl Default for ChiLatencyCalculator {
    fn default() -> Self {
        Self::new(50, 2, 4)
    }
}

/// Bandwidth model for CHI
pub struct ChiBandwidthModel {
    /// Maximum bandwidth (bytes per cycle)
    max_bandwidth: u64,
    /// Current bandwidth usage
    current_usage: u64,
    /// Bandwidth history
    history: Vec<u64>,
    /// History length
    history_length: usize,
}

impl ChiBandwidthModel {
    /// Create a new bandwidth model
    pub fn new(max_bandwidth: u64, history_length: usize) -> Self {
        Self {
            max_bandwidth,
            current_usage: 0,
            history: Vec::with_capacity(history_length),
            history_length,
        }
    }

    /// Record bandwidth usage
    pub fn record(&mut self, bytes: u64) {
        self.current_usage += bytes;
    }

    /// End current cycle
    pub fn end_cycle(&mut self) {
        self.history.push(self.current_usage);
        if self.history.len() > self.history_length {
            self.history.remove(0);
        }
        self.current_usage = 0;
    }

    /// Check if bandwidth is available
    pub fn has_bandwidth(&self, bytes: u64) -> bool {
        self.current_usage + bytes <= self.max_bandwidth
    }

    /// Get average bandwidth
    pub fn average_bandwidth(&self) -> f64 {
        if self.history.is_empty() {
            return 0.0;
        }
        let total: u64 = self.history.iter().sum();
        total as f64 / self.history.len() as f64
    }

    /// Get utilization percentage
    pub fn utilization(&self) -> f64 {
        if self.max_bandwidth == 0 {
            return 0.0;
        }
        self.average_bandwidth() / self.max_bandwidth as f64 * 100.0
    }

    /// Clear history
    pub fn clear(&mut self) {
        self.current_usage = 0;
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timing_model() {
        let config = ChiTimingConfig::default();
        let mut model = ChiTimingModel::new(config);

        let txn = ChiTransaction::new(
            super::super::protocol::ChiTxnId::new(0),
            crate::types::InstructionId(0),
            super::super::protocol::ChiRequestType::ReadNoSnoop,
            0x1000,
            8,
        );

        let complete = model.calculate_completion(&txn);
        assert!(complete > 0);
    }

    #[test]
    fn test_latency_calculator() {
        let calc = ChiLatencyCalculator::new(50, 2, 4);

        let latency = calc.read_latency(2);
        assert_eq!(latency, 54);

        let latency = calc.read_latency(10); // Capped at max_hops
        assert_eq!(latency, 58);
    }

    #[test]
    fn test_bandwidth_model() {
        let mut model = ChiBandwidthModel::new(100, 10);

        assert!(model.has_bandwidth(50));
        model.record(50);

        assert!(!model.has_bandwidth(60));
        assert!(model.has_bandwidth(50));

        model.end_cycle();
        assert!((model.utilization() - 50.0).abs() < 0.1);
    }
}
