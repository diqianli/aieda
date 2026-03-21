//! TopDown Performance Analysis
//!
//! Implements Intel's TopDown methodology for CPU performance analysis.
//! The TopDown hierarchy identifies performance bottlenecks at different levels:
//!
//! Level 1:
//! - Retiring: Instructions that complete successfully
//! - Bad Speculation: Wasted cycles due to branch mispredictions
//! - Frontend Bound: Bottlenecks in fetch/decode stages
//! - Backend Bound: Bottlenecks in execution/memory stages

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// TopDown Level 1 metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopDownMetrics {
    /// Percentage of cycles spent on useful work (retiring instructions)
    pub retiring_pct: f64,
    /// Percentage of cycles wasted due to speculation (branch mispredictions)
    pub bad_speculation_pct: f64,
    /// Percentage of cycles stalled in frontend (fetch/decode)
    pub frontend_bound_pct: f64,
    /// Percentage of cycles stalled in backend (execution/memory)
    pub backend_bound_pct: f64,
}

/// Detailed breakdown of Frontend Bound
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrontendBound {
    /// Fetch latency issues (cache misses, branch misprediction recovery)
    pub fetch_latency_pct: f64,
    /// Fetch bandwidth issues (decode limitations)
    pub fetch_bandwidth_pct: f64,
    /// ICache miss rate
    pub icache_miss_rate: f64,
    /// ITLB miss rate
    pub itlb_miss_rate: f64,
}

/// Detailed breakdown of Backend Bound
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendBound {
    /// Memory bound (DRAM access, cache misses)
    pub memory_bound_pct: f64,
    /// Core bound (execution unit contention)
    pub core_bound_pct: f64,
    /// L1 dcache miss rate
    pub l1_dcache_miss_rate: f64,
    /// L2 cache miss rate
    pub l2_cache_miss_rate: f64,
    /// L3 cache miss rate
    pub l3_cache_miss_rate: f64,
    /// Average memory latency
    pub avg_mem_latency: f64,
}

/// Detailed breakdown of Bad Speculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BadSpeculation {
    /// Branch misprediction rate
    pub branch_mispred_rate: f64,
    /// Percentage of instructions from mispredicted paths
    pub wasted_instructions_pct: f64,
}

/// Detailed breakdown of Retiring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Retiring {
    /// Percentage of instructions that are ALU operations
    pub alu_ops_pct: f64,
    /// Percentage of instructions that are memory operations
    pub memory_ops_pct: f64,
    /// Percentage of instructions that are branch operations
    pub branch_ops_pct: f64,
    /// Percentage of instructions that are SIMD operations
    pub simd_ops_pct: f64,
}

/// Complete TopDown analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopDownReport {
    /// Basic metrics
    pub total_cycles: u64,
    pub total_instructions: u64,
    pub ipc: f64,

    /// TopDown Level 1 metrics
    pub topdown: TopDownMetrics,

    /// Detailed breakdowns
    pub frontend_bound: FrontendBound,
    pub backend_bound: BackendBound,
    pub bad_speculation: BadSpeculation,
    pub retiring: Retiring,

    /// Pipeline stage utilization
    pub stage_utilization: StageUtilization,

    /// Hot functions/PC ranges
    pub hotspots: Vec<Hotspot>,

    /// Cycle distribution
    pub cycle_distribution: CycleDistribution,
}

/// Pipeline stage utilization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageUtilization {
    /// Fetch stage utilization
    pub fetch_util: f64,
    /// Decode stage utilization
    pub decode_util: f64,
    /// Rename stage utilization
    pub rename_util: f64,
    /// Dispatch stage utilization
    pub dispatch_util: f64,
    /// Issue stage utilization
    pub issue_util: f64,
    /// Execute stage utilization
    pub execute_util: f64,
    /// Memory stage utilization
    pub memory_util: f64,
    /// Commit stage utilization
    pub commit_util: f64,
}

/// Hotspot information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hotspot {
    /// Function name or PC range
    pub name: String,
    /// Start PC
    pub start_pc: u64,
    /// End PC
    pub end_pc: u64,
    /// Number of instructions
    pub instruction_count: u64,
    /// Number of cycles spent
    pub cycle_count: u64,
    /// Percentage of total cycles
    pub cycle_pct: f64,
    /// Average IPC for this hotspot
    pub ipc: f64,
}

/// Cycle distribution analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CycleDistribution {
    /// Cycles with full issue width
    pub full_issue_cycles: u64,
    /// Cycles with partial issue
    pub partial_issue_cycles: u64,
    /// Cycles with no issue (stall)
    pub stall_cycles: u64,
    /// Cycles waiting for memory
    pub memory_stall_cycles: u64,
    /// Cycles waiting for dependencies
    pub dependency_stall_cycles: u64,
}

/// Analyzer for TopDown metrics
pub struct TopDownAnalyzer {
    /// Total cycles simulated
    total_cycles: u64,
    /// Total instructions retired
    total_instructions: u64,

    // Stage statistics
    /// Cycles where fetch was active
    fetch_active_cycles: u64,
    /// Cycles where decode was active
    decode_active_cycles: u64,
    /// Cycles where rename was active
    rename_active_cycles: u64,
    /// Cycles where dispatch was active
    dispatch_active_cycles: u64,
    /// Cycles where issue was active
    issue_active_cycles: u64,
    /// Cycles where execute was active
    execute_active_cycles: u64,
    /// Cycles where memory was active
    memory_active_cycles: u64,
    /// Cycles where commit was active
    commit_active_cycles: u64,

    // Bottleneck tracking
    /// Cycles stalled in frontend
    frontend_stall_cycles: u64,
    /// Cycles stalled in backend
    backend_stall_cycles: u64,
    /// Cycles wasted due to speculation
    speculation_waste_cycles: u64,

    // Instruction type counts
    alu_instructions: u64,
    memory_instructions: u64,
    branch_instructions: u64,
    simd_instructions: u64,

    // Memory statistics
    l1_misses: u64,
    l2_misses: u64,
    l3_misses: u64,
    total_memory_accesses: u64,
    total_memory_latency: u64,

    // Branch statistics
    branch_predictions: u64,
    branch_mispredictions: u64,

    // Issue statistics
    full_issue_cycles: u64,
    partial_issue_cycles: u64,
    no_issue_cycles: u64,

    // Hotspot tracking
    pc_histogram: HashMap<u64, u64>, // PC -> instruction count
    pc_cycles: HashMap<u64, u64>,    // PC -> cycle count
}

impl TopDownAnalyzer {
    pub fn new() -> Self {
        Self {
            total_cycles: 0,
            total_instructions: 0,
            fetch_active_cycles: 0,
            decode_active_cycles: 0,
            rename_active_cycles: 0,
            dispatch_active_cycles: 0,
            issue_active_cycles: 0,
            execute_active_cycles: 0,
            memory_active_cycles: 0,
            commit_active_cycles: 0,
            frontend_stall_cycles: 0,
            backend_stall_cycles: 0,
            speculation_waste_cycles: 0,
            alu_instructions: 0,
            memory_instructions: 0,
            branch_instructions: 0,
            simd_instructions: 0,
            l1_misses: 0,
            l2_misses: 0,
            l3_misses: 0,
            total_memory_accesses: 0,
            total_memory_latency: 0,
            branch_predictions: 0,
            branch_mispredictions: 0,
            full_issue_cycles: 0,
            partial_issue_cycles: 0,
            no_issue_cycles: 0,
            pc_histogram: HashMap::new(),
            pc_cycles: HashMap::new(),
        }
    }

    /// Record a cycle with the given issue count
    pub fn record_cycle(&mut self, issue_count: u64, issue_width: u64) {
        self.total_cycles += 1;

        if issue_count == 0 {
            self.no_issue_cycles += 1;
        } else if issue_count == issue_width {
            self.full_issue_cycles += 1;
        } else {
            self.partial_issue_cycles += 1;
        }
    }

    /// Record an instruction retirement
    pub fn record_instruction(&mut self, pc: u64, is_memory: bool, is_branch: bool, cycles: u64) {
        self.total_instructions += 1;

        // Update PC histogram
        *self.pc_histogram.entry(pc).or_insert(0) += 1;
        *self.pc_cycles.entry(pc).or_insert(0) += cycles;

        // Classify instruction
        if is_memory {
            self.memory_instructions += 1;
        } else if is_branch {
            self.branch_instructions += 1;
        } else {
            self.alu_instructions += 1;
        }
    }

    /// Record memory access
    pub fn record_memory_access(&mut self, l1_hit: bool, l2_hit: bool, l3_hit: bool, latency: u64) {
        self.total_memory_accesses += 1;
        self.total_memory_latency += latency;

        if !l1_hit {
            self.l1_misses += 1;
            if !l2_hit {
                self.l2_misses += 1;
                if !l3_hit {
                    self.l3_misses += 1;
                }
            }
        }
    }

    /// Record branch prediction
    pub fn record_branch_prediction(&mut self, mispredicted: bool) {
        self.branch_predictions += 1;
        if mispredicted {
            self.branch_mispredictions += 1;
        }
    }

    /// Generate the analysis report
    pub fn generate_report(&self, stage_util: StageUtilization) -> TopDownReport {
        let total = self.total_cycles.max(1);

        // Calculate TopDown Level 1 metrics
        // These are simplified approximations
        let retiring_pct = if self.total_instructions > 0 {
            (self.total_instructions as f64 / total as f64).min(1.0) * 100.0
        } else {
            0.0
        };

        // Estimate speculation waste from branch mispredictions
        let bad_speculation_pct = if self.branch_predictions > 0 {
            (self.branch_mispredictions as f64 / self.branch_predictions as f64) * 15.0 // Penalty factor
        } else {
            0.0
        };

        // Frontend bound based on issue stalls and fetch activity
        let frontend_bound_pct = if self.no_issue_cycles > 0 {
            (self.no_issue_cycles as f64 / total as f64) * 30.0 // Approximation
        } else {
            0.0
        };

        // Backend bound is the remainder
        let backend_bound_pct = (100.0 - retiring_pct - bad_speculation_pct - frontend_bound_pct).max(0.0);

        // Calculate detailed metrics
        let frontend_bound = FrontendBound {
            fetch_latency_pct: frontend_bound_pct * 0.6,
            fetch_bandwidth_pct: frontend_bound_pct * 0.4,
            icache_miss_rate: if self.total_memory_accesses > 0 {
                (self.l1_misses as f64 / self.total_memory_accesses as f64) * 100.0
            } else {
                0.0
            },
            itlb_miss_rate: 0.0, // Not tracked
        };

        let backend_bound = BackendBound {
            memory_bound_pct: backend_bound_pct * 0.7,
            core_bound_pct: backend_bound_pct * 0.3,
            l1_dcache_miss_rate: if self.total_memory_accesses > 0 {
                (self.l1_misses as f64 / self.total_memory_accesses as f64) * 100.0
            } else {
                0.0
            },
            l2_cache_miss_rate: if self.l1_misses > 0 {
                (self.l2_misses as f64 / self.l1_misses as f64) * 100.0
            } else {
                0.0
            },
            l3_cache_miss_rate: if self.l2_misses > 0 {
                (self.l3_misses as f64 / self.l2_misses as f64) * 100.0
            } else {
                0.0
            },
            avg_mem_latency: if self.total_memory_accesses > 0 {
                self.total_memory_latency as f64 / self.total_memory_accesses as f64
            } else {
                0.0
            },
        };

        let bad_speculation = BadSpeculation {
            branch_mispred_rate: if self.branch_predictions > 0 {
                (self.branch_mispredictions as f64 / self.branch_predictions as f64) * 100.0
            } else {
                0.0
            },
            wasted_instructions_pct: bad_speculation_pct * 0.5,
        };

        let retiring = Retiring {
            alu_ops_pct: if self.total_instructions > 0 {
                (self.alu_instructions as f64 / self.total_instructions as f64) * 100.0
            } else {
                0.0
            },
            memory_ops_pct: if self.total_instructions > 0 {
                (self.memory_instructions as f64 / self.total_instructions as f64) * 100.0
            } else {
                0.0
            },
            branch_ops_pct: if self.total_instructions > 0 {
                (self.branch_instructions as f64 / self.total_instructions as f64) * 100.0
            } else {
                0.0
            },
            simd_ops_pct: if self.total_instructions > 0 {
                (self.simd_instructions as f64 / self.total_instructions as f64) * 100.0
            } else {
                0.0
            },
        };

        // Generate hotspots
        let mut hotspots: Vec<Hotspot> = self.pc_histogram
            .iter()
            .map(|(pc, count)| {
                let cycles = self.pc_cycles.get(pc).copied().unwrap_or(0);
                Hotspot {
                    name: format!("PC_{:08X}", pc),
                    start_pc: *pc,
                    end_pc: *pc + 4,
                    instruction_count: *count,
                    cycle_count: cycles,
                    cycle_pct: (cycles as f64 / total as f64) * 100.0,
                    ipc: if cycles > 0 { *count as f64 / cycles as f64 } else { 0.0 },
                }
            })
            .collect();

        // Sort by cycle count (descending) and take top 20
        hotspots.sort_by(|a, b| b.cycle_count.cmp(&a.cycle_count));
        hotspots.truncate(20);

        let cycle_distribution = CycleDistribution {
            full_issue_cycles: self.full_issue_cycles,
            partial_issue_cycles: self.partial_issue_cycles,
            stall_cycles: self.no_issue_cycles,
            memory_stall_cycles: (self.no_issue_cycles as f64 * 0.4) as u64, // Estimate
            dependency_stall_cycles: (self.no_issue_cycles as f64 * 0.3) as u64, // Estimate
        };

        TopDownReport {
            total_cycles: self.total_cycles,
            total_instructions: self.total_instructions,
            ipc: if self.total_cycles > 0 {
                self.total_instructions as f64 / self.total_cycles as f64
            } else {
                0.0
            },
            topdown: TopDownMetrics {
                retiring_pct,
                bad_speculation_pct,
                frontend_bound_pct,
                backend_bound_pct,
            },
            frontend_bound,
            backend_bound,
            bad_speculation,
            retiring,
            stage_utilization: stage_util,
            hotspots,
            cycle_distribution,
        }
    }
}

impl Default for TopDownAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
