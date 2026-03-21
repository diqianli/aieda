//! Instruction scheduler for out-of-order execution.

use crate::types::{Instruction, InstructionId};
use super::window::{InstructionWindow, WindowEntry};
use std::collections::VecDeque;

/// Instruction scheduler
pub struct Scheduler {
    /// Issue width (max instructions to issue per cycle)
    issue_width: usize,
    /// Commit width (max instructions to commit per cycle)
    commit_width: usize,
    /// Ready queue (instructions ready to execute)
    ready_queue: VecDeque<InstructionId>,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(issue_width: usize, commit_width: usize) -> Self {
        Self {
            issue_width,
            commit_width,
            ready_queue: VecDeque::new(),
        }
    }

    /// Add an instruction to the ready queue
    pub fn add_ready(&mut self, id: InstructionId) {
        if !self.ready_queue.contains(&id) {
            self.ready_queue.push_back(id);
        }
    }

    /// Get instructions ready to execute (up to issue_width)
    pub fn get_ready(&mut self, window: &mut InstructionWindow) -> Vec<(InstructionId, Instruction)> {
        let mut result = Vec::with_capacity(self.issue_width);
        let mut issued = 0;

        while issued < self.issue_width {
            if let Some(id) = self.ready_queue.pop_front() {
                if let Some(entry) = window.get_entry(id) {
                    if entry.status == crate::types::InstrStatus::Ready {
                        result.push((id, entry.instruction.clone()));
                        window.mark_executing(id);
                        issued += 1;
                    }
                }
            } else {
                break;
            }
        }

        result
    }

    /// Get the number of ready instructions
    pub fn ready_count(&self) -> usize {
        self.ready_queue.len()
    }

    /// Check if there are ready instructions
    pub fn has_ready(&self) -> bool {
        !self.ready_queue.is_empty()
    }

    /// Clear the ready queue
    pub fn clear(&mut self) {
        self.ready_queue.clear();
    }

    /// Get instructions ready to commit (in program order)
    pub fn get_commit_candidates(&self, window: &InstructionWindow, next_commit_id: u64) -> Vec<InstructionId> {
        let mut candidates = Vec::with_capacity(self.commit_width);

        for i in 0..self.commit_width {
            let id = InstructionId(next_commit_id + i as u64);
            if let Some(entry) = window.get_entry(id) {
                if entry.status == crate::types::InstrStatus::Completed {
                    candidates.push(id);
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        candidates
    }
}

/// Execution unit type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionUnit {
    /// Integer ALU
    IntAlu,
    /// Integer multiply/divide
    IntMul,
    /// Load unit
    Load,
    /// Store unit
    Store,
    /// Branch unit
    Branch,
    /// FP/SIMD unit
    FpSimd,
    /// System instructions unit
    System,
    /// Cryptography unit
    Crypto,
}

impl ExecutionUnit {
    /// Determine the execution unit for an instruction
    pub fn for_instruction(instr: &Instruction) -> Self {
        use crate::types::OpcodeType;

        match instr.opcode_type {
            // ALU instructions
            OpcodeType::Add | OpcodeType::Sub |
            OpcodeType::And | OpcodeType::Orr | OpcodeType::Eor |
            OpcodeType::Lsl | OpcodeType::Lsr | OpcodeType::Asr |
            OpcodeType::Mov | OpcodeType::Cmp | OpcodeType::Shift |
            OpcodeType::Other => ExecutionUnit::IntAlu,

            // Multiply/divide instructions
            OpcodeType::Mul | OpcodeType::Div => ExecutionUnit::IntMul,

            // Load instructions
            OpcodeType::Load | OpcodeType::LoadPair => ExecutionUnit::Load,

            // Store instructions
            OpcodeType::Store | OpcodeType::StorePair => ExecutionUnit::Store,

            // Branch instructions
            OpcodeType::Branch | OpcodeType::BranchCond | OpcodeType::BranchReg => ExecutionUnit::Branch,

            // Floating-point/SIMD instructions (existing)
            OpcodeType::Fadd | OpcodeType::Fsub | OpcodeType::Fmul | OpcodeType::Fdiv => ExecutionUnit::FpSimd,

            // === New instructions ===
            // Cache maintenance instructions - use System unit
            OpcodeType::DcZva | OpcodeType::DcCivac | OpcodeType::DcCvac |
            OpcodeType::DcCsw | OpcodeType::IcIvau | OpcodeType::IcIallu |
            OpcodeType::IcIalluis => ExecutionUnit::System,

            // Cryptography instructions - use dedicated Crypto unit
            OpcodeType::Aesd | OpcodeType::Aese | OpcodeType::Aesimc | OpcodeType::Aesmc |
            OpcodeType::Sha1H | OpcodeType::Sha256H | OpcodeType::Sha512H => ExecutionUnit::Crypto,

            // SIMD/Vector instructions
            OpcodeType::Vadd | OpcodeType::Vsub | OpcodeType::Vmul |
            OpcodeType::Vmla | OpcodeType::Vmls |
            OpcodeType::Vdup | OpcodeType::Vmov => ExecutionUnit::FpSimd,

            // Vector load/store
            OpcodeType::Vld => ExecutionUnit::Load,
            OpcodeType::Vst => ExecutionUnit::Store,

            // FMA instructions
            OpcodeType::Fmadd | OpcodeType::Fmsub |
            OpcodeType::Fnmadd | OpcodeType::Fnmsub => ExecutionUnit::FpSimd,

            // System instructions
            OpcodeType::Msr | OpcodeType::Mrs | OpcodeType::Sys | OpcodeType::Nop |
            OpcodeType::Dmb | OpcodeType::Dsb | OpcodeType::Isb | OpcodeType::Eret |
            OpcodeType::Yield => ExecutionUnit::IntAlu,

            // Floating-point convert
            OpcodeType::Fcvt => ExecutionUnit::FpSimd,

            // PC-relative addressing
            OpcodeType::Adr => ExecutionUnit::IntAlu,

            // Polynomial multiply - use Crypto unit
            OpcodeType::Pmull => ExecutionUnit::Crypto,
        }
    }

    /// Get the number of available units of this type
    pub fn count(&self) -> usize {
        match self {
            ExecutionUnit::IntAlu => 4,
            ExecutionUnit::IntMul => 1,
            ExecutionUnit::Load => 2,
            ExecutionUnit::Store => 1,
            ExecutionUnit::Branch => 1,
            ExecutionUnit::FpSimd => 2,
            ExecutionUnit::System => 1,
            ExecutionUnit::Crypto => 1,
        }
    }
}

/// Execution pipeline for tracking in-flight operations
pub struct ExecutionPipeline {
    /// Unit type
    unit_type: ExecutionUnit,
    /// Number of parallel units
    unit_count: usize,
    /// Currently executing instructions with their completion cycles
    executing: Vec<(InstructionId, u64)>,
}

impl ExecutionPipeline {
    /// Create a new execution pipeline
    pub fn new(unit_type: ExecutionUnit) -> Self {
        Self {
            unit_type,
            unit_count: unit_type.count(),
            executing: Vec::new(),
        }
    }

    /// Check if the pipeline has available slots
    pub fn has_capacity(&self) -> bool {
        self.executing.len() < self.unit_count
    }

    /// Issue an instruction to this pipeline
    pub fn issue(&mut self, id: InstructionId, complete_cycle: u64) -> bool {
        if !self.has_capacity() {
            return false;
        }
        self.executing.push((id, complete_cycle));
        true
    }

    /// Get all instructions completing at or before the given cycle
    pub fn complete_by(&mut self, cycle: u64) -> Vec<InstructionId> {
        let (completing, remaining): (Vec<_>, Vec<_>) = self.executing
            .drain(..)
            .partition(|&(_, complete)| complete <= cycle);

        self.executing = remaining;
        completing.into_iter().map(|(id, _)| id).collect()
    }

    /// Get the number of instructions currently executing
    pub fn executing_count(&self) -> usize {
        self.executing.len()
    }

    /// Clear all executing instructions
    pub fn clear(&mut self) {
        self.executing.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OpcodeType, Reg};

    #[test]
    fn test_scheduler_basic() {
        let mut scheduler = Scheduler::new(4, 4);
        let mut window = InstructionWindow::new(16);

        let instr = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Add)
            .with_src_reg(Reg(0))
            .with_dst_reg(Reg(1));

        window.insert(instr).unwrap();
        window.mark_ready(InstructionId(0));
        scheduler.add_ready(InstructionId(0));

        let ready = scheduler.get_ready(&mut window);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].0, InstructionId(0));
    }

    #[test]
    fn test_scheduler_issue_width() {
        let mut scheduler = Scheduler::new(2, 4); // Issue width = 2
        let mut window = InstructionWindow::new(16);

        // Add 4 ready instructions
        for i in 0..4 {
            let instr = Instruction::new(InstructionId(i), 0x1000 + i as u64 * 4, 0, OpcodeType::Nop);
            window.insert(instr).unwrap();
            window.mark_ready(InstructionId(i));
            scheduler.add_ready(InstructionId(i));
        }

        // Should only get 2 (issue width)
        let ready = scheduler.get_ready(&mut window);
        assert_eq!(ready.len(), 2);
    }

    #[test]
    fn test_execution_unit() {
        let add = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Add);
        assert_eq!(ExecutionUnit::for_instruction(&add), ExecutionUnit::IntAlu);

        let load = Instruction::new(InstructionId(1), 0x1004, 0, OpcodeType::Load);
        assert_eq!(ExecutionUnit::for_instruction(&load), ExecutionUnit::Load);

        let branch = Instruction::new(InstructionId(2), 0x1008, 0, OpcodeType::Branch);
        assert_eq!(ExecutionUnit::for_instruction(&branch), ExecutionUnit::Branch);
    }

    #[test]
    fn test_execution_pipeline() {
        let mut pipeline = ExecutionPipeline::new(ExecutionUnit::IntAlu);

        assert!(pipeline.has_capacity());
        assert!(pipeline.issue(InstructionId(0), 10));
        assert!(pipeline.issue(InstructionId(1), 12));

        let completed = pipeline.complete_by(10);
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0], InstructionId(0));
        assert_eq!(pipeline.executing_count(), 1);
    }
}
