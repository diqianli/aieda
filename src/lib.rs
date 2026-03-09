//! ARM CPU Emulator for SOC ESL Simulation
//!
//! This crate provides a simplified ARMv8-A (AArch64) CPU emulator designed for
//! accelerating SOC ESL (Electronic System Level) simulation. The emulator
//! features an idealized frontend while maintaining detailed modeling of the
//! memory subsystem and out-of-order execution capabilities.
//!
//! # Features
//!
//! - **Out-of-Order Execution**: Models instruction window, dependency tracking,
//!   and issue/commit bandwidth
//! - **Memory Subsystem**: Detailed L1/L2 cache modeling with configurable
//!   parameters
//! - **CHI Interface**: Optional CHI Issue B protocol support for SOC integration
//! - **Flexible Input**: Support for text, binary, and programmatic trace input
//! - **Performance Statistics**: Comprehensive metrics including IPC, cache
//!   hit rates, and MPKI
//!
//! # Quick Start
//!
//! ```rust
//! use arm_cpu_emulator::{CPUEmulator, CPUConfig, TraceInput, OpcodeType, Reg};
//!
//! // Create emulator with default configuration
//! let config = CPUConfig::default();
//! let mut cpu = CPUEmulator::new(config)?;
//!
//! // Create instruction input
//! let mut input = TraceInput::new();
//! input.builder(0x1000, OpcodeType::Add)
//!     .src_reg(Reg(0))
//!     .src_reg(Reg(1))
//!     .dst_reg(Reg(2))
//!     .disasm("ADD X2, X0, X1")
//!     .build();
//!
//! // Run simulation
//! let metrics = cpu.run(&mut input)?;
//!
//! // Print results
//! println!("IPC: {:.3}", metrics.ipc);
//! println!("L1 Hit Rate: {:.2}%", metrics.l1_hit_rate * 100.0);
//! # Ok::<(), arm_cpu_emulator::EmulatorError>(())
//! ```
//!
//! # Architecture
//!
//! The emulator consists of several key modules:
//!
//! - [`cpu`]: Top-level CPU integration
//! - [`config`]: Configuration management
//! - [`types`]: Core type definitions
//! - [`input`]: Instruction trace input
//! - [`ooo`]: Out-of-order execution engine
//! - [`memory`]: Memory subsystem (LSQ, caches)
//! - [`chi`]: CHI protocol interface
//! - [`stats`]: Performance statistics
//!
//! # Configuration
//!
//! The emulator is highly configurable through [`CPUConfig`]:
//!
//! ```rust
//! use arm_cpu_emulator::CPUConfig;
//!
//! let config = CPUConfig {
//!     window_size: 256,
//!     issue_width: 6,
//!     commit_width: 6,
//!     l1_size: 64 * 1024,
//!     l2_size: 512 * 1024,
//!     ..Default::default()
//! };
//! ```

pub mod chi;
pub mod config;
pub mod cpu;
pub mod input;
pub mod memory;
pub mod ooo;
pub mod stats;
pub mod types;
pub mod visualization;

// Re-export commonly used types at the crate root
pub use config::{CPUConfig, TraceFormat, TraceInputConfig};
pub use cpu::CPUEmulator;
pub use input::{ChampSimTraceParser, ChampSimXzTraceParser, InstructionSource, TraceInput};
pub use stats::{PerformanceMetrics, PerformanceStats, TraceOutput};
pub use types::{
    BranchInfo, CacheLineState, EmulatorError, Instruction, InstructionId, InstrStatus,
    MemAccess, OpcodeType, Operand, Reg, Result, VReg,
};
pub use visualization::{
    DependencyEdge, DependencyType, InstructionSnapshot, InstructionStatus,
    MetricsSnapshot, PipelineSnapshot, VisualizationConfig, VisualizationSnapshot,
    VisualizationState,
};

#[cfg(feature = "visualization")]
pub use visualization::VisualizationServer;

/// Crate version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Generate JSON schema for configuration
pub fn config_schema() -> std::result::Result<String, serde_json::Error> {
    let schema = schemars::schema_for!(CPUConfig);
    serde_json::to_string_pretty(&schema)
}

/// Generate JSON schema for API
pub fn api_schema() -> std::result::Result<String, serde_json::Error> {
    use schemars::schema_for;
    let schema = schema_for!(Instruction);
    serde_json::to_string_pretty(&schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_config_schema() {
        let schema = config_schema().unwrap();
        assert!(schema.contains("CPUConfig"));
    }

    #[test]
    fn test_api_schema() {
        let schema = api_schema().unwrap();
        assert!(schema.contains("Instruction"));
    }
}
