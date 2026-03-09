//! CHI Node base structures and types.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use super::protocol::{
    ChiRequestType, ChiResponseType, ChiSnoopType, ChiTxnId, ChiOrder,
    ChiMessageHeader, ChiOpcode,
};
use super::coherence::ChiCacheState;
use crate::types::InstructionId;

/// CHI node type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ChiNodeType {
    /// Request Node - Fully coherent (e.g., CPU core with L1/L2 cache)
    RnF,
    /// Home Node - Protocol controller and directory
    HnF,
    /// Subordinate Node - Memory controller or downstream device
    SnF,
}

impl Default for ChiNodeType {
    fn default() -> Self {
        Self::RnF
    }
}

/// CHI node identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(pub u8);

impl Default for NodeId {
    fn default() -> Self {
        Self(0)
    }
}

/// CHI request message
#[derive(Debug, Clone)]
pub struct ChiRequest {
    /// Message header
    pub header: ChiMessageHeader,
    /// Request type
    pub req_type: ChiRequestType,
    /// Instruction ID (for tracking)
    pub instr_id: InstructionId,
    /// Whether to expect data response
    pub expect_data: bool,
    /// DBID for write operations
    pub dbid: Option<u16>,
}

impl ChiRequest {
    /// Create a new request
    pub fn new(
        txn_id: ChiTxnId,
        src_id: NodeId,
        dest_id: NodeId,
        req_type: ChiRequestType,
        addr: u64,
        size: u8,
        instr_id: InstructionId,
    ) -> Self {
        Self {
            header: ChiMessageHeader {
                opcode: ChiOpcode::Request(req_type),
                txn_id,
                src_id: src_id.0,
                dest_id: dest_id.0,
                addr,
                size,
                allow_retry: true,
                order: ChiOrder::None,
            },
            req_type,
            instr_id,
            expect_data: req_type.requires_data(),
            dbid: None,
        }
    }
}

/// CHI response message
#[derive(Debug, Clone)]
pub struct ChiResponse {
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Response type
    pub resp_type: ChiResponseType,
    /// Source node ID
    pub src_id: NodeId,
    /// Destination node ID
    pub dest_id: NodeId,
    /// Address (for directory updates)
    pub addr: u64,
    /// DBID (for write responses)
    pub dbid: Option<u16>,
    /// Acknowledge (CompAck)
    pub is_ack: bool,
}

impl ChiResponse {
    /// Create a new response
    pub fn new(
        txn_id: ChiTxnId,
        resp_type: ChiResponseType,
        src_id: NodeId,
        dest_id: NodeId,
        addr: u64,
    ) -> Self {
        Self {
            txn_id,
            resp_type,
            src_id,
            dest_id,
            addr,
            dbid: None,
            is_ack: false,
        }
    }

    /// Create a DBID response
    pub fn dbid_response(txn_id: ChiTxnId, src_id: NodeId, dest_id: NodeId, dbid: u16) -> Self {
        Self {
            txn_id,
            resp_type: ChiResponseType::DBIDResp,
            src_id,
            dest_id,
            addr: 0,
            dbid: Some(dbid),
            is_ack: false,
        }
    }

    /// Create a CompAck
    pub fn comp_ack(txn_id: ChiTxnId, src_id: NodeId, dest_id: NodeId, addr: u64) -> Self {
        Self {
            txn_id,
            resp_type: ChiResponseType::CompAck,
            src_id,
            dest_id,
            addr,
            dbid: None,
            is_ack: true,
        }
    }
}

/// Data descriptor (abstract data representation)
#[derive(Debug, Clone)]
pub struct DataDescriptor {
    /// Data size in bytes
    pub size: u8,
    /// Whether data is valid
    pub valid: bool,
    /// Whether data is dirty
    pub dirty: bool,
    /// Cache state for the data
    pub cache_state: ChiCacheState,
    /// Error indication
    pub error: bool,
}

impl DataDescriptor {
    /// Create a new data descriptor
    pub fn new(size: u8) -> Self {
        Self {
            size,
            valid: true,
            dirty: false,
            cache_state: ChiCacheState::SharedClean,
            error: false,
        }
    }

    /// Create an invalid/empty descriptor
    pub fn empty() -> Self {
        Self {
            size: 0,
            valid: false,
            dirty: false,
            cache_state: ChiCacheState::Invalid,
            error: false,
        }
    }

    /// Create an error descriptor
    pub fn error() -> Self {
        Self {
            size: 0,
            valid: false,
            dirty: false,
            cache_state: ChiCacheState::Invalid,
            error: true,
        }
    }

    /// Set dirty flag
    pub fn with_dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }

    /// Set cache state
    pub fn with_state(mut self, state: ChiCacheState) -> Self {
        self.cache_state = state;
        self
    }
}

/// CHI data message
#[derive(Debug, Clone)]
pub struct ChiData {
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Response type
    pub resp_type: ChiResponseType,
    /// Source node ID
    pub src_id: NodeId,
    /// Destination node ID
    pub dest_id: NodeId,
    /// Address
    pub addr: u64,
    /// Data descriptor
    pub data: DataDescriptor,
}

impl ChiData {
    /// Create a new data message
    pub fn new(
        txn_id: ChiTxnId,
        resp_type: ChiResponseType,
        src_id: NodeId,
        dest_id: NodeId,
        addr: u64,
        data: DataDescriptor,
    ) -> Self {
        Self {
            txn_id,
            resp_type,
            src_id,
            dest_id,
            addr,
            data,
        }
    }

    /// Create CompData response
    pub fn comp_data(
        txn_id: ChiTxnId,
        src_id: NodeId,
        dest_id: NodeId,
        addr: u64,
        data: DataDescriptor,
    ) -> Self {
        Self::new(txn_id, ChiResponseType::CompData, src_id, dest_id, addr, data)
    }
}

/// CHI snoop message
#[derive(Debug, Clone)]
pub struct ChiSnoop {
    /// Snoop type
    pub snoop_type: ChiSnoopType,
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Source node ID (HN-F)
    pub src_id: NodeId,
    /// Destination node ID (RN-F)
    pub dest_id: NodeId,
    /// Address being snooped
    pub addr: u64,
    /// Whether data is requested
    pub data_requested: bool,
}

impl ChiSnoop {
    /// Create a new snoop message
    pub fn new(
        snoop_type: ChiSnoopType,
        txn_id: ChiTxnId,
        src_id: NodeId,
        dest_id: NodeId,
        addr: u64,
    ) -> Self {
        Self {
            snoop_type,
            txn_id,
            src_id,
            dest_id,
            addr,
            data_requested: snoop_type.requires_data(),
        }
    }
}

/// CHI snoop response
#[derive(Debug, Clone)]
pub struct ChiSnoopResp {
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Source node ID (RN-F)
    pub src_id: NodeId,
    /// Destination node ID (HN-F)
    pub dest_id: NodeId,
    /// Address
    pub addr: u64,
    /// Whether data is being returned
    pub data_valid: bool,
    /// Data descriptor (if data is returned)
    pub data: Option<DataDescriptor>,
    /// Cache state after snoop
    pub state: ChiCacheState,
}

impl ChiSnoopResp {
    /// Create a snoop response without data
    pub fn ack(txn_id: ChiTxnId, src_id: NodeId, dest_id: NodeId, addr: u64, state: ChiCacheState) -> Self {
        Self {
            txn_id,
            src_id,
            dest_id,
            addr,
            data_valid: false,
            data: None,
            state,
        }
    }

    /// Create a snoop response with data
    pub fn with_data(
        txn_id: ChiTxnId,
        src_id: NodeId,
        dest_id: NodeId,
        addr: u64,
        data: DataDescriptor,
        state: ChiCacheState,
    ) -> Self {
        Self {
            txn_id,
            src_id,
            dest_id,
            addr,
            data_valid: true,
            data: Some(data),
            state,
        }
    }
}

/// Generic channel for CHI messages
#[derive(Debug)]
pub struct Channel<T> {
    /// Channel buffer
    buffer: VecDeque<T>,
    /// Maximum capacity
    capacity: usize,
    /// Channel name (for debugging)
    name: &'static str,
    /// Statistics
    stats: ChannelStats,
}

/// Channel statistics
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct ChannelStats {
    /// Total messages sent
    pub sent: u64,
    /// Total messages received
    pub received: u64,
    /// Messages dropped due to full buffer
    pub dropped: u64,
    /// Peak occupancy
    pub peak_occupancy: usize,
}

impl<T> Channel<T> {
    /// Create a new channel
    pub fn new(capacity: usize, name: &'static str) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
            name,
            stats: ChannelStats::default(),
        }
    }

    /// Send a message
    pub fn send(&mut self, msg: T) -> bool {
        if self.buffer.len() >= self.capacity {
            self.stats.dropped += 1;
            return false;
        }

        self.buffer.push_back(msg);
        self.stats.sent += 1;

        if self.buffer.len() > self.stats.peak_occupancy {
            self.stats.peak_occupancy = self.buffer.len();
        }

        true
    }

    /// Receive a message
    pub fn recv(&mut self) -> Option<T> {
        if let Some(msg) = self.buffer.pop_front() {
            self.stats.received += 1;
            Some(msg)
        } else {
            None
        }
    }

    /// Peek at the front message
    pub fn peek(&self) -> Option<&T> {
        self.buffer.front()
    }

    /// Check if channel is empty
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    /// Check if channel is full
    pub fn is_full(&self) -> bool {
        self.buffer.len() >= self.capacity
    }

    /// Get current occupancy
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get statistics
    pub fn stats(&self) -> &ChannelStats {
        &self.stats
    }

    /// Clear the channel
    pub fn clear(&mut self) {
        self.buffer.clear();
    }
}

/// Node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChiNodeConfig {
    /// Node ID
    pub node_id: u8,
    /// Node type
    pub node_type: ChiNodeType,
    /// REQ channel capacity
    pub req_channel_capacity: usize,
    /// RSP channel capacity
    pub rsp_channel_capacity: usize,
    /// DAT channel capacity
    pub dat_channel_capacity: usize,
    /// SNP channel capacity
    pub snp_channel_capacity: usize,
}

impl Default for ChiNodeConfig {
    fn default() -> Self {
        Self {
            node_id: 0,
            node_type: ChiNodeType::RnF,
            req_channel_capacity: 16,
            rsp_channel_capacity: 16,
            dat_channel_capacity: 8,
            snp_channel_capacity: 8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_basic() {
        let mut channel: Channel<i32> = Channel::new(4, "TEST");

        assert!(channel.is_empty());
        assert!(channel.send(1));
        assert!(channel.send(2));
        assert_eq!(channel.len(), 2);

        assert_eq!(channel.recv(), Some(1));
        assert_eq!(channel.recv(), Some(2));
        assert!(channel.is_empty());
    }

    #[test]
    fn test_channel_full() {
        let mut channel: Channel<i32> = Channel::new(2, "TEST");

        assert!(channel.send(1));
        assert!(channel.send(2));
        assert!(!channel.send(3)); // Should fail, channel full

        let stats = channel.stats();
        assert_eq!(stats.dropped, 1);
    }

    #[test]
    fn test_data_descriptor() {
        let desc = DataDescriptor::new(64)
            .with_dirty(true)
            .with_state(ChiCacheState::UniqueDirty);

        assert_eq!(desc.size, 64);
        assert!(desc.valid);
        assert!(desc.dirty);
        assert_eq!(desc.cache_state, ChiCacheState::UniqueDirty);
    }

    #[test]
    fn test_chi_request() {
        let req = ChiRequest::new(
            ChiTxnId::new(1),
            NodeId(0),
            NodeId(1),
            ChiRequestType::ReadShared,
            0x1000,
            64,
            InstructionId(0),
        );

        assert_eq!(req.req_type, ChiRequestType::ReadShared);
        assert!(req.expect_data);
    }

    #[test]
    fn test_chi_snoop() {
        let snoop = ChiSnoop::new(
            ChiSnoopType::SnpShared,
            ChiTxnId::new(1),
            NodeId(1),
            NodeId(0),
            0x1000,
        );

        assert_eq!(snoop.snoop_type, ChiSnoopType::SnpShared);
    }
}
