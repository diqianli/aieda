//! CPU Visualization Module
//!
//! This module provides real-time visualization of the CPU emulator's
//! out-of-order execution engine via a web-based interface.
//!
//! # Features
//!
//! - **Instruction Window View**: Shows instructions in the window with their status
//! - **Dependency Graph**: Visualizes dependencies between instructions
//! - **Pipeline View**: Shows instructions flowing through pipeline stages
//! - **Konata Pipeline View**: Detailed stage-by-stage visualization with dependency arrows
//! - **Metrics Dashboard**: Real-time IPC, cache hit rates, etc.
//!
//! # Usage
//!
//! Enable the `visualization` feature and use the visualization server:
//!
//! ```rust,ignore
//! use arm_cpu_emulator::visualization::{VisualizationConfig, VisualizationServer};
//!
//! let config = VisualizationConfig::enabled();
//! let server = VisualizationServer::new(config);
//! server.run().await?;
//! ```

mod snapshot;
mod konata_format;
mod pipeline_tracker;

pub use snapshot::{
    DependencyEdge, DependencyType, InstructionSnapshot, InstructionStatus,
    MetricsSnapshot, PipelineSnapshot, VisualizationConfig, VisualizationSnapshot,
};

pub use konata_format::{
    KonataDependencyRef, KonataDependencyType, KonataLane, KonataMetadata,
    KonataOp, KonataSnapshot, KonataStage, StageId, StageTiming,
};

pub use pipeline_tracker::PipelineTracker;

#[cfg(feature = "visualization")]
mod server;

#[cfg(feature = "visualization")]
pub use server::VisualizationServer;

use crate::types::{Instruction, InstructionId, InstrStatus};
use crate::ooo::{OoOEngine, WindowEntry};
use crate::stats::PerformanceMetrics;
use std::collections::VecDeque;

/// Manages visualization snapshots during simulation.
pub struct VisualizationState {
    /// Configuration
    config: VisualizationConfig,
    /// Snapshots captured so far
    snapshots: VecDeque<VisualizationSnapshot>,
    /// Current cycle
    current_cycle: u64,
    /// Committed instruction count
    committed_count: u64,
    /// Dependencies captured from the last snapshot
    current_dependencies: Vec<DependencyEdge>,
    /// Konata pipeline tracker
    pub pipeline_tracker: PipelineTracker,
    /// Cached Konata snapshots
    konata_snapshots: VecDeque<KonataSnapshot>,
}

impl VisualizationState {
    /// Create a new visualization state.
    pub fn new(config: VisualizationConfig) -> Self {
        let max_snapshots = config.max_snapshots;
        Self {
            config,
            snapshots: VecDeque::with_capacity(max_snapshots),
            current_cycle: 0,
            committed_count: 0,
            current_dependencies: Vec::new(),
            pipeline_tracker: PipelineTracker::new(),
            konata_snapshots: VecDeque::with_capacity(max_snapshots),
        }
    }

    /// Check if visualization is enabled.
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Get the server port.
    pub fn port(&self) -> u16 {
        self.config.port
    }

    /// Update the current cycle.
    pub fn set_cycle(&mut self, cycle: u64) {
        self.current_cycle = cycle;
    }

    /// Update the committed count.
    pub fn set_committed_count(&mut self, count: u64) {
        self.committed_count = count;
    }

    /// Add a dependency edge.
    pub fn add_dependency(&mut self, from: InstructionId, to: InstructionId, dep_type: DependencyType) {
        self.current_dependencies.push(DependencyEdge {
            from: from.0,
            to: to.0,
            dep_type,
        });
    }

    /// Clear dependencies for a new cycle.
    pub fn clear_dependencies(&mut self) {
        self.current_dependencies.clear();
    }

    /// Capture a snapshot of the current CPU state.
    pub fn capture_snapshot(
        &mut self,
        ooo_engine: &OoOEngine,
        metrics: &PerformanceMetrics,
    ) {
        if !self.config.enabled {
            return;
        }

        // Get instruction snapshots from the window
        let instructions = self.collect_instructions(ooo_engine);

        // Collect dependencies from the dependency tracker
        let dependencies = self.collect_dependencies(ooo_engine);

        // Get pipeline counts
        let pipeline = self.collect_pipeline_info(ooo_engine);

        // Create metrics snapshot
        let metrics_snapshot = MetricsSnapshot {
            ipc: metrics.ipc,
            total_cycles: metrics.total_cycles,
            total_instructions: metrics.total_instructions,
            l1_hit_rate: metrics.l1_hit_rate,
            l2_hit_rate: metrics.l2_hit_rate,
            l1_mpki: metrics.l1_mpki,
            l2_mpki: metrics.l2_mpki,
            memory_instr_pct: metrics.memory_instr_pct,
            avg_load_latency: metrics.avg_load_latency,
        };

        let snapshot = VisualizationSnapshot {
            cycle: self.current_cycle,
            committed_count: self.committed_count,
            instructions,
            dependencies,
            metrics: metrics_snapshot,
            pipeline,
        };

        // Add snapshot, maintaining max size
        if self.snapshots.len() >= self.config.max_snapshots {
            self.snapshots.pop_front();
        }
        self.snapshots.push_back(snapshot);

        // Also capture Konata snapshot
        self.capture_konata_snapshot(ooo_engine);
    }

    /// Collect dependencies from the OoO engine.
    fn collect_dependencies(&self, ooo_engine: &OoOEngine) -> Vec<DependencyEdge> {
        let dep_tracker = ooo_engine.dependency_tracker();
        dep_tracker
            .get_all_dependencies()
            .into_iter()
            .map(|(from, to, is_memory)| DependencyEdge {
                from: from.0,
                to: to.0,
                dep_type: if is_memory {
                    DependencyType::Memory
                } else {
                    DependencyType::Register
                },
            })
            .collect()
    }

    /// Collect instruction snapshots from the OoO engine.
    fn collect_instructions(&self, ooo_engine: &OoOEngine) -> Vec<InstructionSnapshot> {
        let mut instructions = Vec::new();
        let dep_tracker = ooo_engine.dependency_tracker();

        for entry in ooo_engine.get_window_entries() {
            let pending_deps = dep_tracker.get_pending_count(entry.instruction.id);
            let snapshot = InstructionSnapshot::from_instruction(
                &entry.instruction,
                entry.status,
                entry.dispatch_cycle,
                entry.issue_cycle,
                entry.complete_cycle,
                pending_deps,
            );
            instructions.push(snapshot);
        }

        instructions
    }

    /// Collect pipeline information.
    fn collect_pipeline_info(&self, ooo_engine: &OoOEngine) -> PipelineSnapshot {
        let stats = ooo_engine.get_stats();
        let (waiting, ready, executing, completed) = ooo_engine.status_counts();

        PipelineSnapshot {
            fetch_count: 0, // Not tracked separately
            dispatch_count: waiting,
            execute_count: ready + executing,
            memory_count: 0, // Not tracked separately
            commit_count: completed,
            window_occupancy: stats.window_occupancy,
            window_capacity: stats.window_capacity,
        }
    }

    /// Get the most recent snapshot.
    pub fn latest_snapshot(&self) -> Option<&VisualizationSnapshot> {
        self.snapshots.back()
    }

    /// Get all snapshots.
    pub fn snapshots(&self) -> &VecDeque<VisualizationSnapshot> {
        &self.snapshots
    }

    /// Get a snapshot by cycle number.
    pub fn get_snapshot(&self, cycle: u64) -> Option<&VisualizationSnapshot> {
        self.snapshots.iter().find(|s| s.cycle == cycle)
    }

    /// Clear all snapshots.
    pub fn clear(&mut self) {
        self.snapshots.clear();
        self.current_cycle = 0;
        self.committed_count = 0;
        self.current_dependencies.clear();
        self.pipeline_tracker.clear();
        self.konata_snapshots.clear();
    }

    /// Get the number of snapshots.
    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    /// Check if there are no snapshots.
    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    /// Get the pipeline tracker.
    pub fn pipeline_tracker(&self) -> &PipelineTracker {
        &self.pipeline_tracker
    }

    /// Get mutable pipeline tracker.
    pub fn pipeline_tracker_mut(&mut self) -> &mut PipelineTracker {
        &mut self.pipeline_tracker
    }

    /// Capture a Konata snapshot.
    pub fn capture_konata_snapshot(&mut self, ooo_engine: &OoOEngine) {
        if !self.config.enabled {
            return;
        }

        // Get all tracked instructions from the window
        let entries: Vec<_> = ooo_engine.get_window_entries().collect();

        // For each instruction in the window, ensure we have timing info
        // This syncs with what the CPU has tracked
        for entry in &entries {
            let id = entry.instruction.id;
            if self.pipeline_tracker.get_timing(id).is_none() {
                // Create timing from window entry data
                let mut timing = StageTiming::new();
                timing.record_dispatch(entry.dispatch_cycle, entry.dispatch_cycle + 1);
                if let Some(issue) = entry.issue_cycle {
                    timing.record_issue(entry.dispatch_cycle, issue);
                    timing.record_execute(issue, entry.complete_cycle.unwrap_or(issue + 1));
                }
                if let Some(complete) = entry.complete_cycle {
                    timing.record_complete(complete);
                }
                if let Some(retire) = entry.retire_cycle {
                    timing.record_retire(retire);
                }
                // Store the timing
                self.pipeline_tracker.timings.insert(id, timing);
            }
        }

        let snapshot = self.pipeline_tracker.to_snapshot(
            entries.into_iter(),
            self.current_cycle,
            self.committed_count,
        );

        // Maintain max size
        if self.konata_snapshots.len() >= self.config.max_snapshots {
            self.konata_snapshots.pop_front();
        }
        self.konata_snapshots.push_back(snapshot);
    }

    /// Get the latest Konata snapshot.
    pub fn latest_konata_snapshot(&self) -> Option<&KonataSnapshot> {
        self.konata_snapshots.back()
    }

    /// Get all Konata snapshots.
    pub fn konata_snapshots(&self) -> &VecDeque<KonataSnapshot> {
        &self.konata_snapshots
    }

    /// Generate a combined Konata snapshot with all tracked instructions.
    pub fn generate_full_konata_snapshot(&self, ooo_engine: &OoOEngine) -> KonataSnapshot {
        let entries: Vec<_> = ooo_engine.get_window_entries().collect();
        self.pipeline_tracker.to_snapshot(
            entries.into_iter(),
            self.current_cycle,
            self.committed_count,
        )
    }
}

impl Default for VisualizationState {
    fn default() -> Self {
        Self::new(VisualizationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_visualization_state() {
        let config = VisualizationConfig::enabled();
        let state = VisualizationState::new(config);
        assert!(state.is_enabled());
        assert_eq!(state.port(), 3000);
    }

    #[test]
    fn test_snapshot_collection() {
        let config = VisualizationConfig {
            max_snapshots: 5,
            ..VisualizationConfig::enabled()
        };
        let mut state = VisualizationState::new(config);

        for i in 0..10 {
            state.set_cycle(i);
            state.set_committed_count(i);
            // Would capture snapshot here with actual engine
        }

        // Should only keep last 5
        // Note: actual snapshots not captured without engine
        assert_eq!(state.current_cycle, 9);
    }
}
