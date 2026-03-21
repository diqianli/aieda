//! Function-level profiling for hotspot analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Statistics for a single function
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionStats {
    /// Function name (from symbol table)
    pub name: String,
    /// Starting PC address
    pub start_pc: u64,
    /// Ending PC address
    pub end_pc: u64,
    /// Total instructions executed in this function
    pub instruction_count: u64,
    /// Total cycles spent in this function
    pub cycle_count: u64,
    /// Instructions per cycle
    pub ipc: f64,
    /// Cache miss rate for this function
    pub cache_miss_rate: f64,
}

impl FunctionStats {
    /// Sort key for hotspot analysis (cycles * (1 - IPC) = wasted cycles)
    pub fn hotspot_score(&self) -> f64 {
        if self.cycle_count == 0 {
            return 0.0;
        }
        // Higher score = more potential for optimization
        let wasted_cycles = self.cycle_count as f64 * (1.0 - self.ipc.min(1.0));
        let cache_penalty = self.cache_miss_rate * 10.0;
        wasted_cycles + cache_penalty * self.instruction_count as f64
    }
}

/// Function profiler for tracking per-function statistics
pub struct FunctionProfiler {
    /// Function statistics by start PC
    functions: HashMap<u64, FunctionStats>,
    /// PC to function mapping
    pc_to_function: HashMap<u64, u64>,
    /// Current function being profiled
    current_function: Option<u64>,
    /// Cache misses in current function
    current_cache_misses: u64,
    /// Memory ops in current function
    current_mem_ops: u64,
}

impl FunctionProfiler {
    /// Create a new function profiler
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            pc_to_function: HashMap::new(),
            current_function: None,
            current_cache_misses: 0,
            current_mem_ops: 0,
        }
    }

    /// Register a function with its address range
    pub fn register_function(&mut self, name: &str, start_pc: u64, end_pc: u64) {
        let stats = FunctionStats {
            name: name.to_string(),
            start_pc,
            end_pc,
            instruction_count: 0,
            cycle_count: 0,
            ipc: 0.0,
            cache_miss_rate: 0.0,
        };

        // Map all PCs in range to this function
        for pc in start_pc..=end_pc {
            self.pc_to_function.insert(pc, start_pc);
        }

        self.functions.insert(start_pc, stats);
    }

    /// Record an instruction execution
    pub fn record_instruction(
        &mut self,
        pc: u64,
        cycles: u64,
        is_memory: bool,
        is_cache_miss: bool,
    ) {
        // Find the function for this PC
        if let Some(&func_pc) = self.pc_to_function.get(&pc) {
            if let Some(stats) = self.functions.get_mut(&func_pc) {
                stats.instruction_count += 1;
                stats.cycle_count += cycles;

                if is_memory {
                    self.current_mem_ops += 1;
                }
                if is_cache_miss {
                    self.current_cache_misses += 1;
                }

                // Update current function
                if self.current_function != Some(func_pc) {
                    // Function changed - finalize previous
                    if let Some(prev_func) = self.current_function {
                        self.finalize_function_stats(prev_func);
                    }
                    self.current_function = Some(func_pc);
                    self.current_cache_misses = if is_cache_miss { 1 } else { 0 };
                    self.current_mem_ops = if is_memory { 1 } else { 0 };
                }
            }
        }
    }

    /// Finalize statistics for a function
    fn finalize_function_stats(&mut self, func_pc: u64) {
        if let Some(stats) = self.functions.get_mut(&func_pc) {
            // Calculate IPC
            if stats.cycle_count > 0 {
                stats.ipc = stats.instruction_count as f64 / stats.cycle_count as f64;
            }

            // Calculate cache miss rate
            if self.current_mem_ops > 0 {
                stats.cache_miss_rate =
                    self.current_cache_misses as f64 / self.current_mem_ops as f64;
            }
        }
    }

    /// Get all function statistics
    pub fn get_stats(&self) -> Vec<&FunctionStats> {
        self.functions.values().collect()
    }

    /// Get hotspots sorted by potential for optimization
    pub fn get_hotspots(&self, limit: usize) -> Vec<&FunctionStats> {
        let mut stats: Vec<_> = self.functions.values().collect();
        stats.sort_by(|a, b| {
            b.hotspot_score()
                .partial_cmp(&a.hotspot_score())
                .unwrap()
        });
        stats.into_iter().take(limit).collect()
    }

    /// Get functions with lowest IPC (performance issues)
    pub fn get_low_ipc_functions(&self, threshold: f64, limit: usize) -> Vec<&FunctionStats> {
        let mut stats: Vec<_> = self
            .functions
            .values()
            .filter(|s| s.ipc < threshold && s.instruction_count > 100)
            .collect();
        stats.sort_by(|a, b| a.ipc.partial_cmp(&b.ipc).unwrap());
        stats.into_iter().take(limit).collect()
    }

    /// Get functions with highest cache miss rates
    pub fn get_cache_bound_functions(&self, threshold: f64, limit: usize) -> Vec<&FunctionStats> {
        let mut stats: Vec<_> = self
            .functions
            .values()
            .filter(|s| s.cache_miss_rate > threshold && s.instruction_count > 100)
            .collect();
        stats.sort_by(|a, b| b.cache_miss_rate.partial_cmp(&a.cache_miss_rate).unwrap());
        stats.into_iter().take(limit).collect()
    }

    /// Finalize profiling and return all stats
    pub fn finalize(mut self) -> Vec<FunctionStats> {
        // Finalize current function
        if let Some(func_pc) = self.current_function {
            self.finalize_function_stats(func_pc);
        }

        self.functions.into_values().collect()
    }
}

impl Default for FunctionProfiler {
    fn default() -> Self {
        Self::new()
    }
}

/// Flame graph node for call stack visualization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlameGraphNode {
    /// Function name
    pub name: String,
    /// Total value (usually cycles or samples)
    pub value: u64,
    /// Self value (excluding children)
    pub self_value: u64,
    /// Child nodes
    pub children: Vec<FlameGraphNode>,
}

impl FlameGraphNode {
    /// Create a new node
    pub fn new(name: &str, value: u64) -> Self {
        Self {
            name: name.to_string(),
            value,
            self_value: value,
            children: Vec::new(),
        }
    }

    /// Add a child node
    pub fn add_child(&mut self, child: FlameGraphNode) {
        self.value += child.value;
        self.children.push(child);
    }

    /// Find or create a child by name
    pub fn find_or_create_child(&mut self, name: &str) -> &mut FlameGraphNode {
        let idx = self.children.iter().position(|c| c.name == name);
        match idx {
            Some(i) => &mut self.children[i],
            None => {
                self.children.push(FlameGraphNode::new(name, 0));
                self.children.last_mut().unwrap()
            }
        }
    }
}

/// Call stack tracker for flame graph generation
pub struct CallStackTracker {
    /// Root node
    root: FlameGraphNode,
    /// Current call stack
    stack: Vec<String>,
    /// Stack to value mapping
    stack_values: HashMap<Vec<String>, u64>,
}

impl CallStackTracker {
    /// Create a new call stack tracker
    pub fn new() -> Self {
        Self {
            root: FlameGraphNode::new("root", 0),
            stack: Vec::new(),
            stack_values: HashMap::new(),
        }
    }

    /// Push a function onto the call stack
    pub fn push(&mut self, function: &str) {
        self.stack.push(function.to_string());
    }

    /// Pop a function from the call stack
    pub fn pop(&mut self) {
        self.stack.pop();
    }

    /// Record samples for the current stack
    pub fn record_samples(&mut self, samples: u64) {
        if self.stack.is_empty() {
            self.root.value += samples;
            return;
        }

        *self.stack_values.entry(self.stack.clone()).or_insert(0) += samples;
    }

    /// Generate flame graph from recorded stacks
    pub fn generate_flame_graph(&self) -> FlameGraphNode {
        let mut root = FlameGraphNode::new("all", 0);

        for (stack, value) in &self.stack_values {
            let mut current = &mut root;

            for func in stack {
                current = current.find_or_create_child(func);
            }

            current.value += value;
            current.self_value += value;
        }

        // Sort children by value
        fn sort_children(node: &mut FlameGraphNode) {
            node.children.sort_by(|a, b| b.value.cmp(&a.value));
            for child in &mut node.children {
                sort_children(child);
            }
        }

        sort_children(&mut root);
        root
    }
}

impl Default for CallStackTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_profiler_basic() {
        let mut profiler = FunctionProfiler::new();

        profiler.register_function("main", 0x1000, 0x10FF);
        profiler.register_function("helper", 0x1100, 0x11FF);

        for i in 0..100 {
            profiler.record_instruction(0x1000 + i, 2, false, false);
        }

        for i in 0..50 {
            profiler.record_instruction(0x1100 + i, 3, true, i % 5 == 0);
        }

        let stats = profiler.finalize();

        assert_eq!(stats.len(), 2);

        let main_stats = stats.iter().find(|s| s.name == "main").unwrap();
        assert_eq!(main_stats.instruction_count, 100);
    }

    #[test]
    fn test_hotspot_detection() {
        let mut profiler = FunctionProfiler::new();

        // Hot function (low IPC, high cache misses)
        profiler.register_function("hot_func", 0x1000, 0x10FF);
        for i in 0..1000 {
            profiler.record_instruction(0x1000 + (i % 256), 5, true, i % 3 == 0);
        }

        // Cold function (high IPC, few cache misses)
        profiler.register_function("cold_func", 0x1100, 0x11FF);
        for i in 0..1000 {
            profiler.record_instruction(0x1100 + (i % 256), 1, false, false);
        }

        let hotspots = profiler.get_hotspots(5);

        assert!(!hotspots.is_empty());
        // Hot function should be first due to low IPC and high cache misses
        assert_eq!(hotspots[0].name, "hot_func");
    }

    #[test]
    fn test_flame_graph() {
        let mut tracker = CallStackTracker::new();

        tracker.push("main");
        tracker.record_samples(100);
        tracker.push("helper");
        tracker.record_samples(50);
        tracker.pop();
        tracker.pop();

        let flame = tracker.generate_flame_graph();

        assert!(flame.value >= 150);
    }
}
