//! Core types and data structures for the ARM CPU emulator.

use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use smallvec::SmallVec;
use std::fmt;

/// ARMv8-A General purpose register (X0-X30)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct Reg(pub u8);

impl Reg {
    pub const SP: Reg = Reg(31); // Stack pointer / Zero register
    pub const XZR: Reg = Reg(31); // Zero register (same encoding as SP, context-dependent)

    pub fn is_valid(&self) -> bool {
        self.0 <= 31
    }

    pub fn name(&self) -> &'static str {
        if self.0 == 31 {
            "SP/XZR"
        } else {
            // Return a static name using const array
            const NAMES: [&str; 31] = [
                "X0", "X1", "X2", "X3", "X4", "X5", "X6", "X7",
                "X8", "X9", "X10", "X11", "X12", "X13", "X14", "X15",
                "X16", "X17", "X18", "X19", "X20", "X21", "X22", "X23",
                "X24", "X25", "X26", "X27", "X28", "X29", "X30",
            ];
            NAMES[self.0 as usize]
        }
    }
}

impl fmt::Display for Reg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

/// ARMv8-A SIMD/FP register (V0-V31, also D0-D31, S0-S31)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct VReg(pub u8);

impl VReg {
    pub fn is_valid(&self) -> bool {
        self.0 <= 31
    }
}

/// Instruction opcode classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum OpcodeType {
    // Computational
    Add,
    Sub,
    Mul,
    Div,
    And,
    Orr,
    Eor,
    Lsl,
    Lsr,
    Asr,
    // Load/Store
    Load,
    Store,
    LoadPair,
    StorePair,
    // Branch
    Branch,
    BranchCond,
    BranchReg,
    // System
    Msr,
    Mrs,
    Sys,
    Nop,
    // SIMD/FP (existing)
    Fadd,
    Fsub,
    Fmul,
    Fdiv,

    // === Cache Maintenance Instructions ===
    /// Data Cache Zero by VA
    DcZva,
    /// Data Cache Clean by VA to PoC
    DcCivac,
    /// Data Cache Clean by VA
    DcCvac,
    /// Data Cache Clean by Set/Way
    DcCsw,
    /// Instruction Cache Invalidate by VA to PoU
    IcIvau,
    /// Instruction Cache Invalidate All to PoU
    IcIallu,
    /// Instruction Cache Invalidate All to PoU (Inner Shareable)
    IcIalluis,

    // === Cryptography Extensions ===
    /// AES Decrypt
    Aesd,
    /// AES Encrypt
    Aese,
    /// AES Inverse Mix Columns
    Aesimc,
    /// AES Mix Columns
    Aesmc,
    /// SHA-1 Hash (part 1)
    Sha1H,
    /// SHA-256 Hash
    Sha256H,
    /// SHA-512 Hash
    Sha512H,

    // === SIMD/Vector (NEON) ===
    /// Vector Add
    Vadd,
    /// Vector Subtract
    Vsub,
    /// Vector Multiply
    Vmul,
    /// Vector Multiply-Accumulate
    Vmla,
    /// Vector Multiply-Subtract
    Vmls,
    /// Vector Load
    Vld,
    /// Vector Store
    Vst,
    /// Vector Duplicate
    Vdup,
    /// Vector Move
    Vmov,

    // === Floating-point FMA ===
    /// Floating-point Fused Multiply-Add
    Fmadd,
    /// Floating-point Fused Multiply-Subtract
    Fmsub,
    /// Floating-point Negated Fused Multiply-Add
    Fnmadd,
    /// Floating-point Negated Fused Multiply-Subtract
    Fnmsub,

    // Other
    Other,
}

impl OpcodeType {
    /// Returns true if this is a memory operation
    pub fn is_memory_op(&self) -> bool {
        matches!(
            self,
            Self::Load | Self::Store | Self::LoadPair | Self::StorePair |
            Self::Vld | Self::Vst  // Vector load/store are also memory operations
        )
    }

    /// Returns true if this is a branch instruction
    pub fn is_branch(&self) -> bool {
        matches!(self, Self::Branch | Self::BranchCond | Self::BranchReg)
    }

    /// Returns true if this is a computational instruction
    pub fn is_compute(&self) -> bool {
        matches!(
            self,
            Self::Add | Self::Sub | Self::Mul | Self::Div |
            Self::And | Self::Orr | Self::Eor |
            Self::Lsl | Self::Lsr | Self::Asr |
            Self::Fadd | Self::Fsub | Self::Fmul | Self::Fdiv |
            // New SIMD/vector instructions
            Self::Vadd | Self::Vsub | Self::Vmul | Self::Vmla | Self::Vmls |
            // New FMA instructions
            Self::Fmadd | Self::Fmsub | Self::Fnmadd | Self::Fnmsub |
            // New crypto instructions
            Self::Aesd | Self::Aese | Self::Aesimc | Self::Aesmc |
            Self::Sha1H | Self::Sha256H | Self::Sha512H
        )
    }

    /// Returns true if this is a cache maintenance instruction
    pub fn is_cache_maintenance(&self) -> bool {
        matches!(
            self,
            Self::DcZva | Self::DcCivac | Self::DcCvac | Self::DcCsw |
            Self::IcIvau | Self::IcIallu | Self::IcIalluis
        )
    }

    /// Returns true if this is a cryptography instruction
    pub fn is_crypto(&self) -> bool {
        matches!(
            self,
            Self::Aesd | Self::Aese | Self::Aesimc | Self::Aesmc |
            Self::Sha1H | Self::Sha256H | Self::Sha512H
        )
    }

    /// Returns true if this is a SIMD/vector instruction
    pub fn is_simd(&self) -> bool {
        matches!(
            self,
            Self::Vadd | Self::Vsub | Self::Vmul | Self::Vmla | Self::Vmls |
            Self::Vld | Self::Vst | Self::Vdup | Self::Vmov
        )
    }

    /// Returns true if this is an FMA (Fused Multiply-Add) instruction
    pub fn is_fma(&self) -> bool {
        matches!(
            self,
            Self::Fmadd | Self::Fmsub | Self::Fnmadd | Self::Fnmsub
        )
    }

    /// Returns the execution latency in cycles (simplified model)
    pub fn latency(&self) -> u64 {
        match self {
            // Existing instructions
            Self::Mul | Self::Div => 3,
            Self::Fadd | Self::Fsub => 2,
            Self::Fmul => 3,
            Self::Fdiv => 8,

            // Cache maintenance instructions (typically slow, involve cache operations)
            Self::DcZva | Self::DcCivac | Self::DcCvac => 20,
            Self::DcCsw => 30,
            Self::IcIvau | Self::IcIallu | Self::IcIalluis => 15,

            // Cryptography instructions (AES/SHA operations)
            Self::Aesd | Self::Aese | Self::Aesimc | Self::Aesmc => 4,
            Self::Sha1H => 10,
            Self::Sha256H => 12,
            Self::Sha512H => 16,

            // SIMD/Vector instructions
            Self::Vadd | Self::Vsub => 2,
            Self::Vmul | Self::Vmla | Self::Vmls => 4,
            Self::Vld | Self::Vst => 3,  // Vector load/store
            Self::Vdup | Self::Vmov => 1,

            // FMA instructions
            Self::Fmadd | Self::Fmsub | Self::Fnmadd | Self::Fnmsub => 4,

            _ => 1,
        }
    }
}

/// Instruction operand
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum Operand {
    /// General purpose register
    Register(Reg),
    /// SIMD/FP register
    VRegister(VReg),
    /// Immediate value
    Immediate(i64),
    /// Memory operand with base register and optional offset
    Memory {
        base: Reg,
        offset: i64,
        size: u8, // Access size in bytes (1, 2, 4, 8, 16)
    },
    /// Shifted register
    ShiftedReg {
        reg: Reg,
        shift_type: ShiftType,
        shift_amount: u8,
    },
    /// Extended register
    ExtendedReg {
        reg: Reg,
        ext: ExtensionType,
    },
}

impl Operand {
    pub fn reg(r: u8) -> Self {
        Self::Register(Reg(r))
    }

    pub fn imm(v: i64) -> Self {
        Self::Immediate(v)
    }

    pub fn mem(base: u8, offset: i64, size: u8) -> Self {
        Self::Memory {
            base: Reg(base),
            offset,
            size,
        }
    }
}

/// Shift type for shifted register operands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ShiftType {
    Lsl,
    Lsr,
    Asr,
    Ror,
}

/// Extension type for extended register operands
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ExtensionType {
    Uxtb,
    Uxth,
    Uxtw,
    Uxtx,
    Sxtb,
    Sxth,
    Sxtw,
    Sxtx,
}

/// Branch information for branch instructions
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BranchInfo {
    /// Whether this is a conditional branch
    pub is_conditional: bool,
    /// Target address
    pub target: u64,
    /// Whether the branch is taken (ideal frontend always predicts correctly)
    pub is_taken: bool,
}

/// Memory access information
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MemAccess {
    /// Memory address
    pub addr: u64,
    /// Access size in bytes
    pub size: u8,
    /// Whether this is a load (true) or store (false)
    pub is_load: bool,
}

/// Unique instruction identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub struct InstructionId(pub u64);

impl Default for InstructionId {
    fn default() -> Self {
        Self(0)
    }
}

/// A single instruction in the trace
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Instruction {
    /// Unique instruction ID
    pub id: InstructionId,
    /// Program counter
    pub pc: u64,
    /// Raw opcode encoding (for reference)
    pub raw_opcode: u32,
    /// Decoded opcode type
    pub opcode_type: OpcodeType,
    /// Source operands (registers read)
    #[schemars(skip)]
    pub src_regs: SmallVec<[Reg; 4]>,
    /// Destination operands (registers written)
    #[schemars(skip)]
    pub dst_regs: SmallVec<[Reg; 2]>,
    /// Source SIMD/FP registers
    #[schemars(skip)]
    pub src_vregs: SmallVec<[VReg; 4]>,
    /// Destination SIMD/FP registers
    #[schemars(skip)]
    pub dst_vregs: SmallVec<[VReg; 2]>,
    /// Memory access info (if this is a load/store)
    pub mem_access: Option<MemAccess>,
    /// Branch info (if this is a branch)
    pub branch_info: Option<BranchInfo>,
    /// Instruction text (for debugging)
    pub disasm: Option<String>,
}

impl Instruction {
    /// Create a new instruction with the given PC and opcode type
    pub fn new(id: InstructionId, pc: u64, raw_opcode: u32, opcode_type: OpcodeType) -> Self {
        Self {
            id,
            pc,
            raw_opcode,
            opcode_type,
            src_regs: SmallVec::new(),
            dst_regs: SmallVec::new(),
            src_vregs: SmallVec::new(),
            dst_vregs: SmallVec::new(),
            mem_access: None,
            branch_info: None,
            disasm: None,
        }
    }

    /// Add a source register
    pub fn with_src_reg(mut self, reg: Reg) -> Self {
        if !self.src_regs.contains(&reg) {
            self.src_regs.push(reg);
        }
        self
    }

    /// Add a destination register
    pub fn with_dst_reg(mut self, reg: Reg) -> Self {
        if !self.dst_regs.contains(&reg) {
            self.dst_regs.push(reg);
        }
        self
    }

    /// Set memory access info
    pub fn with_mem_access(mut self, addr: u64, size: u8, is_load: bool) -> Self {
        self.mem_access = Some(MemAccess { addr, size, is_load });
        self
    }

    /// Set branch info
    pub fn with_branch(mut self, target: u64, is_conditional: bool, is_taken: bool) -> Self {
        self.branch_info = Some(BranchInfo {
            is_conditional,
            target,
            is_taken,
        });
        self
    }

    /// Set disassembly text
    pub fn with_disasm(mut self, disasm: impl Into<String>) -> Self {
        self.disasm = Some(disasm.into());
        self
    }

    /// Check if this instruction reads from the given register
    pub fn reads_reg(&self, reg: Reg) -> bool {
        self.src_regs.contains(&reg)
    }

    /// Check if this instruction writes to the given register
    pub fn writes_reg(&self, reg: Reg) -> bool {
        self.dst_regs.contains(&reg)
    }

    /// Get execution latency for this instruction
    pub fn latency(&self) -> u64 {
        self.opcode_type.latency()
    }
}

/// Instruction status in the execution window
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum InstrStatus {
    /// Waiting for operands
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

impl Default for InstrStatus {
    fn default() -> Self {
        Self::Waiting
    }
}

/// Cache line state for coherence
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum CacheLineState {
    Invalid,
    Shared,
    Exclusive,
    Modified,
    Unique,
}

impl Default for CacheLineState {
    fn default() -> Self {
        Self::Invalid
    }
}

/// CHI request type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ChiRequestType {
    // Read requests
    ReadNoSnoop,
    ReadNotSharedDirty,
    ReadShared,
    ReadMakeUnique,
    // Write requests
    WriteNoSnoop,
    WriteUnique,
    WriteUniquePtl,
    // Coherence requests
    CleanUnique,
    MakeUnique,
    Evict,
    // Data responses
    CompData,
    DataSepResp,
    NonCopyBackWrData,
    // Acknowledgments
    CompAck,
    // Snoop requests
    SnpOnce,
    SnpShared,
    SnpClean,
    SnpData,
}

/// CHI response status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
pub enum ChiResponseStatus {
    Pending,
    Complete,
    Error,
}

/// Error types for the emulator
#[derive(Debug, Clone, thiserror::Error, Serialize, Deserialize, JsonSchema)]
pub enum EmulatorError {
    #[error("Invalid register: {0}")]
    InvalidRegister(u8),

    #[error("Invalid instruction at PC {0:#x}: {1}")]
    InvalidInstruction(u64, String),

    #[error("Trace parsing error: {0}")]
    TraceParseError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Memory access error at address {0:#x}: {1}")]
    MemoryError(u64, String),

    #[error("CHI protocol error: {0}")]
    ChiError(String),

    #[error("Internal error: {0}")]
    InternalError(String),
}

/// Result type for emulator operations
pub type Result<T> = std::result::Result<T, EmulatorError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reg_display() {
        assert_eq!(Reg(0).to_string(), "X0");
        assert_eq!(Reg(30).to_string(), "X30");
        assert_eq!(Reg(31).to_string(), "SP/XZR");
    }

    #[test]
    fn test_opcode_classification() {
        assert!(OpcodeType::Load.is_memory_op());
        assert!(OpcodeType::Branch.is_branch());
        assert!(OpcodeType::Add.is_compute());
    }

    #[test]
    fn test_instruction_builder() {
        let instr = Instruction::new(InstructionId(1), 0x1000, 0x8B000000, OpcodeType::Add)
            .with_src_reg(Reg(0))
            .with_src_reg(Reg(1))
            .with_dst_reg(Reg(2))
            .with_disasm("ADD X2, X0, X1");

        assert_eq!(instr.src_regs.len(), 2);
        assert_eq!(instr.dst_regs.len(), 1);
        assert!(instr.reads_reg(Reg(0)));
        assert!(instr.writes_reg(Reg(2)));
    }
}
