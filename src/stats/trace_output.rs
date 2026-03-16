//! Trace output for the CPU emulator.

use crate::types::{Instruction, InstructionId, OpcodeType};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{self, Write};

/// A single entry in the execution trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEntry {
    /// Instruction ID
    pub id: u64,
    /// Program counter
    pub pc: u64,
    /// Opcode type
    pub opcode: String,
    /// Disassembly (if available)
    pub disasm: Option<String>,
    /// Cycle when dispatched
    pub dispatch_cycle: u64,
    /// Cycle when issued
    pub issue_cycle: Option<u64>,
    /// Cycle when completed
    pub complete_cycle: Option<u64>,
    /// Cycle when committed
    pub commit_cycle: Option<u64>,
    /// Execution latency (issue to complete)
    pub exec_latency: Option<u64>,
    /// Memory address (if applicable)
    pub mem_addr: Option<u64>,
    /// Source registers
    pub src_regs: Vec<u16>,
    /// Destination registers
    pub dst_regs: Vec<u16>,
}

impl TraceEntry {
    /// Create a new trace entry
    pub fn new(id: u64, pc: u64, opcode: OpcodeType) -> Self {
        Self {
            id,
            pc,
            opcode: format!("{:?}", opcode),
            disasm: None,
            dispatch_cycle: 0,
            issue_cycle: None,
            complete_cycle: None,
            commit_cycle: None,
            exec_latency: None,
            mem_addr: None,
            src_regs: Vec::new(),
            dst_regs: Vec::new(),
        }
    }

    /// Create from instruction
    pub fn from_instruction(instr: &Instruction) -> Self {
        let mut entry = Self::new(instr.id.0, instr.pc, instr.opcode_type);
        entry.disasm = instr.disasm.clone();
        entry.mem_addr = instr.mem_access.as_ref().map(|m| m.addr);
        entry.src_regs = instr.src_regs.iter().map(|r| r.0 as u16).collect();
        entry.dst_regs = instr.dst_regs.iter().map(|r| r.0 as u16).collect();
        entry
    }

    /// Calculate total latency (dispatch to commit)
    pub fn total_latency(&self) -> Option<u64> {
        match (self.dispatch_cycle, self.commit_cycle) {
            (dispatch, Some(commit)) => Some(commit.saturating_sub(dispatch)),
            _ => None,
        }
    }
}

/// Trace output manager
pub struct TraceOutput {
    /// Collected entries
    entries: VecDeque<TraceEntry>,
    /// Maximum entries to keep
    max_entries: usize,
    /// Whether tracing is enabled
    enabled: bool,
    /// Current cycle
    current_cycle: u64,
}

impl TraceOutput {
    /// Create a new trace output manager
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: VecDeque::with_capacity(max_entries),
            max_entries,
            enabled: true,
            current_cycle: 0,
        }
    }

    /// Create a disabled trace output
    pub fn disabled() -> Self {
        Self {
            entries: VecDeque::new(),
            max_entries: 0,
            enabled: false,
            current_cycle: 0,
        }
    }

    /// Enable tracing
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable tracing
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if tracing is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record instruction dispatch
    pub fn record_dispatch(&mut self, instr: &Instruction, cycle: u64) {
        if !self.enabled {
            return;
        }

        let mut entry = TraceEntry::from_instruction(instr);
        entry.dispatch_cycle = cycle;
        self.add_entry(entry);
    }

    /// Record instruction issue
    pub fn record_issue(&mut self, id: InstructionId, cycle: u64) {
        if !self.enabled {
            return;
        }

        if let Some(entry) = self.find_entry_mut(id) {
            entry.issue_cycle = Some(cycle);
        }
    }

    /// Record instruction complete
    pub fn record_complete(&mut self, id: InstructionId, cycle: u64) {
        if !self.enabled {
            return;
        }

        if let Some(entry) = self.find_entry_mut(id) {
            entry.complete_cycle = Some(cycle);
            if let Some(issue) = entry.issue_cycle {
                entry.exec_latency = Some(cycle.saturating_sub(issue));
            }
        }
    }

    /// Record instruction commit
    pub fn record_commit(&mut self, id: InstructionId, cycle: u64) {
        if !self.enabled {
            return;
        }

        if let Some(entry) = self.find_entry_mut(id) {
            entry.commit_cycle = Some(cycle);
        }
    }

    /// Add an entry to the trace
    fn add_entry(&mut self, entry: TraceEntry) {
        if self.max_entries > 0 && self.entries.len() >= self.max_entries {
            self.entries.pop_front();
        }
        self.entries.push_back(entry);
    }

    /// Find entry by instruction ID
    fn find_entry_mut(&mut self, id: InstructionId) -> Option<&mut TraceEntry> {
        self.entries.iter_mut().rev().find(|e| e.id == id.0)
    }

    /// Get all entries
    pub fn entries(&self) -> &VecDeque<TraceEntry> {
        &self.entries
    }

    /// Get entry count
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Write trace to a writer in text format
    pub fn write_text<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "ID\tPC\tOpcode\tDispatch\tIssue\tComplete\tCommit\tExecLat\tMemAddr")?;

        for entry in &self.entries {
            writeln!(
                writer,
                "{}\t{:#x}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                entry.id,
                entry.pc,
                entry.opcode,
                entry.dispatch_cycle,
                entry.issue_cycle.map_or("-".to_string(), |c| c.to_string()),
                entry.complete_cycle.map_or("-".to_string(), |c| c.to_string()),
                entry.commit_cycle.map_or("-".to_string(), |c| c.to_string()),
                entry.exec_latency.map_or("-".to_string(), |c| c.to_string()),
                entry.mem_addr.map_or("-".to_string(), |a| format!("{:#x}", a))
            )?;
        }

        Ok(())
    }

    /// Write trace to a writer in JSON format
    pub fn write_json<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        let json = serde_json::to_string_pretty(&self.entries)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        write!(writer, "{}", json)
    }

    /// Write trace to a writer in CSV format
    pub fn write_csv<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        writeln!(writer, "id,pc,opcode,dispatch_cycle,issue_cycle,complete_cycle,commit_cycle,exec_latency,mem_addr")?;

        for entry in &self.entries {
            writeln!(
                writer,
                "{},{:#x},{},{},{},{},{},{},{}",
                entry.id,
                entry.pc,
                entry.opcode,
                entry.dispatch_cycle,
                entry.issue_cycle.unwrap_or(0),
                entry.complete_cycle.unwrap_or(0),
                entry.commit_cycle.unwrap_or(0),
                entry.exec_latency.unwrap_or(0),
                entry.mem_addr.unwrap_or(0)
            )?;
        }

        Ok(())
    }

    /// Export trace as string
    pub fn to_string(&self) -> String {
        let mut output = String::new();
        for entry in &self.entries {
            output.push_str(&format!(
                "[{}] {:#x} {} dispatch={} issue={:?} complete={:?} commit={:?}\n",
                entry.id,
                entry.pc,
                entry.opcode,
                entry.dispatch_cycle,
                entry.issue_cycle,
                entry.complete_cycle,
                entry.commit_cycle
            ));
        }
        output
    }
}

impl Default for TraceOutput {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Reg;

    #[test]
    fn test_trace_output() {
        let mut trace = TraceOutput::new(100);

        let instr = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Add)
            .with_disasm("ADD X0, X1, X2");

        trace.record_dispatch(&instr, 0);
        trace.record_issue(InstructionId(0), 2);
        trace.record_complete(InstructionId(0), 4);
        trace.record_commit(InstructionId(0), 5);

        assert_eq!(trace.len(), 1);

        let entry = &trace.entries()[0];
        assert_eq!(entry.exec_latency, Some(2));
        assert_eq!(entry.total_latency(), Some(5));
    }

    #[test]
    fn test_trace_max_entries() {
        let mut trace = TraceOutput::new(3);

        for i in 0..5 {
            let instr = Instruction::new(InstructionId(i), 0x1000 + i as u64 * 4, 0, OpcodeType::Nop);
            trace.record_dispatch(&instr, i as u64);
        }

        assert_eq!(trace.len(), 3);

        // Should have entries 2, 3, 4 (oldest removed)
        assert_eq!(trace.entries()[0].id, 2);
        assert_eq!(trace.entries()[2].id, 4);
    }

    #[test]
    fn test_trace_text_output() {
        let mut trace = TraceOutput::new(100);

        let instr = Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Add);
        trace.record_dispatch(&instr, 0);
        trace.record_commit(InstructionId(0), 5);

        let mut output = Vec::new();
        trace.write_text(&mut output).unwrap();

        let text = String::from_utf8(output).unwrap();
        assert!(text.contains("0x1000"));
        assert!(text.contains("Add"));
    }
}
