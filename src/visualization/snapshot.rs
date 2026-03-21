//! Snapshot types for CPU visualization.
//!
//! These types capture the CPU state at each cycle for visualization purposes.

use serde::{Deserialize, Serialize};
use crate::types::{Instruction, InstructionId, InstrStatus, OpcodeType};

/// A complete snapshot of the CPU state at a point in time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationSnapshot {
    /// Current cycle number
    pub cycle: u64,
    /// Number of instructions committed so far
    pub committed_count: u64,
    /// Instructions currently in the window
    pub instructions: Vec<InstructionSnapshot>,
    /// Dependencies between instructions
    pub dependencies: Vec<DependencyEdge>,
    /// Performance metrics
    pub metrics: MetricsSnapshot,
    /// Pipeline stage counts
    pub pipeline: PipelineSnapshot,
}

/// Snapshot of a single instruction for visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstructionSnapshot {
    /// Unique instruction ID
    pub id: u64,
    /// Program counter
    pub pc: u64,
    /// Opcode type as string
    pub opcode: String,
    /// Disassembly text
    pub disasm: Option<String>,
    /// Current status
    pub status: InstructionStatus,
    /// Source register numbers
    pub src_regs: Vec<u16>,
    /// Destination register numbers
    pub dst_regs: Vec<u16>,
    /// Whether this is a memory operation
    pub is_memory: bool,
    /// Memory address (if memory operation)
    pub mem_addr: Option<u64>,
    /// Memory access size (if memory operation)
    pub mem_size: Option<u8>,
    /// Whether memory access is a load
    pub is_load: Option<bool>,
    /// Cycle when dispatched
    pub dispatch_cycle: Option<u64>,
    /// Cycle when issued (started execution)
    pub issue_cycle: Option<u64>,
    /// Cycle when completed execution
    pub complete_cycle: Option<u64>,
    /// Number of pending dependencies
    pub pending_deps: usize,
}

/// Instruction status for visualization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InstructionStatus {
    /// Waiting for operands/dependencies
    Waiting,
    /// Ready to execute
    Ready,
    /// Currently executing
    Executing,
    /// Execution complete, waiting to commit
    Completed,
    /// Committed to architectural state
    Committed,
}

impl From<InstrStatus> for InstructionStatus {
    fn from(status: InstrStatus) -> Self {
        match status {
            InstrStatus::Waiting => InstructionStatus::Waiting,
            InstrStatus::Ready => InstructionStatus::Ready,
            InstrStatus::Executing => InstructionStatus::Executing,
            InstrStatus::Completed => InstructionStatus::Completed,
            InstrStatus::Committed => InstructionStatus::Committed,
        }
    }
}

/// A dependency edge between two instructions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    /// Producer instruction ID
    pub from: u64,
    /// Consumer instruction ID
    pub to: u64,
    /// Type of dependency
    pub dep_type: DependencyType,
}

/// Type of dependency between instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DependencyType {
    /// Register dependency (RAW)
    Register,
    /// Memory dependency
    Memory,
}

/// Performance metrics snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    /// Instructions per cycle
    pub ipc: f64,
    /// Total cycles
    pub total_cycles: u64,
    /// Total instructions committed
    pub total_instructions: u64,
    /// L1 cache hit rate (0.0 - 1.0)
    pub l1_hit_rate: f64,
    /// L2 cache hit rate (0.0 - 1.0)
    pub l2_hit_rate: f64,
    /// L1 MPKI (Misses Per Kilo Instructions)
    pub l1_mpki: f64,
    /// L2 MPKI (Misses Per Kilo Instructions)
    pub l2_mpki: f64,
    /// Memory instruction percentage
    pub memory_instr_pct: f64,
    /// Average load latency
    pub avg_load_latency: f64,
}

/// Pipeline stage snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineSnapshot {
    /// Instructions being fetched
    pub fetch_count: usize,
    /// Instructions in dispatch stage
    pub dispatch_count: usize,
    /// Instructions executing
    pub execute_count: usize,
    /// Instructions in memory stage
    pub memory_count: usize,
    /// Instructions being committed
    pub commit_count: usize,
    /// Window occupancy
    pub window_occupancy: usize,
    /// Window capacity
    pub window_capacity: usize,
}

impl InstructionSnapshot {
    /// Create a snapshot from an instruction and its window entry.
    pub fn from_instruction(
        instr: &Instruction,
        status: InstrStatus,
        dispatch_cycle: u64,
        issue_cycle: Option<u64>,
        complete_cycle: Option<u64>,
        pending_deps: usize,
    ) -> Self {
        Self {
            id: instr.id.0,
            pc: instr.pc,
            opcode: opcode_to_string(instr.opcode_type),
            disasm: instr.disasm.clone(),
            status: status.into(),
            src_regs: instr.src_regs.iter().map(|r| r.0 as u16).collect(),
            dst_regs: instr.dst_regs.iter().map(|r| r.0 as u16).collect(),
            is_memory: instr.mem_access.is_some(),
            mem_addr: instr.mem_access.as_ref().map(|m| m.addr),
            mem_size: instr.mem_access.as_ref().map(|m| m.size),
            is_load: instr.mem_access.as_ref().map(|m| m.is_load),
            dispatch_cycle: Some(dispatch_cycle),
            issue_cycle,
            complete_cycle,
            pending_deps,
        }
    }
}

/// Convert opcode type to a human-readable string.
fn opcode_to_string(opcode: OpcodeType) -> String {
    match opcode {
        // Computational
        OpcodeType::Add => "ADD".to_string(),
        OpcodeType::Sub => "SUB".to_string(),
        OpcodeType::Mul => "MUL".to_string(),
        OpcodeType::Div => "DIV".to_string(),
        OpcodeType::And => "AND".to_string(),
        OpcodeType::Orr => "ORR".to_string(),
        OpcodeType::Eor => "EOR".to_string(),
        OpcodeType::Lsl => "LSL".to_string(),
        OpcodeType::Lsr => "LSR".to_string(),
        OpcodeType::Asr => "ASR".to_string(),
        // Data movement
        OpcodeType::Mov => "MOV".to_string(),
        OpcodeType::Cmp => "CMP".to_string(),
        OpcodeType::Shift => "SHIFT".to_string(),
        // Load/Store
        OpcodeType::Load => "LOAD".to_string(),
        OpcodeType::Store => "STORE".to_string(),
        OpcodeType::LoadPair => "LDP".to_string(),
        OpcodeType::StorePair => "STP".to_string(),
        // Branch
        OpcodeType::Branch => "B".to_string(),
        OpcodeType::BranchCond => "B.cond".to_string(),
        OpcodeType::BranchReg => "BR".to_string(),
        // System
        OpcodeType::Msr => "MSR".to_string(),
        OpcodeType::Mrs => "MRS".to_string(),
        OpcodeType::Sys => "SYS".to_string(),
        OpcodeType::Nop => "NOP".to_string(),
        // Floating-point (existing)
        OpcodeType::Fadd => "FADD".to_string(),
        OpcodeType::Fsub => "FSUB".to_string(),
        OpcodeType::Fmul => "FMUL".to_string(),
        OpcodeType::Fdiv => "FDIV".to_string(),
        // Cache Maintenance
        OpcodeType::DcZva => "DC_ZVA".to_string(),
        OpcodeType::DcCivac => "DC_CIVAC".to_string(),
        OpcodeType::DcCvac => "DC_CVAC".to_string(),
        OpcodeType::DcCsw => "DC_CSW".to_string(),
        OpcodeType::IcIvau => "IC_IVAU".to_string(),
        OpcodeType::IcIallu => "IC_IALLU".to_string(),
        OpcodeType::IcIalluis => "IC_IALLUIS".to_string(),
        // Cryptography
        OpcodeType::Aesd => "AESD".to_string(),
        OpcodeType::Aese => "AESE".to_string(),
        OpcodeType::Aesimc => "AESIMC".to_string(),
        OpcodeType::Aesmc => "AESMC".to_string(),
        OpcodeType::Sha1H => "SHA1H".to_string(),
        OpcodeType::Sha256H => "SHA256H".to_string(),
        OpcodeType::Sha512H => "SHA512H".to_string(),
        // SIMD/Vector
        OpcodeType::Vadd => "VADD".to_string(),
        OpcodeType::Vsub => "VSUB".to_string(),
        OpcodeType::Vmul => "VMUL".to_string(),
        OpcodeType::Vmla => "VMLA".to_string(),
        OpcodeType::Vmls => "VMLS".to_string(),
        OpcodeType::Vld => "VLD".to_string(),
        OpcodeType::Vst => "VST".to_string(),
        OpcodeType::Vdup => "VDUP".to_string(),
        OpcodeType::Vmov => "VMOV".to_string(),
        // FMA
        OpcodeType::Fmadd => "FMADD".to_string(),
        OpcodeType::Fmsub => "FMSUB".to_string(),
        OpcodeType::Fnmadd => "FNMADD".to_string(),
        OpcodeType::Fnmsub => "FNMSUB".to_string(),
        // Floating-point convert
        OpcodeType::Fcvt => "FCVT".to_string(),
        // Memory Barriers
        OpcodeType::Dmb => "DMB".to_string(),
        OpcodeType::Dsb => "DSB".to_string(),
        OpcodeType::Isb => "ISB".to_string(),
        // System
        OpcodeType::Eret => "ERET".to_string(),
        OpcodeType::Yield => "YIELD".to_string(),
        OpcodeType::Adr => "ADR".to_string(),
        // Crypto
        OpcodeType::Pmull => "PMULL".to_string(),
        // Other
        OpcodeType::Other => "OTHER".to_string(),
    }
}

/// Configuration for visualization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualizationConfig {
    /// Enable visualization
    pub enabled: bool,
    /// HTTP server port
    pub port: u16,
    /// Maximum snapshots to keep in memory
    pub max_snapshots: usize,
    /// Animation speed (cycles per second)
    pub animation_speed: u32,
}

impl Default for VisualizationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            port: 3000,
            max_snapshots: 10000,
            animation_speed: 10,
        }
    }
}

impl VisualizationConfig {
    /// Create a new visualization config with visualization enabled.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Create a config with a custom port.
    pub fn with_port(port: u16) -> Self {
        Self {
            enabled: true,
            port,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_status_conversion() {
        assert_eq!(InstructionStatus::from(InstrStatus::Waiting), InstructionStatus::Waiting);
        assert_eq!(InstructionStatus::from(InstrStatus::Ready), InstructionStatus::Ready);
        assert_eq!(InstructionStatus::from(InstrStatus::Executing), InstructionStatus::Executing);
        assert_eq!(InstructionStatus::from(InstrStatus::Completed), InstructionStatus::Completed);
    }

    #[test]
    fn test_visualization_config_default() {
        let config = VisualizationConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.port, 3000);
    }

    #[test]
    fn test_snapshot_serialization() {
        let snapshot = VisualizationSnapshot {
            cycle: 100,
            committed_count: 50,
            instructions: vec![],
            dependencies: vec![],
            metrics: MetricsSnapshot::default(),
            pipeline: PipelineSnapshot::default(),
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("\"cycle\":100"));
    }
}
