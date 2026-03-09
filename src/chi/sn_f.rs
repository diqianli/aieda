//! Subordinate Node - Fully coherent (SN-F) implementation.
//!
//! The SN-F represents memory or downstream devices that respond to
//! requests from the HN-F.

use std::collections::VecDeque;
use ahash::AHashMap;

use super::protocol::{ChiRequestType, ChiResponseType, ChiTxnId};
use super::node::{
    ChiNodeConfig, ChiRequest, ChiResponse, ChiData,
    DataDescriptor, Channel, NodeId,
};

/// Memory request tracking
#[derive(Debug, Clone)]
pub struct MemoryRequest {
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Request type
    pub req_type: ChiRequestType,
    /// Address
    pub addr: u64,
    /// Size
    pub size: u8,
    /// Source node ID
    pub src_node: NodeId,
    /// Issue cycle
    pub issue_cycle: u64,
    /// Completion cycle
    pub complete_cycle: u64,
}

/// Subordinate Node - Fully Coherent (SN-F)
pub struct SnFNode {
    /// Node ID
    pub node_id: NodeId,
    /// Configuration
    config: ChiNodeConfig,
    /// REQ channel (incoming from HN-F)
    req_channel: Channel<ChiRequest>,
    /// RSP channel (outgoing to HN-F)
    rsp_channel: Channel<ChiResponse>,
    /// DAT channel (outgoing to HN-F)
    dat_channel: Channel<ChiData>,
    /// Pending memory requests
    pending_requests: AHashMap<ChiTxnId, MemoryRequest>,
    /// Request queue (for timing simulation)
    request_queue: VecDeque<MemoryRequest>,
    /// Current cycle
    current_cycle: u64,
    /// Memory latency in cycles
    memory_latency: u64,
    /// Memory bandwidth (bytes per cycle)
    memory_bandwidth: u64,
    /// Current bandwidth usage
    bandwidth_usage: u64,
    /// Statistics
    stats: SnFStats,
}

/// SN-F statistics
#[derive(Debug, Clone, Default)]
pub struct SnFStats {
    /// Total requests received
    pub requests_received: u64,
    /// Read requests
    pub read_requests: u64,
    /// Write requests
    pub write_requests: u64,
    /// Responses sent
    pub responses_sent: u64,
    /// Data responses sent
    pub data_responses_sent: u64,
    /// Total bytes transferred
    pub bytes_transferred: u64,
    /// Average latency
    pub total_latency: u64,
    /// Bandwidth stalls
    pub bandwidth_stalls: u64,
}

impl SnFNode {
    /// Create a new SN-F node
    pub fn new(config: ChiNodeConfig, memory_latency: u64, memory_bandwidth: u64) -> Self {
        Self {
            node_id: NodeId(config.node_id),
            config,
            req_channel: Channel::new(16, "REQ"),
            rsp_channel: Channel::new(16, "RSP"),
            dat_channel: Channel::new(16, "DAT"),
            pending_requests: AHashMap::new(),
            request_queue: VecDeque::new(),
            current_cycle: 0,
            memory_latency,
            memory_bandwidth,
            bandwidth_usage: 0,
            stats: SnFStats::default(),
        }
    }

    /// Process incoming requests
    pub fn process_requests(&mut self) {
        while let Some(req) = self.req_channel.recv() {
            self.handle_request(req);
        }
    }

    /// Handle an incoming request
    fn handle_request(&mut self, req: ChiRequest) {
        self.stats.requests_received += 1;

        if req.req_type.is_read() {
            self.stats.read_requests += 1;
        } else if req.req_type.is_write() {
            self.stats.write_requests += 1;
        }

        // Check bandwidth
        if self.bandwidth_usage + req.header.size as u64 > self.memory_bandwidth {
            // Queue for later
            self.stats.bandwidth_stalls += 1;
            self.request_queue.push_back(MemoryRequest {
                txn_id: req.header.txn_id,
                req_type: req.req_type,
                addr: req.header.addr,
                size: req.header.size,
                src_node: NodeId(req.header.src_id),
                issue_cycle: self.current_cycle,
                complete_cycle: self.current_cycle + self.memory_latency,
            });
            return;
        }

        // Process request
        self.process_memory_request(req);
    }

    /// Process a memory request
    fn process_memory_request(&mut self, req: ChiRequest) {
        self.bandwidth_usage += req.header.size as u64;

        let mem_req = MemoryRequest {
            txn_id: req.header.txn_id,
            req_type: req.req_type,
            addr: req.header.addr,
            size: req.header.size,
            src_node: NodeId(req.header.src_id),
            issue_cycle: self.current_cycle,
            complete_cycle: self.current_cycle + self.memory_latency,
        };

        self.pending_requests.insert(req.header.txn_id, mem_req.clone());
        self.stats.bytes_transferred += req.header.size as u64;
    }

    /// Check for completed requests
    pub fn process_completions(&mut self) {
        // Find completed requests
        let completed: Vec<ChiTxnId> = self
            .pending_requests
            .iter()
            .filter(|(_, req)| req.complete_cycle <= self.current_cycle)
            .map(|(id, _)| *id)
            .collect();

        for txn_id in completed {
            if let Some(req) = self.pending_requests.remove(&txn_id) {
                self.send_response(req);
            }
        }

        // Process queued requests if bandwidth available
        self.process_queued_requests();
    }

    /// Process queued requests
    fn process_queued_requests(&mut self) {
        while let Some(req) = self.request_queue.pop_front() {
            if self.bandwidth_usage + req.size as u64 > self.memory_bandwidth {
                // Put back and stop
                self.request_queue.push_front(req);
                break;
            }

            self.bandwidth_usage += req.size as u64;

            let mem_req = MemoryRequest {
                complete_cycle: self.current_cycle + self.memory_latency,
                ..req
            };

            self.pending_requests.insert(mem_req.txn_id, mem_req);
        }
    }

    /// Send response for completed request
    fn send_response(&mut self, req: MemoryRequest) {
        let latency = self.current_cycle.saturating_sub(req.issue_cycle);
        self.stats.total_latency += latency;

        if req.req_type.is_read() {
            // Send data response
            let data = DataDescriptor::new(req.size);
            let data_msg = ChiData::comp_data(
                req.txn_id,
                self.node_id,
                req.src_node,
                req.addr,
                data,
            );
            self.dat_channel.send(data_msg);
            self.stats.data_responses_sent += 1;
        } else {
            // Send completion response
            let resp = ChiResponse::new(
                req.txn_id,
                ChiResponseType::Comp,
                self.node_id,
                req.src_node,
                req.addr,
            );
            self.rsp_channel.send(resp);
        }

        self.stats.responses_sent += 1;
    }

    /// Advance simulation by one cycle
    pub fn advance_cycle(&mut self) {
        self.current_cycle += 1;
        // Reset bandwidth for new cycle
        self.bandwidth_usage = 0;
    }

    /// Get the current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get statistics
    pub fn stats(&self) -> &SnFStats {
        &self.stats
    }

    /// Get average latency
    pub fn average_latency(&self) -> f64 {
        if self.stats.responses_sent == 0 {
            0.0
        } else {
            self.stats.total_latency as f64 / self.stats.responses_sent as f64
        }
    }

    /// Check if there are pending requests
    pub fn has_pending_requests(&self) -> bool {
        !self.pending_requests.is_empty()
    }

    /// Get number of pending requests
    pub fn pending_count(&self) -> usize {
        self.pending_requests.len()
    }

    /// Get queue length
    pub fn queue_length(&self) -> usize {
        self.request_queue.len()
    }

    // Channel accessors for interconnect
    pub fn req_channel_mut(&mut self) -> &mut Channel<ChiRequest> {
        &mut self.req_channel
    }

    pub fn rsp_channel(&self) -> &Channel<ChiResponse> {
        &self.rsp_channel
    }

    pub fn rsp_channel_mut(&mut self) -> &mut Channel<ChiResponse> {
        &mut self.rsp_channel
    }

    pub fn dat_channel(&self) -> &Channel<ChiData> {
        &self.dat_channel
    }

    pub fn dat_channel_mut(&mut self) -> &mut Channel<ChiData> {
        &mut self.dat_channel
    }
}

/// Simple memory model for SN-F
pub struct MemoryModel {
    /// Memory size in bytes
    size: u64,
    /// Access latency
    latency: u64,
    /// Number of banks
    banks: u8,
    /// Bank conflict latency
    bank_conflict_latency: u64,
    /// Bank access tracking (for conflict detection)
    bank_last_access: Vec<u64>,
}

impl MemoryModel {
    /// Create a new memory model
    pub fn new(size: u64, latency: u64, banks: u8, bank_conflict_latency: u64) -> Self {
        Self {
            size,
            latency,
            banks,
            bank_conflict_latency,
            bank_last_access: vec![0; banks as usize],
        }
    }

    /// Calculate access latency for an address
    pub fn access_latency(&mut self, addr: u64, cycle: u64) -> u64 {
        let bank = self.get_bank(addr);
        let last_access = self.bank_last_access[bank as usize];

        // Conflict occurs if the bank was accessed previously and hasn't completed yet
        // (last_access > cycle means the previous access completes after current cycle)
        let additional_latency = if last_access > cycle {
            // Bank conflict
            self.bank_conflict_latency
        } else {
            0
        };

        self.bank_last_access[bank as usize] = cycle + self.latency + additional_latency;
        self.latency + additional_latency
    }

    /// Get bank for an address
    fn get_bank(&self, addr: u64) -> u8 {
        // Simple interleaving: use bits 6:3 for bank selection
        ((addr >> 3) & ((self.banks - 1) as u64)) as u8
    }

    /// Reset the model
    pub fn reset(&mut self) {
        self.bank_last_access.fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::InstructionId;

    fn create_test_snf() -> SnFNode {
        SnFNode::new(
            ChiNodeConfig { node_id: 2, ..Default::default() },
            100,
            128, // Large enough bandwidth for 64-byte cache line
        )
    }

    #[test]
    fn test_snf_creation() {
        let snf = create_test_snf();
        assert_eq!(snf.node_id, NodeId(2));
        assert!(!snf.has_pending_requests());
    }

    #[test]
    fn test_snf_read_request() {
        let mut snf = SnFNode::new(
            ChiNodeConfig { node_id: 2, ..Default::default() },
            100,
            128, // Large enough bandwidth for 64-byte request
        );

        let req = ChiRequest::new(
            ChiTxnId::new(1),
            NodeId(1),
            NodeId(2),
            ChiRequestType::ReadNoSnoop,
            0x1000,
            64,
            InstructionId(0),
        );

        snf.req_channel_mut().send(req);
        snf.process_requests();

        assert!(snf.has_pending_requests());
        assert_eq!(snf.stats().requests_received, 1);
        assert_eq!(snf.stats().read_requests, 1);
    }

    #[test]
    fn test_snf_completion() {
        let mut snf = SnFNode::new(
            ChiNodeConfig { node_id: 2, ..Default::default() },
            10, // 10 cycle latency
            128, // Large enough bandwidth
        );

        let req = ChiRequest::new(
            ChiTxnId::new(1),
            NodeId(1),
            NodeId(2),
            ChiRequestType::ReadNoSnoop,
            0x1000,
            64,
            InstructionId(0),
        );

        snf.req_channel_mut().send(req);
        snf.process_requests();

        // Should be pending
        assert!(snf.has_pending_requests());

        // Advance time
        for _ in 0..10 {
            snf.advance_cycle();
            snf.process_completions();
        }

        // Should be complete
        assert!(!snf.has_pending_requests());
        assert_eq!(snf.stats().responses_sent, 1);
        assert!(!snf.dat_channel().is_empty());
    }

    #[test]
    fn test_memory_model() {
        let mut model = MemoryModel::new(1024 * 1024 * 1024, 100, 4, 20);

        // First access should have base latency
        let lat1 = model.access_latency(0x1000, 0);
        assert_eq!(lat1, 100);

        // Access to different bank should have base latency
        // Bank for 0x1010: (0x1010 >> 3) & 3 = 2
        let lat2 = model.access_latency(0x1010, 0);
        assert_eq!(lat2, 100);

        // Access to same bank before previous completes should have conflict
        // Bank for 0x1000: (0x1000 >> 3) & 3 = 0
        // Bank for 0x1004: (0x1004 >> 3) & 3 = 0 (same bank)
        let lat3 = model.access_latency(0x1004, 50); // Same bank as first access (cycle 50 < 0 + 100)
        assert_eq!(lat3, 100 + 20); // Base + conflict (since first access at cycle 0 completes at cycle 100)
    }
}
