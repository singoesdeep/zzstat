use crate::context::StatContext;
use crate::error::StatError;
use crate::numeric::{StatValue, StatNumeric};
use crate::stat_id::StatId;
use crate::transform::core::{TransformPhase, StatTransform, ClampBounds};
use rustc_hash::FxHashMap;

/// A multiplicative transform (percentage modifier).
///
/// Multiplies the input value by a constant factor.
///
/// # Examples
///
/// ```rust
/// use zzstat::transform::{StatTransform, MultiplicativeTransform};
/// use zzstat::StatContext;
/// use rustc_hash::FxHashMap;
///
/// let transform = MultiplicativeTransform::new(1.5);
/// let context = StatContext::new();
/// let deps = FxHashMap::default();
///
/// // 100 * 1.5 = 150
/// assert_eq!(transform.apply(100.0, &deps, &context).unwrap(), 150.0);
/// ```
#[derive(Debug, Clone)]
pub struct MultiplicativeTransform {
    multiplier: f64,
}

impl MultiplicativeTransform {
    /// Create a new multiplicative transform.
    ///
    /// # Arguments
    ///
    /// * `multiplier` - The multiplier to apply (e.g., 1.5 for +50%)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::MultiplicativeTransform;
    ///
    /// // +50% bonus
    /// let bonus = MultiplicativeTransform::new(1.5);
    ///
    /// // -20% penalty
    /// let penalty = MultiplicativeTransform::new(0.8);
    /// ```
    pub fn new(multiplier: f64) -> Self {
        Self { multiplier }
    }

    /// Get the multiplier value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::MultiplicativeTransform;
    ///
    /// let transform = MultiplicativeTransform::new(1.5);
    /// assert_eq!(transform.multiplier(), 1.5);
    /// ```
    pub fn multiplier(&self) -> f64 {
        self.multiplier
    }
}

impl StatTransform for MultiplicativeTransform {
    fn depends_on(&self) -> Vec<StatId> {
        Vec::new()
    }

    fn apply(
        &self,
        input: StatValue,
        _dependencies: &FxHashMap<StatId, StatValue>,
        _context: &StatContext,
    ) -> Result<StatValue, StatError> {
        Ok(input * StatValue::from_f64(self.multiplier))
    }

    fn description(&self) -> String {
        format!("×{:.2}", self.multiplier)
    }
}

/// An additive transform (flat bonus).
///
/// Adds a constant value to the input.
///
/// # Examples
///
/// ```rust
/// use zzstat::transform::{StatTransform, AdditiveTransform};
/// use zzstat::StatContext;
/// use rustc_hash::FxHashMap;
///
/// let transform = AdditiveTransform::new(25.0);
/// let context = StatContext::new();
/// let deps = FxHashMap::default();
///
/// // 100 + 25 = 125
/// assert_eq!(transform.apply(100.0, &deps, &context).unwrap(), 125.0);
/// ```
#[derive(Debug, Clone)]
pub struct AdditiveTransform {
    bonus: f64,
}

impl AdditiveTransform {
    /// Create a new additive transform.
    ///
    /// # Arguments
    ///
    /// * `bonus` - The flat bonus to add (can be negative for penalties)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::AdditiveTransform;
    ///
    /// // +25 flat bonus
    /// let bonus = AdditiveTransform::new(25.0);
    ///
    /// // -10 flat penalty
    /// let penalty = AdditiveTransform::new(-10.0);
    /// ```
    pub fn new(bonus: f64) -> Self {
        Self { bonus }
    }

    /// Get the bonus value.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::AdditiveTransform;
    ///
    /// let transform = AdditiveTransform::new(25.0);
    /// assert_eq!(transform.bonus(), 25.0);
    /// ```
    pub fn bonus(&self) -> f64 {
        self.bonus
    }
}

impl StatTransform for AdditiveTransform {
    fn depends_on(&self) -> Vec<StatId> {
        Vec::new()
    }

    fn phase(&self) -> TransformPhase {
        TransformPhase::Additive
    }

    fn apply(
        &self,
        input: StatValue,
        _dependencies: &FxHashMap<StatId, StatValue>,
        _context: &StatContext,
    ) -> Result<StatValue, StatError> {
        Ok(input + StatValue::from_f64(self.bonus))
    }

    fn description(&self) -> String {
        format!("+{:.2}", self.bonus)
    }
}

/// A clamp transform that restricts values to a range.
///
/// Ensures the output value is between `min` and `max` (inclusive).
/// Either bound can be `None` to indicate no limit in that direction.
///
/// Clamp transforms are recommended for use in `TransformPhase::Final`
/// to enforce final gameplay limits after all buffs, items, and auras
/// have been applied.
///
/// When multiple clamp transforms apply to the same stat in the same phase,
/// they compose deterministically:
/// - `effective_min = max(all mins)` (most restrictive lower bound)
/// - `effective_max = min(all maxes)` (most restrictive upper bound)
/// - `final_value = clamp(value, effective_min, effective_max)`
///
/// # Examples
///
/// ```rust
/// use zzstat::transform::{StatTransform, ClampTransform};
/// use zzstat::{StatContext, StatValue, numeric::StatNumeric};
/// use rustc_hash::FxHashMap;
///
/// // Clamp with both bounds
/// let transform = ClampTransform::with_bounds(
///     Some(StatValue::from_f64(0.0)),
///     Some(StatValue::from_f64(100.0)),
/// );
/// let context = StatContext::new();
/// let deps = FxHashMap::default();
///
/// assert_eq!(transform.apply(150.0.into(), &deps, &context).unwrap().to_f64(), 100.0);
/// assert_eq!(transform.apply((-10.0).into(), &deps, &context).unwrap().to_f64(), 0.0);
/// assert_eq!(transform.apply(50.0.into(), &deps, &context).unwrap().to_f64(), 50.0);
///
/// // Clamp with only max bound (crit chance cap)
/// let crit_cap = ClampTransform::with_max(StatValue::from_f64(0.75));
/// assert_eq!(crit_cap.apply(1.0.into(), &deps, &context).unwrap().to_f64(), 0.75);
///
/// // Clamp with only min bound (movement speed floor)
/// let move_speed_floor = ClampTransform::with_min(StatValue::from_f64(100.0));
/// assert_eq!(move_speed_floor.apply(50.0.into(), &deps, &context).unwrap().to_f64(), 100.0);
/// ```
#[derive(Debug, Clone)]
pub struct ClampTransform {
    /// Minimum allowed value (inclusive). `None` means no lower bound.
    pub min: Option<StatValue>,
    /// Maximum allowed value (inclusive). `None` means no upper bound.
    pub max: Option<StatValue>,
}

impl ClampTransform {
    /// Create a new clamp transform from f64 values (backward compatibility).
    ///
    /// This is a convenience constructor that maintains backward compatibility
    /// with existing code. Both min and max are required.
    ///
    /// # Arguments
    ///
    /// * `min` - Minimum allowed value (inclusive)
    /// * `max` - Maximum allowed value (inclusive)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::ClampTransform;
    ///
    /// // Clamp between 0 and 100
    /// let clamp = ClampTransform::new(0.0, 100.0);
    /// ```
    pub fn new(min: f64, max: f64) -> Self {
        Self {
            min: Some(StatValue::from_f64(min)),
            max: Some(StatValue::from_f64(max)),
        }
    }

    /// Create a new clamp transform with optional min and max bounds.
    ///
    /// # Arguments
    ///
    /// * `min` - Minimum allowed value (inclusive). `None` means no lower bound.
    /// * `max` - Maximum allowed value (inclusive). `None` means no upper bound.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::ClampTransform;
    /// use zzstat::{StatValue, numeric::StatNumeric};
    ///
    /// // Clamp between 0 and 100
    /// let clamp = ClampTransform::with_bounds(
    ///     Some(StatValue::from_f64(0.0)),
    ///     Some(StatValue::from_f64(100.0)),
    /// );
    ///
    /// // Only max bound (crit chance cap)
    /// let crit_cap = ClampTransform::with_bounds(None, Some(StatValue::from_f64(0.75)));
    /// ```
    pub fn with_bounds(min: Option<StatValue>, max: Option<StatValue>) -> Self {
        Self { min, max }
    }

    /// Create a clamp transform with only a minimum bound.
    ///
    /// # Arguments
    ///
    /// * `min` - Minimum allowed value (inclusive)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::ClampTransform;
    /// use zzstat::{StatValue, numeric::StatNumeric};
    ///
    /// // Movement speed floor
    /// let floor = ClampTransform::with_min(StatValue::from_f64(100.0));
    /// ```
    pub fn with_min(min: StatValue) -> Self {
        Self {
            min: Some(min),
            max: None,
        }
    }

    /// Create a clamp transform with only a maximum bound.
    ///
    /// # Arguments
    ///
    /// * `max` - Maximum allowed value (inclusive)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::ClampTransform;
    /// use zzstat::{StatValue, numeric::StatNumeric};
    ///
    /// // Crit chance cap
    /// let cap = ClampTransform::with_max(StatValue::from_f64(0.75));
    /// ```
    pub fn with_max(max: StatValue) -> Self {
        Self {
            min: None,
            max: Some(max),
        }
    }

    /// Get the minimum bound.
    ///
    /// Returns `None` if there is no lower bound.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::ClampTransform;
    /// use zzstat::{StatValue, numeric::StatNumeric};
    ///
    /// let clamp = ClampTransform::new(0.0, 100.0);
    /// assert_eq!(clamp.min().map(|v| v.to_f64()), Some(0.0));
    ///
    /// let cap_only = ClampTransform::with_max(StatValue::from_f64(100.0));
    /// assert_eq!(cap_only.min(), None);
    /// ```
    pub fn min(&self) -> Option<StatValue> {
        self.min
    }

    /// Get the maximum bound.
    ///
    /// Returns `None` if there is no upper bound.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::ClampTransform;
    /// use zzstat::{StatValue, numeric::StatNumeric};
    ///
    /// let clamp = ClampTransform::new(0.0, 100.0);
    /// assert_eq!(clamp.max().map(|v| v.to_f64()), Some(100.0));
    ///
    /// let floor_only = ClampTransform::with_min(StatValue::from_f64(0.0));
    /// assert_eq!(floor_only.max(), None);
    /// ```
    pub fn max(&self) -> Option<StatValue> {
        self.max
    }
}

impl StatTransform for ClampTransform {
    fn depends_on(&self) -> Vec<StatId> {
        Vec::new()
    }

    fn phase(&self) -> TransformPhase {
        TransformPhase::Final
    }

    fn apply(
        &self,
        input: StatValue,
        _dependencies: &FxHashMap<StatId, StatValue>,
        _context: &StatContext,
    ) -> Result<StatValue, StatError> {
        let mut result = input;

        // Apply min bound if present
        if let Some(min) = self.min {
            result = result.max(min);
        }

        // Apply max bound if present
        if let Some(max) = self.max {
            result = result.min(max);
        }

        Ok(result)
    }

    fn description(&self) -> String {
        match (self.min, self.max) {
            (Some(min), Some(max)) => format!("clamp({:.2}, {:.2})", min.to_f64(), max.to_f64()),
            (Some(min), None) => format!("clamp_min({:.2})", min.to_f64()),
            (None, Some(max)) => format!("clamp_max({:.2})", max.to_f64()),
            (None, None) => "clamp(none)".to_string(),
        }
    }
}

impl ClampBounds for ClampTransform {
    fn min_bound(&self) -> Option<StatValue> {
        self.min
    }

    fn max_bound(&self) -> Option<StatValue> {
        self.max
    }
}

/// A transform that scales based on another stat.
///
/// Adds `dependency_value * scale_factor` to the input value.
/// This is commonly used for derived stats (e.g., ATK = base + STR * 2).
///
/// # Examples
///
/// ```rust
/// use zzstat::transform::{StatTransform, ScalingTransform};
/// use zzstat::{StatId, StatContext};
/// use rustc_hash::FxHashMap;
///
/// let str_id = StatId::from("STR");
/// let transform = ScalingTransform::new(str_id.clone(), 2.0);
///
/// let mut deps = FxHashMap::default();
/// deps.insert(str_id.clone(), 10.0);
///
/// let context = StatContext::new();
/// // 100 (base) + 10 (STR) * 2 = 120
/// assert_eq!(transform.apply(100.0, &deps, &context).unwrap(), 120.0);
/// ```
#[derive(Debug, Clone)]
pub struct ScalingTransform {
    dependency: StatId,
    scale_factor: f64,
}

impl ScalingTransform {
    /// Create a new scaling transform.
    ///
    /// # Arguments
    ///
    /// * `dependency` - The stat ID this transform depends on
    /// * `scale_factor` - The multiplier to apply to the dependency value
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::ScalingTransform;
    /// use zzstat::StatId;
    ///
    /// let str_id = StatId::from("STR");
    /// // ATK scales with STR: ATK = base + STR * 2
    /// let transform = ScalingTransform::new(str_id, 2.0);
    /// ```
    pub fn new(dependency: StatId, scale_factor: f64) -> Self {
        Self {
            dependency,
            scale_factor,
        }
    }
}

impl StatTransform for ScalingTransform {
    fn depends_on(&self) -> Vec<StatId> {
        vec![self.dependency.clone()]
    }

    fn phase(&self) -> TransformPhase {
        TransformPhase::Additive
    }

    fn apply(
        &self,
        input: StatValue,
        dependencies: &FxHashMap<StatId, StatValue>,
        _context: &StatContext,
    ) -> Result<StatValue, StatError> {
        let dep_value = dependencies
            .get(&self.dependency)
            .ok_or_else(|| StatError::MissingDependency(self.dependency.clone()))?;
        Ok(input + (*dep_value * StatValue::from_f64(self.scale_factor)))
    }

    fn description(&self) -> String {
        format!("scale({}, {:.2})", self.dependency, self.scale_factor)
    }
}

#[cfg(test)]
mod tests {
    use crate::numeric::StatNumeric;
    use super::*;

    #[test]
    fn test_scaling_transform() {
        let str_id = StatId::from("STR");
        let transform = ScalingTransform::new(str_id.clone(), 2.5);
        
        let mut deps = FxHashMap::default();
        deps.insert(str_id.clone(), StatValue::from_f64(10.0));
        
        let context = StatContext::new();
        let result = transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap();
        assert_eq!(result.to_f64(), 125.0);
    }

    #[test]
    fn test_scaling_transform_missing_dep() {
        let str_id = StatId::from("STR");
        let transform = ScalingTransform::new(str_id.clone(), 2.5);
        
        let deps = FxHashMap::default(); // missing STR
        let context = StatContext::new();
        
        let err = transform.apply(StatValue::from_f64(100.0), &deps, &context);
        assert!(matches!(err, Err(StatError::MissingDependency(_))));
    }
}

