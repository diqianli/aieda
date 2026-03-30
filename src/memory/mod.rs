//! Memory subsystem for the ARM CPU emulator.

mod lsq;
mod cache;
mod controller;
pub mod enhanced;
mod ddr_controller;

pub use lsq::{LoadStoreQueue, LsqHandle};
pub use cache::{Cache, CacheConfig, CacheStats, CacheLineState, CacheLevel, CacheLevelTiming, CacheAccessInfo};
pub use controller::MemoryController;
pub use ddr_controller::{DdrController, DdrAccessResult, DdrStats};
pub use enhanced::{
    EnhancedCache, EnhancedCacheStats, Mshr, MshrEntry, MshrStats, MissType,
    Prefetcher, PrefetchRequest, PrefetcherStats, NextLinePrefetcher, StridePrefetcher,
};

use crate::config::CPUConfig;
use crate::types::{InstructionId, MemAccess, Result};

// CHI integration types
use crate::chi::{
    ChiSystem, ChiInterconnectConfig, ChiNodeConfig,
    RnFNode, HnFNode, SnFNode, NodeId,
    ChiTxnId,
};

/// Memory subsystem combining LSQ, caches, and controller
pub struct MemorySubsystem {
    /// Load/Store Queue
    lsq: LoadStoreQueue,
    /// L1 Data Cache
    l1_cache: Cache,
    /// L2 Cache
    l2_cache: Cache,
    /// L3 Cache (Last-level cache)
    l3_cache: Cache,
    /// DDR Memory Controller
    ddr_controller: DdrController,
    /// Memory controller (legacy, for fallback)
    controller: MemoryController,
    /// Configuration
    config: CPUConfig,
    /// Current cycle
    current_cycle: u64,
    /// Outstanding memory requests
    outstanding_requests: u64,
}

impl MemorySubsystem {
    /// Create a new memory subsystem
    pub fn new(config: CPUConfig) -> Result<Self> {
        let lsq = LoadStoreQueue::new(config.lsq_size, config.load_pipeline_count, config.store_pipeline_count);

        let l1_config = CacheConfig {
            size: config.l1_size,
            associativity: config.l1_associativity,
            line_size: config.l1_line_size,
            hit_latency: config.l1_hit_latency,
            name: "L1".to_string(),
        };
        let l1_cache = Cache::new(l1_config)?;

        let l2_config = CacheConfig {
            size: config.l2_size,
            associativity: config.l2_associativity,
            line_size: config.l2_line_size,
            hit_latency: config.l2_hit_latency,
            name: "L2".to_string(),
        };
        let l2_cache = Cache::new(l2_config)?;

        let l3_config = CacheConfig {
            size: config.l3_size,
            associativity: config.l3_associativity,
            line_size: config.l3_line_size,
            hit_latency: config.l3_hit_latency,
            name: "L3".to_string(),
        };
        let l3_cache = Cache::new(l3_config)?;

        let ddr_controller = DdrController::new(
            config.ddr_base_latency,
            config.ddr_row_buffer_hit_bonus,
            config.ddr_bank_conflict_penalty,
            config.ddr_num_banks,
        );

        let controller = MemoryController::new(config.l2_miss_latency, config.outstanding_requests);

        Ok(Self {
            lsq,
            l1_cache,
            l2_cache,
            l3_cache,
            ddr_controller,
            controller,
            config,
            current_cycle: 0,
            outstanding_requests: 0,
        })
    }

    /// Process a load request
    pub fn load(&mut self, id: InstructionId, access: &MemAccess) -> MemoryRequest {
        // Check L1 cache first
        let l1_result = self.l1_cache.access(access.addr, true);

        match l1_result {
            Ok(hit) if hit => {
                // L1 hit - add to LSQ and complete
                let lsq_entry = self.lsq.add_load(id, access.addr, access.size);
                let complete_cycle = self.current_cycle + self.config.l1_hit_latency;
                self.lsq.complete(lsq_entry);
                MemoryRequest::completed_with_cache_info(
                    id,
                    complete_cycle,
                    CacheAccessInfo::l1_hit(self.current_cycle, self.config.l1_hit_latency),
                )
            }
            _ => {
                // L1 miss, check L2
                let l2_result = self.l2_cache.access(access.addr, true);

                match l2_result {
                    Ok(hit) if hit => {
                        // L2 hit
                        let lsq_entry = self.lsq.add_load(id, access.addr, access.size);
                        let complete_cycle = self.current_cycle + self.config.l1_hit_latency + self.config.l2_hit_latency;
                        self.l1_cache.fill_line(access.addr);
                        self.lsq.complete(lsq_entry);
                        MemoryRequest::completed_with_cache_info(
                            id,
                            complete_cycle,
                            CacheAccessInfo::l2_hit(
                                self.current_cycle,
                                self.config.l1_hit_latency,
                                self.config.l2_hit_latency,
                            ),
                        )
                    }
                    _ => {
                        // L2 miss, check L3
                        let l3_result = self.l3_cache.access(access.addr, true);

                        match l3_result {
                            Ok(hit) if hit => {
                                // L3 hit
                                let lsq_entry = self.lsq.add_load(id, access.addr, access.size);
                                let complete_cycle = self.current_cycle
                                    + self.config.l1_hit_latency
                                    + self.config.l2_hit_latency
                                    + self.config.l3_hit_latency;
                                self.l2_cache.fill_line(access.addr);
                                self.l1_cache.fill_line(access.addr);
                                self.lsq.complete(lsq_entry);
                                MemoryRequest::completed_with_cache_info(
                                    id,
                                    complete_cycle,
                                    CacheAccessInfo::l3_hit(
                                        self.current_cycle,
                                        self.config.l1_hit_latency,
                                        self.config.l2_hit_latency,
                                        self.config.l3_hit_latency,
                                    ),
                                )
                            }
                            _ => {
                                // L3 miss, go to DDR memory
                                let lsq_entry = self.lsq.add_load(id, access.addr, access.size);

                                // Set DDR controller cycle and perform access
                                self.ddr_controller.set_cycle(
                                    self.current_cycle
                                        + self.config.l1_hit_latency
                                        + self.config.l2_hit_latency
                                        + self.config.l3_hit_latency
                                );
                                let ddr_result = self.ddr_controller.access(access.addr);

                                // Fill all cache levels
                                self.l3_cache.fill_line(access.addr);
                                self.l2_cache.fill_line(access.addr);
                                self.l1_cache.fill_line(access.addr);
                                self.lsq.complete(lsq_entry);

                                let cache_info = CacheAccessInfo::memory_access(
                                    self.current_cycle,
                                    self.config.l1_hit_latency,
                                    self.config.l2_hit_latency,
                                    self.config.l3_hit_latency,
                                    ddr_result.latency,
                                    ddr_result.row_hit,
                                    ddr_result.bank,
                                );

                                MemoryRequest::completed_with_cache_info(
                                    id,
                                    ddr_result.complete_cycle,
                                    cache_info,
                                )
                            }
                        }
                    }
                }
            }
        }
    }

    /// Process a store request
    pub fn store(&mut self, id: InstructionId, access: &MemAccess) -> MemoryRequest {
        // Add to LSQ
        let lsq_entry = self.lsq.add_store(id, access.addr, access.size);

        // Check L1 cache first
        let l1_result = self.l1_cache.access(access.addr, false);

        let complete_cycle = self.current_cycle + self.config.l1_hit_latency;
        self.lsq.complete(lsq_entry);

        match l1_result {
            Ok(hit) if hit => {
                // L1 hit
                MemoryRequest::completed_with_cache_info(
                    id,
                    complete_cycle,
                    CacheAccessInfo::l1_hit(self.current_cycle, self.config.l1_hit_latency),
                )
            }
            _ => {
                // L1 miss - for stores, we still complete after L1 latency
                // In a real system, stores would write through the hierarchy
                // For visualization, show it as an L1 hit (simplified)
                MemoryRequest::completed_with_cache_info(
                    id,
                    complete_cycle,
                    CacheAccessInfo::l1_hit(self.current_cycle, self.config.l1_hit_latency),
                )
            }
        }
    }

    /// Advance simulation by one cycle
    pub fn advance_cycle(&mut self) {
        self.current_cycle += 1;

        // Check for completed memory requests
        let completed = self.controller.poll_completed(self.current_cycle);
        for _ in completed {
            self.outstanding_requests = self.outstanding_requests.saturating_sub(1);
        }
    }

    /// Get the current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get L1 cache statistics
    pub fn l1_stats(&self) -> &CacheStats {
        self.l1_cache.stats()
    }

    /// Get L2 cache statistics
    pub fn l2_stats(&self) -> &CacheStats {
        self.l2_cache.stats()
    }

    /// Get L3 cache statistics
    pub fn l3_stats(&self) -> &CacheStats {
        self.l3_cache.stats()
    }

    /// Get DDR controller statistics
    pub fn ddr_stats(&self) -> &DdrStats {
        self.ddr_controller.stats()
    }

    /// Get the number of outstanding requests
    pub fn outstanding_count(&self) -> u64 {
        self.outstanding_requests
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.l1_cache.reset_stats();
        self.l2_cache.reset_stats();
        self.l3_cache.reset_stats();
        self.ddr_controller.reset_stats();
    }

    /// Get combined memory statistics
    pub fn get_stats(&self) -> CacheHierarchyStats {
        CacheHierarchyStats {
            l1_stats: self.l1_cache.stats().clone(),
            l2_stats: self.l2_cache.stats().clone(),
            l3_stats: self.l3_cache.stats().clone(),
            ddr_stats: self.ddr_controller.stats().clone(),
            lsq_occupancy: self.lsq.occupancy(),
            lsq_capacity: self.config.lsq_size,
            outstanding_requests: self.outstanding_requests,
        }
    }
}

/// Memory request state
#[derive(Debug, Clone)]
pub struct MemoryRequest {
    pub instruction_id: InstructionId,
    pub state: MemoryRequestState,
    pub complete_cycle: Option<u64>,
    /// Cache access information for visualization
    pub cache_info: Option<CacheAccessInfo>,
}

impl MemoryRequest {
    pub fn completed(id: InstructionId, cycle: u64) -> Self {
        Self {
            instruction_id: id,
            state: MemoryRequestState::Completed,
            complete_cycle: Some(cycle),
            cache_info: None,
        }
    }

    pub fn completed_with_cache_info(id: InstructionId, cycle: u64, cache_info: CacheAccessInfo) -> Self {
        Self {
            instruction_id: id,
            state: MemoryRequestState::Completed,
            complete_cycle: Some(cycle),
            cache_info: Some(cache_info),
        }
    }

    pub fn pending(id: InstructionId) -> Self {
        Self {
            instruction_id: id,
            state: MemoryRequestState::Pending,
            complete_cycle: None,
            cache_info: None,
        }
    }

    pub fn is_completed(&self) -> bool {
        matches!(self.state, MemoryRequestState::Completed)
    }
}

/// Memory request state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRequestState {
    Pending,
    Completed,
}

/// Cache hierarchy statistics (detailed)
#[derive(Debug, Clone)]
pub struct CacheHierarchyStats {
    pub l1_stats: CacheStats,
    pub l2_stats: CacheStats,
    pub l3_stats: CacheStats,
    pub ddr_stats: DdrStats,
    pub lsq_occupancy: usize,
    pub lsq_capacity: usize,
    pub outstanding_requests: u64,
}

// Re-export for backward compatibility
pub use CacheHierarchyStats as DetailedMemoryStats;

/// CHI-integrated memory subsystem
pub struct ChiMemorySubsystem {
    /// Load/Store Queue
    lsq: LoadStoreQueue,
    /// L1 Data Cache
    l1_cache: Cache,
    /// L2 Cache
    l2_cache: Cache,
    /// Memory controller (fallback when CHI not in use)
    controller: MemoryController,
    /// CHI system (when enabled)
    chi_system: Option<ChiSystem>,
    /// Configuration
    config: CPUConfig,
    /// Current cycle
    current_cycle: u64,
    /// Outstanding memory requests
    outstanding_requests: u64,
    /// Pending CHI transactions (txn_id -> instruction_id)
    pending_chi_txns: std::collections::HashMap<ChiTxnId, (InstructionId, u64)>,
    /// Completed CHI transactions ready for pickup
    completed_chi_txns: std::collections::VecDeque<(InstructionId, u64)>,
}

impl ChiMemorySubsystem {
    /// Create a new CHI-integrated memory subsystem
    pub fn new(config: CPUConfig) -> Result<Self> {
        let lsq = LoadStoreQueue::new(
            config.lsq_size,
            config.load_pipeline_count,
            config.store_pipeline_count,
        );

        let l1_config = CacheConfig {
            size: config.l1_size,
            associativity: config.l1_associativity,
            line_size: config.l1_line_size,
            hit_latency: config.l1_hit_latency,
            name: "L1".to_string(),
        };
        let l1_cache = Cache::new(l1_config)?;

        let l2_config = CacheConfig {
            size: config.l2_size,
            associativity: config.l2_associativity,
            line_size: config.l2_line_size,
            hit_latency: config.l2_hit_latency,
            name: "L2".to_string(),
        };
        let l2_cache = Cache::new(l2_config)?;

        let controller = MemoryController::new(config.l2_miss_latency, config.outstanding_requests);

        // Create CHI system if enabled
        let chi_system = if config.enable_chi {
            Some(Self::create_chi_system(&config)?)
        } else {
            None
        };

        Ok(Self {
            lsq,
            l1_cache,
            l2_cache,
            controller,
            chi_system,
            config,
            current_cycle: 0,
            outstanding_requests: 0,
            pending_chi_txns: std::collections::HashMap::new(),
            completed_chi_txns: std::collections::VecDeque::new(),
        })
    }

    /// Create the CHI system
    fn create_chi_system(config: &CPUConfig) -> Result<ChiSystem> {
        // Create RN-F node
        let rnf_config = ChiNodeConfig {
            node_id: config.chi_rnf_node_id,
            node_type: crate::chi::ChiNodeType::RnF,
            ..Default::default()
        };
        let l1_cache_config = CacheConfig {
            size: config.l1_size,
            associativity: config.l1_associativity,
            line_size: config.l1_line_size,
            hit_latency: config.l1_hit_latency,
            name: "L1".to_string(),
        };
        let l2_cache_config = CacheConfig {
            size: config.l2_size,
            associativity: config.l2_associativity,
            line_size: config.l2_line_size,
            hit_latency: config.l2_hit_latency,
            name: "L2".to_string(),
        };
        let rn_f = RnFNode::new(
            rnf_config,
            l1_cache_config,
            l2_cache_config,
            NodeId(config.chi_hnf_node_id),
        )?;

        // Create HN-F node
        let hnf_config = ChiNodeConfig {
            node_id: config.chi_hnf_node_id,
            node_type: crate::chi::ChiNodeType::HnF,
            ..Default::default()
        };
        let hn_f = HnFNode::new(
            hnf_config,
            config.chi_directory_size,
            config.l2_line_size,
            NodeId(config.chi_snf_node_id),
            config.l2_miss_latency,
        );

        // Create SN-F node
        let snf_config = ChiNodeConfig {
            node_id: config.chi_snf_node_id,
            node_type: crate::chi::ChiNodeType::SnF,
            ..Default::default()
        };
        let sn_f = SnFNode::new(snf_config, config.l2_miss_latency, 32);

        // Create interconnect
        let interconnect_config = ChiInterconnectConfig {
            req_latency: config.chi_request_latency,
            rsp_latency: config.chi_response_latency,
            dat_latency: config.chi_data_latency,
            snp_latency: config.chi_snoop_latency,
        };

        Ok(ChiSystem::new_single_core(rn_f, hn_f, sn_f, interconnect_config))
    }

    /// Process a load request
    pub fn load(&mut self, id: InstructionId, access: &MemAccess) -> MemoryRequest {
        // Add to LSQ
        let lsq_entry = self.lsq.add_load(id, access.addr, access.size);

        // Check L1 cache
        let l1_result = self.l1_cache.access(access.addr, true);

        match l1_result {
            Ok(hit) if hit => {
                // L1 hit
                let complete_cycle = self.current_cycle + self.config.l1_hit_latency;
                self.lsq.complete(lsq_entry);
                MemoryRequest::completed(id, complete_cycle)
            }
            _ => {
                // L1 miss, check L2
                let l2_result = self.l2_cache.access(access.addr, true);

                match l2_result {
                    Ok(hit) if hit => {
                        // L2 hit
                        let complete_cycle = self.current_cycle + self.config.l2_hit_latency;
                        self.l1_cache.fill_line(access.addr);
                        self.lsq.complete(lsq_entry);
                        MemoryRequest::completed(id, complete_cycle)
                    }
                    _ => {
                        // L2 miss - use CHI if enabled
                        if let Some(ref mut chi_system) = self.chi_system {
                            self.handle_chi_l2_miss(id, access, false)
                        } else {
                            // Fallback to simple memory controller
                            self.handle_simple_l2_miss(id, access, lsq_entry)
                        }
                    }
                }
            }
        }
    }

    /// Process a store request
    pub fn store(&mut self, id: InstructionId, access: &MemAccess) -> MemoryRequest {
        // Add to LSQ
        let lsq_entry = self.lsq.add_store(id, access.addr, access.size);

        // Check if we have write permission in cache
        let l1_result = self.l1_cache.access(access.addr, false);

        match l1_result {
            Ok(hit) if hit => {
                // L1 hit - can write (simplified: assume write-through)
                self.lsq.complete(lsq_entry);
                MemoryRequest::completed(id, self.current_cycle + 1)
            }
            _ => {
                // L1 miss - check L2
                let l2_result = self.l2_cache.access(access.addr, false);

                match l2_result {
                    Ok(hit) if hit => {
                        // L2 hit
                        self.l1_cache.fill_line(access.addr);
                        self.lsq.complete(lsq_entry);
                        MemoryRequest::completed(id, self.current_cycle + self.config.l2_hit_latency + 1)
                    }
                    _ => {
                        // L2 miss - use CHI if enabled
                        if let Some(ref mut _chi_system) = self.chi_system {
                            // For stores, need write unique first
                            // Simplified: just complete after latency
                            self.l2_cache.fill_line(access.addr);
                            self.l1_cache.fill_line(access.addr);
                            self.lsq.complete(lsq_entry);
                            MemoryRequest::completed(id, self.current_cycle + self.config.l2_miss_latency + 1)
                        } else {
                            // Fallback
                            self.l2_cache.fill_line(access.addr);
                            self.l1_cache.fill_line(access.addr);
                            self.lsq.complete(lsq_entry);
                            MemoryRequest::completed(id, self.current_cycle + self.config.l2_miss_latency + 1)
                        }
                    }
                }
            }
        }
    }

    /// Handle L2 miss using CHI protocol
    fn handle_chi_l2_miss(&mut self, id: InstructionId, access: &MemAccess, want_unique: bool) -> MemoryRequest {
        if let Some(ref mut chi_system) = self.chi_system {
            if let Some(rn_f) = chi_system.primary_rn_f_mut() {
                // Send CHI read request
                if let Some(txn_id) = rn_f.send_read_request(id, access.addr, access.size, want_unique) {
                    // Track pending transaction
                    self.pending_chi_txns.insert(txn_id, (id, self.current_cycle));
                    return MemoryRequest::pending(id);
                }
            }
        }

        // Fallback if CHI request failed
        MemoryRequest::pending(id)
    }

    /// Handle L2 miss using simple memory controller
    fn handle_simple_l2_miss(
        &mut self,
        id: InstructionId,
        access: &MemAccess,
        lsq_entry: crate::memory::lsq::LsqHandle,
    ) -> MemoryRequest {
        if self.outstanding_requests < self.config.outstanding_requests as u64 {
            self.outstanding_requests += 1;
            let complete_cycle = self.current_cycle + self.config.l2_miss_latency;
            self.l2_cache.fill_line(access.addr);
            self.l1_cache.fill_line(access.addr);
            self.lsq.complete(lsq_entry);
            MemoryRequest::completed(id, complete_cycle)
        } else {
            MemoryRequest::pending(id)
        }
    }

    /// Process CHI system and check for completions
    fn process_chi(&mut self) {
        if let Some(ref mut chi_system) = self.chi_system {
            // Run CHI simulation step
            chi_system.step();

            // Check for completed transactions in RN-F
            if let Some(rn_f) = chi_system.primary_rn_f() {
                // Check outstanding transactions
                // In a real implementation, we'd track which transactions completed
            }
        }
    }

    /// Check for completed memory operations
    pub fn poll_completions(&mut self) -> Vec<(InstructionId, u64)> {
        let mut completed = Vec::new();

        // Check simple memory controller completions
        let mem_completed = self.controller.poll_completed(self.current_cycle);
        for req in mem_completed {
            self.outstanding_requests = self.outstanding_requests.saturating_sub(1);
            completed.push((req.instruction_id, self.current_cycle));
        }

        // Check CHI completions
        if let Some(ref mut chi_system) = self.chi_system {
            if let Some(rn_f) = chi_system.primary_rn_f() {
                // Check if any pending transactions have completed
                // This is simplified - in reality we'd track transaction state
            }
        }

        // Add any queued CHI completions
        while let Some((id, cycle)) = self.completed_chi_txns.pop_front() {
            completed.push((id, cycle));
        }

        completed
    }

    /// Advance simulation by one cycle
    pub fn advance_cycle(&mut self) {
        self.current_cycle += 1;

        // Check for completed memory requests
        let completed = self.controller.poll_completed(self.current_cycle);
        for _ in completed {
            self.outstanding_requests = self.outstanding_requests.saturating_sub(1);
        }

        // Process CHI system
        self.process_chi();
    }

    /// Get the current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get L1 cache statistics
    pub fn l1_stats(&self) -> &CacheStats {
        self.l1_cache.stats()
    }

    /// Get L2 cache statistics
    pub fn l2_stats(&self) -> &CacheStats {
        self.l2_cache.stats()
    }

    /// Get the number of outstanding requests
    pub fn outstanding_count(&self) -> u64 {
        self.outstanding_requests
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.l1_cache.reset_stats();
        self.l2_cache.reset_stats();
    }

    /// Get combined memory statistics
    pub fn get_stats(&self) -> CacheHierarchyStats {
        CacheHierarchyStats {
            l1_stats: self.l1_cache.stats().clone(),
            l2_stats: self.l2_cache.stats().clone(),
            l3_stats: CacheStats::default(),
            ddr_stats: DdrStats::default(),
            lsq_occupancy: self.lsq.occupancy(),
            lsq_capacity: self.config.lsq_size,
            outstanding_requests: self.outstanding_requests,
        }
    }

    /// Check if CHI is enabled
    pub fn is_chi_enabled(&self) -> bool {
        self.chi_system.is_some()
    }

    /// Get CHI system reference (if enabled)
    pub fn chi_system(&self) -> Option<&ChiSystem> {
        self.chi_system.as_ref()
    }

    /// Get mutable CHI system reference (if enabled)
    pub fn chi_system_mut(&mut self) -> Option<&mut ChiSystem> {
        self.chi_system.as_mut()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_subsystem() {
        let config = CPUConfig::minimal();
        let mut mem = MemorySubsystem::new(config).unwrap();

        let access = MemAccess {
            addr: 0x1000,
            size: 8,
            is_load: true,
        };

        let req = mem.load(InstructionId(0), &access);
        assert!(req.is_completed());
    }
}
