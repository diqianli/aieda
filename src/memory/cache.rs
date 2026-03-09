//! Cache implementation for the memory subsystem.

use crate::types::{EmulatorError, Result};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::vec::Vec;

/// Cache line state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
pub enum CacheLineState {
    #[default]
    Invalid,
    Shared,
    Exclusive,
    Modified,
    Unique,
}

/// Cache line
#[derive(Debug, Clone, Default)]
pub struct CacheLine {
    /// Tag (address / line_size)
    pub tag: u64,
    /// Coherence state
    pub state: CacheLineState,
    /// Whether this line is valid
    pub valid: bool,
    /// LRU counter (higher = more recently used)
    pub lru: u32,
    /// Dirty flag (for write-back)
    pub dirty: bool,
}

impl CacheLine {
    pub fn new() -> Self {
        Self {
            tag: 0,
            state: CacheLineState::Invalid,
            valid: false,
            lru: 0,
            dirty: false,
        }
    }
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CacheConfig {
    /// Total cache size in bytes
    pub size: usize,
    /// Associativity (number of ways)
    pub associativity: usize,
    /// Line size in bytes
    pub line_size: usize,
    /// Hit latency in cycles
    pub hit_latency: u64,
    /// Cache name (for debugging)
    pub name: String,
}

impl CacheConfig {
    /// Calculate number of sets
    pub fn num_sets(&self) -> usize {
        self.size / (self.associativity * self.line_size)
    }

    /// Calculate tag bits
    pub fn tag_bits(&self) -> u32 {
        let sets = self.num_sets();
        let set_bits = (sets as f64).log2() as u32;
        let line_bits = (self.line_size as f64).log2() as u32;
        64 - set_bits - line_bits
    }

    /// Get set index from address
    pub fn get_set(&self, addr: u64) -> usize {
        let set_mask = (self.num_sets() - 1) as u64;
        let line_bits = (self.line_size as f64).log2() as u32;
        ((addr >> line_bits) & set_mask) as usize
    }

    /// Get tag from address
    pub fn get_tag(&self, addr: u64) -> u64 {
        let sets = self.num_sets();
        let set_bits = (sets as f64).log2() as u32;
        let line_bits = (self.line_size as f64).log2() as u32;
        addr >> (set_bits + line_bits)
    }
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct CacheStats {
    /// Cache name
    pub name: String,
    /// Total accesses
    pub accesses: u64,
    /// Total hits
    pub hits: u64,
    /// Total misses
    pub misses: u64,
    /// Read accesses
    pub reads: u64,
    /// Write accesses
    pub writes: u64,
    /// Read misses
    pub read_misses: u64,
    /// Write misses
    pub write_misses: u64,
    /// Evictions
    pub evictions: u64,
}

impl CacheStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        if self.accesses == 0 {
            0.0
        } else {
            self.hits as f64 / self.accesses as f64
        }
    }

    /// Calculate miss rate
    pub fn miss_rate(&self) -> f64 {
        if self.accesses == 0 {
            0.0
        } else {
            self.misses as f64 / self.accesses as f64
        }
    }

    /// Calculate MPKI (Misses Per Kilo Instructions)
    pub fn mpki(&self, instructions: u64) -> f64 {
        if instructions == 0 {
            0.0
        } else {
            (self.misses as f64 / instructions as f64) * 1000.0
        }
    }

    /// Calculate average access latency (given miss latency)
    pub fn avg_latency(&self, hit_latency: u64, miss_latency: u64) -> f64 {
        if self.accesses == 0 {
            0.0
        } else {
            let hit_time = self.hits as f64 * hit_latency as f64;
            let miss_time = self.misses as f64 * miss_latency as f64;
            (hit_time + miss_time) / self.accesses as f64
        }
    }
}

/// Cache set (contains multiple ways)
#[derive(Debug, Clone)]
pub struct CacheSet {
    /// Ways (cache lines)
    ways: Vec<CacheLine>,
    /// Associativity
    associativity: usize,
}

impl CacheSet {
    pub fn new(associativity: usize) -> Self {
        let ways = (0..associativity).map(|_| CacheLine::new()).collect();
        Self { ways, associativity }
    }

    /// Find a line by tag
    pub fn find(&self, tag: u64) -> Option<(usize, &CacheLine)> {
        self.ways.iter().enumerate().find(|(_, line)| line.valid && line.tag == tag)
    }

    /// Find a line by tag (mutable)
    pub fn find_mut(&mut self, tag: u64) -> Option<(usize, &mut CacheLine)> {
        self.ways.iter_mut().enumerate().find(|(_, line)| line.valid && line.tag == tag)
    }

    /// Find LRU victim
    pub fn find_victim(&self) -> usize {
        self.ways.iter().enumerate()
            .filter(|(_, line)| !line.valid)
            .map(|(i, _)| i)
            .next()
            .unwrap_or_else(|| {
                self.ways.iter().enumerate()
                    .min_by_key(|(_, line)| line.lru)
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            })
    }

    /// Update LRU state (access the given way)
    pub fn update_lru(&mut self, accessed_way: usize) {
        for (i, way) in self.ways.iter_mut().enumerate() {
            if i == accessed_way {
                way.lru = self.associativity as u32;
            } else if way.lru > 0 {
                way.lru -= 1;
            }
        }
    }

    /// Get a way by index
    pub fn get_way(&mut self, way: usize) -> &mut CacheLine {
        &mut self.ways[way]
    }
}

/// Cache implementation
pub struct Cache {
    /// Configuration
    config: CacheConfig,
    /// Cache sets
    sets: Vec<CacheSet>,
    /// Statistics
    stats: CacheStats,
}

impl Cache {
    /// Create a new cache
    pub fn new(config: CacheConfig) -> Result<Self> {
        let num_sets = config.num_sets();

        if num_sets == 0 {
            return Err(EmulatorError::ConfigError(
                "Invalid cache configuration: zero sets".to_string()
            ));
        }

        if !num_sets.is_power_of_two() {
            return Err(EmulatorError::ConfigError(
                "Number of sets must be a power of 2".to_string()
            ));
        }

        let sets = (0..num_sets).map(|_| CacheSet::new(config.associativity)).collect();

        let stats = CacheStats {
            name: config.name.clone(),
            ..Default::default()
        };

        Ok(Self { config, sets, stats })
    }

    /// Access the cache
    pub fn access(&mut self, addr: u64, is_read: bool) -> Result<bool> {
        let set_idx = self.config.get_set(addr);
        let tag = self.config.get_tag(addr);

        self.stats.accesses += 1;
        if is_read {
            self.stats.reads += 1;
        } else {
            self.stats.writes += 1;
        }

        // First, check if hit using immutable borrow
        let hit_way = {
            let set = &self.sets[set_idx];
            set.find(tag).map(|(way_idx, _)| way_idx)
        };

        if let Some(way_idx) = hit_way {
            // Hit - update LRU and line state using mutable borrow
            let set = &mut self.sets[set_idx];
            set.update_lru(way_idx);
            let line = set.get_way(way_idx);
            line.lru = self.config.associativity as u32;

            if !is_read {
                line.dirty = true;
            }

            self.stats.hits += 1;
            return Ok(true);
        }

        // Miss - update stats
        self.stats.misses += 1;
        if is_read {
            self.stats.read_misses += 1;
        } else {
            self.stats.write_misses += 1;
        }

        Ok(false)
    }

    /// Fill a cache line (after miss)
    pub fn fill_line(&mut self, addr: u64) -> Option<u64> {
        let set_idx = self.config.get_set(addr);
        let tag = self.config.get_tag(addr);

        // Pre-compute values needed for eviction address calculation
        let num_sets = self.sets.len();
        let set_bits = (num_sets as f64).log2() as u32;
        let line_bits = (self.config.line_size as f64).log2() as u32;

        let set = &mut self.sets[set_idx];

        // Check if already present
        if let Some((way_idx, _)) = set.find(tag) {
            set.update_lru(way_idx);
            return None;
        }

        // Find victim
        let victim_way = set.find_victim();
        let victim = set.get_way(victim_way);

        let evicted_addr = if victim.valid && victim.dirty {
            // Calculate evicted address
            Some((victim.tag << (set_bits + line_bits)) | ((set_idx as u64) << line_bits))
        } else {
            None
        };

        // Fill new line
        victim.tag = tag;
        victim.valid = true;
        victim.dirty = false;
        victim.state = CacheLineState::Exclusive;

        set.update_lru(victim_way);

        if evicted_addr.is_some() {
            self.stats.evictions += 1;
        }

        evicted_addr
    }

    /// Invalidate a line
    pub fn invalidate(&mut self, addr: u64) -> bool {
        let set_idx = self.config.get_set(addr);
        let tag = self.config.get_tag(addr);

        let set = &mut self.sets[set_idx];

        if let Some((way_idx, line)) = set.find_mut(tag) {
            line.valid = false;
            line.state = CacheLineState::Invalid;
            true
        } else {
            false
        }
    }

    /// Get hit latency
    pub fn hit_latency(&self) -> u64 {
        self.config.hit_latency
    }

    /// Get statistics
    pub fn stats(&self) -> &CacheStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = CacheStats {
            name: self.config.name.clone(),
            ..Default::default()
        };
    }

    /// Get configuration
    pub fn config(&self) -> &CacheConfig {
        &self.config
    }

    /// Flush the cache
    pub fn flush(&mut self) {
        for set in &mut self.sets {
            for way in &mut set.ways {
                way.valid = false;
                way.dirty = false;
                way.state = CacheLineState::Invalid;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_config() {
        let config = CacheConfig {
            size: 64 * 1024,
            associativity: 4,
            line_size: 64,
            hit_latency: 4,
            name: "L1".to_string(),
        };

        assert_eq!(config.num_sets(), 256);
        assert_eq!(config.get_set(0x1000), 0);
    }

    #[test]
    fn test_cache_access() {
        let config = CacheConfig {
            size: 4 * 1024,
            associativity: 4,
            line_size: 64,
            hit_latency: 4,
            name: "L1".to_string(),
        };

        let mut cache = Cache::new(config).unwrap();

        // First access should miss
        let hit = cache.access(0x1000, true).unwrap();
        assert!(!hit);
        assert_eq!(cache.stats().misses, 1);

        // Fill the line
        cache.fill_line(0x1000);

        // Second access should hit
        let hit = cache.access(0x1000, true).unwrap();
        assert!(hit);
        assert_eq!(cache.stats().hits, 1);
    }

    #[test]
    fn test_cache_stats() {
        let stats = CacheStats {
            name: "L1".to_string(),
            accesses: 1000,
            hits: 950,
            misses: 50,
            ..Default::default()
        };

        assert!((stats.hit_rate() - 0.95).abs() < 0.001);
        assert!((stats.miss_rate() - 0.05).abs() < 0.001);
        assert!((stats.mpki(10000) - 5.0).abs() < 0.001);
    }
}
