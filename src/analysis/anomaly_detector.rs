//! Anomaly detection for performance analysis.
//!
//! Identifies performance issues such as IPC drops, cache miss spikes,
//! and pipeline bubbles.

use serde::{Deserialize, Serialize};

/// Types of performance anomalies
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnomalyType {
    /// IPC dropped significantly
    IPCDrop,
    /// Pipeline bubble (no instructions issued)
    PipelineBubble,
    /// Cache miss rate spiked
    CacheMissSpike,
    /// Memory bottleneck detected
    MemoryBottleneck,
    /// Branch misprediction spike
    BranchMispredict,
    /// Unusual instruction latency
    HighLatency,
}

impl std::fmt::Display for AnomalyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IPCDrop => write!(f, "IPC Drop"),
            Self::PipelineBubble => write!(f, "Pipeline Bubble"),
            Self::CacheMissSpike => write!(f, "Cache Miss Spike"),
            Self::MemoryBottleneck => write!(f, "Memory Bottleneck"),
            Self::BranchMispredict => write!(f, "Branch Mispredict"),
            Self::HighLatency => write!(f, "High Latency"),
        }
    }
}

/// A detected anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    /// Anomaly type
    pub anomaly_type: AnomalyType,
    /// Starting instruction ID
    pub start_instr: u64,
    /// Ending instruction ID
    pub end_instr: u64,
    /// Starting cycle
    pub start_cycle: u64,
    /// Ending cycle
    pub end_cycle: u64,
    /// Severity (0.0 to 1.0)
    pub severity: f64,
    /// Human-readable description
    pub description: String,
    /// Related metadata
    pub metadata: std::collections::HashMap<String, f64>,
}

impl Anomaly {
    /// Create a new anomaly
    pub fn new(
        anomaly_type: AnomalyType,
        start_instr: u64,
        end_instr: u64,
        start_cycle: u64,
        end_cycle: u64,
        severity: f64,
        description: impl Into<String>,
    ) -> Self {
        Self {
            anomaly_type,
            start_instr,
            end_instr,
            start_cycle,
            end_cycle,
            severity,
            description: description.into(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: &str, value: f64) -> Self {
        self.metadata.insert(key.to_string(), value);
        self
    }
}

/// Configuration for anomaly detection
#[derive(Debug, Clone)]
pub struct AnomalyDetectorConfig {
    /// IPC drop threshold (trigger if IPC drops below this fraction of previous)
    pub ipc_drop_threshold: f64,
    /// Absolute minimum IPC threshold
    pub ipc_min_threshold: f64,
    /// Cache miss rate threshold
    pub cache_miss_threshold: f64,
    /// Bubble threshold (consecutive cycles with no progress)
    pub bubble_threshold: u64,
    /// Memory bottleneck threshold (fraction of memory ops)
    pub memory_bottleneck_threshold: f64,
    /// Latency threshold (cycles)
    pub latency_threshold: u64,
}

impl Default for AnomalyDetectorConfig {
    fn default() -> Self {
        Self {
            ipc_drop_threshold: 0.5,     // 50% drop
            ipc_min_threshold: 0.3,      // Minimum 0.3 IPC
            cache_miss_threshold: 0.3,   // 30% miss rate
            bubble_threshold: 10,        // 10 consecutive bubbles
            memory_bottleneck_threshold: 0.5, // 50% memory ops
            latency_threshold: 50,       // 50 cycles latency
        }
    }
}

/// Anomaly detector for performance analysis
pub struct AnomalyDetector {
    config: AnomalyDetectorConfig,
}

impl AnomalyDetector {
    /// Create a new anomaly detector with default config
    pub fn new() -> Self {
        Self::with_config(AnomalyDetectorConfig::default())
    }

    /// Create a new anomaly detector with custom config
    pub fn with_config(config: AnomalyDetectorConfig) -> Self {
        Self { config }
    }

    /// Detect anomalies in aggregated statistics
    pub fn detect(&self, stats: &super::aggregator::AggregatedStatistics) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();

        // Detect IPC drops
        anomalies.extend(self.detect_ipc_drops(stats));

        // Detect cache miss spikes
        anomalies.extend(self.detect_cache_miss_spikes(stats));

        // Detect pipeline bubbles
        anomalies.extend(self.detect_pipeline_bubbles(stats));

        // Detect memory bottlenecks
        anomalies.extend(self.detect_memory_bottlenecks(stats));

        // Sort by severity (highest first)
        anomalies.sort_by(|a, b| b.severity.partial_cmp(&a.severity).unwrap());

        anomalies
    }

    fn detect_ipc_drops(&self, stats: &super::aggregator::AggregatedStatistics) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();

        // Look for significant IPC drops in timeline
        let windows = stats.ipc_timeline.windows(3);
        let mut last_drop_end = 0u64;

        for window in windows {
            let prev = window[0].value;
            let curr = window[1].value;
            let next = window[2].value;

            // Skip if already in a detected drop region
            if window[1].instr <= last_drop_end {
                continue;
            }

            // Check for IPC drop
            if prev > 0.0 {
                let drop_ratio = curr / prev;

                if drop_ratio < self.config.ipc_drop_threshold
                    || curr < self.config.ipc_min_threshold
                {
                    // Calculate severity
                    let severity = if curr < self.config.ipc_min_threshold {
                        1.0
                    } else {
                        1.0 - drop_ratio
                    };

                    // Find end of drop (when IPC recovers)
                    let end_idx = stats
                        .ipc_timeline
                        .iter()
                        .position(|p| p.instr > window[1].instr && p.value > curr * 1.5)
                        .map(|i| i.saturating_sub(1))
                        .unwrap_or(stats.ipc_timeline.len() - 1);

                    let end_point = &stats.ipc_timeline[end_idx];

                    last_drop_end = end_point.instr;

                    anomalies.push(
                        Anomaly::new(
                            AnomalyType::IPCDrop,
                            window[1].instr,
                            end_point.instr,
                            window[1].cycle,
                            end_point.cycle,
                            severity,
                            format!(
                                "IPC dropped from {:.2} to {:.2} ({:.0}% decrease)",
                                prev,
                                curr,
                                (1.0 - drop_ratio) * 100.0
                            ),
                        )
                        .with_metadata("prev_ipc", prev)
                        .with_metadata("curr_ipc", curr)
                        .with_metadata("drop_ratio", drop_ratio),
                    );
                }
            }
        }

        anomalies
    }

    fn detect_cache_miss_spikes(
        &self,
        stats: &super::aggregator::AggregatedStatistics,
    ) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();

        // Check L1 miss timeline
        for window in stats.l1_miss_timeline.windows(2) {
            let prev = window[0].value;
            let curr = window[1].value;

            if curr > self.config.cache_miss_threshold
                && curr > prev * 2.0
            {
                let severity = (curr - prev).min(1.0);

                anomalies.push(
                    Anomaly::new(
                        AnomalyType::CacheMissSpike,
                        window[1].instr,
                        window[1].instr + stats.bin_size,
                        window[1].cycle,
                        window[1].cycle + stats.bin_size as u64,
                        severity,
                        format!(
                            "L1 cache miss rate spiked from {:.1}% to {:.1}%",
                            prev * 100.0,
                            curr * 100.0
                        ),
                    )
                    .with_metadata("prev_rate", prev)
                    .with_metadata("curr_rate", curr),
                );
            }
        }

        // Check L2 miss timeline
        for window in stats.l2_miss_timeline.windows(2) {
            let prev = window[0].value;
            let curr = window[1].value;

            if curr > self.config.cache_miss_threshold * 0.5
                && curr > prev * 2.0
            {
                let severity = (curr - prev).min(1.0);

                anomalies.push(
                    Anomaly::new(
                        AnomalyType::CacheMissSpike,
                        window[1].instr,
                        window[1].instr + stats.bin_size,
                        window[1].cycle,
                        window[1].cycle + stats.bin_size as u64,
                        severity,
                        format!(
                            "L2 cache miss rate spiked from {:.1}% to {:.1}%",
                            prev * 100.0,
                            curr * 100.0
                        ),
                    )
                    .with_metadata("prev_rate", prev)
                    .with_metadata("curr_rate", curr),
                );
            }
        }

        anomalies
    }

    fn detect_pipeline_bubbles(
        &self,
        stats: &super::aggregator::AggregatedStatistics,
    ) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();

        for bin in &stats.bins {
            if bin.bubbles >= self.config.bubble_threshold {
                let severity = (bin.bubbles as f64 / self.config.bubble_threshold as f64 / 5.0)
                    .min(1.0);

                anomalies.push(
                    Anomaly::new(
                        AnomalyType::PipelineBubble,
                        bin.start_instr,
                        bin.end_instr,
                        bin.start_cycle,
                        bin.end_cycle,
                        severity,
                        format!(
                            "Pipeline bubble: {} consecutive cycles with no progress",
                            bin.bubbles
                        ),
                    )
                    .with_metadata("bubble_count", bin.bubbles as f64),
                );
            }
        }

        anomalies
    }

    fn detect_memory_bottlenecks(
        &self,
        stats: &super::aggregator::AggregatedStatistics,
    ) -> Vec<Anomaly> {
        let mut anomalies = Vec::new();

        for bin in &stats.bins {
            if bin.instr_count == 0 {
                continue;
            }

            let mem_ratio = bin.mem_ops as f64 / bin.instr_count as f64;

            if mem_ratio > self.config.memory_bottleneck_threshold {
                let severity = mem_ratio.min(1.0);

                anomalies.push(
                    Anomaly::new(
                        AnomalyType::MemoryBottleneck,
                        bin.start_instr,
                        bin.end_instr,
                        bin.start_cycle,
                        bin.end_cycle,
                        severity,
                        format!(
                            "Memory bottleneck: {:.0}% of instructions are memory operations",
                            mem_ratio * 100.0
                        ),
                    )
                    .with_metadata("mem_ratio", mem_ratio)
                    .with_metadata("mem_ops", bin.mem_ops as f64),
                );
            }
        }

        anomalies
    }
}

impl Default for AnomalyDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::aggregator::{AggregatedStatistics, StatsBin, TimelinePoint};

    fn make_test_stats() -> AggregatedStatistics {
        AggregatedStatistics {
            total_instructions: 1000,
            total_cycles: 2000,
            ipc: 0.5,
            bin_size: 100,
            bin_count: 10,
            ipc_timeline: vec![
                TimelinePoint::new(0, 0, 1.0),
                TimelinePoint::new(100, 100, 0.9),
                TimelinePoint::new(200, 200, 0.3), // IPC drop
                TimelinePoint::new(300, 300, 0.35),
                TimelinePoint::new(400, 400, 0.8), // Recovery
                TimelinePoint::new(500, 500, 1.0),
            ],
            throughput_timeline: vec![],
            l1_miss_timeline: vec![
                TimelinePoint::new(0, 0, 0.05),
                TimelinePoint::new(100, 100, 0.1),
                TimelinePoint::new(200, 200, 0.5), // Spike
                TimelinePoint::new(300, 300, 0.1),
            ],
            l2_miss_timeline: vec![],
            bins: vec![
                StatsBin {
                    start_instr: 0,
                    end_instr: 99,
                    bubbles: 5,
                    ..Default::default()
                },
                StatsBin {
                    start_instr: 100,
                    end_instr: 199,
                    mem_ops: 80,
                    instr_count: 100,
                    bubbles: 15, // Bubble anomaly
                    ..Default::default()
                },
            ],
            function_stats: vec![],
            cache_stats: Default::default(),
            pipeline_utilization: Default::default(),
            anomalies: vec![],
        }
    }

    #[test]
    fn test_detect_ipc_drops() {
        let detector = AnomalyDetector::new();
        let stats = make_test_stats();

        let anomalies = detector.detect(&stats);

        let ipc_drops: Vec<_> = anomalies
            .iter()
            .filter(|a| a.anomaly_type == AnomalyType::IPCDrop)
            .collect();

        assert!(!ipc_drops.is_empty());
    }

    #[test]
    fn test_detect_cache_spikes() {
        let detector = AnomalyDetector::new();
        let stats = make_test_stats();

        let anomalies = detector.detect(&stats);

        let cache_spikes: Vec<_> = anomalies
            .iter()
            .filter(|a| a.anomaly_type == AnomalyType::CacheMissSpike)
            .collect();

        assert!(!cache_spikes.is_empty());
    }

    #[test]
    fn test_detect_bubbles() {
        let detector = AnomalyDetector::new();
        let stats = make_test_stats();

        let anomalies = detector.detect(&stats);

        let bubbles: Vec<_> = anomalies
            .iter()
            .filter(|a| a.anomaly_type == AnomalyType::PipelineBubble)
            .collect();

        assert!(!bubbles.is_empty());
    }
}
