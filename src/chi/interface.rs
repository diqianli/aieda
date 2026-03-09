//! CHI interface implementation.

use super::protocol::{ChiRequestType, ChiResponseType, ChiTxnId};
use crate::types::InstructionId;
use std::collections::VecDeque;

/// CHI transaction state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChiTransactionState {
    /// Transaction created, not yet sent
    Pending,
    /// Request sent, waiting for response
    InFlight,
    /// Response received, waiting for data
    WaitingData,
    /// Data received, transaction complete
    Complete,
    /// Transaction failed
    Failed,
}

impl Default for ChiTransactionState {
    fn default() -> Self {
        Self::Pending
    }
}

/// CHI transaction
#[derive(Debug, Clone)]
pub struct ChiTransaction {
    /// Transaction ID
    pub txn_id: ChiTxnId,
    /// Associated instruction ID
    pub instruction_id: InstructionId,
    /// Request type
    pub request_type: ChiRequestType,
    /// Address
    pub addr: u64,
    /// Size
    pub size: u8,
    /// Current state
    pub state: ChiTransactionState,
    /// Cycle when request was issued
    pub issue_cycle: u64,
    /// Cycle when response was received
    pub response_cycle: Option<u64>,
    /// Cycle when transaction completed
    pub complete_cycle: Option<u64>,
    /// Data payload
    pub data: Option<Vec<u8>>,
}

impl ChiTransaction {
    /// Create a new transaction
    pub fn new(
        txn_id: ChiTxnId,
        instruction_id: InstructionId,
        request_type: ChiRequestType,
        addr: u64,
        size: u8,
    ) -> Self {
        Self {
            txn_id,
            instruction_id,
            request_type,
            addr,
            size,
            state: ChiTransactionState::Pending,
            issue_cycle: 0,
            response_cycle: None,
            complete_cycle: None,
            data: None,
        }
    }

    /// Get expected response type
    pub fn expected_response(&self) -> ChiResponseType {
        self.request_type.expected_response()
    }

    /// Check if transaction is complete
    pub fn is_complete(&self) -> bool {
        self.state == ChiTransactionState::Complete || self.state == ChiTransactionState::Failed
    }
}

/// CHI interface
pub struct ChiInterface {
    /// Maximum outstanding transactions
    max_outstanding: usize,
    /// Transaction ID generator
    next_txn_id: ChiTxnId,
    /// Pending transactions
    pending: VecDeque<ChiTransaction>,
    /// In-flight transactions
    in_flight: VecDeque<ChiTransaction>,
    /// Completed transactions
    completed: VecDeque<ChiTransaction>,
    /// Current cycle
    current_cycle: u64,
    /// Read transaction count
    read_count: u64,
    /// Write transaction count
    write_count: u64,
    /// Snoop count
    snoop_count: u64,
}

impl ChiInterface {
    /// Create a new CHI interface
    pub fn new(max_outstanding: usize) -> Self {
        Self {
            max_outstanding,
            next_txn_id: ChiTxnId::default(),
            pending: VecDeque::with_capacity(max_outstanding),
            in_flight: VecDeque::with_capacity(max_outstanding),
            completed: VecDeque::new(),
            current_cycle: 0,
            read_count: 0,
            write_count: 0,
            snoop_count: 0,
        }
    }

    /// Check if interface can accept new transactions
    pub fn can_accept(&self) -> bool {
        self.pending.len() + self.in_flight.len() < self.max_outstanding
    }

    /// Create a read transaction
    pub fn create_read_transaction(
        &mut self,
        id: InstructionId,
        addr: u64,
        size: u8,
    ) -> Option<ChiTransaction> {
        if !self.can_accept() {
            return None;
        }

        let txn_id = self.next_txn_id.next();
        let txn = ChiTransaction::new(txn_id, id, ChiRequestType::ReadNoSnoop, addr, size);
        self.read_count += 1;

        Some(txn)
    }

    /// Create a write transaction
    pub fn create_write_transaction(
        &mut self,
        id: InstructionId,
        addr: u64,
        size: u8,
    ) -> Option<ChiTransaction> {
        if !self.can_accept() {
            return None;
        }

        let txn_id = self.next_txn_id.next();
        let txn = ChiTransaction::new(txn_id, id, ChiRequestType::WriteNoSnoop, addr, size);
        self.write_count += 1;

        Some(txn)
    }

    /// Issue a transaction
    pub fn issue(&mut self, mut txn: ChiTransaction) {
        txn.state = ChiTransactionState::InFlight;
        txn.issue_cycle = self.current_cycle;
        self.in_flight.push_back(txn);
    }

    /// Receive a response
    pub fn receive_response(&mut self, txn_id: ChiTxnId, response_type: ChiResponseType) {
        if let Some(pos) = self.in_flight.iter().position(|t| t.txn_id == txn_id) {
            let mut txn = self.in_flight.remove(pos).unwrap();

            if response_type == txn.expected_response() {
                if response_type.has_data() {
                    txn.state = ChiTransactionState::WaitingData;
                    self.in_flight.push_back(txn);
                } else {
                    txn.state = ChiTransactionState::Complete;
                    txn.response_cycle = Some(self.current_cycle);
                    txn.complete_cycle = Some(self.current_cycle);
                    self.completed.push_back(txn);
                }
            } else {
                txn.state = ChiTransactionState::Failed;
                self.completed.push_back(txn);
            }
        }
    }

    /// Receive data
    pub fn receive_data(&mut self, txn_id: ChiTxnId, data: Vec<u8>) {
        if let Some(pos) = self.in_flight.iter().position(|t| t.txn_id == txn_id) {
            let mut txn = self.in_flight.remove(pos).unwrap();
            txn.data = Some(data);
            txn.state = ChiTransactionState::Complete;
            txn.response_cycle = Some(self.current_cycle);
            txn.complete_cycle = Some(self.current_cycle);
            self.completed.push_back(txn);
        }
    }

    /// Get completed transactions
    pub fn get_completed(&mut self) -> Vec<ChiTransaction> {
        self.completed.drain(..).collect()
    }

    /// Get transaction by ID
    pub fn get_transaction(&self, txn_id: ChiTxnId) -> Option<&ChiTransaction> {
        self.pending.iter().find(|t| t.txn_id == txn_id)
            .or_else(|| self.in_flight.iter().find(|t| t.txn_id == txn_id))
            .or_else(|| self.completed.iter().find(|t| t.txn_id == txn_id))
    }

    /// Advance simulation by one cycle
    pub fn advance_cycle(&mut self) {
        self.current_cycle += 1;
    }

    /// Get current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get outstanding transaction count
    pub fn outstanding_count(&self) -> usize {
        self.pending.len() + self.in_flight.len()
    }

    /// Get read transaction count
    pub fn read_count(&self) -> u64 {
        self.read_count
    }

    /// Get write transaction count
    pub fn write_count(&self) -> u64 {
        self.write_count
    }

    /// Get snoop count
    pub fn snoop_count(&self) -> u64 {
        self.snoop_count
    }

    /// Get statistics
    pub fn get_stats(&self) -> ChiInterfaceStats {
        ChiInterfaceStats {
            pending_count: self.pending.len(),
            in_flight_count: self.in_flight.len(),
            completed_count: self.completed.len(),
            read_count: self.read_count,
            write_count: self.write_count,
            snoop_count: self.snoop_count,
        }
    }

    /// Clear all transactions
    pub fn clear(&mut self) {
        self.pending.clear();
        self.in_flight.clear();
        self.completed.clear();
    }
}

/// CHI interface statistics
#[derive(Debug, Clone, Copy)]
pub struct ChiInterfaceStats {
    pub pending_count: usize,
    pub in_flight_count: usize,
    pub completed_count: usize,
    pub read_count: u64,
    pub write_count: u64,
    pub snoop_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chi_interface() {
        let mut interface = ChiInterface::new(16);

        assert!(interface.can_accept());

        let txn = interface.create_read_transaction(InstructionId(0), 0x1000, 8);
        assert!(txn.is_some());

        let mut txn = txn.unwrap();
        interface.issue(txn.clone());

        assert_eq!(interface.outstanding_count(), 1);
    }

    #[test]
    fn test_transaction_states() {
        let txn = ChiTransaction::new(
            ChiTxnId::new(0),
            InstructionId(0),
            ChiRequestType::ReadNoSnoop,
            0x1000,
            8,
        );

        assert_eq!(txn.state, ChiTransactionState::Pending);
        assert_eq!(txn.expected_response(), ChiResponseType::CompData);
        assert!(!txn.is_complete());
    }
}
