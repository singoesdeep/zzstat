//! Resolved stat results module.
//!
//! Contains the `ResolvedStat` type, which represents a fully resolved
//! stat value with complete breakdown information for debugging.

use crate::stat_id::StatId;
use serde::{Deserialize, Serialize};

/// A resolved stat value with full breakdown information.
///
/// This is read-only, copyable, network-safe, and replay-safe.
/// Contains the final value along with a complete breakdown of
/// all sources and transforms that contributed to it.
///
/// # Examples
///
/// ```rust
/// use zzstat::ResolvedStat;
/// use zzstat::StatId;
///
/// let mut resolved = ResolvedStat::new(StatId::from_str("HP"), 150.0);
/// resolved.add_source("Base", 100.0);
/// resolved.add_source("Item", 50.0);
/// resolved.add_transform("Multiplier 1.5x", 150.0);
///
/// assert_eq!(resolved.value, 150.0);
/// assert_eq!(resolved.sources.len(), 2);
/// assert_eq!(resolved.transforms.len(), 1);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResolvedStat {
    /// The stat identifier.
    pub stat_id: StatId,
    
    /// The final resolved value.
    pub value: f64,
    
    /// Breakdown of all contributing sources.
    ///
    /// Each entry is `(source_description, value)`.
    /// Sources are listed in the order they were registered.
    pub sources: Vec<(String, f64)>,
    
    /// Breakdown of all applied transforms.
    ///
    /// Each entry is `(transform_description, value_after_transform)`.
    /// Transforms are listed in the order they were applied.
    pub transforms: Vec<(String, f64)>,
}

impl ResolvedStat {
    /// Create a new `ResolvedStat` with the given stat ID and value.
    ///
    /// The sources and transforms vectors start empty and can be
    /// populated using `add_source()` and `add_transform()`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::ResolvedStat;
    /// use zzstat::StatId;
    ///
    /// let resolved = ResolvedStat::new(StatId::from_str("HP"), 100.0);
    /// assert_eq!(resolved.value, 100.0);
    /// ```
    pub fn new(stat_id: StatId, value: f64) -> Self {
        Self {
            stat_id,
            value,
            sources: Vec::new(),
            transforms: Vec::new(),
        }
    }

    /// Add a source contribution to the breakdown.
    ///
    /// This is typically called by the resolver during stat resolution
    /// to build up the breakdown information.
    ///
    /// # Arguments
    ///
    /// * `description` - Human-readable description of the source
    /// * `value` - The value contributed by this source
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::ResolvedStat;
    /// use zzstat::StatId;
    ///
    /// let mut resolved = ResolvedStat::new(StatId::from_str("HP"), 150.0);
    /// resolved.add_source("Base HP", 100.0);
    /// resolved.add_source("Item Bonus", 50.0);
    /// ```
    pub fn add_source(&mut self, description: impl Into<String>, value: f64) {
        self.sources.push((description.into(), value));
    }

    /// Add a transform application to the breakdown.
    ///
    /// This is typically called by the resolver during stat resolution
    /// to build up the breakdown information.
    ///
    /// # Arguments
    ///
    /// * `description` - Human-readable description of the transform
    /// * `value` - The value after applying this transform
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::ResolvedStat;
    /// use zzstat::StatId;
    ///
    /// let mut resolved = ResolvedStat::new(StatId::from_str("ATK"), 150.0);
    /// resolved.add_transform("Multiplier 1.5x", 150.0);
    /// ```
    pub fn add_transform(&mut self, description: impl Into<String>, value: f64) {
        self.transforms.push((description.into(), value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolved_stat_creation() {
        let stat = ResolvedStat::new(StatId::from_str("HP"), 150.0);
        assert_eq!(stat.stat_id.as_str(), "HP");
        assert_eq!(stat.value, 150.0);
        assert!(stat.sources.is_empty());
        assert!(stat.transforms.is_empty());
    }

    #[test]
    fn test_resolved_stat_breakdown() {
        let mut stat = ResolvedStat::new(StatId::from_str("ATK"), 75.0);
        stat.add_source("Base", 50.0);
        stat.add_source("Item", 25.0);
        stat.add_transform("Multiplier 1.5x", 75.0);
        
        assert_eq!(stat.sources.len(), 2);
        assert_eq!(stat.transforms.len(), 1);
    }
}

