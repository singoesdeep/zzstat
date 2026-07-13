//! Stat sources module.
//!
//! Sources produce base values for stats. Multiple sources for the same
//! stat are summed together (additive). Sources are stateless and
//! deterministic - the same input always produces the same output.

use crate::context::StatContext;
use crate::numeric::{StatNumeric, StatValue};
use crate::stat_id::StatId;
use rustc_hash::FxHashMap;

/// Trait for stat sources that produce base values.
///
/// Sources are stateless and deterministic - same input always produces
/// same output. Multiple sources for the same stat are summed together
/// (additive).
///
/// # Examples
///
/// ```rust
/// use zzstat::{StatSource, StatId, StatContext};
/// use zzstat::source::ConstantSource;
///
/// let source = ConstantSource(100.0);
/// let context = StatContext::new();
/// let stat_id = StatId::from("HP");
///
/// let value = source.get_value(&stat_id, &context);
/// assert_eq!(value, 100.0);
/// ```
pub trait StatSource: Send + Sync {
    /// Get the value for a stat from this source.
    ///
    /// # Arguments
    ///
    /// * `stat_id` - The stat identifier
    /// * `context` - The stat context (may be used for conditional values)
    ///
    /// # Returns
    ///
    /// The base value contributed by this source.
    fn get_value(&self, stat_id: &StatId, context: &StatContext) -> StatValue;
}

/// A constant source that always returns the same value.
///
/// This is the simplest source type - it always produces the same
/// value regardless of context.
///
/// # Examples
///
/// ```rust
/// use zzstat::source::{ConstantSource, StatSource};
/// use zzstat::{StatId, StatContext};
///
/// let source = ConstantSource(100.0);
/// let context = StatContext::new();
/// let stat_id = StatId::from("HP");
///
/// assert_eq!(source.get_value(&stat_id, &context), 100.0);
/// ```
#[derive(Debug, Clone)]
pub struct ConstantSource(pub f64);

impl StatSource for ConstantSource {
    fn get_value(&self, _stat_id: &StatId, _context: &StatContext) -> StatValue {
        StatValue::from_f64(self.0)
    }
}

/// A map-based source that looks up values by StatId.
///
/// Useful when you have a collection of stat values that you want
/// to use as sources. Returns 0.0 for stats not in the map.
///
/// # Examples
///
/// ```rust
/// use zzstat::source::{MapSource, StatSource};
/// use zzstat::{StatId, StatContext};
/// use std::collections::HashMap;
///
/// let mut values = HashMap::new();
/// values.insert(StatId::from("HP"), 100.0);
/// values.insert(StatId::from("MP"), 50.0);
///
/// let source = MapSource::new(values);
/// let context = StatContext::new();
///
/// assert_eq!(source.get_value(&StatId::from("HP"), &context), 100.0);
/// assert_eq!(source.get_value(&StatId::from("MP"), &context), 50.0);
/// assert_eq!(source.get_value(&StatId::from("ATK"), &context), 0.0);
/// ```
#[derive(Debug, Clone)]
pub struct MapSource {
    values: FxHashMap<StatId, f64>,
}

impl MapSource {
    /// Create a new `MapSource` from a `HashMap`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::source::MapSource;
    /// use zzstat::StatId;
    /// use std::collections::HashMap;
    ///
    /// let mut values = HashMap::new();
    /// values.insert(StatId::from("HP"), 100.0);
    /// let source = MapSource::new(values);
    /// ```
    pub fn new<I: IntoIterator<Item = (StatId, f64)>>(values: I) -> Self {
        Self {
            values: values.into_iter().collect(),
        }
    }

    /// Create a new empty `MapSource`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::source::MapSource;
    ///
    /// let mut source = MapSource::empty();
    /// source.insert(zzstat::StatId::from("HP"), 100.0);
    /// ```
    pub fn empty() -> Self {
        Self {
            values: FxHashMap::default(),
        }
    }

    /// Insert a value into the map.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::source::MapSource;
    ///
    /// let mut source = MapSource::empty();
    /// source.insert(zzstat::StatId::from("HP"), 100.0);
    /// ```
    pub fn insert(&mut self, stat_id: StatId, value: f64) {
        self.values.insert(stat_id, value);
    }
}

impl StatSource for MapSource {
    fn get_value(&self, stat_id: &StatId, _context: &StatContext) -> StatValue {
        StatValue::from_f64(self.values.get(stat_id).copied().unwrap_or(0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_source() {
        let source = ConstantSource(100.0);
        let context = StatContext::new();
        let stat_id = StatId::from("HP");

        assert_eq!(
            source.get_value(&stat_id, &context),
            StatValue::from_f64(100.0)
        );
    }

    #[test]
    fn test_map_source() {
        let mut source = MapSource::empty();
        let hp_id = StatId::from("HP");
        let atk_id = StatId::from("ATK");

        source.insert(hp_id.clone(), 100.0);
        source.insert(atk_id.clone(), 50.0);

        let context = StatContext::new();
        assert_eq!(
            source.get_value(&hp_id, &context),
            StatValue::from_f64(100.0)
        );
        assert_eq!(
            source.get_value(&atk_id, &context),
            StatValue::from_f64(50.0)
        );
        assert_eq!(
            source.get_value(&StatId::from("MISSING"), &context),
            StatValue::from_f64(0.0)
        );
    }

    #[test]
    fn test_map_source_new() {
        use std::collections::HashMap;
        let mut values = HashMap::new();
        values.insert(StatId::from("HP"), 100.0);
        values.insert(StatId::from("MP"), 50.0);

        let source = MapSource::new(values);
        let context = StatContext::new();

        assert_eq!(
            source.get_value(&StatId::from("HP"), &context),
            StatValue::from_f64(100.0)
        );
        assert_eq!(
            source.get_value(&StatId::from("MP"), &context),
            StatValue::from_f64(50.0)
        );
    }

    #[test]
    fn test_map_source_insert_overwrite() {
        let mut source = MapSource::empty();
        let hp_id = StatId::from("HP");

        source.insert(hp_id.clone(), 100.0);
        let context = StatContext::new();
        assert_eq!(
            source.get_value(&hp_id, &context),
            StatValue::from_f64(100.0)
        );

        source.insert(hp_id.clone(), 200.0);
        assert_eq!(
            source.get_value(&hp_id, &context),
            StatValue::from_f64(200.0)
        );
    }

    #[test]
    fn test_map_source_empty() {
        let source = MapSource::empty();
        let context = StatContext::new();

        assert_eq!(
            source.get_value(&StatId::from("ANY"), &context),
            StatValue::from_f64(0.0)
        );
    }

    #[test]
    fn test_constant_source_negative() {
        let source = ConstantSource(-50.0);
        let context = StatContext::new();
        let stat_id = StatId::from("HP");

        assert_eq!(
            source.get_value(&stat_id, &context),
            StatValue::from_f64(-50.0)
        );
    }

    #[test]
    fn test_constant_source_zero() {
        let source = ConstantSource(0.0);
        let context = StatContext::new();
        let stat_id = StatId::from("HP");

        assert_eq!(
            source.get_value(&stat_id, &context),
            StatValue::from_f64(0.0)
        );
    }

    #[test]
    fn test_constant_source_clone() {
        let source1 = ConstantSource(100.0);
        let source2 = source1.clone();

        let context = StatContext::new();
        let stat_id = StatId::from("HP");

        assert_eq!(
            source1.get_value(&stat_id, &context),
            StatValue::from_f64(100.0)
        );
        assert_eq!(
            source2.get_value(&stat_id, &context),
            StatValue::from_f64(100.0)
        );
    }
}
