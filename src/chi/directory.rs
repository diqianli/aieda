//! Directory-based snoop filter for CHI protocol.

use serde::{Deserialize, Serialize};
use ahash::AHashMap;
use std::collections::HashSet;

use super::protocol::ChiTxnId;

/// Directory state for a cache line
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DirectoryState {
    /// Not present in any RN-F
    NotPresent,
    /// Present in one or more RN-F caches
    Present,
    /// Being processed (transaction in flight)
    Processing,
}

impl Default for DirectoryState {
    fn default() -> Self {
        Self::NotPresent
    }
}

/// Directory entry tracking a cache line's sharing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DirectoryEntry {
    /// Cache line address (aligned to line size)
    pub addr: u64,
    /// Node IDs that have this line cached
    pub sharers: HashSet<u8>,
    /// Current directory state
    pub state: DirectoryState,
    /// Current active transaction (if any)
    pub pending_txn: Option<ChiTxnId>,
    /// Owner node (for unique states)
    pub owner: Option<u8>,
    /// Whether any sharer has dirty data
    pub dirty: bool,
}

impl DirectoryEntry {
    /// Create a new directory entry
    pub fn new(addr: u64) -> Self {
        Self {
            addr,
            sharers: HashSet::new(),
            state: DirectoryState::NotPresent,
            pending_txn: None,
            owner: None,
            dirty: false,
        }
    }

    /// Check if a node is a sharer
    pub fn is_sharer(&self, node_id: u8) -> bool {
        self.sharers.contains(&node_id)
    }

    /// Add a sharer
    pub fn add_sharer(&mut self, node_id: u8) {
        self.sharers.insert(node_id);
        self.state = DirectoryState::Present;
    }

    /// Remove a sharer
    pub fn remove_sharer(&mut self, node_id: u8) {
        self.sharers.remove(&node_id);
        if self.sharers.is_empty() {
            self.state = DirectoryState::NotPresent;
            self.owner = None;
            self.dirty = false;
        } else if self.owner == Some(node_id) {
            self.owner = None;
        }
    }

    /// Get the number of sharers
    pub fn sharer_count(&self) -> usize {
        self.sharers.len()
    }

    /// Check if line is shared (multiple sharers)
    pub fn is_shared(&self) -> bool {
        self.sharers.len() > 1
    }

    /// Check if line is unique (single sharer)
    pub fn is_unique(&self) -> bool {
        self.sharers.len() == 1
    }

    /// Set the owner (for unique state)
    pub fn set_owner(&mut self, node_id: u8) {
        self.owner = Some(node_id);
        self.add_sharer(node_id);
    }

    /// Get all sharers except one
    pub fn other_sharers(&self, exclude: u8) -> Vec<u8> {
        self.sharers.iter().copied().filter(|&id| id != exclude).collect()
    }

    /// Mark as processing
    pub fn start_transaction(&mut self, txn_id: ChiTxnId) {
        self.pending_txn = Some(txn_id);
        self.state = DirectoryState::Processing;
    }

    /// Complete transaction
    pub fn complete_transaction(&mut self) {
        self.pending_txn = None;
        if self.sharers.is_empty() {
            self.state = DirectoryState::NotPresent;
        } else {
            self.state = DirectoryState::Present;
        }
    }

    /// Invalidate all sharers
    pub fn invalidate_all(&mut self) {
        self.sharers.clear();
        self.owner = None;
        self.dirty = false;
        self.state = DirectoryState::NotPresent;
    }
}

/// Directory (Snoop Filter) for tracking cache line sharing
pub struct Directory {
    /// Directory entries indexed by cache line address
    entries: AHashMap<u64, DirectoryEntry>,
    /// Cache line size for address alignment
    line_size: usize,
    /// Maximum number of entries (for memory management)
    max_entries: usize,
    /// Statistics
    stats: DirectoryStats,
}

/// Directory statistics
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct DirectoryStats {
    /// Total lookups
    pub lookups: u64,
    /// Entries found
    pub hits: u64,
    /// Entries not found
    pub misses: u64,
    /// New entries created
    pub inserts: u64,
    /// Entries removed
    pub evictions: u64,
    /// Snoop broadcasts needed
    pub snoop_broadcasts: u64,
    /// Single-sharer cases (point-to-point snoop)
    pub single_sharer_snops: u64,
}

impl Directory {
    /// Create a new directory
    pub fn new(line_size: usize, max_entries: usize) -> Self {
        Self {
            entries: AHashMap::new(),
            line_size,
            max_entries,
            stats: DirectoryStats::default(),
        }
    }

    /// Align address to cache line boundary
    fn align_addr(&self, addr: u64) -> u64 {
        let mask = !(self.line_size as u64 - 1);
        addr & mask
    }

    /// Look up a directory entry
    pub fn lookup(&self, addr: u64) -> Option<&DirectoryEntry> {
        let aligned = self.align_addr(addr);
        self.entries.get(&aligned)
    }

    /// Look up a directory entry (mutable)
    pub fn lookup_mut(&mut self, addr: u64) -> Option<&mut DirectoryEntry> {
        let aligned = self.align_addr(addr);
        self.entries.get_mut(&aligned)
    }

    /// Get or create a directory entry
    pub fn get_or_create(&mut self, addr: u64) -> &mut DirectoryEntry {
        let aligned = self.align_addr(addr);
        self.stats.lookups += 1;

        if !self.entries.contains_key(&aligned) {
            self.stats.misses += 1;
            self.stats.inserts += 1;

            // Evict old entry if at capacity
            if self.entries.len() >= self.max_entries {
                self.evict_oldest();
            }

            self.entries.insert(aligned, DirectoryEntry::new(aligned));
        } else {
            self.stats.hits += 1;
        }

        self.entries.get_mut(&aligned).unwrap()
    }

    /// Evict the oldest entry (simple LRU approximation)
    fn evict_oldest(&mut self) {
        if let Some((&addr, _)) = self.entries.iter().next() {
            self.entries.remove(&addr);
            self.stats.evictions += 1;
        }
    }

    /// Check if a line is present in any cache
    pub fn is_present(&self, addr: u64) -> bool {
        let aligned = self.align_addr(addr);
        self.entries.get(&aligned).map_or(false, |e| !e.sharers.is_empty())
    }

    /// Get all sharers for an address
    pub fn get_sharers(&self, addr: u64) -> Vec<u8> {
        let aligned = self.align_addr(addr);
        self.entries
            .get(&aligned)
            .map(|e| e.sharers.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Add a sharer for an address
    pub fn add_sharer(&mut self, addr: u64, node_id: u8) {
        let entry = self.get_or_create(addr);
        entry.add_sharer(node_id);
    }

    /// Remove a sharer for an address
    pub fn remove_sharer(&mut self, addr: u64, node_id: u8) {
        let aligned = self.align_addr(addr);
        if let Some(entry) = self.entries.get_mut(&aligned) {
            entry.remove_sharer(node_id);
            // Optionally remove empty entries
            if entry.sharers.is_empty() && entry.pending_txn.is_none() {
                // Keep entry for now, may be reused
            }
        }
    }

    /// Record a snoop operation and return targets
    pub fn get_snoop_targets(&mut self, addr: u64, exclude: Option<u8>) -> Vec<u8> {
        let aligned = self.align_addr(addr);

        if let Some(entry) = self.entries.get(&aligned) {
            let targets: Vec<u8> = entry
                .sharers
                .iter()
                .copied()
                .filter(|&id| exclude.map_or(true, |ex| id != ex))
                .collect();

            if targets.len() > 1 {
                self.stats.snoop_broadcasts += 1;
            } else if targets.len() == 1 {
                self.stats.single_sharer_snops += 1;
            }

            targets
        } else {
            vec![]
        }
    }

    /// Set owner for unique state
    pub fn set_owner(&mut self, addr: u64, node_id: u8) {
        let entry = self.get_or_create(addr);
        entry.set_owner(node_id);
    }

    /// Get the owner of a line
    pub fn get_owner(&self, addr: u64) -> Option<u8> {
        let aligned = self.align_addr(addr);
        self.entries.get(&aligned).and_then(|e| e.owner)
    }

    /// Mark a line as dirty
    pub fn set_dirty(&mut self, addr: u64, dirty: bool) {
        if let Some(entry) = self.lookup_mut(addr) {
            entry.dirty = dirty;
        }
    }

    /// Check if a line is dirty
    pub fn is_dirty(&self, addr: u64) -> bool {
        self.lookup(addr).map_or(false, |e| e.dirty)
    }

    /// Start a transaction for an address
    pub fn start_transaction(&mut self, addr: u64, txn_id: ChiTxnId) {
        let entry = self.get_or_create(addr);
        entry.start_transaction(txn_id);
    }

    /// Complete a transaction for an address
    pub fn complete_transaction(&mut self, addr: u64) {
        let aligned = self.align_addr(addr);
        if let Some(entry) = self.entries.get_mut(&aligned) {
            entry.complete_transaction();
        }
    }

    /// Invalidate all sharers for an address
    pub fn invalidate_all(&mut self, addr: u64) {
        let aligned = self.align_addr(addr);
        if let Some(entry) = self.entries.get_mut(&aligned) {
            entry.invalidate_all();
        }
    }

    /// Get statistics
    pub fn stats(&self) -> &DirectoryStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = DirectoryStats::default();
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get entry count
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Handle RN-F evict notification
    pub fn on_evict(&mut self, addr: u64, node_id: u8, is_dirty: bool) {
        let aligned = self.align_addr(addr);

        if let Some(entry) = self.entries.get_mut(&aligned) {
            entry.remove_sharer(node_id);

            if is_dirty {
                entry.dirty = false;
            }

            // If no more sharers, the line is not present
            if entry.sharers.is_empty() {
                entry.state = DirectoryState::NotPresent;
                entry.owner = None;
            }
        }
    }
}

/// Snoop filter using directory
pub type SnoopFilter = Directory;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_directory_entry() {
        let mut entry = DirectoryEntry::new(0x1000);

        assert_eq!(entry.sharer_count(), 0);
        assert!(!entry.is_sharer(1));

        entry.add_sharer(1);
        assert_eq!(entry.sharer_count(), 1);
        assert!(entry.is_sharer(1));
        assert!(entry.is_unique());

        entry.add_sharer(2);
        assert_eq!(entry.sharer_count(), 2);
        assert!(entry.is_shared());

        entry.remove_sharer(1);
        assert_eq!(entry.sharer_count(), 1);
    }

    #[test]
    fn test_directory_basic() {
        let mut dir = Directory::new(64, 1024);

        assert!(!dir.is_present(0x1000));

        dir.add_sharer(0x1000, 1);
        assert!(dir.is_present(0x1000));

        let sharers = dir.get_sharers(0x1000);
        assert_eq!(sharers, vec![1]);

        dir.add_sharer(0x1000, 2);
        let sharers = dir.get_sharers(0x1000);
        assert_eq!(sharers.len(), 2);
    }

    #[test]
    fn test_directory_owner() {
        let mut dir = Directory::new(64, 1024);

        dir.set_owner(0x1000, 1);
        assert_eq!(dir.get_owner(0x1000), Some(1));

        let entry = dir.lookup(0x1000).unwrap();
        assert!(entry.is_unique());
    }

    #[test]
    fn test_snoop_targets() {
        let mut dir = Directory::new(64, 1024);

        dir.add_sharer(0x1000, 1);
        dir.add_sharer(0x1000, 2);
        dir.add_sharer(0x1000, 3);

        // Get all targets
        let targets = dir.get_snoop_targets(0x1000, None);
        assert_eq!(targets.len(), 3);

        // Exclude one target
        let targets = dir.get_snoop_targets(0x1000, Some(1));
        assert_eq!(targets.len(), 2);
        assert!(!targets.contains(&1));
    }

    #[test]
    fn test_evict_notification() {
        let mut dir = Directory::new(64, 1024);

        dir.add_sharer(0x1000, 1);
        dir.add_sharer(0x1000, 2);
        dir.set_dirty(0x1000, true);

        dir.on_evict(0x1000, 1, true);

        let sharers = dir.get_sharers(0x1000);
        assert_eq!(sharers.len(), 1);
        assert!(sharers.contains(&2));
    }

    #[test]
    fn test_address_alignment() {
        let mut dir = Directory::new(64, 1024);

        // All these addresses should map to the same cache line
        dir.add_sharer(0x1000, 1);
        dir.add_sharer(0x1020, 2);
        dir.add_sharer(0x103F, 3);

        let sharers = dir.get_sharers(0x1000);
        assert_eq!(sharers.len(), 3);
    }

    #[test]
    fn test_transaction_tracking() {
        let mut dir = Directory::new(64, 1024);
        let txn_id = ChiTxnId::new(1);

        dir.add_sharer(0x1000, 1);
        dir.start_transaction(0x1000, txn_id);

        let entry = dir.lookup(0x1000).unwrap();
        assert_eq!(entry.state, DirectoryState::Processing);
        assert_eq!(entry.pending_txn, Some(txn_id));

        dir.complete_transaction(0x1000);

        let entry = dir.lookup(0x1000).unwrap();
        assert_eq!(entry.state, DirectoryState::Present);
        assert!(entry.pending_txn.is_none());
    }
}
