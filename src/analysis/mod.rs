//! Analysis module for large-scale CPU simulation.
//!
//! This module provides:
//! - Statistics aggregation for timeline visualization
//! - Anomaly detection for performance issues
//! - Hotspot function analysis
//! - Region selection for focused visualization
//! - TopDown performance analysis (Intel methodology)

pub mod aggregator;
pub mod anomaly_detector;
pub mod function_profiler;
pub mod region_selector;
pub mod topdown;

pub use aggregator::{AggregatedStatistics, StatsBin, TimelinePoint};
pub use anomaly_detector::{Anomaly, AnomalyDetector, AnomalyType};
pub use function_profiler::{FunctionProfiler, FunctionStats};
pub use region_selector::{RegionSelector, RegionOfInterest};
pub use topdown::{
    TopDownAnalyzer, TopDownMetrics, TopDownReport, FrontendBound, BackendBound,
    BadSpeculation, Retiring, StageUtilization, Hotspot, CycleDistribution,
};
