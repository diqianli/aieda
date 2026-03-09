//! Home Node - Fully coherent (HN-F) implementation.
//!
//! The HN-F is the protocol controller that manages cache coherence
//! through a directory-based snoop filter.

use ahash::AHashMap;

use super::protocol::{ChiRequestType, ChiResponseType, ChiSnoopType, ChiTxnId};
use super::coherence::ChiCacheState;
use super::directory::Directory;
use super::qos::{QosCreditManager, DbidAllocator};
use super::node::{
    ChiNodeConfig, ChiRequest, ChiResponse, ChiData, ChiSnoop, ChiSnoopResp,
    DataDescriptor, Channel, NodeId,
};
use crate::types::InstructionId;

/// Pending snoop tracking
#[derive(Debug, Clone)]
pub struct PendingSnoop {
    /// Original transaction ID
    pub txn_id: ChiTxnId,
    /// Original request
    pub request: ChiRequest,
    /// Number of snoop responses expected
    pub responses_expected: u32,
    /// Number of snoop responses received
    pub responses_received: u32,
    /// Whether data has been received
    pub data_received: bool,
    /// Data from snoop response (if any)
    pub snoop_data: Option<DataDescriptor>,
    /// Cycle when snoops were sent
    pub snoop_issue_cycle: u64,
}

/// Transaction state in HN-F
#[derive(Debug, Clone)]
pub struct HnfTransaction {
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Original request
    pub request: ChiRequest,
    /// Source node ID
    pub src_node: NodeId,
    /// State
    pub state: HnfTxnState,
    /// Pending snoops
    pub pending_snoop: Option<PendingSnoop>,
    /// DBID assigned (for writes)
    pub dbid: Option<u16>,
    /// Cycle when request was received
    pub receive_cycle: u64,
}

/// HN-F transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HnfTxnState {
    /// Request received, checking directory
    Received,
    /// Sending snoops
    SnoopPending,
    /// Waiting for memory
    MemoryPending,
    /// Ready to respond
    ReadyToRespond,
    /// Response sent, waiting for CompAck
    WaitingCompAck,
    /// Complete
    Complete,
}

/// Home Node - Fully Coherent (HN-F)
pub struct HnFNode {
    /// Node ID
    pub node_id: NodeId,
    /// Subordinate Node ID (memory)
    pub sn_node_id: NodeId,
    /// Configuration
    config: ChiNodeConfig,
    /// Directory (snoop filter)
    directory: Directory,
    /// REQ channel (incoming)
    req_channel: Channel<ChiRequest>,
    /// RSP channel (outgoing)
    rsp_channel: Channel<ChiResponse>,
    /// DAT channel (outgoing)
    dat_channel: Channel<ChiData>,
    /// SNP channel (outgoing)
    snp_channel: Channel<ChiSnoop>,
    /// SNP response channel (incoming)
    snp_resp_channel: Channel<ChiSnoopResp>,
    /// Request channel to SN-F
    sn_req_channel: Channel<ChiRequest>,
    /// Response channel from SN-F
    sn_rsp_channel: Channel<ChiResponse>,
    /// Data channel from SN-F
    sn_dat_channel: Channel<ChiData>,
    /// Active transactions
    transactions: AHashMap<ChiTxnId, HnfTransaction>,
    /// DBID allocator
    dbid_allocator: DbidAllocator,
    /// QoS credit manager
    qos_manager: QosCreditManager,
    /// Current cycle
    current_cycle: u64,
    /// Memory latency (to SN-F)
    memory_latency: u64,
    /// Statistics
    stats: HnFStats,
}

/// HN-F statistics
#[derive(Debug, Clone, Default)]
pub struct HnFStats {
    /// Requests received
    pub requests_received: u64,
    /// Read requests
    pub read_requests: u64,
    /// Write requests
    pub write_requests: u64,
    /// Snoops sent
    pub snoops_sent: u64,
    /// Snoop responses received
    pub snoop_responses: u64,
    /// Memory requests
    pub memory_requests: u64,
    /// Transactions completed
    pub transactions_completed: u64,
    /// Directory hits
    pub dir_hits: u64,
    /// Directory misses
    pub dir_misses: u64,
}

impl HnFNode {
    /// Create a new HN-F node
    pub fn new(
        config: ChiNodeConfig,
        directory_size: usize,
        cache_line_size: usize,
        sn_node_id: NodeId,
        memory_latency: u64,
    ) -> Self {
        Self {
            node_id: NodeId(config.node_id),
            sn_node_id,
            config,
            directory: Directory::new(cache_line_size, directory_size),
            req_channel: Channel::new(32, "REQ"),
            rsp_channel: Channel::new(32, "RSP"),
            dat_channel: Channel::new(16, "DAT"),
            snp_channel: Channel::new(16, "SNP"),
            snp_resp_channel: Channel::new(16, "SNP_RSP"),
            sn_req_channel: Channel::new(16, "SN_REQ"),
            sn_rsp_channel: Channel::new(16, "SN_RSP"),
            sn_dat_channel: Channel::new(16, "SN_DAT"),
            transactions: AHashMap::new(),
            dbid_allocator: DbidAllocator::new(64),
            qos_manager: QosCreditManager::new(32, 64, 128),
            current_cycle: 0,
            memory_latency,
            stats: HnFStats::default(),
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

        // Create transaction
        let txn = HnfTransaction {
            txn_id: req.header.txn_id,
            request: req.clone(),
            src_node: NodeId(req.header.src_id),
            state: HnfTxnState::Received,
            pending_snoop: None,
            dbid: None,
            receive_cycle: self.current_cycle,
        };

        self.transactions.insert(req.header.txn_id, txn);

        // Start directory lookup
        self.directory.start_transaction(req.header.addr, req.header.txn_id);

        // Process based on request type
        match req.req_type {
            ChiRequestType::ReadShared | ChiRequestType::ReadNotSharedDirty => {
                self.handle_read_request(req, false);
            }
            ChiRequestType::ReadMakeUnique | ChiRequestType::ReadOnce | ChiRequestType::ReadNoSnoop => {
                self.handle_read_request(req, true);
            }
            ChiRequestType::MakeUnique | ChiRequestType::CleanUnique => {
                self.handle_unique_request(req);
            }
            ChiRequestType::WriteUnique | ChiRequestType::WriteNoSnoop => {
                self.handle_write_request(req);
            }
            ChiRequestType::Evict => {
                self.handle_evict_request(req);
            }
            ChiRequestType::CleanShared | ChiRequestType::CleanInvalid | ChiRequestType::MakeInvalid => {
                self.handle_clean_request(req);
            }
            _ => {
                // Unknown request type
            }
        }
    }

    /// Handle a read request
    fn handle_read_request(&mut self, req: ChiRequest, want_unique: bool) {
        let txn_id = req.header.txn_id;
        let addr = req.header.addr;
        let src_id = NodeId(req.header.src_id);

        // Check directory
        let sharers = self.directory.get_sharers(addr);

        if sharers.is_empty() {
            // No sharers, go to memory
            self.stats.dir_misses += 1;
            self.request_from_memory(txn_id, addr, src_id, want_unique);
        } else if sharers.len() == 1 && sharers.contains(&src_id.0) {
            // Requester is the only sharer, upgrade state
            self.stats.dir_hits += 1;
            self.upgrade_state(txn_id, addr, src_id, want_unique);
        } else {
            // Other sharers exist, need to snoop
            self.stats.dir_hits += 1;
            self.send_snoops(txn_id, addr, src_id, want_unique, &sharers);
        }
    }

    /// Handle a request for unique ownership
    fn handle_unique_request(&mut self, req: ChiRequest) {
        let txn_id = req.header.txn_id;
        let addr = req.header.addr;
        let src_id = NodeId(req.header.src_id);

        // Get other sharers (exclude requester)
        let sharers = self.directory.get_snoop_targets(addr, Some(src_id.0));

        if sharers.is_empty() {
            // No other sharers, grant unique
            self.grant_unique(txn_id, addr, src_id);
        } else {
            // Need to invalidate other sharers
            self.send_invalidate_snoops(txn_id, addr, src_id, &sharers);
        }
    }

    /// Handle a write request
    fn handle_write_request(&mut self, req: ChiRequest) {
        let txn_id = req.header.txn_id;
        let addr = req.header.addr;
        let src_id = NodeId(req.header.src_id);

        // Allocate DBID
        let dbid = self.dbid_allocator.allocate();

        if let Some(dbid) = dbid {
            // Get other sharers
            let sharers = self.directory.get_snoop_targets(addr, Some(src_id.0));

            if sharers.is_empty() {
                // No other sharers, send DBIDResp
                self.send_dbid_response(txn_id, addr, src_id, dbid);
            } else {
                // Need to invalidate others first
                if let Some(txn) = self.transactions.get_mut(&txn_id) {
                    txn.dbid = Some(dbid);
                }
                self.send_invalidate_snoops(txn_id, addr, src_id, &sharers);
            }
        } else {
            // No DBID available, retry later
            // In a real implementation, would send retry response
        }
    }

    /// Handle an evict request
    fn handle_evict_request(&mut self, req: ChiRequest) {
        let addr = req.header.addr;
        let src_id = NodeId(req.header.src_id);

        // Update directory
        let is_dirty = self.directory.is_dirty(addr);
        self.directory.on_evict(addr, src_id.0, is_dirty);

        // Send acknowledgment
        let resp = ChiResponse::new(
            req.header.txn_id,
            ChiResponseType::Comp,
            self.node_id,
            src_id,
            addr,
        );
        let _ = self.rsp_channel.send(resp);

        // Complete transaction
        self.transactions.remove(&req.header.txn_id);
        self.directory.complete_transaction(addr);
        self.stats.transactions_completed += 1;
    }

    /// Handle clean/invalidate request
    fn handle_clean_request(&mut self, req: ChiRequest) {
        let txn_id = req.header.txn_id;
        let addr = req.header.addr;
        let src_id = NodeId(req.header.src_id);

        let sharers = self.directory.get_snoop_targets(addr, Some(src_id.0));

        if sharers.is_empty() {
            // No other sharers, respond immediately
            let resp = ChiResponse::new(
                txn_id,
                ChiResponseType::Comp,
                self.node_id,
                src_id,
                addr,
            );
            let _ = self.rsp_channel.send(resp);
            self.transactions.remove(&txn_id);
            self.directory.complete_transaction(addr);
            self.stats.transactions_completed += 1;
        } else {
            // Send clean snoop to others
            self.send_clean_snoops(txn_id, addr, src_id, &sharers, req.req_type);
        }
    }

    /// Request data from memory (SN-F)
    fn request_from_memory(&mut self, txn_id: ChiTxnId, addr: u64, src_id: NodeId, want_unique: bool) {
        if let Some(txn) = self.transactions.get_mut(&txn_id) {
            txn.state = HnfTxnState::MemoryPending;
        }

        // Create request to SN-F
        let mem_req = ChiRequest::new(
            txn_id,
            self.node_id,
            self.sn_node_id,
            ChiRequestType::ReadNoSnoop,
            addr,
            64,
            InstructionId(0),
        );

        self.sn_req_channel.send(mem_req);
        self.stats.memory_requests += 1;
    }

    /// Send snoops to other sharers
    fn send_snoops(
        &mut self,
        txn_id: ChiTxnId,
        addr: u64,
        src_id: NodeId,
        want_unique: bool,
        sharers: &[u8],
    ) {
        let snoop_type = if want_unique {
            ChiSnoopType::SnpClean
        } else {
            ChiSnoopType::SnpShared
        };

        let pending = PendingSnoop {
            txn_id,
            request: self.transactions.get(&txn_id).unwrap().request.clone(),
            responses_expected: sharers.len() as u32,
            responses_received: 0,
            data_received: false,
            snoop_data: None,
            snoop_issue_cycle: self.current_cycle,
        };

        if let Some(txn) = self.transactions.get_mut(&txn_id) {
            txn.state = HnfTxnState::SnoopPending;
            txn.pending_snoop = Some(pending);
        }

        // Send snoop to each sharer
        for &sharer_id in sharers {
            let snoop = ChiSnoop::new(
                snoop_type,
                txn_id,
                self.node_id,
                NodeId(sharer_id),
                addr,
            );
            self.snp_channel.send(snoop);
            self.stats.snoops_sent += 1;
        }
    }

    /// Send invalidate snoops
    fn send_invalidate_snoops(
        &mut self,
        txn_id: ChiTxnId,
        addr: u64,
        src_id: NodeId,
        sharers: &[u8],
    ) {
        let pending = PendingSnoop {
            txn_id,
            request: self.transactions.get(&txn_id).unwrap().request.clone(),
            responses_expected: sharers.len() as u32,
            responses_received: 0,
            data_received: false,
            snoop_data: None,
            snoop_issue_cycle: self.current_cycle,
        };

        if let Some(txn) = self.transactions.get_mut(&txn_id) {
            txn.state = HnfTxnState::SnoopPending;
            txn.pending_snoop = Some(pending);
        }

        for &sharer_id in sharers {
            let snoop = ChiSnoop::new(
                ChiSnoopType::SnpMakeInvalid,
                txn_id,
                self.node_id,
                NodeId(sharer_id),
                addr,
            );
            self.snp_channel.send(snoop);
            self.stats.snoops_sent += 1;
        }
    }

    /// Send clean snoops
    fn send_clean_snoops(
        &mut self,
        txn_id: ChiTxnId,
        addr: u64,
        src_id: NodeId,
        sharers: &[u8],
        req_type: ChiRequestType,
    ) {
        let snoop_type = match req_type {
            ChiRequestType::CleanInvalid | ChiRequestType::MakeInvalid => ChiSnoopType::SnpMakeInvalid,
            _ => ChiSnoopType::SnpCleanShared,
        };

        let pending = PendingSnoop {
            txn_id,
            request: self.transactions.get(&txn_id).unwrap().request.clone(),
            responses_expected: sharers.len() as u32,
            responses_received: 0,
            data_received: false,
            snoop_data: None,
            snoop_issue_cycle: self.current_cycle,
        };

        if let Some(txn) = self.transactions.get_mut(&txn_id) {
            txn.state = HnfTxnState::SnoopPending;
            txn.pending_snoop = Some(pending);
        }

        for &sharer_id in sharers {
            let snoop = ChiSnoop::new(snoop_type, txn_id, self.node_id, NodeId(sharer_id), addr);
            self.snp_channel.send(snoop);
            self.stats.snoops_sent += 1;
        }
    }

    /// Grant unique ownership
    fn grant_unique(&mut self, txn_id: ChiTxnId, addr: u64, src_id: NodeId) {
        self.directory.set_owner(addr, src_id.0);

        let data = DataDescriptor::new(64).with_state(ChiCacheState::UniqueClean);
        let data_msg = ChiData::comp_data(txn_id, self.node_id, src_id, addr, data);
        self.dat_channel.send(data_msg);

        self.directory.complete_transaction(addr);
        self.transactions.remove(&txn_id);
        self.stats.transactions_completed += 1;
    }

    /// Upgrade state without snoop
    fn upgrade_state(&mut self, txn_id: ChiTxnId, addr: u64, src_id: NodeId, want_unique: bool) {
        let state = if want_unique {
            ChiCacheState::UniqueClean
        } else {
            ChiCacheState::SharedClean
        };

        let data = DataDescriptor::new(64).with_state(state);
        let data_msg = ChiData::comp_data(txn_id, self.node_id, src_id, addr, data);
        self.dat_channel.send(data_msg);

        self.directory.complete_transaction(addr);
        self.transactions.remove(&txn_id);
        self.stats.transactions_completed += 1;
    }

    /// Send DBID response
    fn send_dbid_response(&mut self, txn_id: ChiTxnId, addr: u64, src_id: NodeId, dbid: u16) {
        self.directory.set_owner(addr, src_id.0);
        self.directory.set_dirty(addr, true);

        let resp = ChiResponse::dbid_response(txn_id, self.node_id, src_id, dbid);
        self.rsp_channel.send(resp);

        if let Some(txn) = self.transactions.get_mut(&txn_id) {
            txn.state = HnfTxnState::WaitingCompAck;
        }
    }

    /// Process incoming snoop responses
    pub fn process_snoop_responses(&mut self) {
        while let Some(resp) = self.snp_resp_channel.recv() {
            self.handle_snoop_response(resp);
        }
    }

    /// Handle a snoop response
    fn handle_snoop_response(&mut self, resp: ChiSnoopResp) {
        self.stats.snoop_responses += 1;

        if let Some(txn) = self.transactions.get_mut(&resp.txn_id) {
            if let Some(ref mut pending) = txn.pending_snoop {
                pending.responses_received += 1;

                // Save data if provided
                if resp.data_valid {
                    pending.data_received = true;
                    pending.snoop_data = resp.data.clone();
                }

                // Update directory
                self.directory.remove_sharer(resp.addr, resp.src_id.0);

                // Check if all responses received
                if pending.responses_received >= pending.responses_expected {
                    self.complete_snoop_phase(resp.txn_id, resp.addr);
                }
            }
        }
    }

    /// Complete snoop phase
    fn complete_snoop_phase(&mut self, txn_id: ChiTxnId, addr: u64) {
        if let Some(txn) = self.transactions.get(&txn_id) {
            let src_id = txn.src_node;
            let snoop_data = txn.pending_snoop.as_ref().and_then(|p| p.snoop_data.clone());
            let dbid = txn.dbid;

            if let Some(data) = snoop_data {
                // Got data from snoop, forward to requester
                let data_msg = ChiData::comp_data(txn_id, self.node_id, src_id, addr, data);
                self.dat_channel.send(data_msg);

                // Add requester as sharer
                self.directory.add_sharer(addr, src_id.0);
                self.directory.complete_transaction(addr);

                self.transactions.remove(&txn_id);
                self.stats.transactions_completed += 1;
            } else if dbid.is_some() {
                // Write transaction, send DBID response
                self.send_dbid_response(txn_id, addr, src_id, dbid.unwrap());
            } else {
                // Need to get data from memory
                let want_unique = matches!(
                    txn.request.req_type,
                    ChiRequestType::ReadMakeUnique | ChiRequestType::MakeUnique
                );
                self.request_from_memory(txn_id, addr, src_id, want_unique);
            }
        }
    }

    /// Process memory responses
    pub fn process_memory_responses(&mut self) {
        while let Some(data) = self.sn_dat_channel.recv() {
            self.handle_memory_data(data);
        }
    }

    /// Handle data from memory
    fn handle_memory_data(&mut self, data: ChiData) {
        if let Some(txn) = self.transactions.get(&data.txn_id) {
            let src_id = txn.src_node;
            let addr = data.addr;

            // Forward data to requester
            let want_unique = matches!(
                txn.request.req_type,
                ChiRequestType::ReadMakeUnique | ChiRequestType::ReadOnce
            );

            let state = if want_unique {
                ChiCacheState::UniqueClean
            } else {
                ChiCacheState::SharedClean
            };

            let resp_data = DataDescriptor::new(64).with_state(state);
            let data_msg = ChiData::comp_data(data.txn_id, self.node_id, src_id, addr, resp_data);
            self.dat_channel.send(data_msg);

            // Update directory
            self.directory.add_sharer(addr, src_id.0);
            self.directory.complete_transaction(addr);

            self.transactions.remove(&data.txn_id);
            self.stats.transactions_completed += 1;
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
    pub fn stats(&self) -> &HnFStats {
        &self.stats
    }

    /// Get directory statistics
    pub fn dir_stats(&self) -> &super::directory::DirectoryStats {
        self.directory.stats()
    }

    /// Check if there are pending transactions
    pub fn has_pending_transactions(&self) -> bool {
        !self.transactions.is_empty()
    }

    /// Get number of pending transactions
    pub fn pending_count(&self) -> usize {
        self.transactions.len()
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

    pub fn snp_channel(&self) -> &Channel<ChiSnoop> {
        &self.snp_channel
    }

    pub fn snp_channel_mut(&mut self) -> &mut Channel<ChiSnoop> {
        &mut self.snp_channel
    }

    pub fn snp_resp_channel_mut(&mut self) -> &mut Channel<ChiSnoopResp> {
        &mut self.snp_resp_channel
    }

    pub fn sn_req_channel(&self) -> &Channel<ChiRequest> {
        &self.sn_req_channel
    }

    pub fn sn_req_channel_mut(&mut self) -> &mut Channel<ChiRequest> {
        &mut self.sn_req_channel
    }

    pub fn sn_dat_channel_mut(&mut self) -> &mut Channel<ChiData> {
        &mut self.sn_dat_channel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_hnf() -> HnFNode {
        HnFNode::new(
            ChiNodeConfig { node_id: 1, ..Default::default() },
            4096,
            64,
            NodeId(2),
            100,
        )
    }

    #[test]
    fn test_hnf_creation() {
        let hnf = create_test_hnf();
        assert_eq!(hnf.node_id, NodeId(1));
        assert!(!hnf.has_pending_transactions());
    }

    #[test]
    fn test_hnf_read_request() {
        let mut hnf = create_test_hnf();

        let req = ChiRequest::new(
            ChiTxnId::new(1),
            NodeId(0),
            NodeId(1),
            ChiRequestType::ReadShared,
            0x1000,
            64,
            InstructionId(0),
        );

        hnf.req_channel_mut().send(req);
        hnf.process_requests();

        assert!(hnf.has_pending_transactions());
        assert_eq!(hnf.stats().requests_received, 1);
    }
}
