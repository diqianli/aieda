//! Simulation events and event sink trait.
//!
//! This module defines the events emitted during simulation and the trait
//! for consuming these events.

use crate::types::{Instruction, InstructionId, OpcodeType};
use std::sync::{Arc, Mutex};

/// Execution unit types for instructions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutionUnit {
    /// Integer ALU
    IntAlu,
    /// Integer multiplier
    IntMul,
    /// Integer divider
    IntDiv,
    /// Load unit
    Load,
    /// Store unit
    Store,
    /// Branch unit
    Branch,
    /// Floating-point unit
    Fp,
    /// SIMD/NEON unit
    Simd,
    /// Cryptography unit
    Crypto,
    /// System unit
    System,
}

impl ExecutionUnit {
    /// Determine the execution unit for an instruction
    pub fn from_opcode(opcode: OpcodeType) -> Self {
        match opcode {
            // Integer arithmetic
            OpcodeType::Add | OpcodeType::Sub | OpcodeType::And | OpcodeType::Orr |
            OpcodeType::Eor | OpcodeType::Lsl | OpcodeType::Lsr | OpcodeType::Asr |
            OpcodeType::Mov | OpcodeType::Cmp | OpcodeType::Shift |
            OpcodeType::Vmov | OpcodeType::Vdup | OpcodeType::Nop => ExecutionUnit::IntAlu,

            // Integer multiply/divide
            OpcodeType::Mul => ExecutionUnit::IntMul,
            OpcodeType::Div => ExecutionUnit::IntDiv,

            // Load operations
            OpcodeType::Load | OpcodeType::LoadPair | OpcodeType::Vld => ExecutionUnit::Load,

            // Store operations
            OpcodeType::Store | OpcodeType::StorePair | OpcodeType::Vst => ExecutionUnit::Store,

            // Branch operations
            OpcodeType::Branch | OpcodeType::BranchCond | OpcodeType::BranchReg => ExecutionUnit::Branch,

            // Floating-point operations
            OpcodeType::Fadd | OpcodeType::Fsub | OpcodeType::Fmul | OpcodeType::Fdiv |
            OpcodeType::Fmadd | OpcodeType::Fmsub | OpcodeType::Fnmadd | OpcodeType::Fnmsub => ExecutionUnit::Fp,

            // SIMD operations
            OpcodeType::Vadd | OpcodeType::Vsub | OpcodeType::Vmul |
            OpcodeType::Vmla | OpcodeType::Vmls => ExecutionUnit::Simd,

            // Crypto operations
            OpcodeType::Aesd | OpcodeType::Aese | OpcodeType::Aesimc | OpcodeType::Aesmc |
            OpcodeType::Sha1H | OpcodeType::Sha256H | OpcodeType::Sha512H => ExecutionUnit::Crypto,

            // Cache maintenance and system
            OpcodeType::DcZva | OpcodeType::DcCivac | OpcodeType::DcCvac | OpcodeType::DcCsw |
            OpcodeType::IcIvau | OpcodeType::IcIallu | OpcodeType::IcIalluis |
            OpcodeType::Msr | OpcodeType::Mrs | OpcodeType::Sys => ExecutionUnit::System,

            // Default to integer ALU
            _ => ExecutionUnit::IntAlu,
        }
    }
}

/// Events emitted during simulation
#[derive(Debug, Clone)]
pub enum SimulationEvent {
    /// Instruction fetched from trace/ELF
    InstructionFetch {
        /// The fetched instruction
        instr: Instruction,
        /// Cycle when fetched
        cycle: u64,
    },

    /// Instruction dispatched to the issue window
    InstructionDispatch {
        /// Instruction ID
        id: InstructionId,
        /// Cycle when dispatched
        cycle: u64,
    },

    /// Instruction decoded
    InstructionDecode {
        /// Instruction ID
        id: InstructionId,
        /// Cycle when decoded
        cycle: u64,
    },

    /// Instruction renamed
    InstructionRename {
        /// Instruction ID
        id: InstructionId,
        /// Cycle when renamed
        cycle: u64,
    },

    /// Instruction issued for execution
    InstructionIssue {
        /// Instruction ID
        id: InstructionId,
        /// Cycle when issued
        cycle: u64,
        /// Execution unit assigned
        unit: ExecutionUnit,
    },

    /// Instruction execution started
    InstructionExecuteStart {
        /// Instruction ID
        id: InstructionId,
        /// Cycle when execution started
        cycle: u64,
    },

    /// Instruction execution completed
    InstructionExecuteEnd {
        /// Instruction ID
        id: InstructionId,
        /// Cycle when execution ended
        cycle: u64,
    },

    /// Memory access event
    MemoryAccess {
        /// Instruction ID
        id: InstructionId,
        /// Memory address
        addr: u64,
        /// Access size in bytes
        size: u8,
        /// Whether this is a load
        is_load: bool,
        /// Access latency in cycles
        latency: u64,
        /// Cache level where hit (0 = miss to memory)
        hit_level: u8,
    },

    /// Memory operation completed
    MemoryComplete {
        /// Instruction ID
        id: InstructionId,
        /// Cycle when memory operation completed
        cycle: u64,
    },

    /// Instruction completed execution
    InstructionComplete {
        /// Instruction ID
        id: InstructionId,
        /// Cycle when completed
        cycle: u64,
    },

    /// Instruction retired/committed
    InstructionRetire {
        /// Instruction ID
        id: InstructionId,
        /// Cycle when retired
        cycle: u64,
        /// Sequential retire order
        retire_order: u64,
    },

    /// Dependency between instructions
    Dependency {
        /// Consumer instruction ID
        consumer: InstructionId,
        /// Producer instruction ID
        producer: InstructionId,
        /// Whether this is a memory dependency
        is_memory: bool,
    },

    /// Branch prediction (for future use)
    BranchPrediction {
        /// Instruction ID
        id: InstructionId,
        /// Predicted target
        predicted_target: u64,
        /// Whether prediction was correct
        correct: bool,
    },

    /// Cycle boundary
    CycleBoundary {
        /// Current cycle
        cycle: u64,
        /// Number of committed instructions so far
        committed_count: u64,
    },

    /// Simulation started
    SimulationStart {
        /// Start cycle (usually 0)
        start_cycle: u64,
    },

    /// Simulation ended
    SimulationEnd {
        /// Final cycle
        end_cycle: u64,
        /// Total committed instructions
        total_committed: u64,
    },
}

/// Trait for consuming simulation events
///
/// Implement this trait to receive events during simulation.
/// Events are emitted for each significant pipeline stage transition.
pub trait SimulationEventSink: Send {
    /// Handle a simulation event
    fn on_event(&mut self, event: &SimulationEvent);

    /// Flush any buffered output
    fn flush(&mut self) {}

    /// Get the name of this sink for debugging
    fn name(&self) -> &'static str;
}

/// A simple event logger for debugging
pub struct EventLogger {
    /// Whether to log all events or just summary
    verbose: bool,
    /// Event count
    event_count: u64,
}

impl EventLogger {
    /// Create a new event logger
    pub fn new(verbose: bool) -> Self {
        Self {
            verbose,
            event_count: 0,
        }
    }
}

impl SimulationEventSink for EventLogger {
    fn on_event(&mut self, event: &SimulationEvent) {
        self.event_count += 1;
        if self.verbose {
            tracing::debug!("Event #{}: {:?}", self.event_count, event);
        }
    }

    fn name(&self) -> &'static str {
        "EventLogger"
    }
}

/// Multi-sink dispatcher that sends events to multiple sinks
pub struct MultiSink {
    sinks: Vec<Arc<Mutex<dyn SimulationEventSink>>>,
}

impl MultiSink {
    /// Create a new multi-sink dispatcher
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    /// Add a sink
    pub fn add_sink(&mut self, sink: Arc<Mutex<dyn SimulationEventSink>>) {
        self.sinks.push(sink);
    }

    /// Dispatch an event to all sinks
    pub fn dispatch(&self, event: &SimulationEvent) {
        for sink in &self.sinks {
            if let Ok(mut guard) = sink.lock() {
                guard.on_event(event);
            }
        }
    }

    /// Flush all sinks
    pub fn flush_all(&self) {
        for sink in &self.sinks {
            if let Ok(mut guard) = sink.lock() {
                guard.flush();
            }
        }
    }
}

impl Default for MultiSink {
    fn default() -> Self {
        Self::new()
    }
}

/// Null sink that discards all events
pub struct NullSink;

impl SimulationEventSink for NullSink {
    fn on_event(&mut self, _event: &SimulationEvent) {}

    fn name(&self) -> &'static str {
        "NullSink"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_unit_mapping() {
        assert_eq!(ExecutionUnit::from_opcode(OpcodeType::Add), ExecutionUnit::IntAlu);
        assert_eq!(ExecutionUnit::from_opcode(OpcodeType::Mul), ExecutionUnit::IntMul);
        assert_eq!(ExecutionUnit::from_opcode(OpcodeType::Load), ExecutionUnit::Load);
        assert_eq!(ExecutionUnit::from_opcode(OpcodeType::Fadd), ExecutionUnit::Fp);
        assert_eq!(ExecutionUnit::from_opcode(OpcodeType::Aese), ExecutionUnit::Crypto);
    }

    #[test]
    fn test_multi_sink() {
        let mut multi = MultiSink::new();
        multi.add_sink(Arc::new(Mutex::new(NullSink)));
        multi.add_sink(Arc::new(Mutex::new(EventLogger::new(false))));

        let event = SimulationEvent::SimulationStart { start_cycle: 0 };
        multi.dispatch(&event);
        multi.flush_all();
    }
}
