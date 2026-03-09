//! API interface for programmatic instruction input.

use crate::types::{BranchInfo, EmulatorError, Instruction, InstructionId, MemAccess, OpcodeType, Reg, Result};
use std::collections::VecDeque;

/// In-memory trace input for programmatic use
pub struct TraceInput {
    instructions: VecDeque<Instruction>,
    current_id: u64,
    total_count: usize,
}

impl TraceInput {
    /// Create a new empty trace input
    pub fn new() -> Self {
        Self {
            instructions: VecDeque::new(),
            current_id: 0,
            total_count: 0,
        }
    }

    /// Create a trace input with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            instructions: VecDeque::with_capacity(capacity),
            current_id: 0,
            total_count: 0,
        }
    }

    /// Create from an existing vector of instructions
    pub fn from_vec(instructions: Vec<Instruction>) -> Self {
        let total_count = instructions.len();
        Self {
            instructions: instructions.into(),
            current_id: 0,
            total_count,
        }
    }

    /// Add a single instruction
    pub fn push(&mut self, instr: Instruction) {
        self.instructions.push_back(instr);
        self.total_count += 1;
    }

    /// Add multiple instructions
    pub fn extend<I: IntoIterator<Item = Instruction>>(&mut self, instructions: I) {
        let iter = instructions.into_iter();
        let hint = iter.size_hint().0;
        if hint > self.instructions.capacity() - self.instructions.len() {
            self.instructions.reserve(hint);
        }
        for instr in iter {
            self.instructions.push_back(instr);
            self.total_count += 1;
        }
    }

    /// Create an instruction builder for convenient construction
    pub fn builder(&mut self, pc: u64, opcode_type: OpcodeType) -> InstructionBuilder<'_> {
        InstructionBuilder {
            input: self,
            pc,
            raw_opcode: 0,
            opcode_type,
            src_regs: Vec::new(),
            dst_regs: Vec::new(),
            mem_access: None,
            branch_info: None,
            disasm: None,
        }
    }

    /// Get the number of remaining instructions
    pub fn remaining(&self) -> usize {
        self.instructions.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Clear all instructions
    pub fn clear(&mut self) {
        self.instructions.clear();
        self.current_id = 0;
        self.total_count = 0;
    }

    /// Peek at the next instruction without consuming it
    pub fn peek(&self) -> Option<&Instruction> {
        self.instructions.front()
    }
}

impl Default for TraceInput {
    fn default() -> Self {
        Self::new()
    }
}

impl Iterator for TraceInput {
    type Item = Result<Instruction>;

    fn next(&mut self) -> Option<Self::Item> {
        self.instructions.pop_front().map(Ok)
    }
}

impl super::InstructionSource for TraceInput {
    fn total_count(&self) -> Option<usize> {
        Some(self.total_count)
    }

    fn reset(&mut self) -> Result<()> {
        // Cannot reset in-memory input
        Err(EmulatorError::InternalError(
            "Cannot reset in-memory trace input".to_string()
        ))
    }
}

/// Builder for creating instructions
pub struct InstructionBuilder<'a> {
    input: &'a mut TraceInput,
    pc: u64,
    raw_opcode: u32,
    opcode_type: OpcodeType,
    src_regs: Vec<Reg>,
    dst_regs: Vec<Reg>,
    mem_access: Option<MemAccess>,
    branch_info: Option<BranchInfo>,
    disasm: Option<String>,
}

impl<'a> InstructionBuilder<'a> {
    /// Set the raw opcode encoding
    pub fn raw_opcode(mut self, raw: u32) -> Self {
        self.raw_opcode = raw;
        self
    }

    /// Add a source register
    pub fn src_reg(mut self, reg: Reg) -> Self {
        if !self.src_regs.contains(&reg) {
            self.src_regs.push(reg);
        }
        self
    }

    /// Add a destination register
    pub fn dst_reg(mut self, reg: Reg) -> Self {
        if !self.dst_regs.contains(&reg) {
            self.dst_regs.push(reg);
        }
        self
    }

    /// Set memory access info
    pub fn mem_access(mut self, addr: u64, size: u8, is_load: bool) -> Self {
        self.mem_access = Some(MemAccess { addr, size, is_load });
        self
    }

    /// Set branch info
    pub fn branch(mut self, target: u64, is_conditional: bool, is_taken: bool) -> Self {
        self.branch_info = Some(BranchInfo {
            is_conditional,
            target,
            is_taken,
        });
        self
    }

    /// Set disassembly text
    pub fn disasm(mut self, disasm: impl Into<String>) -> Self {
        self.disasm = Some(disasm.into());
        self
    }

    /// Build and add the instruction to the trace
    pub fn build(self) -> InstructionId {
        let id = InstructionId(self.input.current_id);
        self.input.current_id += 1;

        let mut instr = Instruction::new(id, self.pc, self.raw_opcode, self.opcode_type);
        instr.src_regs = self.src_regs.into();
        instr.dst_regs = self.dst_regs.into();
        instr.mem_access = self.mem_access;
        instr.branch_info = self.branch_info;
        instr.disasm = self.disasm;

        self.input.push(instr);
        id
    }
}

/// Helper functions for creating common instruction patterns
pub mod helpers {
    use super::*;

    /// Create a simple compute instruction
    pub fn compute(pc: u64, opcode_type: OpcodeType, srcs: &[Reg], dst: Reg) -> Instruction {
        let mut instr = Instruction::new(InstructionId(0), pc, 0, opcode_type);
        for &src in srcs {
            instr.src_regs.push(src);
        }
        instr.dst_regs.push(dst);
        instr
    }

    /// Create a load instruction
    pub fn load(pc: u64, addr: u64, dst: Reg, size: u8) -> Instruction {
        Instruction::new(InstructionId(0), pc, 0, OpcodeType::Load)
            .with_dst_reg(dst)
            .with_mem_access(addr, size, true)
    }

    /// Create a store instruction
    pub fn store(pc: u64, addr: u64, src: Reg, size: u8) -> Instruction {
        Instruction::new(InstructionId(0), pc, 0, OpcodeType::Store)
            .with_src_reg(src)
            .with_mem_access(addr, size, false)
    }

    /// Create an unconditional branch
    pub fn branch(pc: u64, target: u64) -> Instruction {
        Instruction::new(InstructionId(0), pc, 0, OpcodeType::Branch)
            .with_branch(target, false, true)
    }

    /// Create a conditional branch
    pub fn branch_cond(pc: u64, target: u64, taken: bool) -> Instruction {
        Instruction::new(InstructionId(0), pc, 0, OpcodeType::BranchCond)
            .with_branch(target, true, taken)
    }

    /// Create a NOP instruction
    pub fn nop(pc: u64) -> Instruction {
        Instruction::new(InstructionId(0), pc, 0xD503201F, OpcodeType::Nop)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_input_basic() {
        let mut input = TraceInput::new();

        input.builder(0x1000, OpcodeType::Add)
            .src_reg(Reg(0))
            .src_reg(Reg(1))
            .dst_reg(Reg(2))
            .disasm("ADD X2, X0, X1")
            .build();

        input.builder(0x1004, OpcodeType::Load)
            .dst_reg(Reg(3))
            .mem_access(0x2000, 8, true)
            .build();

        assert_eq!(input.remaining(), 2);

        let instr1 = input.next().unwrap().unwrap();
        assert_eq!(instr1.pc, 0x1000);
        assert_eq!(instr1.opcode_type, OpcodeType::Add);

        let instr2 = input.next().unwrap().unwrap();
        assert_eq!(instr2.pc, 0x1004);
        assert!(instr2.mem_access.is_some());
    }

    #[test]
    fn test_helpers() {
        let instr = helpers::compute(0x1000, OpcodeType::Add, &[Reg(0), Reg(1)], Reg(2));
        assert_eq!(instr.src_regs.len(), 2);
        assert_eq!(instr.dst_regs.len(), 1);

        let ld = helpers::load(0x1004, 0x2000, Reg(3), 8);
        assert!(ld.mem_access.is_some());
        assert!(ld.mem_access.as_ref().unwrap().is_load);

        let st = helpers::store(0x1008, 0x2000, Reg(3), 8);
        assert!(st.mem_access.is_some());
        assert!(!st.mem_access.as_ref().unwrap().is_load);
    }
}
