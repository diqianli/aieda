//! CPU emulator top-level integration.

use crate::chi::ChiManager;
use crate::config::CPUConfig;
use crate::input::InstructionSource;
use crate::memory::MemorySubsystem;
use crate::ooo::OoOEngine;
use crate::stats::{PerformanceMetrics, StatsCollector, TraceOutput};
use crate::types::{EmulatorError, Instruction, InstructionId, MemAccess, OpcodeType, Result};
use crate::visualization::{VisualizationConfig, VisualizationState, PipelineTracker};

/// Main CPU emulator
pub struct CPUEmulator {
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
    /// Visualization state
    visualization: VisualizationState,
    /// Pipeline stage tracker for Konata visualization
    pipeline_tracker: PipelineTracker,
    /// Current cycle
    current_cycle: u64,
    /// Instructions committed
    committed_count: u64,
    /// Whether simulation is running
    running: bool,
}

impl CPUEmulator {
    /// Create a new CPU emulator
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
        let visualization = VisualizationState::new(VisualizationConfig::default());
        let pipeline_tracker = PipelineTracker::new();

        Ok(Self {
            config,
            ooo_engine,
            memory,
            chi_manager,
            stats,
            trace,
            visualization,
            pipeline_tracker,
            current_cycle: 0,
            committed_count: 0,
            running: false,
        })
    }

    /// Create a new CPU emulator with visualization enabled
    pub fn with_visualization(config: CPUConfig, viz_config: VisualizationConfig) -> Result<Self> {
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
        let visualization = VisualizationState::new(viz_config);
        let pipeline_tracker = PipelineTracker::new();

        Ok(Self {
            config,
            ooo_engine,
            memory,
            chi_manager,
            stats,
            trace,
            visualization,
            pipeline_tracker,
            current_cycle: 0,
            committed_count: 0,
            running: false,
        })
    }

    /// Create with default configuration
    pub fn with_defaults() -> Result<Self> {
        Self::new(CPUConfig::default())
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

    /// Run simulation with an instruction source
    pub fn run<S: InstructionSource>(&mut self, source: &mut S) -> Result<PerformanceMetrics> {
        // Use run_with_limit with a very high limit for backward compatibility
        self.run_with_limit(source, 1_000_000_000)
    }

    /// Run simulation with a cycle limit (to prevent infinite loops)
    pub fn run_with_limit<S: InstructionSource>(&mut self, source: &mut S, max_cycles: u64) -> Result<PerformanceMetrics> {
        self.running = true;
        let start_cycle = self.current_cycle;
        let mut last_commit_count = 0u64;
        let mut stall_cycles = 0u64;

        while self.running {
            let committed_before = self.committed_count;

            // Capture pre-execute snapshot for visualization
            self.visualization.set_cycle(self.current_cycle);
            self.visualization.set_committed_count(self.committed_count);
            self.visualization.capture_snapshot(&self.ooo_engine, &self.get_metrics());

            // Fetch and dispatch instructions
            self.fetch_dispatch(source)?;

            // Process completions for this cycle FIRST (releases dependencies)
            // This ensures that instructions that complete this cycle wake up
            // their dependents before we try to issue new instructions
            self.ooo_engine.cycle_tick();

            // Execute ready instructions (including newly woken up dependents)
            self.execute()?;

            // Complete memory operations
            self.complete_memory()?;

            // Commit completed instructions
            self.commit()?;

            // Capture post-execute snapshot for visualization
            self.visualization.capture_snapshot(&self.ooo_engine, &self.get_metrics());

            // Advance cycle
            self.advance_cycle();

            // Check termination
            if self.should_stop(source) {
                self.running = false;
            }

            // Track stall cycles for debugging
            if self.committed_count == committed_before {
                stall_cycles += 1;
            } else {
                stall_cycles = 0;
            }

            // Safety check for infinite loops - if no progress for many cycles
            if stall_cycles > 10000 {
                let (waiting, ready, executing, completed) = self.ooo_engine.status_counts();
                tracing::warn!("No progress for {} cycles, stopping. Window: waiting={}, ready={}, executing={}, completed={}, committed={}",
                    stall_cycles, waiting, ready, executing, completed, self.committed_count);
                self.running = false;
            }

            // Safety check for absolute cycle limit
            if self.current_cycle - start_cycle >= max_cycles {
                let (waiting, ready, executing, completed) = self.ooo_engine.status_counts();
                tracing::warn!("Cycle limit reached ({}), stopping. Window: waiting={}, ready={}, executing={}, completed={}, committed={}",
                    max_cycles, waiting, ready, executing, completed, self.committed_count);
                self.running = false;
            }

            last_commit_count = self.committed_count;
        }

        Ok(self.get_metrics())
    }

    /// Run for a specific number of cycles
    pub fn run_cycles(&mut self, cycles: u64) {
        for _ in 0..cycles {
            self.step();
        }
    }

    /// Single step the simulation
    pub fn step(&mut self) {
        // Capture pre-execute snapshot (shows ready instructions)
        self.visualization.set_cycle(self.current_cycle);
        self.visualization.set_committed_count(self.committed_count);
        self.visualization.capture_snapshot(&self.ooo_engine, &self.get_metrics());

        // Process completions for this cycle FIRST (releases dependencies)
        // This ensures that instructions that complete this cycle wake up
        // their dependents before we try to issue new instructions
        self.ooo_engine.cycle_tick();

        // Execute ready instructions (including newly woken up dependents)
        let _ = self.execute();

        // Complete memory operations
        let _ = self.complete_memory();

        // Capture post-execute snapshot (shows executing/completed instructions)
        self.visualization.capture_snapshot(&self.ooo_engine, &self.get_metrics());

        // Commit
        let _ = self.commit();

        // Advance cycle
        self.advance_cycle();
    }

    /// Dispatch an instruction directly
    pub fn dispatch(&mut self, instr: Instruction) -> Result<()> {
        if !self.ooo_engine.can_accept() {
            return Err(EmulatorError::InternalError("Instruction window full".to_string()));
        }

        let instr_id = instr.id;

        self.stats.record_dispatch(instr.id, self.current_cycle);
        self.trace.record_dispatch(&instr, self.current_cycle);

        // Track pipeline stages for visualization - update both trackers
        self.pipeline_tracker.record_fetch(&instr, self.current_cycle);
        self.pipeline_tracker.record_dispatch(instr.id, self.current_cycle);
        self.visualization.pipeline_tracker_mut().record_fetch(&instr, self.current_cycle);
        self.visualization.pipeline_tracker_mut().record_dispatch(instr.id, self.current_cycle);

        // Dispatch to OoO engine and get dependencies
        let dependencies = self.ooo_engine.dispatch(instr)?;

        // Record dependencies for visualization
        for dep in dependencies {
            self.pipeline_tracker.add_dependency(instr_id, dep.producer, dep.is_memory);
            self.visualization.pipeline_tracker_mut().add_dependency(instr_id, dep.producer, dep.is_memory);
        }

        Ok(())
    }

    /// Fetch and dispatch instructions from source
    fn fetch_dispatch<S: InstructionSource>(&mut self, source: &mut S) -> Result<()> {
        let free_slots = self.ooo_engine.free_slots();
        // Limit fetch to fetch_width per cycle
        let fetch_limit = std::cmp::min(free_slots, self.config.fetch_width);
        let mut dispatched = 0;
        let mut failed = 0;

        while dispatched < fetch_limit {
            match source.next() {
                Some(Ok(instr)) => {
                    match self.dispatch(instr) {
                        Ok(()) => dispatched += 1,
                        Err(e) => {
                            tracing::warn!("Dispatch failed: {}", e);
                            failed += 1;
                            // Continue trying to dispatch other instructions
                            if failed > 10 {
                                tracing::warn!("Too many dispatch failures, stopping");
                                break;
                            }
                        }
                    }
                }
                Some(Err(e)) => {
                    tracing::warn!("Instruction fetch error: {}", e);
                    break;
                }
                None => {
                    // Source exhausted
                    break;
                }
            }
        }

        if dispatched > 0 || failed > 0 {
            tracing::debug!("Cycle {}: Dispatched {} instructions ({} failed), window size {}",
                self.current_cycle, dispatched, failed, self.ooo_engine.window_size());
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

            // Track issue stage for visualization - update both trackers
            self.pipeline_tracker.record_issue(id, self.current_cycle);
            self.visualization.pipeline_tracker_mut().record_issue(id, self.current_cycle);

            if instr.opcode_type.is_memory_op() {
                // Handle memory operation
                if let Some(ref mem_access) = instr.mem_access {
                    self.handle_memory_op(id, mem_access)?;
                } else {
                    // Memory op without address - complete immediately
                    self.ooo_engine.mark_completed(id, self.current_cycle + 1);
                    self.pipeline_tracker.record_complete(id, self.current_cycle + 1);
                    self.visualization.pipeline_tracker_mut().record_complete(id, self.current_cycle + 1);
                }
            } else {
                // Compute instruction - complete after latency
                let latency = instr.latency();
                let complete_cycle = self.current_cycle + latency;
                self.ooo_engine.mark_completed(id, complete_cycle);

                // Track execute and complete for visualization - update both trackers
                self.pipeline_tracker.record_execute_end(id, complete_cycle);
                self.pipeline_tracker.record_complete(id, complete_cycle);
                self.visualization.pipeline_tracker_mut().record_execute_end(id, complete_cycle);
                self.visualization.pipeline_tracker_mut().record_complete(id, complete_cycle);
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

        // Memory requests now always complete (no pending state)
        let complete_cycle = request.complete_cycle.unwrap_or(self.current_cycle + 1);
        self.ooo_engine.mark_completed(id, complete_cycle);

        // Track memory stage for visualization - update both trackers
        self.pipeline_tracker.record_memory(id, self.current_cycle, complete_cycle);
        self.pipeline_tracker.record_complete(id, complete_cycle);
        self.visualization.pipeline_tracker_mut().record_memory(id, self.current_cycle, complete_cycle);
        self.visualization.pipeline_tracker_mut().record_complete(id, complete_cycle);

        // Record stats
        if access.is_load {
            let latency = complete_cycle.saturating_sub(self.current_cycle);
            self.stats.record_load(access.size as u64, latency);
            self.stats.record_l1_access(true); // Simplified
        } else {
            self.stats.record_store(access.size as u64, 1);
        }

        Ok(())
    }

    /// Complete pending memory operations
    fn complete_memory(&mut self) -> Result<()> {
        // In this simplified model, all memory operations complete immediately
        // when issued (no asynchronous completion)
        Ok(())
    }

    /// Commit completed instructions
    fn commit(&mut self) -> Result<()> {
        let commit_candidates = self.ooo_engine.get_commit_candidates();

        for instr in commit_candidates {
            let id = instr.id;

            self.ooo_engine.commit(id);
            self.committed_count += 1;

            // Track retire stage for visualization - update both trackers
            self.pipeline_tracker.record_retire(id, self.current_cycle);
            self.visualization.pipeline_tracker_mut().record_retire(id, self.current_cycle);

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

        // Update visualization cycle counter (snapshot is captured in step())
        self.visualization.set_cycle(self.current_cycle);
        self.visualization.set_committed_count(self.committed_count);
    }

    /// Check if simulation should stop
    fn should_stop<S: InstructionSource>(&self, source: &mut S) -> bool {
        // Stop if window is empty and source is exhausted
        self.ooo_engine.is_empty()
    }

    /// Stop the simulation
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Reset the emulator
    pub fn reset(&mut self) {
        self.ooo_engine = OoOEngine::new(self.config.clone()).unwrap();
        self.memory = MemorySubsystem::new(self.config.clone()).unwrap();
        self.chi_manager = ChiManager::new(&self.config);
        self.stats.reset();
        self.trace.clear();
        self.visualization.clear();
        self.pipeline_tracker.clear();
        self.current_cycle = 0;
        self.committed_count = 0;
        self.running = false;
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

    /// Get visualization state
    pub fn visualization(&self) -> &VisualizationState {
        &self.visualization
    }

    /// Get mutable visualization state
    pub fn visualization_mut(&mut self) -> &mut VisualizationState {
        &mut self.visualization
    }

    /// Get pipeline tracker for Konata visualization
    pub fn pipeline_tracker(&self) -> &PipelineTracker {
        &self.pipeline_tracker
    }

    /// Get mutable pipeline tracker
    pub fn pipeline_tracker_mut(&mut self) -> &mut PipelineTracker {
        &mut self.pipeline_tracker
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::TraceInput;
    use crate::types::Reg;

    #[test]
    fn test_cpu_emulator_basic() {
        let config = CPUConfig::minimal();
        let mut cpu = CPUEmulator::new(config).unwrap();

        let mut input = TraceInput::new();
        input.builder(0x1000, OpcodeType::Add)
            .src_reg(Reg(0))
            .dst_reg(Reg(1))
            .build();

        let metrics = cpu.run(&mut input).unwrap();

        assert_eq!(metrics.total_instructions, 1);
    }

    #[test]
    fn test_cpu_emulator_memory() {
        let config = CPUConfig::minimal();
        let mut cpu = CPUEmulator::new(config).unwrap();

        let mut input = TraceInput::new();
        input.builder(0x1000, OpcodeType::Load)
            .dst_reg(Reg(0))
            .mem_access(0x2000, 8, true)
            .build();

        let metrics = cpu.run(&mut input).unwrap();

        assert_eq!(metrics.total_instructions, 1);
    }

    #[test]
    fn test_cpu_reset() {
        let config = CPUConfig::minimal();
        let mut cpu = CPUEmulator::new(config).unwrap();

        cpu.current_cycle = 100;
        cpu.committed_count = 50;

        cpu.reset();

        assert_eq!(cpu.current_cycle(), 0);
        assert_eq!(cpu.committed_count(), 0);
    }
}
