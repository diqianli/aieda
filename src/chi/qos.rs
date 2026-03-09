//! QoS and Retry mechanism for CHI protocol.

use std::collections::VecDeque;
use serde::{Deserialize, Serialize};

use super::protocol::{ChiRequestType, ChiTxnId};
use crate::types::InstructionId;

/// PCrd (Protocol Credit) for flow control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcrdType {
    /// Type 0 credit
    Type0,
    /// Type 1 credit
    Type1,
    /// Type 2 credit
    Type2,
}

/// PCrd grant or deny response
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PcrdResponse {
    /// Credit granted
    Grant(PcrdType),
    /// Credit denied, retry later
    Deny,
}

/// QoS credit manager
pub struct QosCreditManager {
    /// Available PCrd credits per type
    pcrd_available: [u16; 3],
    /// Maximum PCrd credits per type
    pcrd_max: [u16; 3],
    /// Total PCrd credits in use
    pcrd_in_use: u16,
    /// DBID allocator
    dbid_allocator: DbidAllocator,
    /// Retry queue for denied requests
    retry_queue: VecDeque<PendingRequest>,
    /// Maximum retry queue size
    max_retry_queue_size: usize,
    /// Statistics
    stats: QosStats,
}

/// Pending request waiting for retry
#[derive(Debug, Clone)]
pub struct PendingRequest {
    /// Instruction ID
    pub instruction_id: InstructionId,
    /// Transaction ID (if assigned)
    pub txn_id: Option<ChiTxnId>,
    /// Request type
    pub request_type: ChiRequestType,
    /// Address
    pub addr: u64,
    /// Size
    pub size: u8,
    /// Cycle when request was first made
    pub first_attempt_cycle: u64,
    /// Number of retry attempts
    pub retry_count: u32,
    /// Required PCrd type
    pub required_pcrd: PcrdType,
}

/// DBID (Data Buffer ID) allocator
#[derive(Debug)]
pub struct DbidAllocator {
    /// Next DBID to allocate
    next_dbid: u16,
    /// Maximum DBID
    max_dbid: u16,
    /// Free list of returned DBIDs
    free_list: VecDeque<u16>,
    /// Number of DBIDs currently in use
    in_use: u16,
}

/// QoS statistics
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct QosStats {
    /// Total requests processed
    pub total_requests: u64,
    /// Requests granted immediately
    pub immediate_grants: u64,
    /// Requests denied and queued
    pub denials: u64,
    /// Successful retries
    pub successful_retries: u64,
    /// DBIDs allocated
    pub dbids_allocated: u64,
    /// DBIDs freed
    pub dbids_freed: u64,
    /// Peak retry queue size
    pub peak_retry_queue_size: usize,
}

impl DbidAllocator {
    /// Create a new DBID allocator
    pub fn new(max_dbid: u16) -> Self {
        Self {
            next_dbid: 0,
            max_dbid,
            free_list: VecDeque::with_capacity(max_dbid as usize),
            in_use: 0,
        }
    }

    /// Allocate a DBID
    pub fn allocate(&mut self) -> Option<u16> {
        // Try free list first
        if let Some(dbid) = self.free_list.pop_front() {
            self.in_use += 1;
            return Some(dbid);
        }

        // Allocate new DBID
        if self.next_dbid < self.max_dbid {
            let dbid = self.next_dbid;
            self.next_dbid += 1;
            self.in_use += 1;
            Some(dbid)
        } else {
            None
        }
    }

    /// Free a DBID
    pub fn free(&mut self, dbid: u16) {
        if dbid < self.max_dbid {
            self.free_list.push_back(dbid);
            self.in_use = self.in_use.saturating_sub(1);
        }
    }

    /// Get number of available DBIDs
    pub fn available(&self) -> u16 {
        self.max_dbid - self.in_use
    }

    /// Get number of DBIDs in use
    pub fn in_use(&self) -> u16 {
        self.in_use
    }

    /// Reset allocator
    pub fn reset(&mut self) {
        self.next_dbid = 0;
        self.free_list.clear();
        self.in_use = 0;
    }
}

impl QosCreditManager {
    /// Create a new QoS credit manager
    pub fn new(max_pcrd_credits: u16, max_dbid: u16, max_retry_queue_size: usize) -> Self {
        Self {
            pcrd_available: [max_pcrd_credits; 3],
            pcrd_max: [max_pcrd_credits; 3],
            pcrd_in_use: 0,
            dbid_allocator: DbidAllocator::new(max_dbid),
            retry_queue: VecDeque::with_capacity(max_retry_queue_size),
            max_retry_queue_size,
            stats: QosStats::default(),
        }
    }

    /// Get required PCrd type for a request
    pub fn get_pcrd_type(request_type: ChiRequestType) -> PcrdType {
        match request_type {
            // Read requests typically use Type0
            ChiRequestType::ReadNoSnoop
            | ChiRequestType::ReadNotSharedDirty
            | ChiRequestType::ReadShared
            | ChiRequestType::ReadMakeUnique
            | ChiRequestType::ReadOnce
            | ChiRequestType::ReadOnceCleanInvalid
            | ChiRequestType::ReadOnceMakeInvalid => PcrdType::Type0,

            // Write requests typically use Type1
            ChiRequestType::WriteNoSnoop
            | ChiRequestType::WriteUnique
            | ChiRequestType::WriteUniquePtl
            | ChiRequestType::WriteUniqueFull
            | ChiRequestType::WriteEvictFull
            | ChiRequestType::WriteEvictPtl => PcrdType::Type1,

            // Coherence requests typically use Type2
            ChiRequestType::CleanUnique
            | ChiRequestType::MakeUnique
            | ChiRequestType::Evict
            | ChiRequestType::CleanShared
            | ChiRequestType::CleanInvalid
            | ChiRequestType::MakeInvalid
            | ChiRequestType::DVMOp
            | ChiRequestType::PCrdReturn => PcrdType::Type2,
        }
    }

    /// Check if credits are available for a request
    pub fn has_credit(&self, pcrd_type: PcrdType) -> bool {
        self.pcrd_available[pcrd_type as usize] > 0
    }

    /// Request a credit
    pub fn request_credit(&mut self, pcrd_type: PcrdType) -> PcrdResponse {
        let idx = pcrd_type as usize;
        if self.pcrd_available[idx] > 0 {
            self.pcrd_available[idx] -= 1;
            self.pcrd_in_use += 1;
            PcrdResponse::Grant(pcrd_type)
        } else {
            PcrdResponse::Deny
        }
    }

    /// Return a credit
    pub fn return_credit(&mut self, pcrd_type: PcrdType) {
        let idx = pcrd_type as usize;
        if self.pcrd_available[idx] < self.pcrd_max[idx] {
            self.pcrd_available[idx] += 1;
            self.pcrd_in_use = self.pcrd_in_use.saturating_sub(1);
        }
    }

    /// Allocate a DBID
    pub fn allocate_dbid(&mut self) -> Option<u16> {
        let dbid = self.dbid_allocator.allocate();
        if dbid.is_some() {
            self.stats.dbids_allocated += 1;
        }
        dbid
    }

    /// Free a DBID
    pub fn free_dbid(&mut self, dbid: u16) {
        self.dbid_allocator.free(dbid);
        self.stats.dbids_freed += 1;
    }

    /// Try to process a request, queueing if denied
    pub fn process_request(
        &mut self,
        instruction_id: InstructionId,
        request_type: ChiRequestType,
        addr: u64,
        size: u8,
        current_cycle: u64,
    ) -> Result<ChiTxnId, PendingRequest> {
        self.stats.total_requests += 1;

        let pcrd_type = Self::get_pcrd_type(request_type);

        if let PcrdResponse::Grant(_) = self.request_credit(pcrd_type) {
            self.stats.immediate_grants += 1;
            // Return a placeholder transaction ID
            // In a real implementation, this would be assigned by the interface
            Ok(ChiTxnId::new(0))
        } else {
            self.stats.denials += 1;

            let pending = PendingRequest {
                instruction_id,
                txn_id: None,
                request_type,
                addr,
                size,
                first_attempt_cycle: current_cycle,
                retry_count: 0,
                required_pcrd: pcrd_type,
            };

            if self.retry_queue.len() < self.max_retry_queue_size {
                self.retry_queue.push_back(pending.clone());
                if self.retry_queue.len() > self.stats.peak_retry_queue_size {
                    self.stats.peak_retry_queue_size = self.retry_queue.len();
                }
            }

            Err(pending)
        }
    }

    /// Process retry queue, attempting to grant credits
    pub fn process_retries(&mut self, current_cycle: u64) -> Vec<PendingRequest> {
        let mut granted = Vec::new();
        let mut remaining = VecDeque::with_capacity(self.retry_queue.len());

        while let Some(mut pending) = self.retry_queue.pop_front() {
            if let PcrdResponse::Grant(_) = self.request_credit(pending.required_pcrd) {
                pending.retry_count += 1;
                self.stats.successful_retries += 1;
                granted.push(pending);
            } else {
                remaining.push_back(pending);
            }
        }

        self.retry_queue = remaining;
        granted
    }

    /// Get retry queue size
    pub fn retry_queue_size(&self) -> usize {
        self.retry_queue.len()
    }

    /// Get available credits for a type
    pub fn available_credits(&self, pcrd_type: PcrdType) -> u16 {
        self.pcrd_available[pcrd_type as usize]
    }

    /// Get total credits in use
    pub fn credits_in_use(&self) -> u16 {
        self.pcrd_in_use
    }

    /// Get available DBIDs
    pub fn available_dbids(&self) -> u16 {
        self.dbid_allocator.available()
    }

    /// Get statistics
    pub fn stats(&self) -> &QosStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = QosStats::default();
    }

    /// Reset all state
    pub fn reset(&mut self) {
        self.pcrd_available = self.pcrd_max;
        self.pcrd_in_use = 0;
        self.dbid_allocator.reset();
        self.retry_queue.clear();
        self.stats = QosStats::default();
    }
}

/// Channel credit tracker for flow control
#[derive(Debug)]
pub struct ChannelCredits {
    /// Available credits
    available: u16,
    /// Maximum credits
    max_credits: u16,
    /// Channel name (for debugging)
    name: &'static str,
}

impl ChannelCredits {
    /// Create a new channel credit tracker
    pub fn new(max_credits: u16, name: &'static str) -> Self {
        Self {
            available: max_credits,
            max_credits,
            name,
        }
    }

    /// Check if credit is available
    pub fn has_credit(&self) -> bool {
        self.available > 0
    }

    /// Use a credit
    pub fn use_credit(&mut self) -> bool {
        if self.available > 0 {
            self.available -= 1;
            true
        } else {
            false
        }
    }

    /// Return a credit
    pub fn return_credit(&mut self) {
        if self.available < self.max_credits {
            self.available += 1;
        }
    }

    /// Get available credits
    pub fn available(&self) -> u16 {
        self.available
    }

    /// Get credits in use
    pub fn in_use(&self) -> u16 {
        self.max_credits - self.available
    }

    /// Reset credits
    pub fn reset(&mut self) {
        self.available = self.max_credits;
    }
}

/// All channel credits for a node
#[derive(Debug)]
pub struct NodeChannelCredits {
    /// Request channel credits (REQ)
    pub req: ChannelCredits,
    /// Response channel credits (RSP)
    pub rsp: ChannelCredits,
    /// Data channel credits (DAT)
    pub dat: ChannelCredits,
    /// Snoop channel credits (SNP)
    pub snp: ChannelCredits,
}

impl NodeChannelCredits {
    /// Create channel credits with default values
    pub fn new(req_credits: u16, rsp_credits: u16, dat_credits: u16, snp_credits: u16) -> Self {
        Self {
            req: ChannelCredits::new(req_credits, "REQ"),
            rsp: ChannelCredits::new(rsp_credits, "RSP"),
            dat: ChannelCredits::new(dat_credits, "DAT"),
            snp: ChannelCredits::new(snp_credits, "SNP"),
        }
    }

    /// Create with typical default values
    pub fn typical() -> Self {
        Self::new(16, 16, 8, 8)
    }

    /// Reset all credits
    pub fn reset(&mut self) {
        self.req.reset();
        self.rsp.reset();
        self.dat.reset();
        self.snp.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dbid_allocator() {
        let mut allocator = DbidAllocator::new(16);

        // Allocate DBIDs
        let dbid1 = allocator.allocate();
        let dbid2 = allocator.allocate();
        assert!(dbid1.is_some());
        assert!(dbid2.is_some());
        assert_eq!(allocator.in_use(), 2);

        // Free a DBID
        allocator.free(dbid1.unwrap());
        assert_eq!(allocator.in_use(), 1);

        // Re-allocation should use free list
        let dbid3 = allocator.allocate();
        assert_eq!(dbid3, dbid1);
    }

    #[test]
    fn test_dbid_allocator_exhaustion() {
        let mut allocator = DbidAllocator::new(2);

        assert!(allocator.allocate().is_some());
        assert!(allocator.allocate().is_some());
        assert!(allocator.allocate().is_none()); // Exhausted

        allocator.free(0);
        assert!(allocator.allocate().is_some()); // Should work now
    }

    #[test]
    fn test_qos_credit_manager() {
        let mut manager = QosCreditManager::new(4, 16, 10);

        // Check initial state
        assert!(manager.has_credit(PcrdType::Type0));
        assert_eq!(manager.available_credits(PcrdType::Type0), 4);

        // Request credit
        let response = manager.request_credit(PcrdType::Type0);
        assert!(matches!(response, PcrdResponse::Grant(_)));
        assert_eq!(manager.available_credits(PcrdType::Type0), 3);

        // Return credit
        manager.return_credit(PcrdType::Type0);
        assert_eq!(manager.available_credits(PcrdType::Type0), 4);
    }

    #[test]
    fn test_qos_credit_exhaustion() {
        let mut manager = QosCreditManager::new(2, 16, 10);

        // Use all credits
        manager.request_credit(PcrdType::Type0);
        manager.request_credit(PcrdType::Type0);

        // Should be denied
        let response = manager.request_credit(PcrdType::Type0);
        assert!(matches!(response, PcrdResponse::Deny));

        // Return credit
        manager.return_credit(PcrdType::Type0);
        let response = manager.request_credit(PcrdType::Type0);
        assert!(matches!(response, PcrdResponse::Grant(_)));
    }

    #[test]
    fn test_retry_queue() {
        let mut manager = QosCreditManager::new(1, 16, 10);

        // Use the only credit
        manager.request_credit(PcrdType::Type0);

        // Next request should be queued
        let result = manager.process_request(
            InstructionId(1),
            ChiRequestType::ReadShared,
            0x1000,
            8,
            0,
        );

        assert!(result.is_err());
        assert_eq!(manager.retry_queue_size(), 1);

        // Return the credit
        manager.return_credit(PcrdType::Type0);

        // Process retries
        let granted = manager.process_retries(10);
        assert_eq!(granted.len(), 1);
        assert_eq!(manager.retry_queue_size(), 0);
    }

    #[test]
    fn test_channel_credits() {
        let mut credits = ChannelCredits::new(4, "TEST");

        assert!(credits.has_credit());
        assert_eq!(credits.available(), 4);

        credits.use_credit();
        credits.use_credit();
        assert_eq!(credits.in_use(), 2);

        credits.return_credit();
        assert_eq!(credits.available(), 3);
    }

    #[test]
    fn test_pcrd_type_selection() {
        assert_eq!(
            QosCreditManager::get_pcrd_type(ChiRequestType::ReadShared),
            PcrdType::Type0
        );
        assert_eq!(
            QosCreditManager::get_pcrd_type(ChiRequestType::WriteUnique),
            PcrdType::Type1
        );
        assert_eq!(
            QosCreditManager::get_pcrd_type(ChiRequestType::MakeUnique),
            PcrdType::Type2
        );
    }
}
