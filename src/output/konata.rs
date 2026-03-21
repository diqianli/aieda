//! Konata format generator for pipeline visualization.
//!
//! This module generates Konata-compatible JSON output for visualizing
//! instruction pipeline execution.
//!
//! # Format
//!
//! The output is a JSON file containing an array of operations, each with:
//! - `id`: Visualization ID (sequential)
//! - `gid`: Program ID (instruction ID)
//! - `rid`: Retire ID (retire order)
//! - `pc`: Program counter
//! - `label_name`: Disassembly text
//! - `lanes`: Array of pipeline stages with timing
//! - `prods`: Dependencies on other instructions
//! - `src_regs`: Source register numbers
//! - `dst_regs`: Destination register numbers
//! - `is_memory`: Whether this is a memory operation
//! - `mem_addr`: Memory address (if memory op)

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::Path;

use crate::simulation::event::{SimulationEvent, SimulationEventSink};
use crate::simulation::tracker::{DependencyType, StageInfo, TrackedInstruction};
use crate::types::InstructionId;

/// Configuration for Konata output
#[derive(Debug, Clone)]
pub struct KonataConfig {
    /// Include register information
    pub include_registers: bool,
    /// Include memory addresses
    pub include_memory: bool,
    /// Include dependencies
    pub include_dependencies: bool,
    /// Minimum stage duration to include (0 = all)
    pub min_stage_duration: u64,
    /// Pretty-print JSON output
    pub pretty_print: bool,
}

impl Default for KonataConfig {
    fn default() -> Self {
        Self {
            include_registers: true,
            include_memory: true,
            include_dependencies: true,
            min_stage_duration: 0,
            pretty_print: true,
        }
    }
}

impl KonataConfig {
    /// Create a minimal config for small output
    pub fn minimal() -> Self {
        Self {
            include_registers: false,
            include_memory: false,
            include_dependencies: false,
            min_stage_duration: 1,
            pretty_print: false,
        }
    }

    /// Create a full config with all details
    pub fn full() -> Self {
        Self {
            include_registers: true,
            include_memory: true,
            include_dependencies: true,
            min_stage_duration: 0,
            pretty_print: true,
        }
    }
}

/// A single pipeline stage in Konata format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonataStage {
    /// Stage name (F, Dc, Rn, Ds, Is, Ex, Me, Cm, Rt)
    pub name: String,
    /// Start cycle
    pub start_cycle: u64,
    /// End cycle
    pub end_cycle: u64,
}

impl KonataStage {
    /// Create a new stage
    pub fn new(name: impl Into<String>, start: u64, end: u64) -> Self {
        Self {
            name: name.into(),
            start_cycle: start,
            end_cycle: end.max(start), // Ensure end >= start
        }
    }

    /// Get the duration of this stage
    pub fn duration(&self) -> u64 {
        self.end_cycle.saturating_sub(self.start_cycle)
    }
}

/// Dependency reference in Konata format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonataDependency {
    /// Producer instruction ID
    pub producer_id: u64,
    /// Dependency type ("reg" or "mem")
    #[serde(rename = "type")]
    pub dep_type: String,
}

impl KonataDependency {
    /// Create a register dependency
    pub fn register(producer_id: u64) -> Self {
        Self {
            producer_id,
            dep_type: "reg".to_string(),
        }
    }

    /// Create a memory dependency
    pub fn memory(producer_id: u64) -> Self {
        Self {
            producer_id,
            dep_type: "mem".to_string(),
        }
    }
}

/// A lane containing stages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonataLane {
    /// Lane name
    pub name: String,
    /// Stages in this lane
    pub stages: Vec<KonataStage>,
}

/// A single operation (instruction) in Konata format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonataOp {
    /// Visualization ID (sequential)
    pub id: u64,
    /// Program ID (instruction ID)
    pub gid: u64,
    /// Retire ID (retire order)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rid: Option<u64>,
    /// Program counter
    pub pc: u64,
    /// Disassembly text
    pub label_name: String,
    /// Pipeline lanes
    pub lanes: Vec<KonataLane>,
    /// Dependencies
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub prods: Vec<KonataDependency>,
    /// Source registers
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub src_regs: Vec<u16>,
    /// Destination registers
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dst_regs: Vec<u16>,
    /// Whether this is a memory operation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_memory: Option<bool>,
    /// Memory address (if memory op)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mem_addr: Option<u64>,
    /// Fetched cycle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fetched_cycle: Option<u64>,
    /// Retired cycle
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retired_cycle: Option<u64>,
}

impl KonataOp {
    /// Create a new Konata operation
    pub fn new(id: u64, gid: u64, pc: u64, label_name: String) -> Self {
        Self {
            id,
            gid,
            rid: None,
            pc,
            label_name,
            lanes: Vec::new(),
            prods: Vec::new(),
            src_regs: Vec::new(),
            dst_regs: Vec::new(),
            is_memory: None,
            mem_addr: None,
            fetched_cycle: None,
            retired_cycle: None,
        }
    }

    /// Add a stage to the main lane
    pub fn add_stage(&mut self, name: &str, start: u64, end: u64) {
        // Find or create the main lane
        if let Some(lane) = self.lanes.first_mut() {
            lane.stages.push(KonataStage::new(name, start, end));
        } else {
            let mut lane = KonataLane {
                name: "main".to_string(),
                stages: Vec::new(),
            };
            lane.stages.push(KonataStage::new(name, start, end));
            self.lanes.push(lane);
        }
    }

    /// Add a dependency
    pub fn add_dependency(&mut self, dep: KonataDependency) {
        self.prods.push(dep);
    }

    /// Set register info
    pub fn set_registers(&mut self, src: Vec<u16>, dst: Vec<u16>) {
        self.src_regs = src;
        self.dst_regs = dst;
    }

    /// Set memory info
    pub fn set_memory(&mut self, addr: u64) {
        self.is_memory = Some(true);
        self.mem_addr = Some(addr);
    }

    /// Convert from tracked instruction
    pub fn from_tracked(instr: &TrackedInstruction, config: &KonataConfig) -> Self {
        let mut op = Self::new(
            instr.viz_id,
            instr.program_id,
            instr.pc,
            instr.disasm.clone(),
        );

        // Convert stages
        let stages = instr.timing.to_stages();
        let mut lane = KonataLane {
            name: "main".to_string(),
            stages: Vec::new(),
        };

        for stage in stages {
            if stage.end_cycle - stage.start_cycle >= config.min_stage_duration {
                lane.stages.push(KonataStage::new(
                    &stage.name,
                    stage.start_cycle,
                    stage.end_cycle,
                ));
            }
        }
        op.lanes.push(lane);

        // Add registers if configured
        if config.include_registers {
            op.src_regs = instr.src_regs.clone();
            op.dst_regs = instr.dst_regs.clone();
        }

        // Add memory info if configured
        if config.include_memory && instr.is_memory {
            op.is_memory = Some(true);
            op.mem_addr = instr.mem_addr;
        }

        // Add dependencies if configured
        if config.include_dependencies {
            for dep in &instr.dependencies {
                let konata_dep = match dep.dep_type {
                    DependencyType::Register => KonataDependency::register(dep.producer_id),
                    DependencyType::Memory => KonataDependency::memory(dep.producer_id),
                };
                op.prods.push(konata_dep);
            }
        }

        op.fetched_cycle = instr.timing.fetch_start;
        op.retired_cycle = instr.timing.retire_cycle;

        op
    }
}

/// Complete Konata output file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonataOutput {
    /// Format version
    pub version: String,
    /// Total simulation cycles
    pub total_cycles: u64,
    /// Total committed instructions
    pub total_instructions: u64,
    /// Number of operations
    pub ops_count: usize,
    /// Operations
    pub ops: Vec<KonataOp>,
}

impl KonataOutput {
    /// Create a new Konata output
    pub fn new() -> Self {
        Self {
            version: "1.0".to_string(),
            total_cycles: 0,
            total_instructions: 0,
            ops_count: 0,
            ops: Vec::new(),
        }
    }

    /// Add an operation
    pub fn add_op(&mut self, op: KonataOp) {
        self.ops.push(op);
        self.ops_count = self.ops.len();
    }

    /// Set summary info
    pub fn set_summary(&mut self, cycles: u64, instructions: u64) {
        self.total_cycles = cycles;
        self.total_instructions = instructions;
    }

    /// Convert to JSON string
    pub fn to_json(&self, pretty: bool) -> serde_json::Result<String> {
        if pretty {
            serde_json::to_string_pretty(self)
        } else {
            serde_json::to_string(self)
        }
    }

    /// Write to a file
    pub fn write_to_file(&self, path: &Path, pretty: bool) -> io::Result<()> {
        let json = self.to_json(pretty).map_err(|e| {
            io::Error::new(io::ErrorKind::Other, e)
        })?;
        let mut file = std::fs::File::create(path)?;
        file.write_all(json.as_bytes())
    }

    /// Create from tracked instructions
    pub fn from_instructions(
        instructions: &[TrackedInstruction],
        total_cycles: u64,
        total_instructions: u64,
        config: &KonataConfig,
    ) -> Self {
        let mut output = Self::new();
        output.set_summary(total_cycles, total_instructions);

        for instr in instructions {
            let op = KonataOp::from_tracked(instr, config);
            output.add_op(op);
        }

        output
    }
}

impl Default for KonataOutput {
    fn default() -> Self {
        Self::new()
    }
}

/// Konata writer that consumes simulation events
pub struct KonataWriter {
    /// Configuration
    config: KonataConfig,
    /// Collected operations
    ops: Vec<KonataOp>,
    /// Current cycle
    current_cycle: u64,
    /// Committed count
    committed_count: u64,
    /// ID mapping
    id_map: std::collections::HashMap<InstructionId, usize>,
}

impl KonataWriter {
    /// Create a new Konata writer
    pub fn new() -> Self {
        Self::with_config(KonataConfig::default())
    }

    /// Create with custom config
    pub fn with_config(config: KonataConfig) -> Self {
        Self {
            config,
            ops: Vec::new(),
            current_cycle: 0,
            committed_count: 0,
            id_map: std::collections::HashMap::new(),
        }
    }

    /// Handle a simulation event
    pub fn handle_event(&mut self, event: &SimulationEvent) {
        match event {
            SimulationEvent::InstructionFetch { instr, cycle } => {
                let op = KonataOp::new(
                    self.ops.len() as u64,
                    instr.id.0,
                    instr.pc,
                    instr.disasm.clone().unwrap_or_else(|| format!("{:?}", instr.opcode_type)),
                );
                let idx = self.ops.len();
                self.id_map.insert(instr.id, idx);
                self.ops.push(op);

                // Add fetch stage
                if let Some(op) = self.ops.last_mut() {
                    op.add_stage("F", *cycle, cycle + 1);
                    op.fetched_cycle = Some(*cycle);
                }
            }

            SimulationEvent::InstructionDispatch { id, cycle } => {
                if let Some(&idx) = self.id_map.get(id) {
                    if let Some(op) = self.ops.get_mut(idx) {
                        // Add decode, rename, dispatch stages
                        if let Some(ref last_stage) = op.lanes.first().and_then(|l| l.stages.last()) {
                            let fetch_end = last_stage.end_cycle;
                            if *cycle > fetch_end {
                                if *cycle - fetch_end >= 3 {
                                    op.add_stage("Dc", fetch_end, fetch_end + 1);
                                    op.add_stage("Rn", fetch_end + 1, fetch_end + 2);
                                    op.add_stage("Ds", fetch_end + 2, *cycle);
                                } else {
                                    op.add_stage("Dc", fetch_end, *cycle);
                                    op.add_stage("Rn", fetch_end, *cycle);
                                    op.add_stage("Ds", *cycle, *cycle);
                                }
                            } else {
                                op.add_stage("Dc", *cycle, *cycle);
                                op.add_stage("Rn", *cycle, *cycle);
                                op.add_stage("Ds", *cycle, *cycle);
                            }
                        }
                    }
                }
            }

            SimulationEvent::InstructionIssue { id, cycle, .. } => {
                if let Some(&idx) = self.id_map.get(id) {
                    if let Some(op) = self.ops.get_mut(idx) {
                        if let Some(ref last_stage) = op.lanes.first().and_then(|l| l.stages.last()) {
                            let dispatch_end = last_stage.end_cycle;
                            op.add_stage("Is", dispatch_end, (*cycle).max(dispatch_end));
                        }
                    }
                }
            }

            SimulationEvent::InstructionExecuteEnd { id, cycle } => {
                if let Some(&idx) = self.id_map.get(id) {
                    if let Some(op) = self.ops.get_mut(idx) {
                        if let Some(ref last_stage) = op.lanes.first().and_then(|l| l.stages.last()) {
                            let issue_end = last_stage.end_cycle;
                            op.add_stage("Ex", issue_end, (*cycle).max(issue_end));
                        }
                    }
                }
            }

            SimulationEvent::MemoryComplete { id, cycle } => {
                if let Some(&idx) = self.id_map.get(id) {
                    if let Some(op) = self.ops.get_mut(idx) {
                        if let Some(ref last_stage) = op.lanes.first().and_then(|l| l.stages.last()) {
                            let issue_end = last_stage.end_cycle;
                            op.add_stage("Me", issue_end, (*cycle).max(issue_end));
                        }
                    }
                }
            }

            SimulationEvent::InstructionComplete { id, cycle } => {
                if let Some(&idx) = self.id_map.get(id) {
                    if let Some(op) = self.ops.get_mut(idx) {
                        op.add_stage("Cm", *cycle, *cycle);
                    }
                }
            }

            SimulationEvent::InstructionRetire { id, cycle, retire_order } => {
                if let Some(&idx) = self.id_map.get(id) {
                    if let Some(op) = self.ops.get_mut(idx) {
                        op.add_stage("Rt", *cycle, *cycle);
                        op.rid = Some(*retire_order);
                        op.retired_cycle = Some(*cycle);
                    }
                }
                self.committed_count = (*retire_order).max(self.committed_count);
            }

            SimulationEvent::MemoryAccess { id, addr, .. } => {
                if let Some(&idx) = self.id_map.get(id) {
                    if let Some(op) = self.ops.get_mut(idx) {
                        if self.config.include_memory {
                            op.set_memory(*addr);
                        }
                    }
                }
            }

            SimulationEvent::Dependency { consumer, producer, is_memory } => {
                if self.config.include_dependencies {
                    if let Some(&idx) = self.id_map.get(consumer) {
                        if let Some(&producer_idx) = self.id_map.get(producer) {
                            if let Some(op) = self.ops.get_mut(idx) {
                                let dep = if *is_memory {
                                    KonataDependency::memory(producer_idx as u64)
                                } else {
                                    KonataDependency::register(producer_idx as u64)
                                };
                                op.add_dependency(dep);
                            }
                        }
                    }
                }
            }

            SimulationEvent::CycleBoundary { cycle, .. } => {
                self.current_cycle = *cycle;
            }

            SimulationEvent::SimulationEnd { end_cycle, .. } => {
                self.current_cycle = *end_cycle;
            }

            _ => {}
        }
    }

    /// Get the output
    pub fn get_output(&self) -> KonataOutput {
        let mut output = KonataOutput::new();
        output.set_summary(self.current_cycle, self.committed_count);
        output.ops = self.ops.clone();
        output.ops_count = output.ops.len();
        output
    }

    /// Write to a file
    pub fn write_to_file(&self, path: &Path) -> io::Result<()> {
        let output = self.get_output();
        output.write_to_file(path, self.config.pretty_print)
    }

    /// Clear all collected data
    pub fn clear(&mut self) {
        self.ops.clear();
        self.id_map.clear();
        self.current_cycle = 0;
        self.committed_count = 0;
    }

    /// Get the number of collected operations
    pub fn len(&self) -> usize {
        self.ops.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

impl Default for KonataWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Implementation of SimulationEventSink for KonataWriter
impl SimulationEventSink for KonataWriter {
    fn on_event(&mut self, event: &SimulationEvent) {
        self.handle_event(event);
    }

    fn flush(&mut self) {
        // Nothing to flush for in-memory writer
    }

    fn name(&self) -> &'static str {
        "KonataWriter"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_konata_stage() {
        let stage = KonataStage::new("F", 0, 1);
        assert_eq!(stage.name, "F");
        assert_eq!(stage.duration(), 1);
    }

    #[test]
    fn test_konata_op() {
        let mut op = KonataOp::new(0, 0, 0x1000, "ADD X0, X1, X2".to_string());
        op.add_stage("F", 0, 1);
        op.add_stage("Dc", 1, 2);
        op.add_stage("Ex", 2, 4);

        assert_eq!(op.lanes.len(), 1);
        assert_eq!(op.lanes[0].stages.len(), 3);
    }

    #[test]
    fn test_konata_output() {
        let mut output = KonataOutput::new();
        let op = KonataOp::new(0, 0, 0x1000, "ADD X0, X1, X2".to_string());
        output.add_op(op);
        output.set_summary(100, 10);

        let json = output.to_json(true).unwrap();
        assert!(json.contains("\"version\""));
        assert!(json.contains("\"total_cycles\""));
    }

    #[test]
    fn test_konata_writer() {
        let mut writer = KonataWriter::new();

        // Create a simple event sequence
        let instr = crate::types::Instruction::new(
            crate::types::InstructionId(0),
            0x1000,
            0x8B000000,
            crate::types::OpcodeType::Add,
        );

        writer.on_event(&SimulationEvent::InstructionFetch {
            instr,
            cycle: 0,
        });
        writer.on_event(&SimulationEvent::InstructionDispatch {
            id: crate::types::InstructionId(0),
            cycle: 2,
        });
        writer.on_event(&SimulationEvent::InstructionIssue {
            id: crate::types::InstructionId(0),
            cycle: 3,
            unit: crate::simulation::event::ExecutionUnit::IntAlu,
        });
        writer.on_event(&SimulationEvent::InstructionExecuteEnd {
            id: crate::types::InstructionId(0),
            cycle: 5,
        });
        writer.on_event(&SimulationEvent::InstructionRetire {
            id: crate::types::InstructionId(0),
            cycle: 6,
            retire_order: 1,
        });

        let output = writer.get_output();
        assert_eq!(output.ops.len(), 1);
        assert_eq!(output.ops[0].lanes[0].stages.len(), 6); // F, Dc, Rn, Ds, Is, Ex, Cm, Rt (some may be combined)
    }
}
