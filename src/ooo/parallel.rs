//! Parallel simulation support for large-scale traces.
//!
//! This module provides batch processing and parallel execution
//! capabilities for handling 1M-10M instruction traces.

use std::sync::Arc;
use std::thread::{self, JoinHandle};

use crate::config::CPUConfig;
use crate::types::{Instruction, InstructionId, OpcodeType, Reg, Result, EmulatorError};
use crate::stats::StatsCollector;

#[cfg(feature = "parallel")]
use rayon::prelude::*;

/// Batch of instructions for parallel processing
#[derive(Debug, Clone)]
pub struct InstructionBatch {
    /// Instructions in this batch
    pub instructions: Vec<Instruction>,
    /// Batch ID
    pub batch_id: u64,
    /// Starting instruction ID
    pub start_id: u64,
}

impl InstructionBatch {
    /// Create a new batch
    pub fn new(batch_id: u64, capacity: usize) -> Self {
        Self {
            instructions: Vec::with_capacity(capacity),
            batch_id,
            start_id: 0,
        }
    }

    /// Add an instruction to the batch
    pub fn push(&mut self, instr: Instruction) {
        if self.instructions.is_empty() {
            self.start_id = instr.id.0;
        }
        self.instructions.push(instr);
    }

    /// Get batch size
    pub fn len(&self) -> usize {
        self.instructions.len()
    }

    /// Check if batch is empty
    pub fn is_empty(&self) -> bool {
        self.instructions.is_empty()
    }

    /// Check if batch is full
    pub fn is_full(&self, max_size: usize) -> bool {
        self.instructions.len() >= max_size
    }

    /// Clear the batch
    pub fn clear(&mut self) {
        self.instructions.clear();
    }
}

/// Result of batch processing
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Batch ID
    pub batch_id: u64,
    /// Number of instructions processed
    pub instr_count: usize,
    /// Total cycles consumed
    pub cycles: u64,
    /// IPC for this batch
    pub ipc: f64,
    /// Cache misses
    pub cache_misses: u64,
    /// Memory operations
    pub mem_ops: u64,
    /// Branch operations
    pub branch_ops: u64,
}

impl Default for BatchResult {
    fn default() -> Self {
        Self {
            batch_id: 0,
            instr_count: 0,
            cycles: 0,
            ipc: 0.0,
            cache_misses: 0,
            mem_ops: 0,
            branch_ops: 0,
        }
    }
}

/// Parallel simulation configuration
#[derive(Debug, Clone)]
pub struct ParallelConfig {
    /// Number of worker threads
    pub num_workers: usize,
    /// Batch size for processing
    pub batch_size: usize,
    /// Enable parallel dependency analysis
    pub parallel_deps: bool,
    /// Statistics collection interval
    pub stats_interval: u64,
}

impl Default for ParallelConfig {
    fn default() -> Self {
        Self {
            num_workers: num_cpus::get().max(1),
            batch_size: 10000,
            parallel_deps: true,
            stats_interval: 100000,
        }
    }
}

/// Dependency analysis result for parallel processing
#[derive(Debug, Clone)]
pub struct DependencyAnalysis {
    /// Instruction ID
    pub instr_id: u64,
    /// Producer instruction IDs (raw dependencies)
    pub producers: Vec<u64>,
    /// Whether this is a memory dependency
    pub is_memory_dep: bool,
}

/// Analyze dependencies in a batch of instructions
pub fn analyze_dependencies_batch(instructions: &[Instruction]) -> Vec<DependencyAnalysis> {
    use std::collections::HashMap;

    let mut last_writer: HashMap<Reg, u64> = HashMap::new();
    let mut last_mem_writer: Option<u64> = None;
    let mut analyses = Vec::with_capacity(instructions.len());

    for instr in instructions {
        let mut producers = Vec::new();
        let mut is_memory_dep = false;

        // Check register dependencies
        for &reg in &instr.src_regs {
            if let Some(&producer_id) = last_writer.get(&reg) {
                producers.push(producer_id);
            }
        }

        // Check memory dependencies
        if instr.mem_access.is_some() {
            is_memory_dep = true;
            if let Some(producer_id) = last_mem_writer {
                if !producers.contains(&producer_id) {
                    producers.push(producer_id);
                }
            }
        }

        analyses.push(DependencyAnalysis {
            instr_id: instr.id.0,
            producers,
            is_memory_dep,
        });

        // Update last writer for destination registers
        for &reg in &instr.dst_regs {
            last_writer.insert(reg, instr.id.0);
        }

        // Update last memory writer
        if instr.mem_access.as_ref().map_or(false, |m| !m.is_load) {
            last_mem_writer = Some(instr.id.0);
        }
    }

    analyses
}

/// Parallel dependency analyzer
#[cfg(feature = "parallel")]
pub struct ParallelDependencyAnalyzer {
    config: ParallelConfig,
}

#[cfg(feature = "parallel")]
impl ParallelDependencyAnalyzer {
    pub fn new(config: ParallelConfig) -> Self {
        Self { config }
    }

    /// Analyze dependencies across multiple batches in parallel
    pub fn analyze_batches(&self, batches: &[InstructionBatch]) -> Vec<Vec<DependencyAnalysis>> {
        batches
            .par_iter()
            .map(|batch| analyze_dependencies_batch(&batch.instructions))
            .collect()
    }
}

/// Batch simulator for parallel execution
pub struct BatchSimulator {
    /// CPU configuration
    config: CPUConfig,
    /// Parallel configuration
    parallel_config: ParallelConfig,
    /// Statistics collector
    stats: StatsCollector,
    /// Current cycle
    current_cycle: u64,
    /// Committed instruction count
    committed_count: u64,
}

impl BatchSimulator {
    /// Create a new batch simulator
    pub fn new(config: CPUConfig) -> Self {
        Self {
            config,
            parallel_config: ParallelConfig::default(),
            stats: StatsCollector::new(),
            current_cycle: 0,
            committed_count: 0,
        }
    }

    /// Set parallel configuration
    pub fn with_parallel_config(mut self, parallel_config: ParallelConfig) -> Self {
        self.parallel_config = parallel_config;
        self
    }

    /// Process a batch of instructions (fast-path simulation)
    pub fn process_batch(&mut self, batch: &InstructionBatch) -> BatchResult {
        let mut result = BatchResult {
            batch_id: batch.batch_id,
            ..Default::default()
        };

        // Analyze dependencies
        let deps = analyze_dependencies_batch(&batch.instructions);

        // Simulate execution
        let mut issue_width = self.config.issue_width;
        let mut issued_this_cycle = 0;

        for (instr, dep) in batch.instructions.iter().zip(deps.iter()) {
            result.instr_count += 1;

            // Check if this is a memory operation
            if instr.opcode_type.is_memory_op() {
                result.mem_ops += 1;
            }

            // Check if this is a branch
            if instr.opcode_type.is_branch() {
                result.branch_ops += 1;
            }

            // Calculate execution latency
            let base_latency = instr.latency();

            // Add dependency stall cycles
            let dep_stall = if dep.producers.is_empty() {
                0
            } else {
                // Simplified: assume each dependency adds some stall
                dep.producers.len() as u64 * 2
            };

            // Issue width throttling
            if issued_this_cycle >= issue_width {
                result.cycles += 1;
                issued_this_cycle = 0;
            }
            issued_this_cycle += 1;

            // Track execution
            let exec_cycles = base_latency + dep_stall;
            result.cycles += exec_cycles;
        }

        // Calculate IPC
        if result.cycles > 0 {
            result.ipc = result.instr_count as f64 / result.cycles as f64;
        }

        // Update global state
        self.current_cycle += result.cycles;
        self.committed_count += result.instr_count as u64;

        result
    }

    /// Process multiple batches
    pub fn process_batches(&mut self, batches: &[InstructionBatch]) -> Vec<BatchResult> {
        batches.iter().map(|b| self.process_batch(b)).collect()
    }

    /// Get total cycles
    pub fn total_cycles(&self) -> u64 {
        self.current_cycle
    }

    /// Get committed instruction count
    pub fn committed_count(&self) -> u64 {
        self.committed_count
    }

    /// Get overall IPC
    pub fn overall_ipc(&self) -> f64 {
        if self.current_cycle > 0 {
            self.committed_count as f64 / self.current_cycle as f64
        } else {
            0.0
        }
    }
}

#[cfg(feature = "parallel")]
impl BatchSimulator {
    /// Process batches in parallel
    pub fn process_batches_parallel(&mut self, batches: &[InstructionBatch]) -> Vec<BatchResult> {
        use std::sync::Mutex;

        let config = Arc::new(self.config.clone());
        let results = Arc::new(Mutex::new(Vec::with_capacity(batches.len())));

        batches.par_iter().for_each(|batch| {
            let mut local_sim = BatchSimulator::new((*config).clone());
            let result = local_sim.process_batch(batch);

            let mut results = results.lock().unwrap();
            results.push(result);
        });

        let mut results = Arc::try_unwrap(results).unwrap().into_inner().unwrap();
        results.sort_by_key(|r| r.batch_id);

        // Aggregate results
        for result in &results {
            self.current_cycle += result.cycles;
            self.committed_count += result.instr_count as u64;
        }

        results
    }
}

/// Create batches from an instruction iterator
pub fn create_batches<I: Iterator<Item = Instruction>>(
    instructions: I,
    batch_size: usize,
) -> Vec<InstructionBatch> {
    let mut batches = Vec::new();
    let mut current_batch = InstructionBatch::new(0, batch_size);
    let mut batch_id = 0u64;

    for instr in instructions {
        current_batch.push(instr);

        if current_batch.len() >= batch_size {
            batches.push(current_batch);
            batch_id += 1;
            current_batch = InstructionBatch::new(batch_id, batch_size);
        }
    }

    // Add remaining instructions
    if !current_batch.is_empty() {
        batches.push(current_batch);
    }

    batches
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_instructions(count: usize) -> Vec<Instruction> {
        (0..count)
            .map(|i| {
                Instruction::new(InstructionId(i as u64), 0x1000 + i as u64 * 4, 0, OpcodeType::Add)
                    .with_src_reg(Reg((i % 30) as u8))
                    .with_dst_reg(Reg(((i + 1) % 30) as u8))
            })
            .collect()
    }

    #[test]
    fn test_batch_creation() {
        let instructions = make_test_instructions(250);
        let batches = create_batches(instructions.into_iter(), 100);

        assert_eq!(batches.len(), 3);
        assert_eq!(batches[0].len(), 100);
        assert_eq!(batches[1].len(), 100);
        assert_eq!(batches[2].len(), 50);
    }

    #[test]
    fn test_dependency_analysis() {
        let instructions = vec![
            Instruction::new(InstructionId(0), 0x1000, 0, OpcodeType::Add)
                .with_src_reg(Reg(0))
                .with_dst_reg(Reg(1)),
            Instruction::new(InstructionId(1), 0x1004, 0, OpcodeType::Add)
                .with_src_reg(Reg(1))
                .with_dst_reg(Reg(2)),
            Instruction::new(InstructionId(2), 0x1008, 0, OpcodeType::Add)
                .with_src_reg(Reg(0))
                .with_dst_reg(Reg(3)),
        ];

        let deps = analyze_dependencies_batch(&instructions);

        assert_eq!(deps.len(), 3);
        assert!(deps[0].producers.is_empty()); // First instruction has no dependencies
        assert_eq!(deps[1].producers, vec![0]); // Depends on instruction 0
        assert!(deps[2].producers.is_empty()); // X0 is not written by previous
    }

    #[test]
    fn test_batch_simulator() {
        let config = CPUConfig::default();
        let mut simulator = BatchSimulator::new(config);

        let mut batch = InstructionBatch::new(0, 100);
        for instr in make_test_instructions(100) {
            batch.push(instr);
        }

        let result = simulator.process_batch(&batch);

        assert_eq!(result.instr_count, 100);
        assert!(result.cycles > 0);
        assert!(result.ipc > 0.0);
    }
}
