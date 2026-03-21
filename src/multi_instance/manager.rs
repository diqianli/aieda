//! Instance Manager
//!
//! This module provides management of multiple simulation instances,
//! including parallel execution and result aggregation.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use rayon::prelude::*;

use super::instance::{
    InstanceId, InstanceMetrics, InstanceResult, InstanceStats, SimulationInstance,
};
use crate::config::CPUConfig;
use crate::types::Result;

/// Global instance ID counter
static INSTANCE_ID_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Generate a new unique instance ID
pub fn generate_instance_id() -> InstanceId {
    InstanceId(INSTANCE_ID_COUNTER.fetch_add(1, Ordering::SeqCst))
}

/// Configuration for running multiple instances
#[derive(Debug, Clone)]
pub struct MultiRunConfig {
    /// Maximum cycles per instance
    pub max_cycles: u64,
    /// Maximum instructions per instance
    pub max_instructions: u64,
    /// Whether to run in parallel
    pub parallel: bool,
    /// Number of threads for parallel execution (0 = auto)
    pub num_threads: usize,
    /// Whether to save traces
    pub save_traces: bool,
    /// Output directory for traces
    pub trace_output_dir: Option<PathBuf>,
}

impl Default for MultiRunConfig {
    fn default() -> Self {
        Self {
            max_cycles: 1_000_000,
            max_instructions: 1_000_000,
            parallel: true,
            num_threads: 0,
            save_traces: false,
            trace_output_dir: None,
        }
    }
}

/// Aggregated results from multiple instances
#[derive(Debug, Clone, Default)]
pub struct AggregatedResults {
    /// Total number of instances
    pub total_instances: usize,
    /// Number of successful instances
    pub successful_instances: usize,
    /// Number of failed instances
    pub failed_instances: usize,
    /// Average IPC across instances
    pub avg_ipc: f64,
    /// Min IPC
    pub min_ipc: f64,
    /// Max IPC
    pub max_ipc: f64,
    /// Average cache hit rate
    pub avg_cache_hit_rate: f64,
    /// Total execution time in milliseconds
    pub total_execution_time_ms: u64,
    /// Individual instance results
    pub instance_results: Vec<InstanceResult>,
}

impl AggregatedResults {
    /// Calculate aggregated statistics from individual results
    pub fn from_results(results: Vec<InstanceResult>) -> Self {
        let total_instances = results.len();
        let successful_instances = results.iter().filter(|r| r.error.is_none()).count();
        let failed_instances = total_instances - successful_instances;

        let ipcs: Vec<f64> = results.iter().map(|r| r.metrics.perf.ipc).collect();
        let avg_ipc = if !ipcs.is_empty() {
            ipcs.iter().sum::<f64>() / ipcs.len() as f64
        } else {
            0.0
        };
        let min_ipc = ipcs
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min);
        let max_ipc = ipcs
            .iter()
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        let cache_rates: Vec<f64> = results
            .iter()
            .map(|r| r.metrics.perf.l1_hit_rate)
            .collect();
        let avg_cache_hit_rate = if !cache_rates.is_empty() {
            cache_rates.iter().sum::<f64>() / cache_rates.len() as f64
        } else {
            1.0
        };

        let total_execution_time_ms = results
            .iter()
            .map(|r| r.metrics.execution_time_ms)
            .sum();

        Self {
            total_instances,
            successful_instances,
            failed_instances,
            avg_ipc,
            min_ipc,
            max_ipc,
            avg_cache_hit_rate,
            total_execution_time_ms,
            instance_results: results,
        }
    }

    /// Get a summary string
    pub fn summary(&self) -> String {
        format!(
            "Aggregated Results:\n\
             ==================\n\
             Total instances: {}\n\
             Successful: {}\n\
             Failed: {}\n\
             \n\
             IPC Statistics:\n\
               Average: {:.3}\n\
               Min: {:.3}\n\
               Max: {:.3}\n\
             \n\
             Cache Hit Rate: {:.2}%\n\
             Total execution time: {} ms",
            self.total_instances,
            self.successful_instances,
            self.failed_instances,
            self.avg_ipc,
            self.min_ipc,
            self.max_ipc,
            self.avg_cache_hit_rate * 100.0,
            self.total_execution_time_ms
        )
    }
}

/// Manager for multiple simulation instances
pub struct InstanceManager {
    /// CPU configuration template
    config_template: CPUConfig,
    /// Multi-run configuration
    run_config: MultiRunConfig,
    /// Active instances
    instances: Arc<Mutex<HashMap<InstanceId, SimulationInstance>>>,
    /// Completed results
    results: Arc<Mutex<Vec<InstanceResult>>>,
    /// Cancellation flag
    cancelled: Arc<AtomicBool>,
}

impl InstanceManager {
    /// Create a new instance manager
    pub fn new(config_template: CPUConfig) -> Self {
        Self {
            config_template,
            run_config: MultiRunConfig::default(),
            instances: Arc::new(Mutex::new(HashMap::new())),
            results: Arc::new(Mutex::new(Vec::new())),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create an instance manager with custom run configuration
    pub fn with_run_config(config_template: CPUConfig, run_config: MultiRunConfig) -> Self {
        Self {
            config_template,
            run_config,
            instances: Arc::new(Mutex::new(HashMap::new())),
            results: Arc::new(Mutex::new(Vec::new())),
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a new simulation instance
    pub fn create_instance(&self) -> InstanceId {
        let id = generate_instance_id();
        let instance = SimulationInstance::new(id, self.config_template.clone());

        let mut instances = self.instances.lock().unwrap();
        instances.insert(id, instance);

        id
    }

    /// Get an instance by ID (returns a clone)
    pub fn get_instance(&self, id: InstanceId) -> Option<SimulationInstance> {
        let instances = self.instances.lock().unwrap();
        instances.get(&id).cloned()
    }

    /// Remove an instance
    pub fn remove_instance(&self, id: InstanceId) -> Option<SimulationInstance> {
        let mut instances = self.instances.lock().unwrap();
        instances.remove(&id)
    }

    /// Run a single instance
    pub fn run_instance(&self, id: InstanceId) -> Result<InstanceResult> {
        let mut instances = self.instances.lock().unwrap();

        let instance = instances
            .get_mut(&id)
            .ok_or_else(|| crate::types::EmulatorError::InternalError("Instance not found".into()))?;

        let result = instance.run_cycles(self.run_config.max_cycles)?;

        // Store result
        let mut results = self.results.lock().unwrap();
        results.push(result.clone());

        Ok(result)
    }

    /// Run all instances in parallel
    pub fn run_all_parallel(&self) -> Result<AggregatedResults> {
        // Collect all instance IDs
        let ids: Vec<InstanceId> = {
            let instances = self.instances.lock().unwrap();
            instances.keys().copied().collect()
        };

        if ids.is_empty() {
            return Ok(AggregatedResults::default());
        }

        // Configure thread pool
        let num_threads = if self.run_config.num_threads > 0 {
            self.run_config.num_threads
        } else {
            num_cpus::get()
        };

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(num_threads)
            .build()
            .map_err(|e| {
                crate::types::EmulatorError::InternalError(format!("Thread pool error: {}", e))
            })?;

        let config = self.config_template.clone();
        let max_cycles = self.run_config.max_cycles;

        // Run instances in parallel
        let results: Vec<InstanceResult> = pool.install(|| {
            ids.into_par_iter()
                .map(|id| {
                    let mut instance = SimulationInstance::new(id, config.clone());
                    instance.run_cycles(max_cycles).unwrap_or_else(|e| InstanceResult {
                        instance_id: id,
                        metrics: InstanceMetrics::default(),
                        stats: InstanceStats::default(),
                        trace_path: None,
                        error: Some(format!("Error: {:?}", e)),
                    })
                })
                .collect()
        });

        // Update stored results
        {
            let mut stored_results = self.results.lock().unwrap();
            stored_results.extend(results.clone());
        }

        Ok(AggregatedResults::from_results(results))
    }

    /// Cancel all running instances
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Get all completed results
    pub fn get_results(&self) -> Vec<InstanceResult> {
        self.results.lock().unwrap().clone()
    }

    /// Clear all results
    pub fn clear_results(&self) {
        self.results.lock().unwrap().clear();
    }

    /// Get number of active instances
    pub fn instance_count(&self) -> usize {
        self.instances.lock().unwrap().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_manager_creation() {
        let config = CPUConfig::default();
        let manager = InstanceManager::new(config);
        assert_eq!(manager.instance_count(), 0);
    }

    #[test]
    fn test_create_instance() {
        let config = CPUConfig::default();
        let manager = InstanceManager::new(config);
        let id = manager.create_instance();
        assert_eq!(manager.instance_count(), 1);
    }

    #[test]
    fn test_generate_instance_id() {
        let id1 = generate_instance_id();
        let id2 = generate_instance_id();
        assert_ne!(id1, id2);
    }
}
