//! Enhanced Cache with MSHR Support, 3C Miss Classification, and Prefetcher
//!
//! This module provides an enhanced cache implementation with:
//! - MSHR (Miss Status Holding Register) for tracking outstanding misses
//! - 3C miss classification (Compulsory, Capacity, Conflict)
//! - Prefetcher support

use std::collections::{HashMap, HashSet, VecDeque};

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

use super::cache::{Cache, CacheConfig, CacheStats};
use crate::types::{EmulatorError, Result};

/// MSHR (Miss Status Holding Register) entry
#[derive(Debug, Clone)]
pub struct MshrEntry {
    /// Address being fetched
    pub addr: u64,
    /// Cycle when the miss was detected
    pub miss_cycle: u64,
    /// List of instructions waiting for this data
    pub waiting_instructions: Vec<u64>,
    /// Whether this is a prefetch request
    pub is_prefetch: bool,
    /// Priority (higher = more important)
    pub priority: u8,
}

/// MSHR (Miss Status Holding Register) for tracking outstanding misses
#[derive(Debug, Clone)]
pub struct Mshr {
    /// Maximum number of outstanding misses
    capacity: usize,
    /// Entries indexed by address
    entries: HashMap<u64, MshrEntry>,
    /// FIFO order of miss addresses
    order: VecDeque<u64>,
}

impl Mshr {
    /// Create a new MSHR with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            capacity,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// Check if there's capacity for a new miss
    pub fn can_accept(&self) -> bool {
        self.entries.len() < self.capacity
    }

    /// Check if address is already being fetched
    pub fn contains(&self, addr: u64) -> bool {
        self.entries.contains_key(&addr)
    }

    /// Get number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Allocate a new MSHR entry
    pub fn allocate(&mut self, addr: u64, miss_cycle: u64, is_prefetch: bool) -> Result<()> {
        if self.entries.len() >= self.capacity {
            return Err(EmulatorError::InternalError("MSHR full".into()));
        }

        if self.entries.contains_key(&addr) {
            // Already pending - just add to waiting list
            return Ok(());
        }

        let entry = MshrEntry {
            addr,
            miss_cycle,
            waiting_instructions: Vec::new(),
            is_prefetch,
            priority: if is_prefetch { 1 } else { 2 },
        };

        self.entries.insert(addr, entry);
        self.order.push_back(addr);

        Ok(())
    }

    /// Add a waiting instruction to an existing entry
    pub fn add_waiter(&mut self, addr: u64, instr_id: u64) {
        if let Some(entry) = self.entries.get_mut(&addr) {
            entry.waiting_instructions.push(instr_id);
        }
    }

    /// Complete a miss and return the entry
    pub fn complete(&mut self, addr: u64) -> Option<MshrEntry> {
        if let Some(entry) = self.entries.remove(&addr) {
            self.order.retain(|&a| a != addr);
            Some(entry)
        } else {
            None
        }
    }

    /// Get the oldest pending address
    pub fn oldest(&self) -> Option<u64> {
        self.order.front().copied()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
    }

    /// Get statistics
    pub fn stats(&self) -> MshrStats {
        MshrStats {
            total_misses: self.entries.len() as u64,
            outstanding_misses: self.entries.len() as u64,
            mshr_occupation: if self.capacity > 0 {
                self.entries.len() as f64 / self.capacity as f64
            } else {
                0.0
            },
        }
    }
}

/// MSHR statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct MshrStats {
    /// Total misses tracked
    pub total_misses: u64,
    /// Currently outstanding misses
    pub outstanding_misses: u64,
    /// MSHR occupation rate
    pub mshr_occupation: f64,
}

/// 3C Miss classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum MissType {
    /// First access to a cache line (cold miss)
    Compulsory,
    /// Cache is full, line was evicted
    Capacity,
    /// Multiple lines map to same set
    Conflict,
}

/// Enhanced cache statistics with 3C classification
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct EnhancedCacheStats {
    /// Base cache statistics
    #[serde(flatten)]
    pub base: CacheStats,
    /// Compulsory misses
    pub compulsory_misses: u64,
    /// Capacity misses
    pub capacity_misses: u64,
    /// Conflict misses
    pub conflict_misses: u64,
    /// Prefetch requests
    pub prefetch_requests: u64,
    /// Prefetch hits (useful prefetches)
    pub prefetch_hits: u64,
    /// Prefetch misses (wasted prefetches)
    pub prefetch_misses: u64,
    /// MSHR statistics
    pub mshr_stats: MshrStats,
}

impl EnhancedCacheStats {
    /// Calculate compulsory miss rate
    pub fn compulsory_miss_rate(&self) -> f64 {
        if self.base.misses == 0 {
            0.0
        } else {
            self.compulsory_misses as f64 / self.base.misses as f64
        }
    }

    /// Calculate capacity miss rate
    pub fn capacity_miss_rate(&self) -> f64 {
        if self.base.misses == 0 {
            0.0
        } else {
            self.capacity_misses as f64 / self.base.misses as f64
        }
    }

    /// Calculate conflict miss rate
    pub fn conflict_miss_rate(&self) -> f64 {
        if self.base.misses == 0 {
            0.0
        } else {
            self.conflict_misses as f64 / self.base.misses as f64
        }
    }

    /// Calculate prefetch coverage
    pub fn prefetch_coverage(&self) -> f64 {
        if self.base.misses == 0 {
            0.0
        } else {
            self.prefetch_hits as f64 / self.base.misses as f64
        }
    }

    /// Calculate prefetch accuracy
    pub fn prefetch_accuracy(&self) -> f64 {
        let total_prefetches = self.prefetch_hits + self.prefetch_misses;
        if total_prefetches == 0 {
            0.0
        } else {
            self.prefetch_hits as f64 / total_prefetches as f64
        }
    }
}

/// Prefetch request
#[derive(Debug, Clone)]
pub struct PrefetchRequest {
    /// Address to prefetch
    pub addr: u64,
    /// Priority (higher = more urgent)
    pub priority: u8,
    /// PC that triggered the prefetch
    pub trigger_pc: Option<u64>,
}

/// Prefetcher trait
pub trait Prefetcher: Send + Sync {
    /// Called on each memory access
    fn on_access(&mut self, pc: u64, addr: u64, is_miss: bool) -> Vec<PrefetchRequest>;

    /// Called when a prefetch completes
    fn on_prefetch_complete(&mut self, addr: u64);

    /// Get prefetcher statistics
    fn stats(&self) -> PrefetcherStats;

    /// Reset the prefetcher state
    fn reset(&mut self);
}

/// Prefetcher statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct PrefetcherStats {
    /// Total prefetch requests generated
    pub requests_generated: u64,
    /// Useful prefetches (prefetched line was used)
    pub useful_prefetches: u64,
    /// Useless prefetches (prefetched line was evicted without being used)
    pub useless_prefetches: u64,
    /// Prefetch accuracy
    pub accuracy: f64,
}

/// Next-line prefetcher (simple sequential prefetcher)
#[derive(Debug, Clone)]
pub struct NextLinePrefetcher {
    /// Cache line size
    line_size: u64,
    /// Number of lines to prefetch ahead
    degree: usize,
    /// Statistics
    stats: PrefetcherStats,
    /// Recently accessed addresses
    recent_addrs: VecDeque<u64>,
}

impl NextLinePrefetcher {
    /// Create a new next-line prefetcher
    pub fn new(line_size: u64, degree: usize) -> Self {
        Self {
            line_size,
            degree,
            stats: PrefetcherStats::default(),
            recent_addrs: VecDeque::new(),
        }
    }
}

impl Prefetcher for NextLinePrefetcher {
    fn on_access(&mut self, _pc: u64, addr: u64, is_miss: bool) -> Vec<PrefetchRequest> {
        let mut requests = Vec::new();

        // Only prefetch on miss
        if is_miss {
            let line_addr = addr & !(self.line_size - 1);

            // Prefetch next lines
            for i in 1..=self.degree {
                let prefetch_addr = line_addr + (i as u64 * self.line_size);
                requests.push(PrefetchRequest {
                    addr: prefetch_addr,
                    priority: 1,
                    trigger_pc: Some(_pc),
                });
                self.stats.requests_generated += 1;
            }

            // Track recent accesses
            self.recent_addrs.push_back(addr);
            if self.recent_addrs.len() > 16 {
                self.recent_addrs.pop_front();
            }
        }

        requests
    }

    fn on_prefetch_complete(&mut self, _addr: u64) {
        // Track completed prefetches
    }

    fn stats(&self) -> PrefetcherStats {
        let accuracy = if self.stats.requests_generated > 0 {
            self.stats.useful_prefetches as f64 / self.stats.requests_generated as f64
        } else {
            0.0
        };
        PrefetcherStats {
            accuracy,
            ..self.stats.clone()
        }
    }

    fn reset(&mut self) {
        self.stats = PrefetcherStats::default();
        self.recent_addrs.clear();
    }
}

/// Stride prefetcher (detects regular access patterns)
#[derive(Debug, Clone)]
pub struct StridePrefetcher {
    /// Cache line size
    line_size: u64,
    /// Maximum stride to track
    max_stride: i64,
    /// PC-stride table (maps PC to detected stride)
    stride_table: HashMap<u64, i64>,
    /// Last address accessed by PC
    last_addr: HashMap<u64, u64>,
    /// Statistics
    stats: PrefetcherStats,
}

impl StridePrefetcher {
    /// Create a new stride prefetcher
    pub fn new(line_size: u64) -> Self {
        Self {
            line_size,
            max_stride: 4096,
            stride_table: HashMap::new(),
            last_addr: HashMap::new(),
            stats: PrefetcherStats::default(),
        }
    }
}

impl Prefetcher for StridePrefetcher {
    fn on_access(&mut self, pc: u64, addr: u64, _is_miss: bool) -> Vec<PrefetchRequest> {
        let mut requests = Vec::new();

        // Track stride
        if let Some(&last) = self.last_addr.get(&pc) {
            let stride = addr as i64 - last as i64;

            if stride.abs() <= self.max_stride && stride != 0 {
                // Record the stride
                self.stride_table.insert(pc, stride);

                // Generate prefetch request
                let prefetch_addr = (addr as i64 + stride) as u64;
                requests.push(PrefetchRequest {
                    addr: prefetch_addr,
                    priority: 2,
                    trigger_pc: Some(pc),
                });
                self.stats.requests_generated += 1;
            }
        }

        // Update last address
        self.last_addr.insert(pc, addr);

        // Limit table size
        if self.last_addr.len() > 64 {
            if let Some(oldest) = self.last_addr.keys().next().copied() {
                self.last_addr.remove(&oldest);
            }
        }

        requests
    }

    fn on_prefetch_complete(&mut self, _addr: u64) {}

    fn stats(&self) -> PrefetcherStats {
        let accuracy = if self.stats.requests_generated > 0 {
            self.stats.useful_prefetches as f64 / self.stats.requests_generated as f64
        } else {
            0.0
        };
        PrefetcherStats {
            accuracy,
            ..self.stats.clone()
        }
    }

    fn reset(&mut self) {
        self.stride_table.clear();
        self.last_addr.clear();
        self.stats = PrefetcherStats::default();
    }
}

/// Enhanced cache wrapper with MSHR, 3C classification, and prefetcher support
pub struct EnhancedCache {
    /// Base cache
    cache: Cache,
    /// MSHR for tracking outstanding misses
    mshr: Mshr,
    /// Optional prefetcher
    prefetcher: Option<Box<dyn Prefetcher>>,
    /// Enhanced statistics
    stats: EnhancedCacheStats,
    /// Set of accessed addresses (for compulsory miss detection)
    accessed_lines: HashSet<u64>,
    /// Pending prefetch requests
    pending_prefetches: HashSet<u64>,
}

impl EnhancedCache {
    /// Create a new enhanced cache
    pub fn new(config: CacheConfig) -> Result<Self> {
        let mshr_capacity = 16;
        let cache = Cache::new(config)?;

        Ok(Self {
            cache,
            mshr: Mshr::new(mshr_capacity),
            prefetcher: None,
            stats: EnhancedCacheStats::default(),
            accessed_lines: HashSet::new(),
            pending_prefetches: HashSet::new(),
        })
    }

    /// Create with a prefetcher
    pub fn with_prefetcher(config: CacheConfig, prefetcher: Box<dyn Prefetcher>) -> Result<Self> {
        let mut enhanced = Self::new(config)?;
        enhanced.prefetcher = Some(prefetcher);
        Ok(enhanced)
    }

    /// Access the cache (returns hit status and miss type if applicable)
    pub fn access(&mut self, pc: u64, addr: u64, is_read: bool, current_cycle: u64) -> Result<(bool, Option<MissType>)> {
        let line_addr = addr & !(self.cache.config().line_size as u64 - 1);

        // Try base cache access
        let hit = self.cache.access(addr, is_read)?;

        if hit {
            // Check if this was a prefetched line
            if self.pending_prefetches.remove(&line_addr) {
                self.stats.prefetch_hits += 1;
                if let Some(ref mut pf) = self.prefetcher {
                    pf.on_prefetch_complete(addr);
                }
            }

            self.stats.base.accesses += 1;
            if is_read {
                self.stats.base.reads += 1;
            } else {
                self.stats.base.writes += 1;
            }
            self.stats.base.hits += 1;

            return Ok((true, None));
        }

        // Miss - classify it
        let set_idx = self.cache.config().get_set(addr);
        let miss_type = self.classify_miss(addr, set_idx);

        // Update statistics
        self.stats.base.accesses += 1;
        if is_read {
            self.stats.base.reads += 1;
            self.stats.base.read_misses += 1;
        } else {
            self.stats.base.writes += 1;
            self.stats.base.write_misses += 1;
        }
        self.stats.base.misses += 1;

        match miss_type {
            MissType::Compulsory => self.stats.compulsory_misses += 1,
            MissType::Capacity => self.stats.capacity_misses += 1,
            MissType::Conflict => self.stats.conflict_misses += 1,
        }

        // Allocate MSHR entry
        if !self.mshr.contains(addr) {
            self.mshr.allocate(addr, current_cycle, false)?;
        }

        // Record access for compulsory miss detection
        self.accessed_lines.insert(line_addr);

        // Trigger prefetcher
        if let Some(ref mut pf) = self.prefetcher {
            let requests = pf.on_access(pc, addr, true);
            for req in requests {
                if !self.mshr.contains(req.addr) && self.mshr.can_accept() {
                    self.mshr.allocate(req.addr, current_cycle, true)?;
                    self.pending_prefetches.insert(req.addr & !(self.cache.config().line_size as u64 - 1));
                    self.stats.prefetch_requests += 1;
                }
            }
        }

        Ok((false, Some(miss_type)))
    }

    /// Classify a miss using 3C model
    fn classify_miss(&self, addr: u64, set_idx: usize) -> MissType {
        let line_addr = addr & !(self.cache.config().line_size as u64 - 1);

        // Check if this is the first access to this line
        if !self.accessed_lines.contains(&line_addr) {
            return MissType::Compulsory;
        }

        // If we've accessed this line before but it's not in cache,
        // it was evicted - classify as capacity miss
        // (In a real implementation, we'd need more info to distinguish capacity from conflict)
        MissType::Capacity
    }

    /// Fill a cache line (after miss returns)
    pub fn fill_line(&mut self, addr: u64) -> Option<u64> {
        // Complete MSHR entry
        self.mshr.complete(addr);

        // Fill the line in the base cache
        self.cache.fill_line(addr)
    }

    /// Check if an address has a pending MSHR entry
    pub fn has_pending_miss(&self, addr: u64) -> bool {
        self.mshr.contains(addr)
    }

    /// Check if MSHR can accept a new miss
    pub fn can_accept_miss(&self) -> bool {
        self.mshr.can_accept()
    }

    /// Get enhanced statistics
    pub fn enhanced_stats(&self) -> &EnhancedCacheStats {
        &self.stats
    }

    /// Get base statistics
    pub fn stats(&self) -> &CacheStats {
        self.cache.stats()
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.cache.reset_stats();
        self.stats = EnhancedCacheStats::default();
        self.accessed_lines.clear();
        self.pending_prefetches.clear();
        self.mshr.clear();
    }

    /// Get configuration
    pub fn config(&self) -> &CacheConfig {
        self.cache.config()
    }

    /// Flush the cache
    pub fn flush(&mut self) {
        self.cache.flush();
        self.accessed_lines.clear();
        self.pending_prefetches.clear();
        self.mshr.clear();
    }

    /// Get hit latency
    pub fn hit_latency(&self) -> u64 {
        self.cache.hit_latency()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mshr() {
        let mut mshr = Mshr::new(4);
        assert!(mshr.can_accept());
        assert!(!mshr.contains(0x1000));

        mshr.allocate(0x1000, 0, false).unwrap();
        assert!(mshr.contains(0x1000));
        assert_eq!(mshr.len(), 1);
    }

    #[test]
    fn test_next_line_prefetcher() {
        let mut prefetcher = NextLinePrefetcher::new(64, 2);

        let requests = prefetcher.on_access(0x100, 0x1000, true);
        assert_eq!(requests.len(), 2);

        // First request should be next line
        assert_eq!(requests[0].addr, 0x1040);
        assert_eq!(requests[1].addr, 0x1080);
    }

    #[test]
    fn test_enhanced_cache_creation() {
        let config = CacheConfig {
            size: 4 * 1024,
            associativity: 4,
            line_size: 64,
            hit_latency: 4,
            name: "L1".to_string(),
        };

        let cache = EnhancedCache::new(config);
        assert!(cache.is_ok());
    }
}
