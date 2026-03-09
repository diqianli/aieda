//! Load/Store Queue implementation.

use crate::types::{InstructionId, MemAccess};
use ahash::AHashMap;
use std::collections::VecDeque;

/// Entry in the Load/Store Queue
#[derive(Debug, Clone)]
pub struct LsqEntry {
    /// Instruction ID
    pub instruction_id: InstructionId,
    /// Memory address
    pub addr: u64,
    /// Access size in bytes
    pub size: u8,
    /// Whether this is a load (true) or store (false)
    pub is_load: bool,
    /// Whether this entry is completed
    pub completed: bool,
    /// Cycle when issued
    pub issue_cycle: u64,
    /// Cycle when completed
    pub complete_cycle: Option<u64>,
}

impl LsqEntry {
    pub fn new_load(id: InstructionId, addr: u64, size: u8, issue_cycle: u64) -> Self {
        Self {
            instruction_id: id,
            addr,
            size,
            is_load: true,
            completed: false,
            issue_cycle,
            complete_cycle: None,
        }
    }

    pub fn new_store(id: InstructionId, addr: u64, size: u8, issue_cycle: u64) -> Self {
        Self {
            instruction_id: id,
            addr,
            size,
            is_load: false,
            completed: false,
            issue_cycle,
            complete_cycle: None,
        }
    }
}

/// Handle to an LSQ entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LsqHandle(pub usize);

/// Load/Store Queue
pub struct LoadStoreQueue {
    /// Maximum capacity
    capacity: usize,
    /// Number of load pipelines
    load_pipelines: usize,
    /// Number of store pipelines
    store_pipelines: usize,
    /// Queue entries
    entries: VecDeque<LsqEntry>,
    /// Map from instruction ID to entry index
    id_to_entry: AHashMap<InstructionId, usize>,
    /// Currently executing loads
    active_loads: usize,
    /// Currently executing stores
    active_stores: usize,
    /// Current cycle (for tracking)
    current_cycle: u64,
}

impl LoadStoreQueue {
    /// Create a new LSQ
    pub fn new(capacity: usize, load_pipelines: usize, store_pipelines: usize) -> Self {
        Self {
            capacity,
            load_pipelines,
            store_pipelines,
            entries: VecDeque::with_capacity(capacity),
            id_to_entry: AHashMap::new(),
            active_loads: 0,
            active_stores: 0,
            current_cycle: 0,
        }
    }

    /// Check if the queue has space
    pub fn has_space(&self) -> bool {
        self.entries.len() < self.capacity
    }

    /// Get the number of free slots
    pub fn free_slots(&self) -> usize {
        self.capacity.saturating_sub(self.entries.len())
    }

    /// Add a load to the queue
    pub fn add_load(&mut self, id: InstructionId, addr: u64, size: u8) -> LsqHandle {
        let entry = LsqEntry::new_load(id, addr, size, self.current_cycle);
        self.entries.push_back(entry);
        let handle = LsqHandle(self.entries.len() - 1);
        self.id_to_entry.insert(id, handle.0);
        handle
    }

    /// Add a store to the queue
    pub fn add_store(&mut self, id: InstructionId, addr: u64, size: u8) -> LsqHandle {
        let entry = LsqEntry::new_store(id, addr, size, self.current_cycle);
        self.entries.push_back(entry);
        let handle = LsqHandle(self.entries.len() - 1);
        self.id_to_entry.insert(id, handle.0);
        handle
    }

    /// Mark an entry as completed
    pub fn complete(&mut self, handle: LsqHandle) {
        if let Some(entry) = self.entries.get_mut(handle.0) {
            entry.completed = true;
            entry.complete_cycle = Some(self.current_cycle);

            if entry.is_load {
                self.active_loads = self.active_loads.saturating_sub(1);
            } else {
                self.active_stores = self.active_stores.saturating_sub(1);
            }
        }
    }

    /// Check if an address conflicts with any pending store
    pub fn check_store_conflict(&self, addr: u64, size: u8) -> Option<InstructionId> {
        let end_addr = addr.wrapping_add(size as u64);

        for entry in &self.entries {
            if !entry.is_load && !entry.completed {
                let entry_end = entry.addr.wrapping_add(entry.size as u64);

                // Check for overlap
                if addr < entry_end && end_addr > entry.addr {
                    return Some(entry.instruction_id);
                }
            }
        }

        None
    }

    /// Get entries ready to issue
    pub fn get_ready(&mut self) -> Vec<(LsqHandle, InstructionId, u64, bool)> {
        let mut ready = Vec::new();
        let mut loads_issued = 0;
        let mut stores_issued = 0;

        for (i, entry) in self.entries.iter().enumerate() {
            if entry.completed {
                continue;
            }

            let can_issue = if entry.is_load {
                loads_issued < self.load_pipelines && self.active_loads < self.load_pipelines
            } else {
                stores_issued < self.store_pipelines && self.active_stores < self.store_pipelines
            };

            if can_issue {
                ready.push((LsqHandle(i), entry.instruction_id, entry.addr, entry.is_load));

                if entry.is_load {
                    loads_issued += 1;
                    self.active_loads += 1;
                } else {
                    stores_issued += 1;
                    self.active_stores += 1;
                }
            }

            // Stop if we've filled all pipelines
            if loads_issued >= self.load_pipelines && stores_issued >= self.store_pipelines {
                break;
            }
        }

        ready
    }

    /// Remove completed entries from the head of the queue
    pub fn retire_completed(&mut self) -> Vec<LsqEntry> {
        let mut retired = Vec::new();

        while let Some(entry) = self.entries.front() {
            if entry.completed {
                let entry = self.entries.pop_front().unwrap();
                self.id_to_entry.remove(&entry.instruction_id);
                retired.push(entry);
            } else {
                break;
            }
        }

        // Rebuild index map
        self.id_to_entry.clear();
        for (i, entry) in self.entries.iter().enumerate() {
            self.id_to_entry.insert(entry.instruction_id, i);
        }

        retired
    }

    /// Get the current occupancy
    pub fn occupancy(&self) -> usize {
        self.entries.len()
    }

    /// Check if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Advance the current cycle
    pub fn advance_cycle(&mut self) {
        self.current_cycle += 1;
    }

    /// Get the current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Clear the queue
    pub fn clear(&mut self) {
        self.entries.clear();
        self.id_to_entry.clear();
        self.active_loads = 0;
        self.active_stores = 0;
    }

    /// Get statistics
    pub fn get_stats(&self) -> LsqStats {
        let loads = self.entries.iter().filter(|e| e.is_load).count();
        let stores = self.entries.iter().filter(|e| !e.is_load).count();
        let completed = self.entries.iter().filter(|e| e.completed).count();

        LsqStats {
            capacity: self.capacity,
            occupancy: self.entries.len(),
            loads,
            stores,
            completed,
            active_loads: self.active_loads,
            active_stores: self.active_stores,
        }
    }
}

/// LSQ statistics
#[derive(Debug, Clone, Copy)]
pub struct LsqStats {
    pub capacity: usize,
    pub occupancy: usize,
    pub loads: usize,
    pub stores: usize,
    pub completed: usize,
    pub active_loads: usize,
    pub active_stores: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsq_basic() {
        let mut lsq = LoadStoreQueue::new(16, 2, 1);

        assert!(lsq.has_space());

        let handle = lsq.add_load(InstructionId(0), 0x1000, 8);
        assert_eq!(lsq.occupancy(), 1);

        lsq.complete(handle);
        assert!(lsq.entries.front().unwrap().completed);
    }

    #[test]
    fn test_store_conflict() {
        let mut lsq = LoadStoreQueue::new(16, 2, 1);

        lsq.add_store(InstructionId(0), 0x1000, 8);

        // Load to overlapping address should conflict
        let conflict = lsq.check_store_conflict(0x1004, 8);
        assert!(conflict.is_some());

        // Load to non-overlapping address should not conflict
        let no_conflict = lsq.check_store_conflict(0x2000, 8);
        assert!(no_conflict.is_none());
    }

    #[test]
    fn test_retire_completed() {
        let mut lsq = LoadStoreQueue::new(16, 2, 1);

        let h0 = lsq.add_load(InstructionId(0), 0x1000, 8);
        let h1 = lsq.add_load(InstructionId(1), 0x1008, 8);

        lsq.complete(h0);

        let retired = lsq.retire_completed();
        assert_eq!(retired.len(), 1);
        assert_eq!(lsq.occupancy(), 1);
    }
}
