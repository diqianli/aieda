//! Konata-compatible data format for pipeline visualization.
//!
//! This module defines data structures compatible with the Konata pipeline
//! visualization tool format, enabling detailed stage-by-stage visualization
//! of instruction flow through the CPU pipeline.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::memory::CacheAccessInfo;

/// Pipeline stage identifiers used in Konata format.
/// Uses standard Konata naming convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageId {
    /// Instruction Fetch stage
    IF,
    /// Decode stage
    DE,
    /// Rename stage
    RN,
    /// Dispatch stage
    DI,
    /// Issue stage
    IS,
    /// Execute stage
    EX,
    /// Memory stage
    ME,
    /// Writeback stage
    WB,
    /// Retire stage
    RR,
}

impl StageId {
    /// Get the display name for this stage.
    pub fn name(&self) -> &'static str {
        match self {
            StageId::IF => "IF",
            StageId::DE => "DE",
            StageId::RN => "RN",
            StageId::DI => "DI",
            StageId::IS => "IS",
            StageId::EX => "EX",
            StageId::ME => "ME",
            StageId::WB => "WB",
            StageId::RR => "RR",
        }
    }
}

impl StageId {
    /// Get the full name for this stage.
    pub fn full_name(&self) -> &'static str {
        match self {
            StageId::IF => "Instruction Fetch",
            StageId::DE => "Decode",
            StageId::RN => "Rename",
            StageId::DI => "Dispatch",
            StageId::IS => "Issue",
            StageId::EX => "Execute",
            StageId::ME => "Memory",
            StageId::WB => "Writeback",
            StageId::RR => "Retire",
        }
    }

    /// Get the HSL color for this stage.
    pub fn color(&self) -> (u16, u8, u8) {
        match self {
            StageId::IF => (200, 70, 60),   // Blue
            StageId::DE => (180, 60, 55),   // Cyan
            StageId::RN => (160, 50, 50),   // Teal
            StageId::DI => (140, 60, 55),   // Green
            StageId::IS => (120, 70, 45),   // Yellow-green
            StageId::EX => (60, 80, 55),    // Yellow
            StageId::ME => (30, 80, 55),    // Orange
            StageId::WB => (280, 60, 55),   // Purple
            StageId::RR => (320, 50, 50),   // Pink
        }
    }

    /// Get the CSS color string for this stage.
    pub fn css_color(&self) -> String {
        let (h, s, l) = self.color();
        format!("hsl({}, {}%, {}%)", h, s, l)
    }

    /// Get the CSS color string for this stage with transparency.
    pub fn css_color_transparent(&self, alpha: f32) -> String {
        let (h, s, l) = self.color();
        format!("hsla({}, {}%, {}%, {})", h, s, l, alpha)
    }
}

/// A single pipeline stage duration for an instruction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonataStage {
    /// Stage identifier
    pub name: String,
    /// Cycle when stage started
    pub start_cycle: u64,
    /// Cycle when stage ended
    pub end_cycle: u64,
}

impl KonataStage {
    /// Create a new stage.
    pub fn new(name: impl Into<String>, start_cycle: u64, end_cycle: u64) -> Self {
        Self {
            name: name.into(),
            start_cycle,
            end_cycle,
        }
    }

    /// Get the duration of this stage in cycles.
    pub fn duration(&self) -> u64 {
        self.end_cycle.saturating_sub(self.start_cycle)
    }
}

/// A lane represents a resource/execution unit in the pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonataLane {
    /// Lane identifier (e.g., "ALU0", "MEM", "BR")
    pub name: String,
    /// Stages in this lane
    pub stages: Vec<KonataStage>,
}

impl KonataLane {
    /// Create a new lane.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            stages: Vec::new(),
        }
    }

    /// Add a stage to this lane.
    pub fn add_stage(&mut self, stage: KonataStage) {
        self.stages.push(stage);
    }
}

/// Reference to a dependency for Konata visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonataDependencyRef {
    /// ID of the instruction this depends on
    pub producer_id: u64,
    /// Type of dependency
    pub dep_type: KonataDependencyType,
}

/// Type of dependency between instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KonataDependencyType {
    /// Register dependency (RAW)
    Register,
    /// Memory dependency
    Memory,
}

impl KonataDependencyType {
    /// Get the color for this dependency type.
    pub fn color(&self) -> &'static str {
        match self {
            KonataDependencyType::Register => "#ff6600", // Orange
            KonataDependencyType::Memory => "#0066ff",   // Blue
        }
    }
}

/// Konata instruction format for visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonataOp {
    /// Sequential instruction number in the visualization
    pub id: u64,
    /// Original instruction ID (program order)
    pub gid: u64,
    /// Retire order ID (commit order)
    pub rid: u64,
    /// Cycle when instruction was fetched
    pub fetched_cycle: u64,
    /// Cycle when instruction was retired/committed
    pub retired_cycle: Option<u64>,
    /// Display label (opcode or disassembly)
    pub label_name: String,
    /// Program counter
    pub pc: u64,
    /// Lanes representing pipeline stages
    pub lanes: HashMap<String, KonataLane>,
    /// Dependencies on other instructions
    pub prods: Vec<KonataDependencyRef>,
    /// Source registers
    pub src_regs: Vec<u16>,
    /// Destination registers
    pub dst_regs: Vec<u16>,
    /// Whether this is a memory operation
    pub is_memory: bool,
    /// Memory address (if applicable)
    pub mem_addr: Option<u64>,
}

impl KonataOp {
    /// Create a new Konata operation.
    pub fn new(id: u64, gid: u64, pc: u64, label: impl Into<String>) -> Self {
        Self {
            id,
            gid,
            rid: 0,
            fetched_cycle: 0,
            retired_cycle: None,
            label_name: label.into(),
            pc,
            lanes: HashMap::new(),
            prods: Vec::new(),
            src_regs: Vec::new(),
            dst_regs: Vec::new(),
            is_memory: false,
            mem_addr: None,
        }
    }

    /// Add a stage to the main pipeline lane.
    pub fn add_stage(&mut self, stage_id: StageId, start_cycle: u64, end_cycle: u64) {
        let lane = self.lanes.entry("main".to_string()).or_insert_with(|| KonataLane::new("main"));
        lane.add_stage(KonataStage::new(stage_id.name(), start_cycle, end_cycle));
    }

    /// Add a stage with a custom name to the main pipeline lane.
    pub fn add_stage_with_name(&mut self, name: &str, start_cycle: u64, end_cycle: u64) {
        let lane = self.lanes.entry("main".to_string()).or_insert_with(|| KonataLane::new("main"));
        lane.add_stage(KonataStage::new(name, start_cycle, end_cycle));
    }

    /// Add a dependency.
    pub fn add_dependency(&mut self, producer_id: u64, dep_type: KonataDependencyType) {
        self.prods.push(KonataDependencyRef {
            producer_id,
            dep_type,
        });
    }
    /// Get the total latency (fetch to retire).
    pub fn total_latency(&self) -> Option<u64> {
        self.retired_cycle.map(|r| r.saturating_sub(self.fetched_cycle))
    }

    /// Get all stages across all lanes.
    pub fn all_stages(&self) -> Vec<&KonataStage> {
        self.lanes.values().flat_map(|lane| &lane.stages).collect()
    }
}

/// Complete Konata snapshot for visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonataSnapshot {
    /// All instructions in the visualization
    pub ops: Vec<KonataOp>,
    /// Current cycle
    pub cycle: u64,
    /// Total committed instructions
    pub committed_count: u64,
    /// Visualization metadata
    pub metadata: KonataMetadata,
}

impl KonataSnapshot {
    /// Create an empty snapshot.
    pub fn new(cycle: u64, committed_count: u64) -> Self {
        Self {
            ops: Vec::new(),
            cycle,
            committed_count,
            metadata: KonataMetadata::default(),
        }
    }

    /// Add an operation to the snapshot.
    pub fn add_op(&mut self, op: KonataOp) {
        self.ops.push(op);
    }
    /// Get an operation by ID.
    pub fn get_op(&self, id: u64) -> Option<&KonataOp> {
        self.ops.iter().find(|op| op.id == id)
    }
    /// Get the number of operations.
    pub fn len(&self) -> usize {
        self.ops.len()
    }
    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.ops.is_empty()
    }
}

/// Metadata for Konata visualization.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KonataMetadata {
    /// CPU configuration info
    pub config: Option<KonataConfigInfo>,
    /// Visualization timestamp
    pub timestamp: Option<u64>,
}

/// Configuration information for display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KonataConfigInfo {
    /// Window size
    pub window_size: usize,
    /// Issue width
    pub issue_width: usize,
    /// Commit width
    pub commit_width: usize,
}

/// Stage timing information for a single instruction.
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
    /// Ready cycle (when all operands become available)
    pub ready_cycle: Option<u64>,
    /// Execute start cycle
    pub execute_start: Option<u64>,
    /// Execute end cycle
    pub execute_end: Option<u64>,
    /// Memory start cycle (if applicable)
    pub memory_start: Option<u64>,
    /// Memory end cycle (if applicable)
    pub memory_end: Option<u64>,
    /// Complete cycle
    pub complete_cycle: Option<u64>,
    /// Retire/commit cycle
    pub retire_cycle: Option<u64>,
    /// Cache access information for memory operations
    pub cache_access: Option<CacheAccessInfo>,
}

impl StageTiming {
    /// Create new stage timing.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record fetch stage.
    pub fn record_fetch(&mut self, start: u64, end: u64) {
        self.fetch_start = Some(start);
        self.fetch_end = Some(end);
    }

    /// Record decode stage.
    pub fn record_decode(&mut self, start: u64, end: u64) {
        self.decode_start = Some(start);
        self.decode_end = Some(end);
    }

    /// Record rename stage.
    pub fn record_rename(&mut self, start: u64, end: u64) {
        self.rename_start = Some(start);
        self.rename_end = Some(end);
    }

    /// Record dispatch stage.
    pub fn record_dispatch(&mut self, start: u64, end: u64) {
        self.dispatch_start = Some(start);
        self.dispatch_end = Some(end);
    }

    /// Record issue stage.
    pub fn record_issue(&mut self, start: u64, end: u64) {
        self.issue_start = Some(start);
        self.issue_end = Some(end);
    }

    /// Record execute stage.
    pub fn record_execute(&mut self, start: u64, end: u64) {
        self.execute_start = Some(start);
        self.execute_end = Some(end);
    }

    /// Record memory stage.
    pub fn record_memory(&mut self, start: u64, end: u64) {
        self.memory_start = Some(start);
        self.memory_end = Some(end);
    }

    /// Record complete.
    pub fn record_complete(&mut self, cycle: u64) {
        self.complete_cycle = Some(cycle);
    }

    /// Record retire.
    pub fn record_retire(&mut self, cycle: u64) {
        self.retire_cycle = Some(cycle);
    }

    /// Convert to Konata stages.
    /// Ensures each stage is visible and stages don't overlap.
    /// For visualization, stages are shown sequentially even if timing shows overlap.
    pub fn to_stages(&self) -> Vec<KonataStage> {
        let mut stages = Vec::new();
        let mut last_end = 0u64; // Track the end of the previous stage
        let mut exec_mem_end: Option<u64> = None; // Store original Execute/Memory end cycle

        // Helper function to add a stage with proper timing
        // Ensures stages don't overlap by making each stage start after the previous one
        fn add_stage_sequential(
            stages: &mut Vec<KonataStage>,
            name: &str,
            start: u64,
            end: u64,
            last_end: &mut u64,
        ) {
            // Ensure minimum duration of 1 cycle
            let adjusted_end = std::cmp::max(end, start + 1);
            // Make sure this stage starts after the previous one ends
            let visual_start = std::cmp::max(start, *last_end);
            let visual_end = std::cmp::max(adjusted_end, visual_start + 1);
            stages.push(KonataStage::new(name, visual_start, visual_end));
            *last_end = visual_end;
        }

        // Add stages in pipeline order, ensuring sequential visualization
        if let (Some(s), Some(e)) = (self.fetch_start, self.fetch_end) {
            add_stage_sequential(&mut stages, "IF", s, e, &mut last_end);
        }
        if let (Some(s), Some(e)) = (self.decode_start, self.decode_end) {
            add_stage_sequential(&mut stages, "DE", s, e, &mut last_end);
        }
        if let (Some(s), Some(e)) = (self.rename_start, self.rename_end) {
            add_stage_sequential(&mut stages, "RN", s, e, &mut last_end);
        }
        if let (Some(s), Some(e)) = (self.dispatch_start, self.dispatch_end) {
            add_stage_sequential(&mut stages, "DI", s, e, &mut last_end);
        }
        if let (Some(s), Some(e)) = (self.issue_start, self.issue_end) {
            add_stage_sequential(&mut stages, "IS", s, e, &mut last_end);
        }

        // Memory stage - with cache level breakdown if available
        if let (Some(s), Some(e)) = (self.memory_start, self.memory_end) {
            exec_mem_end = Some(e);

            // Check if we have cache access info to generate sub-stages
            if let Some(ref cache_info) = self.cache_access {
                // Generate sub-stages for each cache level in the hierarchy
                for level_timing in &cache_info.level_timing {
                    let stage_name = format!("ME:{}", level_timing.level.name());
                    add_stage_sequential(
                        &mut stages,
                        &stage_name,
                        level_timing.start_cycle,
                        level_timing.end_cycle,
                        &mut last_end,
                    );
                }
            } else {
                // No cache info - use single ME stage
                add_stage_sequential(&mut stages, "ME", s, e, &mut last_end);
            }
        } else if let (Some(s), Some(e)) = (self.execute_start, self.execute_end) {
            add_stage_sequential(&mut stages, "EX", s, e, &mut last_end);
            exec_mem_end = self.execute_end.or(self.memory_end);
        }

        // Writeback stage: from Execute/Memory end to complete
        if let Some(c) = self.complete_cycle {
            if let Some(e) = exec_mem_end {
                let wb_start = std::cmp::max(e, last_end);
                let wb_end = std::cmp::max(c, wb_start + 1);
                stages.push(KonataStage::new("WB", wb_start, wb_end));
                last_end = wb_end;
            }
        }

        // Retire stage: from Writeback end to retire
        if let Some(r) = self.retire_cycle {
            let rr_start = last_end;
            let rr_end = std::cmp::max(r, rr_start + 1);
            stages.push(KonataStage::new("RR", rr_start, rr_end));
        }

        stages
    }

    /// Record cache access information for memory operations
    pub fn record_cache_access(&mut self, cache_info: CacheAccessInfo) {
        self.cache_access = Some(cache_info);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stage_id_color() {
        let color = StageId::IF.css_color();
        assert!(color.contains("hsl"));
    }

    #[test]
    fn test_konata_op() {
        let mut op = KonataOp::new(0, 0, 0x1000, "ADD");
        op.add_stage(StageId::IF, 0, 1);
        op.add_stage(StageId::DE, 1, 2);

        assert_eq!(op.lanes.len(), 1);
        assert_eq!(op.lanes["main"].stages.len(), 2);
    }

    #[test]
    fn test_stage_timing() {
        let mut timing = StageTiming::new();
        timing.record_fetch(0, 1);
        timing.record_decode(1, 2);
        timing.record_dispatch(2, 3);

        let stages = timing.to_stages();
        assert_eq!(stages.len(), 3);
    }
}
