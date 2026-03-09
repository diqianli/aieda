//! Request Node - Fully coherent (RN-F) implementation.
//!
//! The RN-F represents a CPU core with L1/L2 caches that participates
//! in the CHI coherence protocol.

use std::collections::VecDeque;
use ahash::AHashMap;

use super::protocol::{ChiRequestType, ChiResponseType, ChiTxnId};
use super::coherence::{ChiCacheState, CoherenceStateMachine};
use super::qos::{QosCreditManager, PendingRequest};
use super::node::{
    ChiNodeConfig, ChiRequest, ChiResponse, ChiData, ChiSnoop, ChiSnoopResp,
    DataDescriptor, Channel, NodeId,
};
use crate::types::{InstructionId, Result};
use crate::memory::{Cache, CacheConfig};

/// Outstanding transaction tracking
#[derive(Debug, Clone)]
pub struct OutstandingTxn {
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Instruction ID
    pub instr_id: InstructionId,
    /// Request type
    pub req_type: ChiRequestType,
    /// Address
    pub addr: u64,
    /// Issue cycle
    pub issue_cycle: u64,
    /// Whether waiting for data
    pub waiting_data: bool,
    /// Whether waiting for DBID
    pub waiting_dbid: bool,
    /// DBID assigned
    pub dbid: Option<u16>,
}

/// Local cache entry with CHI state
#[derive(Debug, Clone)]
pub struct RnFCacheEntry {
    /// CHI cache state
    pub state: ChiCacheState,
    /// Whether entry is valid
    pub valid: bool,
}

/// Request Node - Fully Coherent (RN-F)
pub struct RnFNode {
    /// Node ID
    pub node_id: NodeId,
    /// Home Node ID
    pub home_node_id: NodeId,
    /// Configuration
    config: ChiNodeConfig,
    /// L1 Data Cache
    l1_cache: Cache,
    /// L2 Cache
    l2_cache: Cache,
    /// CHI cache states for L2 lines
    chi_states: AHashMap<u64, ChiCacheState>,
    /// REQ channel (outgoing)
    req_channel: Channel<ChiRequest>,
    /// RSP channel (incoming)
    rsp_channel: Channel<ChiResponse>,
    /// DAT channel (incoming data)
    dat_channel: Channel<ChiData>,
    /// SNP channel (incoming snoop)
    snp_channel: Channel<ChiSnoop>,
    /// SNP response channel (outgoing)
    snp_resp_channel: Channel<ChiSnoopResp>,
    /// Outstanding transactions
    outstanding: AHashMap<ChiTxnId, OutstandingTxn>,
    /// Transaction ID generator
    next_txn_id: u16,
    /// QoS credit manager
    qos_manager: QosCreditManager,
    /// Retry queue
    retry_queue: VecDeque<PendingRequest>,
    /// Store buffer for pending writes
    store_buffer: VecDeque<StoreBufferEntry>,
    /// Current cycle
    current_cycle: u64,
    /// Statistics
    stats: RnFStats,
}

/// Store buffer entry
#[derive(Debug, Clone)]
pub struct StoreBufferEntry {
    /// Instruction ID
    pub instr_id: InstructionId,
    /// Address
    pub addr: u64,
    /// Size
    pub size: u8,
    /// Issue cycle
    pub issue_cycle: u64,
    /// Whether committed
    pub committed: bool,
}

/// RN-F statistics
#[derive(Debug, Clone, Default)]
pub struct RnFStats {
    /// Read requests sent
    pub read_requests: u64,
    /// Write requests sent
    pub write_requests: u64,
    /// Snoop requests received
    pub snoops_received: u64,
    /// Snoop responses sent
    pub snoop_responses: u64,
    /// Cache hits (L1)
    pub l1_hits: u64,
    /// Cache misses (L1)
    pub l1_misses: u64,
    /// L2 hits
    pub l2_hits: u64,
    /// L2 misses
    pub l2_misses: u64,
    /// Transactions completed
    pub transactions_completed: u64,
    /// Retries
    pub retries: u64,
}

impl RnFNode {
    /// Create a new RN-F node
    pub fn new(
        config: ChiNodeConfig,
        l1_config: CacheConfig,
        l2_config: CacheConfig,
        home_node_id: NodeId,
    ) -> Result<Self> {
        let l1_cache = Cache::new(l1_config)?;
        let l2_cache = Cache::new(l2_config)?;

        Ok(Self {
            node_id: NodeId(config.node_id),
            home_node_id,
            config,
            l1_cache,
            l2_cache,
            chi_states: AHashMap::new(),
            req_channel: Channel::new(16, "REQ"),
            rsp_channel: Channel::new(16, "RSP"),
            dat_channel: Channel::new(8, "DAT"),
            snp_channel: Channel::new(8, "SNP"),
            snp_resp_channel: Channel::new(8, "SNP_RSP"),
            outstanding: AHashMap::new(),
            next_txn_id: 0,
            qos_manager: QosCreditManager::new(16, 32, 64),
            retry_queue: VecDeque::new(),
            store_buffer: VecDeque::new(),
            current_cycle: 0,
            stats: RnFStats::default(),
        })
    }

    /// Generate a new transaction ID
    fn alloc_txn_id(&mut self) -> ChiTxnId {
        let id = self.next_txn_id;
        self.next_txn_id = self.next_txn_id.wrapping_add(1);
        ChiTxnId::new(id)
    }

    /// Process a load request
    pub fn load(&mut self, instr_id: InstructionId, addr: u64, size: u8) -> Option<u64> {
        // Check L1 cache
        let l1_hit = self.l1_cache.access(addr, true).unwrap_or(false);

        if l1_hit {
            self.stats.l1_hits += 1;
            return Some(self.current_cycle + self.l1_cache.hit_latency());
        }

        self.stats.l1_misses += 1;

        // Check L2 cache
        let l2_hit = self.l2_cache.access(addr, true).unwrap_or(false);

        if l2_hit {
            self.stats.l2_hits += 1;
            // Get CHI state for this line
            let state = self.get_chi_state(addr);
            if state.can_read() {
                // Fill L1 from L2
                self.l1_cache.fill_line(addr);
                let latency = self.l1_cache.hit_latency() + self.l2_cache.hit_latency();
                return Some(self.current_cycle + latency);
            }
            // Need to upgrade state
        } else {
            self.stats.l2_misses += 1;
        }

        // Need to send CHI request
        None
    }

    /// Process a store request
    pub fn store(&mut self, instr_id: InstructionId, addr: u64, size: u8) -> Option<u64> {
        // Check L1 cache
        let l1_hit = self.l1_cache.access(addr, false).unwrap_or(false);

        if l1_hit {
            // Check if we have write permission
            let state = self.get_chi_state(addr);
            if state.can_write() {
                return Some(self.current_cycle + 1);
            }
            // Need to upgrade to unique
        }

        // Add to store buffer
        self.store_buffer.push_back(StoreBufferEntry {
            instr_id,
            addr,
            size,
            issue_cycle: self.current_cycle,
            committed: false,
        });

        // Need to send CHI request for write permission
        None
    }

    /// Send a read request to HN-F
    pub fn send_read_request(
        &mut self,
        instr_id: InstructionId,
        addr: u64,
        size: u8,
        want_unique: bool,
    ) -> Option<ChiTxnId> {
        let req_type = if want_unique {
            ChiRequestType::ReadMakeUnique
        } else {
            ChiRequestType::ReadShared
        };

        self.send_request(instr_id, req_type, addr, size)
    }

    /// Send a write request to HN-F
    pub fn send_write_request(
        &mut self,
        instr_id: InstructionId,
        addr: u64,
        size: u8,
    ) -> Option<ChiTxnId> {
        // First need to get unique ownership
        self.send_request(instr_id, ChiRequestType::MakeUnique, addr, size)
    }

    /// Send a request to HN-F
    fn send_request(
        &mut self,
        instr_id: InstructionId,
        req_type: ChiRequestType,
        addr: u64,
        size: u8,
    ) -> Option<ChiTxnId> {
        // Check QoS credits
        let pcrd_type = QosCreditManager::get_pcrd_type(req_type);
        if !self.qos_manager.has_credit(pcrd_type) {
            // Queue for retry
            self.retry_queue.push_back(PendingRequest {
                instruction_id: instr_id,
                txn_id: None,
                request_type: req_type,
                addr,
                size,
                first_attempt_cycle: self.current_cycle,
                retry_count: 0,
                required_pcrd: pcrd_type,
            });
            self.stats.retries += 1;
            return None;
        }

        // Allocate transaction ID
        let txn_id = self.alloc_txn_id();

        // Create request
        let request = ChiRequest::new(
            txn_id,
            self.node_id,
            self.home_node_id,
            req_type,
            addr,
            size,
            instr_id,
        );

        // Track outstanding transaction
        self.outstanding.insert(txn_id, OutstandingTxn {
            txn_id,
            instr_id,
            req_type,
            addr,
            issue_cycle: self.current_cycle,
            waiting_data: req_type.requires_data(),
            waiting_dbid: req_type.is_write(),
            dbid: None,
        });

        // Send request
        if self.req_channel.send(request) {
            if req_type.is_read() {
                self.stats.read_requests += 1;
            } else {
                self.stats.write_requests += 1;
            }
            Some(txn_id)
        } else {
            self.outstanding.remove(&txn_id);
            None
        }
    }

    /// Handle incoming response
    pub fn handle_response(&mut self, resp: ChiResponse) {
        if let Some(txn) = self.outstanding.get_mut(&resp.txn_id) {
            match resp.resp_type {
                ChiResponseType::CompData => {
                    // Data included with response
                    txn.waiting_data = false;
                }
                ChiResponseType::DBIDResp => {
                    // DBID assigned for write
                    txn.waiting_dbid = false;
                    txn.dbid = resp.dbid;
                }
                ChiResponseType::Comp => {
                    // Simple completion
                    txn.waiting_data = false;
                    txn.waiting_dbid = false;
                }
                ChiResponseType::CompAck => {
                    // Acknowledgment received
                }
                _ => {}
            }

            // Check if transaction is complete
            let txn = self.outstanding.get(&resp.txn_id).unwrap();
            if !txn.waiting_data && !txn.waiting_dbid {
                self.complete_transaction(resp.txn_id, resp.addr);
            }
        }
    }

    /// Handle incoming data
    pub fn handle_data(&mut self, data: ChiData) {
        // Extract needed info before mutation
        let txn_info = self.outstanding.get(&data.txn_id).map(|txn| {
            (txn.req_type, txn.waiting_dbid)
        });

        if let Some((req_type, waiting_dbid)) = txn_info {
            // Update transaction
            if let Some(txn) = self.outstanding.get_mut(&data.txn_id) {
                txn.waiting_data = false;
            }

            // Fill caches with received data
            self.l2_cache.fill_line(data.addr);
            self.l1_cache.fill_line(data.addr);

            // Update CHI state
            let state = if data.data.dirty {
                ChiCacheState::UniqueDirty
            } else if req_type == ChiRequestType::ReadMakeUnique {
                ChiCacheState::UniqueClean
            } else {
                ChiCacheState::SharedClean
            };
            self.set_chi_state(data.addr, state);

            // Check if transaction is complete
            if !waiting_dbid {
                self.complete_transaction(data.txn_id, data.addr);
            }

            // Send CompAck
            let ack = ChiResponse::comp_ack(data.txn_id, self.node_id, self.home_node_id, data.addr);
            let _ = self.rsp_channel.send(ack);
        }
    }

    /// Handle incoming snoop request
    pub fn handle_snoop(&mut self, snoop: ChiSnoop) {
        self.stats.snoops_received += 1;

        let current_state = self.get_chi_state(snoop.addr);

        // Use coherence state machine to determine response
        let coherence_resp = CoherenceStateMachine::on_snoop_request(current_state, snoop.snoop_type);

        // Update local cache state
        self.set_chi_state(snoop.addr, coherence_resp.final_state);

        // Update L2 cache state
        if coherence_resp.final_state == ChiCacheState::Invalid {
            self.l2_cache.invalidate(snoop.addr);
            self.l1_cache.invalidate(snoop.addr);
        }

        // Send snoop response
        let snoop_resp = if coherence_resp.data_valid {
            let data = DataDescriptor::new(64)
                .with_dirty(coherence_resp.data_dirty)
                .with_state(coherence_resp.final_state);
            ChiSnoopResp::with_data(
                snoop.txn_id,
                self.node_id,
                snoop.src_id,
                snoop.addr,
                data,
                coherence_resp.final_state,
            )
        } else {
            ChiSnoopResp::ack(
                snoop.txn_id,
                self.node_id,
                snoop.src_id,
                snoop.addr,
                coherence_resp.final_state,
            )
        };

        self.snp_resp_channel.send(snoop_resp);
        self.stats.snoop_responses += 1;
    }

    /// Complete a transaction
    fn complete_transaction(&mut self, txn_id: ChiTxnId, addr: u64) {
        if let Some(txn) = self.outstanding.remove(&txn_id) {
            self.stats.transactions_completed += 1;

            // Return QoS credit
            let pcrd_type = QosCreditManager::get_pcrd_type(txn.req_type);
            self.qos_manager.return_credit(pcrd_type);

            // Free DBID if assigned
            if let Some(dbid) = txn.dbid {
                self.qos_manager.free_dbid(dbid);
            }
        }
    }

    /// Get CHI state for an address
    fn get_chi_state(&self, addr: u64) -> ChiCacheState {
        // Get line-aligned address
        let line_size = self.l2_cache.config().line_size;
        let aligned = addr & !((line_size - 1) as u64);

        self.chi_states.get(&aligned).copied().unwrap_or(ChiCacheState::Invalid)
    }

    /// Set CHI state for an address
    fn set_chi_state(&mut self, addr: u64, state: ChiCacheState) {
        let line_size = self.l2_cache.config().line_size;
        let aligned = addr & !((line_size - 1) as u64);

        if state == ChiCacheState::Invalid {
            self.chi_states.remove(&aligned);
        } else {
            self.chi_states.insert(aligned, state);
        }
    }

    /// Process pending retries
    pub fn process_retries(&mut self) {
        let granted = self.qos_manager.process_retries(self.current_cycle);
        for pending in granted {
            if let Some(_txn_id) = self.send_request(
                pending.instruction_id,
                pending.request_type,
                pending.addr,
                pending.size,
            ) {
                // Request sent successfully
            }
        }
    }

    /// Advance simulation by one cycle
    pub fn advance_cycle(&mut self) {
        self.current_cycle += 1;
    }

    /// Get the current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get statistics
    pub fn stats(&self) -> &RnFStats {
        &self.stats
    }

    /// Get L1 cache statistics
    pub fn l1_stats(&self) -> &crate::memory::CacheStats {
        self.l1_cache.stats()
    }

    /// Get L2 cache statistics
    pub fn l2_stats(&self) -> &crate::memory::CacheStats {
        self.l2_cache.stats()
    }

    /// Check if there are pending transactions
    pub fn has_pending_transactions(&self) -> bool {
        !self.outstanding.is_empty()
    }

    /// Get number of outstanding transactions
    pub fn outstanding_count(&self) -> usize {
        self.outstanding.len()
    }

    /// Get REQ channel (for interconnect)
    pub fn req_channel(&self) -> &Channel<ChiRequest> {
        &self.req_channel
    }

    /// Get REQ channel (mutable, for interconnect)
    pub fn req_channel_mut(&mut self) -> &mut Channel<ChiRequest> {
        &mut self.req_channel
    }

    /// Get RSP channel (for interconnect)
    pub fn rsp_channel(&self) -> &Channel<ChiResponse> {
        &self.rsp_channel
    }

    /// Get RSP channel (mutable, for interconnect)
    pub fn rsp_channel_mut(&mut self) -> &mut Channel<ChiResponse> {
        &mut self.rsp_channel
    }

    /// Get DAT channel (for interconnect)
    pub fn dat_channel(&self) -> &Channel<ChiData> {
        &self.dat_channel
    }

    /// Get DAT channel (mutable, for interconnect)
    pub fn dat_channel_mut(&mut self) -> &mut Channel<ChiData> {
        &mut self.dat_channel
    }

    /// Get SNP channel (for interconnect)
    pub fn snp_channel(&self) -> &Channel<ChiSnoop> {
        &self.snp_channel
    }

    /// Get SNP channel (mutable, for interconnect)
    pub fn snp_channel_mut(&mut self) -> &mut Channel<ChiSnoop> {
        &mut self.snp_channel
    }

    /// Get SNP response channel (for interconnect)
    pub fn snp_resp_channel(&self) -> &Channel<ChiSnoopResp> {
        &self.snp_resp_channel
    }

    /// Get SNP response channel (mutable, for interconnect)
    pub fn snp_resp_channel_mut(&mut self) -> &mut Channel<ChiSnoopResp> {
        &mut self.snp_resp_channel
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CPUConfig;
    use super::super::protocol::ChiSnoopType;

    fn create_test_rnf() -> RnFNode {
        let node_config = ChiNodeConfig::default();
        let l1_config = crate::memory::CacheConfig {
            size: 64 * 1024,
            associativity: 4,
            line_size: 64,
            hit_latency: 4,
            name: "L1".to_string(),
        };
        let l2_config = crate::memory::CacheConfig {
            size: 512 * 1024,
            associativity: 8,
            line_size: 64,
            hit_latency: 12,
            name: "L2".to_string(),
        };

        RnFNode::new(node_config, l1_config, l2_config, NodeId(1)).unwrap()
    }

    #[test]
    fn test_rnf_creation() {
        let rnf = create_test_rnf();
        assert_eq!(rnf.node_id, NodeId(0));
        assert!(!rnf.has_pending_transactions());
    }

    #[test]
    fn test_rnf_load_miss() {
        let mut rnf = create_test_rnf();

        // L1 and L2 miss, should return None (need CHI request)
        let result = rnf.load(InstructionId(0), 0x1000, 8);
        assert!(result.is_none());

        // Should have recorded L1 and L2 misses
        assert_eq!(rnf.stats().l1_misses, 1);
        assert_eq!(rnf.stats().l2_misses, 1);
    }

    #[test]
    fn test_rnf_send_read_request() {
        let mut rnf = create_test_rnf();

        let txn_id = rnf.send_read_request(InstructionId(0), 0x1000, 8, false);
        assert!(txn_id.is_some());
        assert!(rnf.has_pending_transactions());
        assert_eq!(rnf.outstanding_count(), 1);

        // Request should be in channel
        assert!(!rnf.req_channel().is_empty());
    }

    #[test]
    fn test_rnf_snoop_handling() {
        let mut rnf = create_test_rnf();

        // Set up a cache line in UniqueDirty state
        rnf.set_chi_state(0x1000, ChiCacheState::UniqueDirty);

        // Receive a SnpShared snoop
        let snoop = ChiSnoop::new(
            ChiSnoopType::SnpShared,
            ChiTxnId::new(1),
            NodeId(1),
            NodeId(0),
            0x1000,
        );

        rnf.handle_snoop(snoop);

        // Should have sent a snoop response
        assert!(!rnf.snp_resp_channel().is_empty());
        assert_eq!(rnf.stats().snoops_received, 1);
        assert_eq!(rnf.stats().snoop_responses, 1);
    }
}
