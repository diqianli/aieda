//! Out-of-Order execution engine for the ARM CPU emulator.

mod dependency;
mod window;
mod scheduler;
pub mod parallel;

pub use dependency::{DependencyTracker, DependencyInfo};
pub use window::{InstructionWindow, WindowEntry};
pub use scheduler::Scheduler;
pub use parallel::{BatchSimulator, BatchResult, InstructionBatch, ParallelConfig};

use crate::config::CPUConfig;
use crate::types::{Instruction, InstructionId, InstrStatus, Result};
use std::collections::BTreeMap;

/// Out-of-Order execution engine
pub struct OoOEngine {
    /// Configuration
    config: CPUConfig,
    /// Instruction window
    window: InstructionWindow,
    /// Dependency tracker
    dependency_tracker: DependencyTracker,
    /// Scheduler
    scheduler: Scheduler,
    /// Current cycle
    current_cycle: u64,
    /// Next instruction ID to commit
    next_commit_id: u64,
    /// Instructions scheduled to complete, keyed by completion cycle
    /// Map from complete_cycle -> Vec<InstructionId>
    pending_completions: BTreeMap<u64, Vec<InstructionId>>,
}

impl OoOEngine {
    /// Create a new out-of-order engine
    pub fn new(config: CPUConfig) -> Result<Self> {
        let window = InstructionWindow::new(config.window_size);
        let dependency_tracker = DependencyTracker::new();
        let scheduler = Scheduler::new(config.issue_width, config.commit_width);

        Ok(Self {
            config,
            window,
            dependency_tracker,
            scheduler,
            current_cycle: 0,
            next_commit_id: 0,
            pending_completions: BTreeMap::new(),
        })
    }

    /// Check if the engine can accept more instructions
    pub fn can_accept(&self) -> bool {
        self.window.has_space()
    }

    /// Get the number of free slots in the window
    pub fn free_slots(&self) -> usize {
        self.window.free_slots()
    }

    /// Dispatch an instruction into the window
    /// Returns the dependencies for visualization
    pub fn dispatch(&mut self, instr: Instruction) -> Result<Vec<DependencyInfo>> {
        // Get the instruction ID
        let entry_id = instr.id;

        // Register dependencies first (needs reference to instr)
        let dependencies = self.dependency_tracker.register_instruction(&instr, entry_id, self.current_cycle);

        // Add to window (moves instr)
        self.window.insert(instr)?;

        // Check if immediately ready
        if self.dependency_tracker.is_ready(entry_id) {
            self.window.mark_ready(entry_id);
            self.scheduler.add_ready(entry_id);
        }

        Ok(dependencies)
    }

    /// Get instructions ready to execute
    pub fn get_ready_instructions(&mut self) -> Vec<(InstructionId, Instruction)> {
        self.scheduler.get_ready(&mut self.window)
    }

    /// Mark an instruction as executing
    pub fn mark_executing(&mut self, id: InstructionId) {
        self.window.mark_executing(id);
    }

    /// Mark an instruction as completed (execution finished)
    /// This schedules the completion for the specified cycle.
    /// The status will be set to Completed when process_completions is called.
    pub fn mark_completed(&mut self, id: InstructionId, complete_cycle: u64) {
        // Only set the complete_cycle, not the status
        // Status will be set to Completed in process_completions
        self.window.set_complete_cycle(id, complete_cycle);

        // Schedule the completion for the specified cycle
        // Dependencies will be released when process_completions is called
        self.pending_completions
            .entry(complete_cycle)
            .or_insert_with(Vec::new)
            .push(id);
    }

    /// Process all completions that are due at or before the current cycle
    /// This releases dependencies and wakes up dependent instructions
    pub fn process_completions(&mut self) {
        // Get all completion cycles that are due (<= current_cycle)
        let due_cycles: Vec<u64> = self.pending_completions
            .keys()
            .filter(|&&cycle| cycle <= self.current_cycle)
            .copied()
            .collect();

        // Process completions for each due cycle
        for cycle in due_cycles {
            if let Some(ids) = self.pending_completions.remove(&cycle) {
                for id in ids {
                    // Get dependents BEFORE releasing (release removes the list)
                    let dependents = self.dependency_tracker.get_dependents(id);

                    // Debug: log for specific instructions
                    if id.0 <= 10 || (id.0 >= 118 && id.0 <= 122) {
                        tracing::debug!("Instruction {} completing at cycle {}, has {} dependents",
                            id.0, cycle, dependents.len());
                    }

                    // Mark instruction as Completed NOW (when completion is processed)
                    self.window.set_status_completed(id);

                    // Mark completion as processed (allows commit)
                    self.window.mark_completion_processed(id);

                    // Release dependencies - we only need the id, not the entry
                    // (the instruction may have already been committed and removed from window)
                    self.dependency_tracker.release_dependencies_by_id(id);

                    // Wake up dependent instructions
                    for &dep_id in &dependents {
                        let pending = self.dependency_tracker.pending_count(dep_id);
                        let is_ready = self.dependency_tracker.is_ready(dep_id);
                        if id.0 <= 10 || (id.0 >= 118 && id.0 <= 122) {
                            tracing::debug!("  Dependent {} pending: {}, ready: {}", dep_id.0, pending, is_ready);
                        }
                        if is_ready {
                            self.window.mark_ready(dep_id);
                            self.scheduler.add_ready(dep_id);
                        }
                    }
                }
            }
        }
    }

    /// Get instructions ready to commit (in program order)
    pub fn get_commit_candidates(&mut self) -> Vec<Instruction> {
        let mut candidates = Vec::new();
        let commit_width = self.config.commit_width;

        for _ in 0..commit_width {
            let id = InstructionId(self.next_commit_id);
            if let Some(entry) = self.window.get_entry(id) {
                // Must have completion processed (dependencies released) AND status Completed
                if entry.status == InstrStatus::Completed && entry.completion_processed {
                    candidates.push(entry.instruction.clone());
                    self.next_commit_id += 1;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        candidates
    }

    /// Commit an instruction
    pub fn commit(&mut self, id: InstructionId) {
        self.window.remove(id);
    }

    /// Advance the simulation by one cycle
    pub fn advance_cycle(&mut self) {
        self.current_cycle += 1;
    }

    /// Process completions for the current cycle
    /// This should be called before commit() in each cycle
    pub fn cycle_tick(&mut self) {
        self.process_completions();
    }

    /// Get the current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get the number of instructions in the window
    pub fn window_size(&self) -> usize {
        self.window.len()
    }

    /// Check if the window is empty
    pub fn is_empty(&self) -> bool {
        self.window.is_empty()
    }

    /// Get statistics about the engine state
    pub fn get_stats(&self) -> OoOStats {
        OoOStats {
            window_occupancy: self.window.len(),
            window_capacity: self.config.window_size,
            ready_count: self.scheduler.ready_count(),
            current_cycle: self.current_cycle,
            next_commit_id: self.next_commit_id,
        }
    }

    /// Get instruction status counts for debugging
    pub fn status_counts(&self) -> (usize, usize, usize, usize) {
        self.window.status_counts()
    }

    /// Get window entry for debugging
    pub fn get_window_entry(&self, id: InstructionId) -> Option<&WindowEntry> {
        self.window.get_entry_debug(id)
    }

    /// Get the next commit ID
    pub fn next_commit_id(&self) -> u64 {
        self.next_commit_id
    }

    /// Get all window entries for visualization
    pub fn get_window_entries(&self) -> impl Iterator<Item = &WindowEntry> {
        self.window.iter()
    }

    /// Get the dependency tracker for visualization
    pub fn dependency_tracker(&self) -> &DependencyTracker {
        &self.dependency_tracker
    }
}

/// Statistics about the OoO engine
#[derive(Debug, Clone, Copy)]
pub struct OoOStats {
    pub window_occupancy: usize,
    pub window_capacity: usize,
    pub ready_count: usize,
    pub current_cycle: u64,
    pub next_commit_id: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OpcodeType, Reg};

    #[test]
    fn test_ooo_engine_basic() {
        let config = CPUConfig::minimal();
        let mut engine = OoOEngine::new(config).unwrap();

        let instr1 = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Add)
            .with_src_reg(Reg(0))
            .with_src_reg(Reg(1))
            .with_dst_reg(Reg(2));

        let instr2 = Instruction::new(InstructionId(1), 0x1004, 0, OpcodeType::Add)
            .with_src_reg(Reg(2))
            .with_dst_reg(Reg(3));

        engine.dispatch(instr1).unwrap();
        engine.dispatch(instr2).unwrap();

        assert_eq!(engine.window_size(), 2);

        // First instruction should be ready
        let ready = engine.get_ready_instructions();
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].0, InstructionId(0));
    }
}
