//! CHI (Coherent Hub Interface) Issue B protocol implementation.
//!
//! This module implements the CHI Issue B protocol for ARM CPU cache coherence.
//! It includes:
//! - Protocol definitions (requests, responses, snoop messages)
//! - Coherence state machine (extended MOESI states)
//! - Directory-based snoop filter
//! - QoS and retry mechanism
//! - Node implementations (RN-F, HN-F, SN-F)
//! - Interconnect for node communication

mod protocol;
mod interface;
mod timing;
mod coherence;
mod directory;
mod qos;
mod node;
mod rn_f;
mod hn_f;
mod sn_f;
mod interconnect;

// Re-export protocol types
pub use protocol::{
    ChiRequestType, ChiResponseType, ChiSnoopType, ChiOpcode,
    ChiChannel, ChiOrder, ChiTxnId, ChiMessageHeader,
    ChiDataResponse, ChiSnoopRequest, ChiSnoopResponse,
};

// Re-export interface types
pub use interface::{ChiInterface, ChiTransaction, ChiTransactionState};

// Re-export timing types
pub use timing::{ChiTimingModel, ChiTimingConfig};

// Re-export coherence types
pub use coherence::{
    ChiCacheState, CoherenceRequest, CoherenceResponse,
    CoherenceStateMachine, StateTransitionResult,
};

// Re-export directory types
pub use directory::{Directory, DirectoryEntry, DirectoryState, SnoopFilter, DirectoryStats};

// Re-export QoS types
pub use qos::{
    QosCreditManager, QosStats, DbidAllocator, PendingRequest,
    PcrdType, PcrdResponse, ChannelCredits, NodeChannelCredits,
};

// Re-export node types
pub use node::{
    ChiNodeType, NodeId, ChiNodeConfig,
    ChiRequest, ChiResponse, ChiData, ChiSnoop, ChiSnoopResp,
    DataDescriptor, Channel, ChannelStats,
};

// Re-export RN-F types
pub use rn_f::{RnFNode, RnFStats, OutstandingTxn, StoreBufferEntry};

// Re-export HN-F types
pub use hn_f::{HnFNode, HnFStats, HnfTransaction, HnfTxnState, PendingSnoop};

// Re-export SN-F types
pub use sn_f::{SnFNode, SnFStats, MemoryRequest, MemoryModel};

// Re-export interconnect types
pub use interconnect::{
    ChiInterconnect, ChiInterconnectConfig, ChiSystem, InterconnectStats,
};

use crate::types::InstructionId;

/// CHI interface manager
pub struct ChiManager {
    /// Timing model
    timing: ChiTimingModel,
    /// Interface
    interface: ChiInterface,
    /// Enable CHI modeling
    enabled: bool,
}

impl ChiManager {
    /// Create a new CHI manager
    pub fn new(config: &crate::config::CPUConfig) -> Self {
        let timing_config = ChiTimingConfig {
            request_latency: config.chi_request_latency,
            response_latency: config.chi_response_latency,
            data_latency: 2,
            snoop_latency: 2,
        };

        Self {
            timing: ChiTimingModel::new(timing_config),
            interface: ChiInterface::new(config.outstanding_requests),
            enabled: config.enable_chi,
        }
    }

    /// Check if CHI is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Send a read request
    pub fn send_read(&mut self, id: InstructionId, addr: u64, size: u8) -> Option<u64> {
        if !self.enabled {
            return Some(0);
        }

        let txn = self.interface.create_read_transaction(id, addr, size)?;
        let complete_cycle = self.timing.calculate_completion(&txn);
        Some(complete_cycle)
    }

    /// Send a write request
    pub fn send_write(&mut self, id: InstructionId, addr: u64, size: u8) -> Option<u64> {
        if !self.enabled {
            return Some(0);
        }

        let txn = self.interface.create_write_transaction(id, addr, size)?;
        let complete_cycle = self.timing.calculate_completion(&txn);
        Some(complete_cycle)
    }

    /// Advance simulation by one cycle
    pub fn advance_cycle(&mut self) {
        self.timing.advance_cycle();
        self.interface.advance_cycle();
    }

    /// Get the current cycle
    pub fn current_cycle(&self) -> u64 {
        self.timing.current_cycle()
    }

    /// Get statistics
    pub fn get_stats(&self) -> ChiStats {
        ChiStats {
            read_transactions: self.interface.read_count(),
            write_transactions: self.interface.write_count(),
            snoop_count: self.interface.snoop_count(),
            outstanding_count: self.interface.outstanding_count(),
        }
    }
}

/// CHI statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct ChiStats {
    pub read_transactions: u64,
    pub write_transactions: u64,
    pub snoop_count: u64,
    pub outstanding_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CPUConfig;

    #[test]
    fn test_chi_manager() {
        let config = CPUConfig {
            enable_chi: true,
            ..Default::default()
        };

        let mut manager = ChiManager::new(&config);
        assert!(manager.is_enabled());

        let complete = manager.send_read(InstructionId(0), 0x1000, 8);
        assert!(complete.is_some());
    }
}
