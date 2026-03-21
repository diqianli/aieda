//! Region selection for focused visualization.

use serde::{Deserialize, Serialize};

/// A region of interest in the trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionOfInterest {
    /// Region ID
    pub id: u64,
    /// Region name/description
    pub name: String,
    /// Starting instruction ID
    pub start_instr: u64,
    /// Ending instruction ID
    pub end_instr: u64,
    /// Starting cycle
    pub start_cycle: u64,
    /// Ending cycle
    pub end_cycle: u64,
    /// Region type
    pub region_type: RegionType,
    /// Importance score (0.0 to 1.0)
    pub importance: f64,
}

/// Types of regions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionType {
    /// Anomaly region (performance issue detected)
    Anomaly,
    /// Hotspot region (function with high execution time)
    Hotspot,
    /// User-selected region
    UserSelected,
    /// Automatic region of interest
    AutoDetected,
    /// Benchmark region (marked in trace)
    Benchmark,
}

/// Region selector for identifying and managing regions of interest
pub struct RegionSelector {
    /// Known regions
    regions: Vec<RegionOfInterest>,
    /// Maximum number of regions to track
    max_regions: usize,
    /// Minimum region size in instructions
    min_region_size: u64,
}

impl RegionSelector {
    /// Create a new region selector
    pub fn new() -> Self {
        Self {
            regions: Vec::new(),
            max_regions: 100,
            min_region_size: 100,
        }
    }

    /// Set maximum number of regions
    pub fn with_max_regions(mut self, max: usize) -> Self {
        self.max_regions = max;
        self
    }

    /// Set minimum region size
    pub fn with_min_region_size(mut self, min: u64) -> Self {
        self.min_region_size = min;
        self
    }

    /// Add a region of interest
    pub fn add_region(&mut self, region: RegionOfInterest) {
        // Check minimum size
        if region.end_instr < region.start_instr + self.min_region_size {
            return;
        }

        // Check for overlaps with existing regions
        let overlaps: Vec<_> = self
            .regions
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                r.start_instr <= region.end_instr && r.end_instr >= region.start_instr
            })
            .map(|(i, _)| i)
            .collect();

        // Remove overlapping regions with lower importance
        for i in overlaps.into_iter().rev() {
            if self.regions[i].importance < region.importance {
                self.regions.remove(i);
            }
        }

        self.regions.push(region);

        // Sort by importance and trim
        self.regions.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap());
        self.regions.truncate(self.max_regions);
    }

    /// Add an anomaly region
    pub fn add_anomaly_region(
        &mut self,
        start_instr: u64,
        end_instr: u64,
        start_cycle: u64,
        end_cycle: u64,
        description: &str,
        severity: f64,
    ) {
        let region = RegionOfInterest {
            id: self.regions.len() as u64,
            name: description.to_string(),
            start_instr,
            end_instr,
            start_cycle,
            end_cycle,
            region_type: RegionType::Anomaly,
            importance: severity,
        };
        self.add_region(region);
    }

    /// Add a hotspot region
    pub fn add_hotspot_region(
        &mut self,
        start_instr: u64,
        end_instr: u64,
        start_cycle: u64,
        end_cycle: u64,
        function_name: &str,
        hotspot_score: f64,
    ) {
        let region = RegionOfInterest {
            id: self.regions.len() as u64,
            name: format!("Hotspot: {}", function_name),
            start_instr,
            end_instr,
            start_cycle,
            end_cycle,
            region_type: RegionType::Hotspot,
            importance: hotspot_score.min(1.0),
        };
        self.add_region(region);
    }

    /// Add a user-selected region
    pub fn add_user_region(
        &mut self,
        start_instr: u64,
        end_instr: u64,
        start_cycle: u64,
        end_cycle: u64,
        name: &str,
    ) {
        let region = RegionOfInterest {
            id: self.regions.len() as u64,
            name: name.to_string(),
            start_instr,
            end_instr,
            start_cycle,
            end_cycle,
            region_type: RegionType::UserSelected,
            importance: 1.0, // User regions always have high importance
        };
        self.add_region(region);
    }

    /// Get all regions
    pub fn get_regions(&self) -> &[RegionOfInterest] {
        &self.regions
    }

    /// Get regions overlapping with a given range
    pub fn get_regions_in_range(&self, start: u64, end: u64) -> Vec<&RegionOfInterest> {
        self.regions
            .iter()
            .filter(|r| r.start_instr <= end && r.end_instr >= start)
            .collect()
    }

    /// Get region containing a specific instruction
    pub fn get_region_at(&self, instr: u64) -> Option<&RegionOfInterest> {
        self.regions
            .iter()
            .find(|r| r.start_instr <= instr && r.end_instr >= instr)
    }

    /// Get regions by type
    pub fn get_regions_by_type(&self, region_type: RegionType) -> Vec<&RegionOfInterest> {
        self.regions
            .iter()
            .filter(|r| r.region_type == region_type)
            .collect()
    }

    /// Auto-detect regions from statistics
    pub fn auto_detect_regions(
        &mut self,
        stats: &super::aggregator::AggregatedStatistics,
        anomalies: &[super::anomaly_detector::Anomaly],
    ) {
        // Add anomaly regions
        for anomaly in anomalies {
            self.add_anomaly_region(
                anomaly.start_instr,
                anomaly.end_instr,
                anomaly.start_cycle,
                anomaly.end_cycle,
                &anomaly.description,
                anomaly.severity,
            );
        }

        // Auto-detect from IPC timeline (find low IPC regions)
        self.detect_low_ipc_regions(stats);

        // Auto-detect from cache miss timeline
        self.detect_cache_bound_regions(stats);
    }

    fn detect_low_ipc_regions(&mut self, stats: &super::aggregator::AggregatedStatistics) {
        let threshold = stats.ipc * 0.5; // 50% of average IPC

        let mut region_start = None;

        for (i, point) in stats.ipc_timeline.iter().enumerate() {
            if point.value < threshold {
                if region_start.is_none() {
                    region_start = Some(i);
                }
            } else if let Some(start_idx) = region_start.take() {
                // End of low IPC region
                let start_point = &stats.ipc_timeline[start_idx];
                let end_point = &stats.ipc_timeline[i - 1];

                let importance = 1.0 - (point.value / stats.ipc);

                self.add_region(RegionOfInterest {
                    id: self.regions.len() as u64,
                    name: format!("Low IPC region ({:.2})", stats.ipc_timeline[start_idx].value),
                    start_instr: start_point.instr,
                    end_instr: end_point.instr,
                    start_cycle: start_point.cycle,
                    end_cycle: end_point.cycle,
                    region_type: RegionType::AutoDetected,
                    importance,
                });
            }
        }
    }

    fn detect_cache_bound_regions(&mut self, stats: &super::aggregator::AggregatedStatistics) {
        let threshold = 0.2; // 20% cache miss rate

        let mut region_start = None;

        for (i, point) in stats.l1_miss_timeline.iter().enumerate() {
            if point.value > threshold {
                if region_start.is_none() {
                    region_start = Some(i);
                }
            } else if let Some(start_idx) = region_start.take() {
                // End of cache bound region
                let start_point = &stats.l1_miss_timeline[start_idx];
                let end_point = &stats.l1_miss_timeline[i - 1];

                let importance = stats.l1_miss_timeline[start_idx].value;

                self.add_region(RegionOfInterest {
                    id: self.regions.len() as u64,
                    name: format!(
                        "Cache bound region ({:.0}% miss)",
                        stats.l1_miss_timeline[start_idx].value * 100.0
                    ),
                    start_instr: start_point.instr,
                    end_instr: end_point.instr,
                    start_cycle: start_point.cycle,
                    end_cycle: end_point.cycle,
                    region_type: RegionType::AutoDetected,
                    importance,
                });
            }
        }
    }

    /// Clear all regions
    pub fn clear(&mut self) {
        self.regions.clear();
    }
}

impl Default for RegionSelector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_region() {
        let mut selector = RegionSelector::new();

        selector.add_region(RegionOfInterest {
            id: 0,
            name: "Test region".to_string(),
            start_instr: 0,
            end_instr: 1000,
            start_cycle: 0,
            end_cycle: 2000,
            region_type: RegionType::UserSelected,
            importance: 0.8,
        });

        assert_eq!(selector.get_regions().len(), 1);
    }

    #[test]
    fn test_region_overlap() {
        let mut selector = RegionSelector::new();

        // Add first region
        selector.add_region(RegionOfInterest {
            id: 0,
            name: "Region 1".to_string(),
            start_instr: 0,
            end_instr: 1000,
            start_cycle: 0,
            end_cycle: 2000,
            region_type: RegionType::UserSelected,
            importance: 0.5,
        });

        // Add overlapping region with higher importance
        selector.add_region(RegionOfInterest {
            id: 1,
            name: "Region 2".to_string(),
            start_instr: 500,
            end_instr: 1500,
            start_cycle: 1000,
            end_cycle: 3000,
            region_type: RegionType::Anomaly,
            importance: 0.9,
        });

        // Higher importance region should replace lower
        let regions = selector.get_regions();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].name, "Region 2");
    }

    #[test]
    fn test_get_regions_in_range() {
        let mut selector = RegionSelector::new();

        selector.add_region(RegionOfInterest {
            id: 0,
            name: "Region 1".to_string(),
            start_instr: 0,
            end_instr: 1000,
            start_cycle: 0,
            end_cycle: 2000,
            region_type: RegionType::UserSelected,
            importance: 0.8,
        });

        selector.add_region(RegionOfInterest {
            id: 1,
            name: "Region 2".to_string(),
            start_instr: 2000,
            end_instr: 3000,
            start_cycle: 4000,
            end_cycle: 6000,
            region_type: RegionType::UserSelected,
            importance: 0.8,
        });

        let in_range = selector.get_regions_in_range(500, 1500);
        assert_eq!(in_range.len(), 1);
        assert_eq!(in_range[0].name, "Region 1");
    }
}
