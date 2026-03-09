//! Instruction window for out-of-order execution.

use crate::types::{EmulatorError, Instruction, InstructionId, InstrStatus, Result};
use ahash::AHashMap;
use std::collections::VecDeque;

/// Entry in the instruction window
#[derive(Debug, Clone)]
pub struct WindowEntry {
    /// The instruction
    pub instruction: Instruction,
    /// Current status
    pub status: InstrStatus,
    /// Cycle when instruction was dispatched
    pub dispatch_cycle: u64,
    /// Cycle when instruction started execution
    pub issue_cycle: Option<u64>,
    /// Cycle when instruction completed execution (scheduled)
    pub complete_cycle: Option<u64>,
    /// Cycle when instruction was decoded
    pub decode_cycle: Option<u64>,
    /// Cycle when instruction was renamed
    pub rename_cycle: Option<u64>,
    /// Cycle when instruction was committed/retired
    pub retire_cycle: Option<u64>,
    /// Whether this is a memory operation
    pub is_memory_op: bool,
    /// Whether the completion has been processed (dependencies released)
    pub completion_processed: bool,
}

impl WindowEntry {
    pub fn new(instr: Instruction, dispatch_cycle: u64) -> Self {
        let is_memory_op = instr.mem_access.is_some();
        Self {
            instruction: instr,
            status: InstrStatus::Waiting,
            dispatch_cycle,
            issue_cycle: None,
            complete_cycle: None,
            decode_cycle: None,
            rename_cycle: None,
            retire_cycle: None,
            is_memory_op,
            completion_processed: false,
        }
    }

    /// Get the instruction
    pub fn instr(&self) -> &Instruction {
        &self.instruction
    }

    /// Calculate execution latency
    pub fn execution_latency(&self) -> Option<u64> {
        match (self.issue_cycle, self.complete_cycle) {
            (Some(issue), Some(complete)) => Some(complete.saturating_sub(issue)),
            _ => None,
        }
    }

    /// Set the decode cycle
    pub fn set_decode_cycle(&mut self, cycle: u64) {
        self.decode_cycle = Some(cycle);
    }

    /// Set the rename cycle
    pub fn set_rename_cycle(&mut self, cycle: u64) {
        self.rename_cycle = Some(cycle);
    }

    /// Set the retire cycle
    pub fn set_retire_cycle(&mut self, cycle: u64) {
        self.retire_cycle = Some(cycle);
    }
}

/// Instruction window for tracking in-flight instructions
pub struct InstructionWindow {
    /// Maximum capacity
    capacity: usize,
    /// Entries indexed by instruction ID
    entries: AHashMap<InstructionId, WindowEntry>,
    /// Queue of instruction IDs in program order
    order: VecDeque<InstructionId>,
}

impl InstructionWindow {
    /// Create a new instruction window
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: AHashMap::with_capacity(capacity),
            order: VecDeque::with_capacity(capacity),
        }
    }

    /// Check if there's space for more instructions
    pub fn has_space(&self) -> bool {
        self.entries.len() < self.capacity
    }

    /// Get the number of free slots
    pub fn free_slots(&self) -> usize {
        self.capacity.saturating_sub(self.entries.len())
    }

    /// Insert an instruction into the window
    pub fn insert(&mut self, instr: Instruction) -> Result<InstructionId> {
        if !self.has_space() {
            return Err(EmulatorError::InternalError(
                "Instruction window is full".to_string()
            ));
        }

        let id = instr.id;
        let entry = WindowEntry::new(instr, 0); // dispatch_cycle will be set externally

        self.entries.insert(id, entry);
        self.order.push_back(id);

        Ok(id)
    }

    /// Get an entry by ID
    pub fn get_entry(&self, id: InstructionId) -> Option<&WindowEntry> {
        self.entries.get(&id)
    }

    /// Get a mutable entry by ID
    pub fn get_entry_mut(&mut self, id: InstructionId) -> Option<&mut WindowEntry> {
        self.entries.get_mut(&id)
    }

    /// Mark an instruction as ready
    pub fn mark_ready(&mut self, id: InstructionId) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.status = InstrStatus::Ready;
        }
    }

    /// Mark an instruction as executing
    pub fn mark_executing(&mut self, id: InstructionId) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.status = InstrStatus::Executing;
        }
    }

    /// Mark an instruction as completed (execution finished, but not yet processed)
    /// This is kept for backward compatibility
    pub fn mark_completed(&mut self, id: InstructionId, complete_cycle: u64) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.complete_cycle = Some(complete_cycle);
            // Note: status is NOT set here anymore - it's set in set_status_completed
        }
    }

    /// Set only the complete_cycle without changing status
    pub fn set_complete_cycle(&mut self, id: InstructionId, complete_cycle: u64) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.complete_cycle = Some(complete_cycle);
        }
    }

    /// Set status to Completed (called when completion is actually processed)
    pub fn set_status_completed(&mut self, id: InstructionId) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.status = InstrStatus::Completed;
        }
    }

    /// Mark that completion has been processed (dependencies released, can now commit)
    pub fn mark_completion_processed(&mut self, id: InstructionId) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.completion_processed = true;
        }
    }

    /// Check if completion has been processed (ready for commit)
    pub fn is_completion_processed(&self, id: InstructionId) -> bool {
        self.entries.get(&id).map(|e| e.completion_processed).unwrap_or(false)
    }

    /// Get entry for debugging
    pub fn get_entry_debug(&self, id: InstructionId) -> Option<&WindowEntry> {
        self.entries.get(&id)
    }

    /// Set the issue cycle for an instruction
    pub fn set_issue_cycle(&mut self, id: InstructionId, cycle: u64) {
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.issue_cycle = Some(cycle);
        }
    }

    /// Remove an instruction (after commit)
    pub fn remove(&mut self, id: InstructionId) -> Option<WindowEntry> {
        let entry = self.entries.remove(&id);
        if entry.is_some() {
            // Remove from order queue
            self.order.retain(|&x| x != id);
        }
        entry
    }

    /// Get the number of instructions in the window
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the window is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all ready instruction IDs in program order
    pub fn get_ready_ids(&self) -> Vec<InstructionId> {
        self.order
            .iter()
            .filter(|&&id| {
                self.entries
                    .get(&id)
                    .map(|e| e.status == InstrStatus::Ready)
                    .unwrap_or(false)
            })
            .copied()
            .collect()
    }

    /// Get all completed instruction IDs in program order (ready for commit)
    pub fn get_completed_ids(&self) -> Vec<InstructionId> {
        self.order
            .iter()
            .filter(|&&id| {
                self.entries
                    .get(&id)
                    .map(|e| e.status == InstrStatus::Completed)
                    .unwrap_or(false)
            })
            .copied()
            .collect()
    }

    /// Get the oldest instruction ID (head of program order)
    pub fn oldest(&self) -> Option<InstructionId> {
        self.order.front().copied()
    }

    /// Get status counts for debugging
    pub fn status_counts(&self) -> (usize, usize, usize, usize) {
        let mut waiting = 0;
        let mut ready = 0;
        let mut executing = 0;
        let mut completed = 0;

        for entry in self.entries.values() {
            match entry.status {
                InstrStatus::Waiting => waiting += 1,
                InstrStatus::Ready => ready += 1,
                InstrStatus::Executing => executing += 1,
                InstrStatus::Completed | InstrStatus::Committed => completed += 1,
            }
        }

        (waiting, ready, executing, completed)
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    /// Iterate over all entries
    pub fn iter(&self) -> impl Iterator<Item = &WindowEntry> {
        self.entries.values()
    }

    /// Get window statistics
    pub fn get_stats(&self) -> WindowStats {
        let mut stats = WindowStats::default();
        stats.capacity = self.capacity;
        stats.occupancy = self.entries.len();

        for entry in self.entries.values() {
            match entry.status {
                InstrStatus::Waiting => stats.waiting += 1,
                InstrStatus::Ready => stats.ready += 1,
                InstrStatus::Executing => stats.executing += 1,
                InstrStatus::Completed => stats.completed += 1,
                InstrStatus::Committed => stats.committed += 1,
            }
        }

        stats
    }
}

/// Statistics about the instruction window
#[derive(Debug, Clone, Copy, Default)]
pub struct WindowStats {
    pub capacity: usize,
    pub occupancy: usize,
    pub waiting: usize,
    pub ready: usize,
    pub executing: usize,
    pub completed: usize,
    pub committed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OpcodeType, Reg};

    #[test]
    fn test_window_basic() {
        let mut window = InstructionWindow::new(16);

        let instr = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Add)
            .with_src_reg(Reg(0))
            .with_dst_reg(Reg(1));

        assert!(window.has_space());
        window.insert(instr).unwrap();
        assert_eq!(window.len(), 1);

        let entry = window.get_entry(InstructionId(0)).unwrap();
        assert_eq!(entry.status, InstrStatus::Waiting);
    }

    #[test]
    fn test_window_status_transitions() {
        let mut window = InstructionWindow::new(16);

        let instr = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Add);
        window.insert(instr).unwrap();

        window.mark_ready(InstructionId(0));
        assert_eq!(window.get_entry(InstructionId(0)).unwrap().status, InstrStatus::Ready);

        window.mark_executing(InstructionId(0));
        assert_eq!(window.get_entry(InstructionId(0)).unwrap().status, InstrStatus::Executing);

        // mark_completed only sets complete_cycle, not status
        window.mark_completed(InstructionId(0), 10);
        let entry = window.get_entry(InstructionId(0)).unwrap();
        assert_eq!(entry.complete_cycle, Some(10));
        // Status should still be Executing until set_status_completed is called
        assert_eq!(entry.status, InstrStatus::Executing);

        // set_status_completed changes status to Completed
        window.set_status_completed(InstructionId(0));
        let entry = window.get_entry(InstructionId(0)).unwrap();
        assert_eq!(entry.status, InstrStatus::Completed);

        window.remove(InstructionId(0));
        assert!(window.is_empty());
    }

    #[test]
    fn test_window_capacity() {
        let mut window = InstructionWindow::new(4);

        for i in 0..4 {
            let instr = Instruction::new(InstructionId(i), 0x1000 + i as u64 * 4, 0, OpcodeType::Nop);
            window.insert(instr).unwrap();
        }

        assert!(!window.has_space());

        let instr = Instruction::new(InstructionId(4), 0x1010, 0, OpcodeType::Nop);
        assert!(window.insert(instr).is_err());
    }
}
