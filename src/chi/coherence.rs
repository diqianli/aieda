//! CHI coherence state machine with extended MOESI states.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

/// CHI cache line state (extended MOESI)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ChiCacheState {
    // Stable states
    /// I - Invalid (not present in cache)
    Invalid,
    /// UC - Unique Clean (exclusive, clean)
    UniqueClean,
    /// UD - Unique Dirty (exclusive, dirty/modified)
    UniqueDirty,
    /// SC - Shared Clean (shared, may be in multiple caches)
    SharedClean,
    /// SD - Shared Dirty (shared, but this copy may have dirty data)
    SharedDirty,

    // Intermediate states (transaction in progress)
    /// UC -> I transition
    UcToI,
    /// UD -> I transition
    UdToI,
    /// SC -> I transition
    ScToI,
    /// SD -> I transition
    SdToI,
    /// I -> UC transition
    IToUc,
    /// I -> UD transition
    IToUd,
    /// I -> SC transition
    IToSc,
    /// I -> SD transition
    IToSd,
    /// UC -> SC transition
    UcToSc,
    /// UD -> SD transition
    UdToSd,
}

impl Default for ChiCacheState {
    fn default() -> Self {
        Self::Invalid
    }
}

impl ChiCacheState {
    /// Check if this is a stable state
    pub fn is_stable(&self) -> bool {
        matches!(
            self,
            Self::Invalid
                | Self::UniqueClean
                | Self::UniqueDirty
                | Self::SharedClean
                | Self::SharedDirty
        )
    }

    /// Check if the line is valid (present in cache)
    pub fn is_valid(&self) -> bool {
        !matches!(self, Self::Invalid)
    }

    /// Check if this node has exclusive ownership
    pub fn is_unique(&self) -> bool {
        matches!(
            self,
            Self::UniqueClean
                | Self::UniqueDirty
                | Self::UcToI
                | Self::UdToI
                | Self::IToUc
                | Self::IToUd
        )
    }

    /// Check if this node has dirty data
    pub fn is_dirty(&self) -> bool {
        matches!(
            self,
            Self::UniqueDirty
                | Self::SharedDirty
                | Self::UdToI
                | Self::SdToI
                | Self::IToUd
                | Self::IToSd
                | Self::UdToSd
        )
    }

    /// Check if this node can respond to snoop with data
    pub fn can_provide_data(&self) -> bool {
        matches!(
            self,
            Self::UniqueClean
                | Self::UniqueDirty
                | Self::SharedDirty
                | Self::UcToI
                | Self::UdToI
                | Self::SdToI
        )
    }

    /// Check if read is allowed in this state
    pub fn can_read(&self) -> bool {
        self.is_valid()
    }

    /// Check if write is allowed in this state
    pub fn can_write(&self) -> bool {
        matches!(
            self,
            Self::UniqueClean
                | Self::UniqueDirty
                | Self::IToUc
                | Self::IToUd
        )
    }

    /// Get the stable state from an intermediate state
    pub fn stable_state(&self) -> Option<Self> {
        match self {
            Self::UcToI | Self::UdToI | Self::ScToI | Self::SdToI => Some(Self::Invalid),
            Self::IToUc => Some(Self::UniqueClean),
            Self::IToUd => Some(Self::UniqueDirty),
            Self::IToSc => Some(Self::SharedClean),
            Self::IToSd => Some(Self::SharedDirty),
            Self::UcToSc => Some(Self::SharedClean),
            Self::UdToSd => Some(Self::SharedDirty),
            _ => None, // Already stable
        }
    }

    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Invalid => "I",
            Self::UniqueClean => "UC",
            Self::UniqueDirty => "UD",
            Self::SharedClean => "SC",
            Self::SharedDirty => "SD",
            Self::UcToI => "UC->I",
            Self::UdToI => "UD->I",
            Self::ScToI => "SC->I",
            Self::SdToI => "SD->I",
            Self::IToUc => "I->UC",
            Self::IToUd => "I->UD",
            Self::IToSc => "I->SC",
            Self::IToSd => "I->SD",
            Self::UcToSc => "UC->SC",
            Self::UdToSd => "UD->SD",
        }
    }
}

/// Coherence request type for state transitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoherenceRequest {
    /// Request shared access (read)
    ReadShared,
    /// Request unique/exclusive access (read with intent to write)
    ReadUnique,
    /// Request unique access for write
    MakeUnique,
    /// Request clean unique for write
    CleanUnique,
    /// Evict the line
    Evict,
    /// Clean and keep shared
    CleanShared,
    /// Clean and invalidate
    CleanInvalid,
    /// Invalidate without writeback
    MakeInvalid,
}

/// Coherence response info
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoherenceResponse {
    /// Whether data is being provided
    pub data_valid: bool,
    /// Whether data is dirty
    pub data_dirty: bool,
    /// Final state after response
    pub final_state: ChiCacheState,
    /// Whether the response is an ack only
    pub ack_only: bool,
}

impl CoherenceResponse {
    /// Create a response that provides data
    pub fn with_data(dirty: bool, final_state: ChiCacheState) -> Self {
        Self {
            data_valid: true,
            data_dirty: dirty,
            final_state,
            ack_only: false,
        }
    }

    /// Create an ack-only response
    pub fn ack(final_state: ChiCacheState) -> Self {
        Self {
            data_valid: false,
            data_dirty: false,
            final_state,
            ack_only: true,
        }
    }
}

/// State transition result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateTransitionResult {
    /// Transition completed successfully
    Complete(ChiCacheState),
    /// Transition in progress, waiting for response
    Pending(ChiCacheState),
    /// Invalid transition
    Invalid,
    /// Need to wait for ongoing transaction
    Blocked,
}

/// Coherence state machine for CHI protocol
pub struct CoherenceStateMachine;

impl CoherenceStateMachine {
    /// Calculate the next state for a read request
    pub fn on_read_request(current: ChiCacheState, want_unique: bool) -> StateTransitionResult {
        match current {
            ChiCacheState::Invalid => {
                let next = if want_unique {
                    ChiCacheState::IToUc
                } else {
                    ChiCacheState::IToSc
                };
                StateTransitionResult::Pending(next)
            }
            ChiCacheState::UniqueClean | ChiCacheState::UniqueDirty => {
                // Already have unique access, no transition needed
                StateTransitionResult::Complete(current)
            }
            ChiCacheState::SharedClean | ChiCacheState::SharedDirty => {
                if want_unique {
                    // Need to upgrade to unique
                    StateTransitionResult::Pending(current) // Will send CleanUnique
                } else {
                    // Already have shared access
                    StateTransitionResult::Complete(current)
                }
            }
            _ => StateTransitionResult::Blocked, // Intermediate state
        }
    }

    /// Calculate the next state for a write request
    pub fn on_write_request(current: ChiCacheState) -> StateTransitionResult {
        match current {
            ChiCacheState::Invalid => StateTransitionResult::Pending(ChiCacheState::IToUd),
            ChiCacheState::UniqueClean => {
                // Can write, will become dirty
                StateTransitionResult::Complete(ChiCacheState::UniqueDirty)
            }
            ChiCacheState::UniqueDirty => {
                // Already have write permission
                StateTransitionResult::Complete(ChiCacheState::UniqueDirty)
            }
            ChiCacheState::SharedClean | ChiCacheState::SharedDirty => {
                // Need to upgrade to unique
                StateTransitionResult::Pending(current)
            }
            _ => StateTransitionResult::Blocked,
        }
    }

    /// Handle snoop request and return response
    pub fn on_snoop_request(
        current: ChiCacheState,
        snoop_type: super::protocol::ChiSnoopType,
    ) -> CoherenceResponse {
        use super::protocol::ChiSnoopType;

        match snoop_type {
            ChiSnoopType::SnpOnce => {
                // Just checking, don't change state
                if current.can_provide_data() {
                    CoherenceResponse::with_data(current.is_dirty(), current)
                } else {
                    CoherenceResponse::ack(current)
                }
            }
            ChiSnoopType::SnpShared => {
                // Downgrade to shared if unique
                match current {
                    ChiCacheState::UniqueClean => {
                        CoherenceResponse::with_data(false, ChiCacheState::SharedClean)
                    }
                    ChiCacheState::UniqueDirty => {
                        CoherenceResponse::with_data(true, ChiCacheState::SharedDirty)
                    }
                    ChiCacheState::SharedClean | ChiCacheState::SharedDirty => {
                        CoherenceResponse::ack(current)
                    }
                    _ => CoherenceResponse::ack(ChiCacheState::Invalid),
                }
            }
            ChiSnoopType::SnpClean | ChiSnoopType::SnpData => {
                // Return data if we have it
                if current.can_provide_data() {
                    let final_state = if current.is_dirty() {
                        ChiCacheState::SharedDirty
                    } else if current.is_unique() {
                        ChiCacheState::SharedClean
                    } else {
                        current
                    };
                    CoherenceResponse::with_data(current.is_dirty(), final_state)
                } else {
                    CoherenceResponse::ack(current)
                }
            }
            ChiSnoopType::SnpCleanShared => {
                // Clean and remain shared
                match current {
                    ChiCacheState::UniqueDirty | ChiCacheState::SharedDirty => {
                        CoherenceResponse::with_data(true, ChiCacheState::SharedClean)
                    }
                    ChiCacheState::UniqueClean => {
                        CoherenceResponse::with_data(false, ChiCacheState::SharedClean)
                    }
                    ChiCacheState::SharedClean => CoherenceResponse::ack(current),
                    _ => CoherenceResponse::ack(ChiCacheState::Invalid),
                }
            }
            ChiSnoopType::SnpCleanInvalid | ChiSnoopType::SnpMakeInvalid => {
                // Invalidate and return data if dirty
                match current {
                    ChiCacheState::UniqueDirty | ChiCacheState::SharedDirty => {
                        CoherenceResponse::with_data(true, ChiCacheState::Invalid)
                    }
                    ChiCacheState::UniqueClean => {
                        CoherenceResponse::with_data(false, ChiCacheState::Invalid)
                    }
                    ChiCacheState::SharedClean => {
                        CoherenceResponse::ack(ChiCacheState::Invalid)
                    }
                    _ => CoherenceResponse::ack(ChiCacheState::Invalid),
                }
            }
            ChiSnoopType::SnpStashUnique | ChiSnoopType::SnpStashShared => {
                // Stash requests - not fully implemented
                CoherenceResponse::ack(current)
            }
        }
    }

    /// Complete a pending state transition
    pub fn complete_transition(current: ChiCacheState, got_unique: bool, got_dirty: bool) -> ChiCacheState {
        match current {
            ChiCacheState::IToUc => ChiCacheState::UniqueClean,
            ChiCacheState::IToUd => ChiCacheState::UniqueDirty,
            ChiCacheState::IToSc => ChiCacheState::SharedClean,
            ChiCacheState::IToSd => ChiCacheState::SharedDirty,
            ChiCacheState::UcToSc => ChiCacheState::SharedClean,
            ChiCacheState::UdToSd => ChiCacheState::SharedDirty,
            ChiCacheState::UcToI | ChiCacheState::UdToI | ChiCacheState::ScToI | ChiCacheState::SdToI => {
                ChiCacheState::Invalid
            }
            // For stable states, upgrade if needed
            ChiCacheState::SharedClean | ChiCacheState::SharedDirty => {
                if got_unique {
                    if got_dirty {
                        ChiCacheState::UniqueDirty
                    } else {
                        ChiCacheState::UniqueClean
                    }
                } else {
                    current
                }
            }
            _ => current,
        }
    }

    /// Calculate eviction handling
    pub fn on_evict(current: ChiCacheState) -> Option<(ChiCacheState, bool)> {
        match current {
            ChiCacheState::UniqueDirty | ChiCacheState::SharedDirty => {
                // Need to writeback dirty data
                Some((ChiCacheState::Invalid, true))
            }
            ChiCacheState::UniqueClean | ChiCacheState::SharedClean => {
                // Clean eviction, no writeback needed
                Some((ChiCacheState::Invalid, false))
            }
            ChiCacheState::Invalid => None,
            _ => None, // Intermediate states, cannot evict
        }
    }
}

/// Convert from CacheLineState to ChiCacheState
impl From<crate::memory::CacheLineState> for ChiCacheState {
    fn from(state: crate::memory::CacheLineState) -> Self {
        match state {
            crate::memory::CacheLineState::Invalid => ChiCacheState::Invalid,
            crate::memory::CacheLineState::Shared => ChiCacheState::SharedClean,
            crate::memory::CacheLineState::Exclusive => ChiCacheState::UniqueClean,
            crate::memory::CacheLineState::Modified => ChiCacheState::UniqueDirty,
            crate::memory::CacheLineState::Unique => ChiCacheState::UniqueClean,
        }
    }
}

/// Convert from ChiCacheState to CacheLineState
impl From<ChiCacheState> for crate::memory::CacheLineState {
    fn from(state: ChiCacheState) -> Self {
        match state {
            ChiCacheState::Invalid => crate::memory::CacheLineState::Invalid,
            ChiCacheState::UniqueClean | ChiCacheState::IToUc | ChiCacheState::UcToSc | ChiCacheState::UcToI => {
                crate::memory::CacheLineState::Unique
            }
            ChiCacheState::UniqueDirty | ChiCacheState::IToUd | ChiCacheState::UdToSd | ChiCacheState::UdToI => {
                crate::memory::CacheLineState::Modified
            }
            ChiCacheState::SharedClean | ChiCacheState::IToSc | ChiCacheState::ScToI | ChiCacheState::SdToI => {
                crate::memory::CacheLineState::Shared
            }
            ChiCacheState::SharedDirty | ChiCacheState::IToSd => {
                crate::memory::CacheLineState::Modified // Treat as modified for simplicity
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_properties() {
        assert!(ChiCacheState::Invalid.is_stable());
        assert!(!ChiCacheState::Invalid.is_valid());
        assert!(!ChiCacheState::Invalid.is_dirty());

        assert!(ChiCacheState::UniqueClean.is_stable());
        assert!(ChiCacheState::UniqueClean.is_valid());
        assert!(ChiCacheState::UniqueClean.is_unique());
        assert!(!ChiCacheState::UniqueClean.is_dirty());

        assert!(ChiCacheState::UniqueDirty.is_dirty());
        assert!(ChiCacheState::UniqueDirty.can_write());

        assert!(ChiCacheState::SharedClean.can_read());
        assert!(!ChiCacheState::SharedClean.can_write());
    }

    #[test]
    fn test_read_request_transition() {
        let result = CoherenceStateMachine::on_read_request(ChiCacheState::Invalid, false);
        match result {
            StateTransitionResult::Pending(next) => assert_eq!(next, ChiCacheState::IToSc),
            _ => panic!("Expected pending transition"),
        }

        let result = CoherenceStateMachine::on_read_request(ChiCacheState::Invalid, true);
        match result {
            StateTransitionResult::Pending(next) => assert_eq!(next, ChiCacheState::IToUc),
            _ => panic!("Expected pending transition"),
        }

        let result = CoherenceStateMachine::on_read_request(ChiCacheState::UniqueClean, false);
        match result {
            StateTransitionResult::Complete(state) => assert_eq!(state, ChiCacheState::UniqueClean),
            _ => panic!("Expected complete"),
        }
    }

    #[test]
    fn test_snoop_response() {
        let resp = CoherenceStateMachine::on_snoop_request(
            ChiCacheState::UniqueDirty,
            super::super::protocol::ChiSnoopType::SnpShared,
        );
        assert!(resp.data_valid);
        assert!(resp.data_dirty);
        assert_eq!(resp.final_state, ChiCacheState::SharedDirty);

        let resp = CoherenceStateMachine::on_snoop_request(
            ChiCacheState::SharedClean,
            super::super::protocol::ChiSnoopType::SnpMakeInvalid,
        );
        assert!(!resp.data_valid);
        assert_eq!(resp.final_state, ChiCacheState::Invalid);
    }

    #[test]
    fn test_eviction() {
        let result = CoherenceStateMachine::on_evict(ChiCacheState::UniqueDirty);
        assert!(result.is_some());
        let (state, needs_writeback) = result.unwrap();
        assert_eq!(state, ChiCacheState::Invalid);
        assert!(needs_writeback);

        let result = CoherenceStateMachine::on_evict(ChiCacheState::UniqueClean);
        assert!(result.is_some());
        let (_, needs_writeback) = result.unwrap();
        assert!(!needs_writeback);
    }

    #[test]
    fn test_state_conversion() {
        let chi_state: ChiCacheState = crate::memory::CacheLineState::Modified.into();
        assert_eq!(chi_state, ChiCacheState::UniqueDirty);

        let cache_state: crate::memory::CacheLineState = ChiCacheState::SharedClean.into();
        assert_eq!(cache_state, crate::memory::CacheLineState::Shared);
    }
}
