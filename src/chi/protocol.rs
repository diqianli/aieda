//! CHI protocol definitions.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// CHI request opcode types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ChiRequestType {
    // Read requests
    ReadNoSnoop,
    ReadNotSharedDirty,
    ReadShared,
    ReadMakeUnique,
    ReadOnce,
    ReadOnceCleanInvalid,
    ReadOnceMakeInvalid,

    // Write requests
    WriteNoSnoop,
    WriteUnique,
    WriteUniquePtl,
    WriteUniqueFull,
    WriteEvictFull,
    WriteEvictPtl,

    // Coherence requests
    CleanUnique,
    MakeUnique,
    Evict,
    CleanShared,
    CleanInvalid,
    MakeInvalid,

    // Dataless requests
    DVMOp,
    PCrdReturn,
}

/// CHI response types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ChiResponseType {
    // Data responses
    CompData,
    DataSepResp,
    NonCopyBackWrData,
    CopyBackWrData,

    // Acknowledgments
    CompAck,
    DBIDResp,
    RespSepData,

    // Combined responses
    Comp,
    CompCMO,
}

/// CHI snoop types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ChiSnoopType {
    SnpOnce,
    SnpShared,
    SnpClean,
    SnpData,
    SnpCleanShared,
    SnpCleanInvalid,
    SnpMakeInvalid,
    SnpStashUnique,
    SnpStashShared,
}

/// CHI opcode (combines request/response/snoop)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ChiOpcode {
    Request(ChiRequestType),
    Response(ChiResponseType),
    Snoop(ChiSnoopType),
}

impl ChiRequestType {
    /// Check if this is a read request
    pub fn is_read(&self) -> bool {
        matches!(
            self,
            Self::ReadNoSnoop
            | Self::ReadNotSharedDirty
            | Self::ReadShared
            | Self::ReadMakeUnique
            | Self::ReadOnce
            | Self::ReadOnceCleanInvalid
            | Self::ReadOnceMakeInvalid
        )
    }

    /// Check if this is a write request
    pub fn is_write(&self) -> bool {
        matches!(
            self,
            Self::WriteNoSnoop
            | Self::WriteUnique
            | Self::WriteUniquePtl
            | Self::WriteUniqueFull
            | Self::WriteEvictFull
            | Self::WriteEvictPtl
        )
    }

    /// Check if this requires data response
    pub fn requires_data(&self) -> bool {
        self.is_read()
    }

    /// Get expected response type
    pub fn expected_response(&self) -> ChiResponseType {
        if self.is_read() {
            ChiResponseType::CompData
        } else {
            ChiResponseType::DBIDResp
        }
    }
}

impl ChiResponseType {
    /// Check if this carries data
    pub fn has_data(&self) -> bool {
        matches!(
            self,
            Self::CompData
            | Self::DataSepResp
            | Self::NonCopyBackWrData
            | Self::CopyBackWrData
        )
    }

    /// Check if this is an acknowledgment
    pub fn is_ack(&self) -> bool {
        matches!(self, Self::CompAck | Self::DBIDResp | Self::RespSepData | Self::Comp)
    }
}

impl ChiSnoopType {
    /// Check if this snoop requires data response
    pub fn requires_data(&self) -> bool {
        matches!(self, Self::SnpData | Self::SnpClean | Self::SnpStashUnique | Self::SnpStashShared)
    }
}

/// CHI channel identifiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ChiChannel {
    /// Request channel (REQ)
    Request,
    /// Response channel (RSP)
    Response,
    /// Data channel (DAT)
    Data,
    /// Snoop channel (SNP)
    Snoop,
}

/// CHI transaction ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, JsonSchema)]
pub struct ChiTxnId(pub u16);

impl ChiTxnId {
    pub fn new(id: u16) -> Self {
        Self(id)
    }

    pub fn next(&mut self) -> Self {
        let current = self.0;
        self.0 = self.0.wrapping_add(1);
        Self(current)
    }
}

/// CHI message header
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ChiMessageHeader {
    /// Opcode
    pub opcode: ChiOpcode,
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Source node ID
    pub src_id: u8,
    /// Destination node ID
    pub dest_id: u8,
    /// Address (aligned to cache line)
    pub addr: u64,
    /// Size in bytes
    pub size: u8,
    /// Allow retry
    pub allow_retry: bool,
    /// Order requirement
    pub order: ChiOrder,
}

/// CHI ordering requirements
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ChiOrder {
    /// No ordering
    None,
    /// Ordered to same endpoint
    Endpoint,
    /// Ordered to all endpoints
    Global,
}

/// CHI data response
#[derive(Debug, Clone)]
pub struct ChiDataResponse {
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Response type
    pub resp_type: ChiResponseType,
    /// Data payload (cache line)
    pub data: Option<Vec<u8>>,
    /// Error indication
    pub error: bool,
    /// Poison bits
    pub poison: bool,
}

/// CHI snoop request
#[derive(Debug, Clone)]
pub struct ChiSnoopRequest {
    /// Snoop type
    pub snoop_type: ChiSnoopType,
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Address
    pub addr: u64,
}

/// CHI snoop response
#[derive(Debug, Clone)]
pub struct ChiSnoopResponse {
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Response data
    pub data: Option<Vec<u8>>,
    /// Was data present
    pub data_present: bool,
    /// Cache state after snoop
    pub state: crate::memory::CacheLineState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_types() {
        assert!(ChiRequestType::ReadNoSnoop.is_read());
        assert!(!ChiRequestType::ReadNoSnoop.is_write());

        assert!(ChiRequestType::WriteNoSnoop.is_write());
        assert!(!ChiRequestType::WriteNoSnoop.is_read());
    }

    #[test]
    fn test_response_types() {
        assert!(ChiResponseType::CompData.has_data());
        assert!(!ChiResponseType::Comp.has_data());

        assert!(ChiResponseType::CompAck.is_ack());
    }

    #[test]
    fn test_txn_id() {
        let mut id = ChiTxnId::new(0);
        assert_eq!(id.next().0, 0);
        assert_eq!(id.next().0, 1);
    }
}
