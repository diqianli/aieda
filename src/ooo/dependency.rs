//! Dependency tracking for out-of-order execution.

use crate::types::{Instruction, InstructionId, Reg};
use ahash::AHashMap;
use std::collections::HashSet;

/// Dependency information for visualization
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    /// Producer instruction ID
    pub producer: InstructionId,
    /// Whether this is a memory dependency
    pub is_memory: bool,
}

/// Tracks dependencies between instructions
pub struct DependencyTracker {
    /// Maps each register to the instruction that last wrote to it
    register_producers: AHashMap<Reg, InstructionId>,

    /// Maps each instruction to its unresolved dependencies count
    pending_dependencies: AHashMap<InstructionId, usize>,

    /// Maps each instruction to the instructions that depend on it
    dependents: AHashMap<InstructionId, Vec<InstructionId>>,

    /// Memory dependencies: maps address to last store instruction
    memory_producers: AHashMap<u64, InstructionId>,

    /// Track which instructions have memory dependencies
    memory_dependents: AHashMap<InstructionId, Vec<InstructionId>>,

    /// Track which instructions have completed (for avoiding stale dependencies)
    completed_instructions: HashSet<InstructionId>,
}

impl DependencyTracker {
    /// Create a new dependency tracker
    pub fn new() -> Self {
        Self {
            register_producers: AHashMap::new(),
            pending_dependencies: AHashMap::new(),
            dependents: AHashMap::new(),
            memory_producers: AHashMap::new(),
            memory_dependents: AHashMap::new(),
            completed_instructions: HashSet::new(),
        }
    }

    /// Register a new instruction and set up its dependencies
    /// Returns a list of dependencies for visualization
    pub fn register_instruction(&mut self, instr: &Instruction, id: InstructionId, _current_cycle: u64) -> Vec<DependencyInfo> {
        let mut deps_count = 0;
        let mut dependencies = Vec::new();

        // Register register dependencies (RAW - Read After Write)
        for &src_reg in &instr.src_regs {
            if let Some(&producer_id) = self.register_producers.get(&src_reg) {
                if producer_id != id && !self.completed_instructions.contains(&producer_id) {
                    self.add_dependency(producer_id, id);
                    deps_count += 1;
                    dependencies.push(DependencyInfo {
                        producer: producer_id,
                        is_memory: false,
                    });
                }
            }
        }

        // Register memory dependencies for loads/stores
        if let Some(ref mem_access) = instr.mem_access {
            let addr = mem_access.addr;

            if mem_access.is_load {
                // Load: depends on previous store to the same address
                if let Some(&producer_id) = self.memory_producers.get(&addr) {
                    if producer_id != id && !self.completed_instructions.contains(&producer_id) {
                        self.add_memory_dependency(producer_id, id);
                        deps_count += 1;
                        dependencies.push(DependencyInfo {
                            producer: producer_id,
                            is_memory: true,
                        });
                    }
                }
            } else {
                // Store: depends on previous store to the same address
                // Also, future loads/stores will depend on this store
                if let Some(&producer_id) = self.memory_producers.get(&addr) {
                    if producer_id != id && !self.completed_instructions.contains(&producer_id) {
                        self.add_memory_dependency(producer_id, id);
                        deps_count += 1;
                        dependencies.push(DependencyInfo {
                            producer: producer_id,
                            is_memory: true,
                        });
                    }
                }
            }
        }

        // Update register producers for destination registers
        for &dst_reg in &instr.dst_regs {
            self.register_producers.insert(dst_reg, id);
        }

        // Update memory producer for stores
        if let Some(ref mem_access) = instr.mem_access {
            if !mem_access.is_load {
                self.memory_producers.insert(mem_access.addr, id);
            }
        }

        // Store pending count (0 means ready)
        self.pending_dependencies.insert(id, deps_count);

        // Debug: log for specific instructions
        if id.0 <= 10 {
            tracing::debug!("Instruction {} registered with {} pending dependencies, is_ready: {}",
                id.0, deps_count, deps_count == 0);
        }

        // Debug: log specific dependency chain
        if id.0 == 2 {
            tracing::info!("Instruction 2 (MUL) registered with {} deps", deps_count);
        }
        if id.0 == 4 {
            tracing::info!("Instruction 4 (SUB) registered with {} deps, depends on: {:?}",
                deps_count, dependencies.iter().map(|d| d.producer.0).collect::<Vec<_>>());
        }

        dependencies
    }

    /// Add a register dependency
    fn add_dependency(&mut self, producer: InstructionId, consumer: InstructionId) {
        // If the producer has already completed, don't add the dependency
        if self.completed_instructions.contains(&producer) {
            return;
        }

        self.dependents
            .entry(producer)
            .or_insert_with(Vec::new)
            .push(consumer);
    }

    /// Add a memory dependency
    fn add_memory_dependency(&mut self, producer: InstructionId, consumer: InstructionId) {
        self.memory_dependents
            .entry(producer)
            .or_insert_with(Vec::new)
            .push(consumer);

        // Also add to regular dependents for unified tracking
        self.add_dependency(producer, consumer);
    }

    /// Check if an instruction is ready (all dependencies resolved)
    pub fn is_ready(&self, id: InstructionId) -> bool {
        let pending = self.pending_dependencies.get(&id).copied().unwrap_or(0);
        let ready = pending == 0;

        // Debug: log for specific instructions
        if id.0 == 4 {
            tracing::info!("is_ready check for instruction 4: pending={}, ready={}", pending, ready);
        }

        ready
    }

    /// Release dependencies when an instruction completes
    pub fn release_dependencies(&mut self, _instr: &Instruction, id: InstructionId) {
        self.release_dependencies_by_id(id);
    }

    /// Release dependencies when an instruction completes (only needs ID)
    pub fn release_dependencies_by_id(&mut self, id: InstructionId) {
        // Mark this instruction as completed (so future consumers don't wait for it)
        self.completed_instructions.insert(id);

        // Decrement pending count for all dependents
        if let Some(dependents) = self.dependents.remove(&id) {
            for &dep_id in &dependents {
                if let Some(count) = self.pending_dependencies.get_mut(&dep_id) {
                    *count = count.saturating_sub(1);
                }
            }
        }

        // Clean up memory dependents
        self.memory_dependents.remove(&id);
    }

    /// Get all instructions that depend on the given instruction
    pub fn get_dependents(&self, id: InstructionId) -> Vec<InstructionId> {
        self.dependents.get(&id).cloned().unwrap_or_default()
    }

    /// Get the number of pending dependencies for an instruction
    pub fn pending_count(&self, id: InstructionId) -> usize {
        self.pending_dependencies.get(&id).copied().unwrap_or(0)
    }

    /// Clear all tracking state
    pub fn clear(&mut self) {
        self.register_producers.clear();
        self.pending_dependencies.clear();
        self.dependents.clear();
        self.memory_producers.clear();
        self.memory_dependents.clear();
        self.completed_instructions.clear();
    }

    /// Get statistics
    pub fn get_stats(&self) -> DependencyStats {
        DependencyStats {
            register_producers: self.register_producers.len(),
            pending_instructions: self.pending_dependencies.len(),
            total_dependents: self.dependents.values().map(|v| v.len()).sum(),
        }
    }

    /// Get all current dependencies for visualization
    pub fn get_all_dependencies(&self) -> Vec<(InstructionId, InstructionId, bool)> {
        let mut deps = Vec::new();

        // Get register dependencies
        for (&producer, dependents) in &self.dependents {
            for &consumer in dependents {
                // Check if this is a memory dependency
                let is_memory = self.memory_dependents
                    .get(&producer)
                    .map(|mem_deps| mem_deps.contains(&consumer))
                    .unwrap_or(false);
                deps.push((producer, consumer, is_memory));
            }
        }

        deps
    }

    /// Get pending dependency count for an instruction
    pub fn get_pending_count(&self, id: InstructionId) -> usize {
        self.pending_dependencies.get(&id).copied().unwrap_or(0)
    }
}

impl Default for DependencyTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about dependency tracking
#[derive(Debug, Clone, Copy)]
pub struct DependencyStats {
    pub register_producers: usize,
    pub pending_instructions: usize,
    pub total_dependents: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{MemAccess, OpcodeType};

    #[test]
    fn test_register_dependency() {
        let mut tracker = DependencyTracker::new();

        // Instruction 0: ADD X2, X0, X1 (writes X2)
        let instr0 = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Add)
            .with_src_reg(Reg(0))
            .with_src_reg(Reg(1))
            .with_dst_reg(Reg(2));

        // Instruction 1: ADD X3, X2, X0 (reads X2, depends on instr0)
        let instr1 = Instruction::new(InstructionId(1), 0x1004, 0, OpcodeType::Add)
            .with_src_reg(Reg(2))
            .with_src_reg(Reg(0))
            .with_dst_reg(Reg(3));

        tracker.register_instruction(&instr0, InstructionId(0), 0);
        tracker.register_instruction(&instr1, InstructionId(1), 0);

        // Instr0 should be ready immediately
        assert!(tracker.is_ready(InstructionId(0)));

        // Instr1 should not be ready (depends on instr0)
        assert!(!tracker.is_ready(InstructionId(1)));

        // Release instr0
        tracker.release_dependencies(&instr0, InstructionId(0));

        // Now instr1 should be ready
        assert!(tracker.is_ready(InstructionId(1)));
    }

    #[test]
    fn test_memory_dependency() {
        let mut tracker = DependencyTracker::new();

        // Store to address 0x1000
        let store = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Store)
            .with_src_reg(Reg(0))
            .with_mem_access(0x1000, 8, false);

        // Load from address 0x1000 (depends on store)
        let load = Instruction::new(InstructionId(1), 0x1004, 0, OpcodeType::Load)
            .with_dst_reg(Reg(1))
            .with_mem_access(0x1000, 8, true);

        tracker.register_instruction(&store, InstructionId(0), 0);
        tracker.register_instruction(&load, InstructionId(1), 0);

        // Store should be ready
        assert!(tracker.is_ready(InstructionId(0)));

        // Load should wait for store
        assert!(!tracker.is_ready(InstructionId(1)));

        // Release store
        tracker.release_dependencies(&store, InstructionId(0));

        // Now load should be ready
        assert!(tracker.is_ready(InstructionId(1)));
    }

    #[test]
    fn test_independent_instructions() {
        let mut tracker = DependencyTracker::new();

        // Two independent instructions
        let instr0 = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Add)
            .with_src_reg(Reg(0))
            .with_dst_reg(Reg(1));

        let instr1 = Instruction::new(InstructionId(1), 0x1004, 0, OpcodeType::Add)
            .with_src_reg(Reg(2))
            .with_dst_reg(Reg(3));

        tracker.register_instruction(&instr0, InstructionId(0), 0);
        tracker.register_instruction(&instr1, InstructionId(1), 0);

        // Both should be ready
        assert!(tracker.is_ready(InstructionId(0)));
        assert!(tracker.is_ready(InstructionId(1)));
    }
}
