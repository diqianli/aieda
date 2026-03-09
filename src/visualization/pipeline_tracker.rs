//! Pipeline stage tracker for Konata visualization.
//!
//! This module tracks detailed pipeline stage timing for each instruction,
//! enabling the generation of Konata-compatible visualization data.

use crate::types::{Instruction, InstructionId};
use crate::ooo::WindowEntry;
use ahash::AHashMap;
use std::collections::VecDeque;

use super::konata_format::{
    KonataDependencyRef, KonataDependencyType, KonataOp, KonataSnapshot, KonataStage,
    StageId, StageTiming,
};

/// Tracks pipeline stages for all instructions.
pub struct PipelineTracker {
    /// Stage timing for each instruction, keyed by instruction ID
    pub timings: AHashMap<InstructionId, StageTiming>,
    /// Instructions in program order
    order: VecDeque<InstructionId>,
    /// Mapping from instruction ID to sequential visualization ID
    viz_id_map: AHashMap<InstructionId, u64>,
    /// Next visualization ID to assign
    next_viz_id: u64,
    /// Retire order counter
    retire_counter: u64,
    /// Maximum number of completed instructions to keep
    max_completed: usize,
    /// Completed instruction IDs (for dependency tracking)
    completed: VecDeque<InstructionId>,
    /// Dependencies recorded
    dependencies: AHashMap<InstructionId, Vec<KonataDependencyRef>>,
    /// Fetch width (max instructions per cycle)
    fetch_width: usize,
    /// Count of instructions fetched in current cycle
    fetch_count_in_cycle: usize,
    /// Current fetch cycle (adjusted for fetch width)
    current_fetch_cycle: u64,
}

impl PipelineTracker {
    /// Create a new pipeline tracker.
    pub fn new() -> Self {
        Self {
            timings: AHashMap::new(),
            order: VecDeque::new(),
            viz_id_map: AHashMap::new(),
            next_viz_id: 0,
            retire_counter: 0,
            max_completed: 1000,
            completed: VecDeque::new(),
            dependencies: AHashMap::new(),
            fetch_width: 8,
            fetch_count_in_cycle: 0,
            current_fetch_cycle: 0,
        }
    }

    /// Create a tracker with custom fetch width.
    pub fn with_fetch_width(fetch_width: usize) -> Self {
        Self {
            fetch_width,
            ..Self::new()
        }
    }

    /// Create a tracker with a custom max completed size.
    pub fn with_max_completed(max: usize) -> Self {
        Self {
            max_completed: max,
            ..Self::new()
        }
    }

    /// Record an instruction being fetched.
    /// This respects fetch width limits - only fetch_width instructions can be fetched per cycle.
    pub fn record_fetch(&mut self, instr: &Instruction, cycle: u64) {
        let id = instr.id;
        let viz_id = self.get_or_assign_viz_id(id);

        // Calculate the actual fetch cycle based on fetch width limit
        // Start at cycle 0, count instructions in this cycle
        if self.fetch_count_in_cycle >= self.fetch_width {
            self.current_fetch_cycle += 1;
            self.fetch_count_in_cycle = 0;
        } else if self.fetch_count_in_cycle == 0 {
            // First instruction - start at cycle 0
            self.current_fetch_cycle = 0;
            self.fetch_count_in_cycle = 0;
        }

        // Use the adjusted fetch cycle, but ensure it's at least the provided cycle
        let adjusted_cycle = std::cmp::max(self.current_fetch_cycle, cycle);
        self.fetch_count_in_cycle += 1;

        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        // Fetch stage: 1 cycle (using adjusted cycle for fetch width limit)
        timing.record_fetch(adjusted_cycle, adjusted_cycle + 1);

        // Note: Decode and Rename stages are now recorded dynamically
        // when dispatch happens, not pre-computed here.
        // This allows for accurate timing when instructions are
        // fetched and dispatched in the same cycle.

        // Note: Decode and Rename stages are now recorded dynamically
        // when dispatch happens, not pre-computed here.
        // This allows for accurate timing when instructions are
        // fetched and dispatched in the same cycle.

        // Add to order if new
        if !self.order.contains(&id) {
            self.order.push_back(id);
        }

        // Record dependencies
        self.record_instruction_dependencies(instr);
    }

    /// Record an instruction being decoded.
    pub fn record_decode(&mut self, id: InstructionId, start_cycle: u64, end_cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        timing.record_decode(start_cycle, end_cycle);
    }

    /// Record an instruction being renamed.
    pub fn record_rename(&mut self, id: InstructionId, start_cycle: u64, end_cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        timing.record_rename(start_cycle, end_cycle);
    }

    /// Record an instruction being dispatched to the window.
    /// This also records Decode and Rename stages if not already recorded.
    pub fn record_dispatch(&mut self, id: InstructionId, cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);

        // Get fetch end cycle
        let fetch_end = timing.fetch_end.unwrap_or(cycle);

        // If dispatch cycle is after fetch, we can infer decode and rename stages
        // Otherwise, all stages (fetch, decode, rename, dispatch) happen in the same cycle
        if cycle > fetch_end {
            // We have time for decode and rename stages between fetch and dispatch
            // Calculate how many cycles we have for decode + rename
            let available_cycles = cycle - fetch_end;

            if available_cycles >= 2 {
                // Normal case: 1 cycle each for decode and rename
                // Record Decode stage if not already recorded
                if timing.decode_start.is_none() {
                    timing.record_decode(fetch_end, fetch_end + 1);
                }
                let decode_end = timing.decode_end.unwrap_or(fetch_end + 1);

                // Record Rename stage if not already recorded
                if timing.rename_start.is_none() {
                    timing.record_rename(decode_end, decode_end + 1);
                }
                let rename_end = timing.rename_end.unwrap_or(decode_end + 1);

                // Dispatch: starts at rename_end, ends at cycle
                let dispatch_start = rename_end;
                let dispatch_end = cycle;
                timing.record_dispatch(dispatch_start, dispatch_end);
            } else if available_cycles == 1 {
                // Compressed: decode and rename share 1 cycle
                if timing.decode_start.is_none() {
                    timing.record_decode(fetch_end, cycle);
                }
                if timing.rename_start.is_none() {
                    timing.record_rename(fetch_end, cycle);
                }
                // Dispatch is zero-duration at cycle
                timing.record_dispatch(cycle, cycle);
            } else {
                // available_cycles == 0: all stages happen at the same cycle
                if timing.decode_start.is_none() {
                    timing.record_decode(fetch_end, fetch_end);
                }
                if timing.rename_start.is_none() {
                    timing.record_rename(fetch_end, fetch_end);
                }
                timing.record_dispatch(fetch_end, fetch_end);
            }
        } else {
            // cycle <= fetch_end: dispatch happens at or before fetch_end
            // All stages are zero-duration or collapsed
            if timing.decode_start.is_none() {
                timing.record_decode(cycle, cycle);
            }
            if timing.rename_start.is_none() {
                timing.record_rename(cycle, cycle);
            }
            timing.record_dispatch(cycle, cycle);
        }
    }

    /// Record an instruction becoming ready (all operands available).
    /// This marks the end of the issue wait period.
    pub fn record_ready(&mut self, id: InstructionId, cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        timing.ready_cycle = Some(cycle);
    }

    /// Record an instruction being issued for execution.
    /// The issue stage represents the time from when the instruction becomes ready
    /// (dispatch ends) to when it's selected for execution.
    pub fn record_issue(&mut self, id: InstructionId, cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);

        // Issue stage starts when instruction is ready (dispatch end)
        let dispatch_end = timing.dispatch_end.unwrap_or(cycle);

        // Issue starts at dispatch_end (when instruction enters window and becomes ready)
        // Issue ends when the instruction is actually selected for execution
        // If dispatch hasn't been recorded yet, use the current cycle
        let issue_start = dispatch_end;
        let issue_end = std::cmp::max(dispatch_end, cycle);

        timing.record_issue(issue_start, issue_end);
    }

    /// Record an instruction starting execution.
    pub fn record_execute_start(&mut self, id: InstructionId, cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        let issue_start = timing.issue_start.unwrap_or(cycle);
        timing.record_execute(issue_start, cycle);
    }

    /// Record an instruction completing execution.
    /// Execute stage spans from issue (when execution starts) to complete_cycle.
    pub fn record_execute_end(&mut self, id: InstructionId, cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);

        // Execute starts when instruction is issued (issue_end)
        let exec_start = timing.issue_end.unwrap_or(cycle);

        // Execute ends at completion cycle
        // Ensure exec_end >= exec_start (at least a zero-length stage)
        let exec_end = std::cmp::max(exec_start, cycle);

        timing.record_execute(exec_start, exec_end);
    }

    /// Record a memory operation.
    /// Memory stage spans from issue (when address is computed) to data return.
    pub fn record_memory(&mut self, id: InstructionId, start_cycle: u64, end_cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);

        // Memory starts when instruction is issued (issue_end)
        let mem_start = timing.issue_end.unwrap_or(start_cycle);

        // Ensure mem_end >= mem_start (at least a zero-length stage)
        let mem_end = std::cmp::max(mem_start, end_cycle);

        timing.record_memory(mem_start, mem_end);
    }

    /// Record an instruction completing.
    pub fn record_complete(&mut self, id: InstructionId, cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);

        // Only set execute/memory end if not already set
        // (record_execute_end or record_memory may have already set the proper timing)
        if timing.memory_start.is_some() && timing.memory_end.is_none() {
            timing.record_memory(timing.memory_start.unwrap(), cycle);
        } else if timing.execute_start.is_some() && timing.execute_end.is_none() {
            timing.record_execute(timing.execute_start.unwrap(), cycle);
        }

        timing.record_complete(cycle);

        // Track completed instructions
        if !self.completed.contains(&id) {
            self.completed.push_back(id);
            if self.completed.len() > self.max_completed {
                self.completed.pop_front();
            }
        }
    }

    /// Record an instruction being retired/committed.
    pub fn record_retire(&mut self, id: InstructionId, cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        timing.record_retire(cycle);

        // Assign retire order ID
        self.retire_counter += 1;
    }

    /// Record dependencies from an instruction.
    fn record_instruction_dependencies(&mut self, instr: &Instruction) {
        // Dependencies are recorded separately and merged when generating Konata ops
        // This is called during fetch to set up the initial structure
        let _ = instr; // Placeholder - actual dependency tracking happens via add_dependency
    }

    /// Add a dependency between instructions.
    pub fn add_dependency(&mut self, consumer: InstructionId, producer: InstructionId, is_memory: bool) {
        let dep_type = if is_memory {
            KonataDependencyType::Memory
        } else {
            KonataDependencyType::Register
        };

        // Get or assign viz_id first to avoid borrow issues
        let viz_producer_id = self.get_or_assign_viz_id(producer);

        // Then get or create the dependencies vector
        let deps = self.dependencies.entry(consumer).or_insert_with(Vec::new);

        // Avoid duplicate dependencies
        if !deps.iter().any(|d| d.producer_id == viz_producer_id && d.dep_type == dep_type) {
            deps.push(KonataDependencyRef {
                producer_id: viz_producer_id,
                dep_type,
            });
        }
    }

    /// Get or assign a visualization ID for an instruction.
    fn get_or_assign_viz_id(&mut self, id: InstructionId) -> u64 {
        *self.viz_id_map.entry(id).or_insert_with(|| {
            let viz_id = self.next_viz_id;
            self.next_viz_id += 1;
            viz_id
        })
    }

    /// Get the visualization ID for an instruction (if assigned).
    pub fn get_viz_id(&self, id: InstructionId) -> Option<u64> {
        self.viz_id_map.get(&id).copied()
    }

    /// Convert tracked data to Konata operations.
    /// `current_cycle` is used to show in-progress stages for instructions that haven't completed yet.
    pub fn to_konata_ops<'a>(&self, entries: impl Iterator<Item = &'a WindowEntry>, current_cycle: u64) -> Vec<KonataOp> {
        let mut ops = Vec::new();

        for entry in entries {
            let id = entry.instruction.id;

            // Get visualization ID
            let viz_id = match self.viz_id_map.get(&id) {
                Some(&vid) => vid,
                None => continue, // Skip if not tracked
            };

            // Get timing
            let mut timing = self.timings.get(&id).cloned().unwrap_or_default();

            // Debug: log timing for specific instructions
            if id.0 <= 5 {
                tracing::debug!(
                    "to_konata_ops: Instr {} issue_start={:?} issue_end={:?} execute_start={:?} execute_end={:?} complete_cycle={:?}",
                    id.0, timing.issue_start, timing.issue_end,
                    timing.execute_start, timing.execute_end, timing.complete_cycle
                );
            }

            // For instructions that are waiting (dispatched but not yet issued),
            // add an in-progress Issue stage from dispatch_end to current_cycle
            if timing.issue_start.is_none() && timing.dispatch_end.is_some() {
                let dispatch_end = timing.dispatch_end.unwrap();
                // Issue stage is in progress - from dispatch_end to current_cycle
                timing.record_issue(dispatch_end, current_cycle);
            }

            // For instructions that have issued but not completed execution,
            // add in-progress Execute stage
            if timing.execute_start.is_none() && timing.issue_end.is_some() && timing.complete_cycle.is_none() {
                let issue_end = timing.issue_end.unwrap();
                // Execute stage is in progress - from issue_end to current_cycle
                timing.record_execute(issue_end, current_cycle);
            }

            // Create Konata operation
            let mut op = KonataOp::new(
                viz_id,
                id.0, // Use instruction ID as gid
                entry.instruction.pc,
                entry.instruction.disasm.as_ref()
                    .map(|s| s.clone())
                    .unwrap_or_else(|| format!("{:?}", entry.instruction.opcode_type)),
            );

            op.fetched_cycle = timing.fetch_start.unwrap_or(0);
            op.retired_cycle = timing.retire_cycle;

            // Add stages
            for stage in timing.to_stages() {
                let stage_id = match stage.name.as_str() {
                    "F" => StageId::F,
                    "Dc" => StageId::Dc,
                    "Rn" => StageId::Rn,
                    "Ds" => StageId::Ds,
                    "Is" => StageId::Is,
                    "Ex" => StageId::Ex,
                    "Me" => StageId::Me,
                    "Cm" => StageId::Cm,
                    "Rt" => StageId::Rt,
                    _ => continue,
                };
                op.add_stage(stage_id, stage.start_cycle, stage.end_cycle);
            }

            // Add registers
            op.src_regs = entry.instruction.src_regs.iter().map(|r| r.0 as u16).collect();
            op.dst_regs = entry.instruction.dst_regs.iter().map(|r| r.0 as u16).collect();

            // Add memory info
            if let Some(ref mem) = entry.instruction.mem_access {
                op.is_memory = true;
                op.mem_addr = Some(mem.addr);
            }

            // Add dependencies
            if let Some(deps) = self.dependencies.get(&id) {
                op.prods = deps.clone();
            }

            ops.push(op);
        }

        // Sort by visualization ID
        ops.sort_by_key(|op| op.id);

        ops
    }

    /// Generate a complete Konata snapshot.
    pub fn to_snapshot<'a>(
        &self,
        entries: impl Iterator<Item = &'a WindowEntry>,
        cycle: u64,
        committed_count: u64,
    ) -> KonataSnapshot {
        let ops = self.to_konata_ops(entries, cycle);

        let mut snapshot = KonataSnapshot::new(cycle, committed_count);
        for op in ops {
            snapshot.add_op(op);
        }

        snapshot
    }

    /// Clear all tracking data.
    pub fn clear(&mut self) {
        self.timings.clear();
        self.order.clear();
        self.viz_id_map.clear();
        self.next_viz_id = 0;
        self.retire_counter = 0;
        self.completed.clear();
        self.dependencies.clear();
    }

    /// Get the number of tracked instructions.
    pub fn len(&self) -> usize {
        self.timings.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.timings.is_empty()
    }

    /// Get timing for a specific instruction.
    pub fn get_timing(&self, id: InstructionId) -> Option<&StageTiming> {
        self.timings.get(&id)
    }

    /// Get the retire counter.
    pub fn retire_count(&self) -> u64 {
        self.retire_counter
    }
}

impl Default for PipelineTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OpcodeType, Reg};

    fn make_test_instruction(id: u64, pc: u64) -> Instruction {
        Instruction::new(InstructionId(id), pc, 0, OpcodeType::Add)
            .with_src_reg(Reg(0))
            .with_dst_reg(Reg(1))
    }

    #[test]
    fn test_track_fetch() {
        let mut tracker = PipelineTracker::new();
        let instr = make_test_instruction(0, 0x1000);

        tracker.record_fetch(&instr, 0);

        let timing = tracker.get_timing(InstructionId(0)).unwrap();
        assert!(timing.fetch_start.is_some());
    }

    #[test]
    fn test_full_pipeline_tracking() {
        let mut tracker = PipelineTracker::new();
        let instr = make_test_instruction(0, 0x1000);

        tracker.record_fetch(&instr, 0);
        tracker.record_dispatch(InstructionId(0), 2);
        tracker.record_issue(InstructionId(0), 3);
        tracker.record_execute_end(InstructionId(0), 5);
        tracker.record_complete(InstructionId(0), 5);
        tracker.record_retire(InstructionId(0), 6);

        let timing = tracker.get_timing(InstructionId(0)).unwrap();
        assert_eq!(timing.fetch_start, Some(0));
        assert_eq!(timing.retire_cycle, Some(6));
    }

    #[test]
    fn test_dependencies() {
        let mut tracker = PipelineTracker::new();

        let instr0 = make_test_instruction(0, 0x1000);
        let instr1 = make_test_instruction(1, 0x1004);

        tracker.record_fetch(&instr0, 0);
        tracker.record_fetch(&instr1, 1);

        // instr1 depends on instr0
        tracker.add_dependency(InstructionId(1), InstructionId(0), false);

        let deps = tracker.dependencies.get(&InstructionId(1)).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dep_type, KonataDependencyType::Register);
    }
}
