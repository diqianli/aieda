//! Core simulation engine with event-based output.
//!
//! This module provides the main simulation engine that executes instructions
//! and emits events for visualization and analysis.

use crate::chi::ChiManager;
use crate::config::CPUConfig;
use crate::input::InstructionSource;
use crate::memory::MemorySubsystem;
use crate::ooo::OoOEngine;
use crate::stats::{PerformanceMetrics, StatsCollector, TraceOutput};
use crate::types::{EmulatorError, Instruction, InstructionId, MemAccess, Result};
use std::sync::{Arc, Mutex};

use super::event::{ExecutionUnit, SimulationEvent, SimulationEventSink};
use super::tracker::PipelineTracker;

/// Core simulation engine with event-based output
pub struct SimulationEngine {
    /// Configuration
    config: CPUConfig,
    /// Out-of-order execution engine
    ooo_engine: OoOEngine,
    /// Memory subsystem
    memory: MemorySubsystem,
    /// CHI interface manager
    chi_manager: ChiManager,
    /// Statistics collector
    stats: StatsCollector,
    /// Trace output
    trace: TraceOutput,
    /// Pipeline tracker for visualization
    pipeline_tracker: PipelineTracker,
    /// Event sinks
    event_sinks: Vec<Arc<Mutex<dyn SimulationEventSink>>>,
    /// Current cycle
    current_cycle: u64,
    /// Instructions committed
    committed_count: u64,
    /// Whether simulation is running
    running: bool,
}

impl SimulationEngine {
    /// Create a new simulation engine
    pub fn new(config: CPUConfig) -> Result<Self> {
        config.validate()?;

        let ooo_engine = OoOEngine::new(config.clone())?;
        let memory = MemorySubsystem::new(config.clone())?;
        let chi_manager = ChiManager::new(&config);
        let stats = StatsCollector::new();
        let trace = if config.enable_trace_output {
            TraceOutput::new(config.max_trace_output)
        } else {
            TraceOutput::disabled()
        };
        let pipeline_tracker = PipelineTracker::new();

        Ok(Self {
            config,
            ooo_engine,
            memory,
            chi_manager,
            stats,
            trace,
            pipeline_tracker,
            event_sinks: Vec::new(),
            current_cycle: 0,
            committed_count: 0,
            running: false,
        })
    }

    /// Add an event sink
    pub fn add_event_sink(&mut self, sink: Arc<Mutex<dyn SimulationEventSink>>) {
        self.event_sinks.push(sink);
    }

    /// Remove all event sinks
    pub fn clear_event_sinks(&mut self) {
        self.event_sinks.clear();
    }

    /// Emit an event to all sinks
    fn emit_event(&self, event: SimulationEvent) {
        for sink in &self.event_sinks {
            if let Ok(mut guard) = sink.lock() {
                guard.on_event(&event);
            }
        }
    }

    /// Get configuration
    pub fn config(&self) -> &CPUConfig {
        &self.config
    }

    /// Get current cycle
    pub fn current_cycle(&self) -> u64 {
        self.current_cycle
    }

    /// Get committed instruction count
    pub fn committed_count(&self) -> u64 {
        self.committed_count
    }

    /// Get the pipeline tracker
    pub fn pipeline_tracker(&self) -> &PipelineTracker {
        &self.pipeline_tracker
    }

    /// Get mutable pipeline tracker
    pub fn pipeline_tracker_mut(&mut self) -> &mut PipelineTracker {
        &mut self.pipeline_tracker
    }

    /// Run simulation with an instruction source
    pub fn run<S: InstructionSource>(&mut self, source: &mut S) -> Result<PerformanceMetrics> {
        self.run_with_limit(source, 1_000_000_000)
    }

    /// Run simulation with a cycle limit
    pub fn run_with_limit<S: InstructionSource>(
        &mut self,
        source: &mut S,
        max_cycles: u64,
    ) -> Result<PerformanceMetrics> {
        self.running = true;
        let start_cycle = self.current_cycle;
        let mut last_commit_count = 0u64;
        let mut stall_cycles = 0u64;

        // Emit simulation start
        self.emit_event(SimulationEvent::SimulationStart {
            start_cycle: self.current_cycle,
        });

        while self.running {
            let committed_before = self.committed_count;

            // Emit cycle boundary
            self.emit_event(SimulationEvent::CycleBoundary {
                cycle: self.current_cycle,
                committed_count: self.committed_count,
            });

            // Fetch and dispatch instructions
            self.fetch_dispatch(source)?;

            // Process completions for this cycle FIRST
            self.ooo_engine.cycle_tick();

            // Execute ready instructions
            self.execute()?;

            // Complete memory operations
            self.complete_memory()?;

            // Commit completed instructions
            self.commit()?;

            // Advance cycle
            self.advance_cycle();

            // Check termination
            if self.should_stop(source) {
                self.running = false;
            }

            // Track stall cycles
            if self.committed_count == committed_before {
                stall_cycles += 1;
            } else {
                stall_cycles = 0;
            }

            // Safety check for infinite loops
            if stall_cycles > 10000 {
                tracing::warn!(
                    "No progress for {} cycles, stopping",
                    stall_cycles
                );
                self.running = false;
            }

            // Safety check for absolute cycle limit
            if self.current_cycle - start_cycle >= max_cycles {
                tracing::warn!(
                    "Cycle limit reached ({})",
                    max_cycles
                );
                self.running = false;
            }

            last_commit_count = self.committed_count;
        }

        // Emit simulation end
        self.emit_event(SimulationEvent::SimulationEnd {
            end_cycle: self.current_cycle,
            total_committed: self.committed_count,
        });

        Ok(self.get_metrics())
    }

    /// Dispatch an instruction directly
    pub fn dispatch(&mut self, instr: Instruction) -> Result<()> {
        if !self.ooo_engine.can_accept() {
            return Err(EmulatorError::InternalError(
                "Instruction window full".to_string(),
            ));
        }

        let instr_id = instr.id;

        // Emit fetch event
        self.emit_event(SimulationEvent::InstructionFetch {
            instr: instr.clone(),
            cycle: self.current_cycle,
        });

        // Record in pipeline tracker
        self.pipeline_tracker.record_fetch(&instr, self.current_cycle);

        self.stats.record_dispatch(instr.id, self.current_cycle);
        self.trace.record_dispatch(&instr, self.current_cycle);

        // Record dispatch stage
        self.pipeline_tracker
            .record_dispatch(instr_id, self.current_cycle);

        // Dispatch to OoO engine and get dependencies
        let dependencies = self.ooo_engine.dispatch(instr)?;

        // Record dependencies
        for dep in dependencies {
            self.pipeline_tracker
                .add_dependency(instr_id, dep.producer, dep.is_memory);
            self.emit_event(SimulationEvent::Dependency {
                consumer: instr_id,
                producer: dep.producer,
                is_memory: dep.is_memory,
            });
        }

        Ok(())
    }

    /// Fetch and dispatch instructions from source
    fn fetch_dispatch<S: InstructionSource>(&mut self, source: &mut S) -> Result<()> {
        let free_slots = self.ooo_engine.free_slots();
        let fetch_limit = std::cmp::min(free_slots, self.config.fetch_width);
        let mut dispatched = 0;
        let mut failed = 0;

        while dispatched < fetch_limit {
            match source.next() {
                Some(Ok(instr)) => match self.dispatch(instr) {
                    Ok(()) => dispatched += 1,
                    Err(e) => {
                        tracing::warn!("Dispatch failed: {}", e);
                        failed += 1;
                        if failed > 10 {
                            break;
                        }
                    }
                },
                Some(Err(e)) => {
                    tracing::warn!("Instruction fetch error: {}", e);
                    break;
                }
                None => break,
            }
        }

        Ok(())
    }

    /// Execute ready instructions
    fn execute(&mut self) -> Result<()> {
        let ready = self.ooo_engine.get_ready_instructions();

        for (id, instr) in ready {
            self.ooo_engine.mark_executing(id);
            self.stats.record_issue(id, self.current_cycle);
            self.trace.record_issue(id, self.current_cycle);

            // Determine execution unit
            let unit = ExecutionUnit::from_opcode(instr.opcode_type);

            // Emit issue event
            self.emit_event(SimulationEvent::InstructionIssue {
                id,
                cycle: self.current_cycle,
                unit,
            });

            // Record issue stage
            self.pipeline_tracker.record_issue(id, self.current_cycle);

            // Emit execute start
            self.emit_event(SimulationEvent::InstructionExecuteStart {
                id,
                cycle: self.current_cycle,
            });

            if instr.opcode_type.is_memory_op() {
                // Handle memory operation
                if let Some(ref mem_access) = instr.mem_access {
                    self.handle_memory_op(id, mem_access)?;
                } else {
                    let complete_cycle = self.current_cycle + 1;
                    self.ooo_engine.mark_completed(id, complete_cycle);
                    self.pipeline_tracker.record_complete(id, complete_cycle);
                }
            } else {
                // Compute instruction - complete after latency
                let latency = instr.latency();
                let complete_cycle = self.current_cycle + latency;
                self.ooo_engine.mark_completed(id, complete_cycle);

                // Record execute and complete stages
                self.pipeline_tracker
                    .record_execute(id, self.current_cycle, complete_cycle);
                self.pipeline_tracker.record_complete(id, complete_cycle);

                // Emit execute end
                self.emit_event(SimulationEvent::InstructionExecuteEnd {
                    id,
                    cycle: complete_cycle,
                });
            }
        }

        Ok(())
    }

    /// Handle memory operation
    fn handle_memory_op(&mut self, id: InstructionId, access: &MemAccess) -> Result<()> {
        let request = if access.is_load {
            self.memory.load(id, access)
        } else {
            self.memory.store(id, access)
        };

        let complete_cycle = request.complete_cycle.unwrap_or(self.current_cycle + 1);
        self.ooo_engine.mark_completed(id, complete_cycle);

        // Calculate latency
        let latency = complete_cycle.saturating_sub(self.current_cycle);

        // Record memory stage
        self.pipeline_tracker
            .record_memory(id, self.current_cycle, complete_cycle);
        self.pipeline_tracker.record_complete(id, complete_cycle);

        // Emit memory access event
        self.emit_event(SimulationEvent::MemoryAccess {
            id,
            addr: access.addr,
            size: access.size,
            is_load: access.is_load,
            latency,
            hit_level: if latency <= 4 { 1 } else if latency <= 12 { 2 } else { 0 },
        });

        // Emit memory complete event
        self.emit_event(SimulationEvent::MemoryComplete {
            id,
            cycle: complete_cycle,
        });

        // Record stats
        if access.is_load {
            self.stats.record_load(access.size as u64, latency);
            self.stats.record_l1_access(true);
        } else {
            self.stats.record_store(access.size as u64, 1);
        }

        Ok(())
    }

    /// Complete pending memory operations
    fn complete_memory(&mut self) -> Result<()> {
        // In this simplified model, all memory operations complete immediately
        Ok(())
    }

    /// Commit completed instructions
    fn commit(&mut self) -> Result<()> {
        let commit_candidates = self.ooo_engine.get_commit_candidates();

        for instr in commit_candidates {
            let id = instr.id;

            self.ooo_engine.commit(id);
            self.committed_count += 1;

            // Record retire stage
            self.pipeline_tracker
                .record_retire(id, self.current_cycle);

            // Emit retire event
            self.emit_event(SimulationEvent::InstructionRetire {
                id,
                cycle: self.current_cycle,
                retire_order: self.committed_count,
            });

            self.stats.record_commit(&instr, self.current_cycle);
            self.trace.record_commit(id, self.current_cycle);
        }

        Ok(())
    }

    /// Advance simulation by one cycle
    fn advance_cycle(&mut self) {
        self.current_cycle += 1;
        self.ooo_engine.advance_cycle();
        self.memory.advance_cycle();
        self.chi_manager.advance_cycle();
        self.stats.record_cycles(1);
    }

    /// Check if simulation should stop
    fn should_stop<S: InstructionSource>(&self, _source: &mut S) -> bool {
        self.ooo_engine.is_empty()
    }

    /// Stop the simulation
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Reset the emulator
    pub fn reset(&mut self) -> Result<()> {
        self.ooo_engine = OoOEngine::new(self.config.clone())?;
        self.memory = MemorySubsystem::new(self.config.clone())?;
        self.chi_manager = ChiManager::new(&self.config);
        self.stats.reset();
        self.trace.clear();
        self.pipeline_tracker.clear();
        self.current_cycle = 0;
        self.committed_count = 0;
        self.running = false;
        Ok(())
    }

    /// Get performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        self.stats.get_metrics()
    }

    /// Get statistics collector
    pub fn stats(&self) -> &StatsCollector {
        &self.stats
    }

    /// Get mutable statistics collector
    pub fn stats_mut(&mut self) -> &mut StatsCollector {
        &mut self.stats
    }

    /// Get trace output
    pub fn trace(&self) -> &TraceOutput {
        &self.trace
    }

    /// Get mutable trace output
    pub fn trace_mut(&mut self) -> &mut TraceOutput {
        &mut self.trace
    }

    /// Get memory subsystem
    pub fn memory(&self) -> &MemorySubsystem {
        &self.memory
    }

    /// Get OoO engine
    pub fn ooo_engine(&self) -> &OoOEngine {
        &self.ooo_engine
    }

    /// Print summary
    pub fn print_summary(&self) {
        let metrics = self.get_metrics();
        println!("{}", metrics.summary());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::TraceInput;
    use crate::types::{OpcodeType, Reg};

    #[test]
    fn test_simulation_engine_basic() {
        let config = CPUConfig::minimal();
        let mut engine = SimulationEngine::new(config).unwrap();

        let mut input = TraceInput::new();
        input
            .builder(0x1000, OpcodeType::Add)
            .src_reg(Reg(0))
            .dst_reg(Reg(1))
            .build();

        let metrics = engine.run(&mut input).unwrap();
        assert_eq!(metrics.total_instructions, 1);
    }
}
