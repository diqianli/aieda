//! Pipeline stage tracker for simulation events.
//!
//! This module provides a tracker that records pipeline stage timing
//! for each instruction based on simulation events.

use crate::types::{Instruction, InstructionId};
use ahash::AHashMap;
use std::collections::VecDeque;

/// Pipeline stage timing information for a single instruction
#[derive(Debug, Clone, Default)]
pub struct StageTiming {
    /// Fetch start cycle
    pub fetch_start: Option<u64>,
    /// Fetch end cycle
    pub fetch_end: Option<u64>,
    /// Decode start cycle
    pub decode_start: Option<u64>,
    /// Decode end cycle
    pub decode_end: Option<u64>,
    /// Rename start cycle
    pub rename_start: Option<u64>,
    /// Rename end cycle
    pub rename_end: Option<u64>,
    /// Dispatch start cycle
    pub dispatch_start: Option<u64>,
    /// Dispatch end cycle
    pub dispatch_end: Option<u64>,
    /// Issue start cycle
    pub issue_start: Option<u64>,
    /// Issue end cycle
    pub issue_end: Option<u64>,
    /// Execute start cycle
    pub execute_start: Option<u64>,
    /// Execute end cycle
    pub execute_end: Option<u64>,
    /// Memory start cycle
    pub memory_start: Option<u64>,
    /// Memory end cycle
    pub memory_end: Option<u64>,
    /// Complete cycle
    pub complete_cycle: Option<u64>,
    /// Retire cycle
    pub retire_cycle: Option<u64>,
}

impl StageTiming {
    /// Create new stage timing
    pub fn new() -> Self {
        Self::default()
    }

    /// Record fetch stage
    pub fn record_fetch(&mut self, start: u64, end: u64) {
        self.fetch_start = Some(start);
        self.fetch_end = Some(end.max(start));
    }

    /// Record decode stage
    pub fn record_decode(&mut self, start: u64, end: u64) {
        self.decode_start = Some(start);
        self.decode_end = Some(end.max(start));
    }

    /// Record rename stage
    pub fn record_rename(&mut self, start: u64, end: u64) {
        self.rename_start = Some(start);
        self.rename_end = Some(end.max(start));
    }

    /// Record dispatch stage
    pub fn record_dispatch(&mut self, start: u64, end: u64) {
        self.dispatch_start = Some(start);
        self.dispatch_end = Some(end.max(start));
    }

    /// Record issue stage
    pub fn record_issue(&mut self, start: u64, end: u64) {
        self.issue_start = Some(start);
        self.issue_end = Some(end.max(start));
    }

    /// Record execute stage
    pub fn record_execute(&mut self, start: u64, end: u64) {
        self.execute_start = Some(start);
        self.execute_end = Some(end.max(start));
    }

    /// Record memory stage
    pub fn record_memory(&mut self, start: u64, end: u64) {
        self.memory_start = Some(start);
        self.memory_end = Some(end.max(start));
    }

    /// Record complete
    pub fn record_complete(&mut self, cycle: u64) {
        self.complete_cycle = Some(cycle);
    }

    /// Record retire
    pub fn record_retire(&mut self, cycle: u64) {
        self.retire_cycle = Some(cycle);
    }

    /// Convert to Konata-style stage list
    pub fn to_stages(&self) -> Vec<StageInfo> {
        let mut stages = Vec::new();

        if let (Some(start), Some(end)) = (self.fetch_start, self.fetch_end) {
            stages.push(StageInfo {
                name: "F".to_string(),
                start_cycle: start,
                end_cycle: end,
            });
        }

        if let (Some(start), Some(end)) = (self.decode_start, self.decode_end) {
            stages.push(StageInfo {
                name: "Dc".to_string(),
                start_cycle: start,
                end_cycle: end,
            });
        }

        if let (Some(start), Some(end)) = (self.rename_start, self.rename_end) {
            stages.push(StageInfo {
                name: "Rn".to_string(),
                start_cycle: start,
                end_cycle: end,
            });
        }

        if let (Some(start), Some(end)) = (self.dispatch_start, self.dispatch_end) {
            stages.push(StageInfo {
                name: "Ds".to_string(),
                start_cycle: start,
                end_cycle: end,
            });
        }

        if let (Some(start), Some(end)) = (self.issue_start, self.issue_end) {
            stages.push(StageInfo {
                name: "Is".to_string(),
                start_cycle: start,
                end_cycle: end,
            });
        }

        // Execute or Memory stage (mutually exclusive for most instructions)
        if let (Some(start), Some(end)) = (self.memory_start, self.memory_end) {
            stages.push(StageInfo {
                name: "Me".to_string(),
                start_cycle: start,
                end_cycle: end,
            });
        } else if let (Some(start), Some(end)) = (self.execute_start, self.execute_end) {
            stages.push(StageInfo {
                name: "Ex".to_string(),
                start_cycle: start,
                end_cycle: end,
            });
        }

        if let Some(cycle) = self.complete_cycle {
            stages.push(StageInfo {
                name: "Cm".to_string(),
                start_cycle: cycle,
                end_cycle: cycle,
            });
        }

        if let Some(cycle) = self.retire_cycle {
            stages.push(StageInfo {
                name: "Rt".to_string(),
                start_cycle: cycle,
                end_cycle: cycle,
            });
        }

        stages
    }
}

/// Information about a single pipeline stage
#[derive(Debug, Clone)]
pub struct StageInfo {
    /// Stage name (F, Dc, Rn, Ds, Is, Ex, Me, Cm, Rt)
    pub name: String,
    /// Start cycle
    pub start_cycle: u64,
    /// End cycle
    pub end_cycle: u64,
}

/// Dependency reference for Konata output
#[derive(Debug, Clone)]
pub struct DependencyRef {
    /// Producer instruction visualization ID
    pub producer_id: u64,
    /// Dependency type
    pub dep_type: DependencyType,
}

/// Dependency type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyType {
    /// Register dependency
    Register,
    /// Memory dependency
    Memory,
}

/// Instruction info for output
#[derive(Debug, Clone)]
pub struct TrackedInstruction {
    /// Visualization ID (sequential)
    pub viz_id: u64,
    /// Program ID (instruction ID)
    pub program_id: u64,
    /// Program counter
    pub pc: u64,
    /// Disassembly text
    pub disasm: String,
    /// Stage timing
    pub timing: StageTiming,
    /// Source registers
    pub src_regs: Vec<u16>,
    /// Destination registers
    pub dst_regs: Vec<u16>,
    /// Whether this is a memory operation
    pub is_memory: bool,
    /// Memory address (if memory op)
    pub mem_addr: Option<u64>,
    /// Dependencies
    pub dependencies: Vec<DependencyRef>,
}

/// Pipeline tracker that records instruction timing from simulation events
pub struct PipelineTracker {
    /// Stage timing for each instruction
    timings: AHashMap<InstructionId, StageTiming>,
    /// Instruction info for output
    instructions: AHashMap<InstructionId, TrackedInstruction>,
    /// Instructions in program order
    order: VecDeque<InstructionId>,
    /// Mapping from instruction ID to visualization ID
    viz_id_map: AHashMap<InstructionId, u64>,
    /// Next visualization ID
    next_viz_id: u64,
    /// Retire counter
    retire_counter: u64,
    /// Maximum completed instructions to keep
    max_completed: usize,
    /// Completed instruction IDs
    completed: VecDeque<InstructionId>,
    /// Dependencies
    dependencies: AHashMap<InstructionId, Vec<DependencyRef>>,
}

impl PipelineTracker {
    /// Create a new pipeline tracker
    pub fn new() -> Self {
        Self {
            timings: AHashMap::new(),
            instructions: AHashMap::new(),
            order: VecDeque::new(),
            viz_id_map: AHashMap::new(),
            next_viz_id: 0,
            retire_counter: 0,
            max_completed: 10000,
            completed: VecDeque::new(),
            dependencies: AHashMap::new(),
        }
    }

    /// Create with custom max completed size
    pub fn with_max_completed(max: usize) -> Self {
        Self {
            max_completed: max,
            ..Self::new()
        }
    }

    /// Get or assign a visualization ID
    fn get_or_assign_viz_id(&mut self, id: InstructionId) -> u64 {
        *self.viz_id_map.entry(id).or_insert_with(|| {
            let viz_id = self.next_viz_id;
            self.next_viz_id += 1;
            viz_id
        })
    }

    /// Record instruction fetch
    pub fn record_fetch(&mut self, instr: &Instruction, cycle: u64) {
        let id = instr.id;
        let viz_id = self.get_or_assign_viz_id(id);

        // Initialize tracking
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        timing.record_fetch(cycle, cycle + 1);

        // Store instruction info
        self.instructions.entry(id).or_insert_with(|| TrackedInstruction {
            viz_id,
            program_id: id.0,
            pc: instr.pc,
            disasm: instr.disasm.clone().unwrap_or_else(|| format!("{:?}", instr.opcode_type)),
            timing: StageTiming::new(),
            src_regs: instr.src_regs.iter().map(|r| r.0 as u16).collect(),
            dst_regs: instr.dst_regs.iter().map(|r| r.0 as u16).collect(),
            is_memory: instr.mem_access.is_some(),
            mem_addr: instr.mem_access.as_ref().map(|m| m.addr),
            dependencies: Vec::new(),
        });

        // Add to order if new
        if !self.order.contains(&id) {
            self.order.push_back(id);
        }
    }

    /// Record instruction decode
    pub fn record_decode(&mut self, id: InstructionId, start: u64, end: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        timing.record_decode(start, end);
    }

    /// Record instruction rename
    pub fn record_rename(&mut self, id: InstructionId, start: u64, end: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        timing.record_rename(start, end);
    }

    /// Record instruction dispatch
    pub fn record_dispatch(&mut self, id: InstructionId, cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);

        // Infer decode/rename if not set
        let fetch_end = timing.fetch_end.unwrap_or(cycle);
        if timing.decode_start.is_none() {
            if cycle > fetch_end {
                let available = cycle - fetch_end;
                if available >= 2 {
                    timing.record_decode(fetch_end, fetch_end + 1);
                    timing.record_rename(fetch_end + 1, fetch_end + 2);
                } else {
                    timing.record_decode(fetch_end, cycle);
                    timing.record_rename(fetch_end, cycle);
                }
            } else {
                timing.record_decode(cycle, cycle);
                timing.record_rename(cycle, cycle);
            }
        }
        timing.record_dispatch(cycle, cycle);
    }

    /// Record instruction issue
    pub fn record_issue(&mut self, id: InstructionId, cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        let dispatch_end = timing.dispatch_end.unwrap_or(cycle);
        timing.record_issue(dispatch_end, cycle.max(dispatch_end));
    }

    /// Record instruction execute
    pub fn record_execute(&mut self, id: InstructionId, start: u64, end: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        let issue_end = timing.issue_end.unwrap_or(start);
        timing.record_execute(issue_end.max(start), end.max(issue_end.max(start)));
    }

    /// Record memory access
    pub fn record_memory(&mut self, id: InstructionId, start: u64, end: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        let issue_end = timing.issue_end.unwrap_or(start);
        timing.record_memory(issue_end.max(start), end.max(issue_end.max(start)));
    }

    /// Record instruction complete
    pub fn record_complete(&mut self, id: InstructionId, cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        timing.record_complete(cycle);

        // Track completed
        if !self.completed.contains(&id) {
            self.completed.push_back(id);
            if self.completed.len() > self.max_completed {
                self.completed.pop_front();
            }
        }
    }

    /// Record instruction retire
    pub fn record_retire(&mut self, id: InstructionId, cycle: u64) {
        let timing = self.timings.entry(id).or_insert_with(StageTiming::new);
        timing.record_retire(cycle);
        self.retire_counter += 1;
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, consumer: InstructionId, producer: InstructionId, is_memory: bool) {
        let viz_producer_id = self.get_or_assign_viz_id(producer);
        let dep_type = if is_memory {
            DependencyType::Memory
        } else {
            DependencyType::Register
        };

        let deps = self.dependencies.entry(consumer).or_insert_with(Vec::new);
        if !deps.iter().any(|d| d.producer_id == viz_producer_id && d.dep_type == dep_type) {
            deps.push(DependencyRef {
                producer_id: viz_producer_id,
                dep_type,
            });
        }
    }

    /// Get timing for an instruction
    pub fn get_timing(&self, id: InstructionId) -> Option<&StageTiming> {
        self.timings.get(&id)
    }

    /// Get visualization ID for an instruction
    pub fn get_viz_id(&self, id: InstructionId) -> Option<u64> {
        self.viz_id_map.get(&id).copied()
    }

    /// Get the retire counter
    pub fn retire_count(&self) -> u64 {
        self.retire_counter
    }

    /// Get number of tracked instructions
    pub fn len(&self) -> usize {
        self.timings.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.timings.is_empty()
    }

    /// Clear all tracking data
    pub fn clear(&mut self) {
        self.timings.clear();
        self.instructions.clear();
        self.order.clear();
        self.viz_id_map.clear();
        self.next_viz_id = 0;
        self.retire_counter = 0;
        self.completed.clear();
        self.dependencies.clear();
    }

    /// Export all tracked instructions with their stages
    pub fn export_instructions(&self) -> Vec<TrackedInstruction> {
        let mut result = Vec::new();

        for id in &self.order {
            if let (Some(timing), Some(viz_id)) =
                (self.timings.get(id), self.viz_id_map.get(id))
            {
                if let Some(instr) = self.instructions.get(id) {
                    let mut instr_clone = instr.clone();
                    instr_clone.timing = timing.clone();
                    if let Some(deps) = self.dependencies.get(id) {
                        instr_clone.dependencies = deps.clone();
                    }
                    result.push(instr_clone);
                }
            }
        }

        result
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
    use crate::types::{Instruction, OpcodeType, Reg};

    fn make_test_instruction(id: u64, pc: u64) -> Instruction {
        Instruction::new(InstructionId(id), pc, 0, OpcodeType::Add)
            .with_src_reg(Reg(0))
            .with_dst_reg(Reg(1))
    }

    #[test]
    fn test_full_pipeline_tracking() {
        let mut tracker = PipelineTracker::new();
        let instr = make_test_instruction(0, 0x1000);

        tracker.record_fetch(&instr, 0);
        tracker.record_dispatch(InstructionId(0), 2);
        tracker.record_issue(InstructionId(0), 3);
        tracker.record_execute(InstructionId(0), 3, 5);
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

        tracker.add_dependency(InstructionId(1), InstructionId(0), false);

        let deps = tracker.dependencies.get(&InstructionId(1)).unwrap();
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].dep_type, DependencyType::Register);
    }
}
