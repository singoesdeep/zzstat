//! Stat transforms module.
//!
//! Transforms modify stat values after sources are collected.
//! Transforms can read other stats (dependencies) and must declare
//! them explicitly via `depends_on()`.

use crate::context::StatContext;
use crate::error::StatError;
use crate::numeric::{StatNumeric, StatValue};
use crate::stat_id::StatId;
use std::collections::HashMap;

/// Phase for transform application order.
///
/// Transforms are grouped by phase and applied in phase order.
/// Within each phase, transforms are applied in registration order.
///
/// # Examples
///
/// ```rust
/// use zzstat::transform::TransformPhase;
///
/// // Standard phases
/// let additive = TransformPhase::Additive;
/// let multiplicative = TransformPhase::Multiplicative;
/// let final_phase = TransformPhase::Final;
///
/// // Custom phase (u8 >= 3)
/// let custom = TransformPhase::Custom(10);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TransformPhase {
    /// Additive phase (phase 0).
    ///
    /// Transforms that add or subtract values.
    /// Applied first, before multiplicative transforms.
    Additive,

    /// Multiplicative phase (phase 1).
    ///
    /// Transforms that multiply or scale values.
    /// Applied after additive transforms, before final transforms.
    Multiplicative,

    /// Final phase (phase 2).
    ///
    /// Final adjustments like clamping or rounding.
    /// Applied last, after all other transforms.
    Final,

    /// Custom phase (phase >= 3).
    ///
    /// User-defined phases for custom ordering.
    /// Higher values are applied later.
    Custom(u8),
}

impl TransformPhase {
    /// Get the numeric value of this phase for ordering.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::TransformPhase;
    ///
    /// assert_eq!(TransformPhase::Additive.value(), 0);
    /// assert_eq!(TransformPhase::Multiplicative.value(), 1);
    /// assert_eq!(TransformPhase::Final.value(), 2);
    /// assert_eq!(TransformPhase::Custom(10).value(), 10);
    /// ```
    pub fn value(self) -> u8 {
        match self {
            TransformPhase::Additive => 0,
            TransformPhase::Multiplicative => 1,
            TransformPhase::Final => 2,
            TransformPhase::Custom(n) => n.max(3),
        }
    }
}

/// Stack rule for how transforms in the same phase combine.
///
/// Stack rules determine how multiple transforms affecting the same stat
/// in the same phase are combined. Transforms are grouped by stack rule
/// and applied in a fixed priority order within each phase:
/// Override → Additive → Multiplicative → Diminishing → Min → Max → MinMax
///
/// # Examples
///
/// ```rust
/// use zzstat::transform::StackRule;
/// use zzstat::numeric::StatNumeric;
///
/// // Additive stacking: values are summed
/// let additive = StackRule::Additive;
///
/// // Multiplicative stacking: values are multiplied
/// let multiplicative = StackRule::Multiplicative;
///
/// // Override: last transform wins
/// let override_rule = StackRule::Override;
///
/// // Diminishing returns: uses exponential formula
/// let diminishing = StackRule::Diminishing { k: 0.5.into() };
///
/// // MinMax: for clamp transforms that provide both min and max bounds
/// let minmax = StackRule::MinMax;
/// ```
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StackRule {
    /// Override: Last transform wins (deterministic order).
    Override,
    /// Additive: Base + sum of all additive values.
    Additive,
    /// Multiplicative: Base × product of all multipliers.
    Multiplicative,
    /// Diminishing returns: Each transform applies `value × (1 - exp(-k × 1))`.
    Diminishing { k: StatValue },
    /// Minimum: Clamp to minimum value.
    Min,
    /// Maximum: Clamp to maximum value.
    Max,
    /// MinMax: Clamp transforms that provide both min and/or max bounds.
    ///
    /// When multiple clamp transforms with `MinMax` stack rule apply:
    /// - `effective_min = max(all mins)` (most restrictive lower bound)
    /// - `effective_max = min(all maxes)` (most restrictive upper bound)
    /// - `final_value = clamp(value, effective_min, effective_max)`
    ///
    /// This is the recommended stack rule for `ClampTransform`.
    MinMax,
}

impl StackRule {
    /// Get the priority value for ordering stack rules.
    ///
    /// Lower values are applied first.
    /// Order: Override (0) → Additive (1) → Multiplicative (2) → Diminishing (3) → Min (4) → Max (5) → MinMax (6)
    pub fn priority(self) -> u8 {
        match self {
            StackRule::Override => 0,
            StackRule::Additive => 1,
            StackRule::Multiplicative => 2,
            StackRule::Diminishing { .. } => 3,
            StackRule::Min => 4,
            StackRule::Max => 5,
            StackRule::MinMax => 6,
        }
    }
}

impl PartialOrd for StackRule {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.priority().cmp(&other.priority()))
    }
}

/// Entry that associates a transform with its phase and stack rule.
///
/// This wrapper stores the metadata needed for stack rule resolution
/// while maintaining the transform itself.
pub struct TransformEntry {
    /// The phase this transform belongs to.
    pub phase: TransformPhase,
    /// The stack rule for combining this transform with others in the same phase.
    pub rule: StackRule,
    /// The actual transform to apply.
    pub transform: Box<dyn StatTransform>,
}

impl std::fmt::Debug for TransformEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformEntry")
            .field("phase", &self.phase)
            .field("rule", &self.rule)
            .field("transform", &format!("<{}>", self.transform.description()))
            .finish()
    }
}

/// Trait for transforms that provide clamp bounds.
///
/// This trait allows the resolver to extract min/max bounds directly from
/// transforms without applying them to extreme values. This is more efficient
/// and deterministic than the fallback method of applying transforms to
/// very large/small values.
///
/// # Examples
///
/// ```rust
/// use zzstat::transform::{ClampBounds, ClampTransform};
/// use zzstat::{StatValue, numeric::StatNumeric};
///
/// let clamp = ClampTransform::with_bounds(
///     Some(StatValue::from_f64(0.0)),
///     Some(StatValue::from_f64(100.0)),
/// );
///
/// assert_eq!(clamp.min_bound(), Some(StatValue::from_f64(0.0)));
/// assert_eq!(clamp.max_bound(), Some(StatValue::from_f64(100.0)));
/// ```
pub trait ClampBounds {
    /// Get the minimum bound provided by this transform.
    ///
    /// Returns `None` if this transform does not provide a minimum bound.
    fn min_bound(&self) -> Option<StatValue>;

    /// Get the maximum bound provided by this transform.
    ///
    /// Returns `None` if this transform does not provide a maximum bound.
    fn max_bound(&self) -> Option<StatValue>;
}

/// Infer a default stack rule for a transform based on its phase.
///
/// This is used for backward compatibility when transforms are registered
/// without an explicit stack rule. The inference is based on the transform's
/// declared phase.
///
/// # Arguments
///
/// * `transform` - The transform to infer a stack rule for
///
/// # Returns
///
/// A `StackRule` that should be used for this transform.
pub fn infer_stack_rule(transform: &dyn StatTransform) -> StackRule {
    match transform.phase() {
        TransformPhase::Additive => StackRule::Additive,
        TransformPhase::Multiplicative => StackRule::Multiplicative,
        TransformPhase::Final => StackRule::MinMax, // Default for final phase (clamps use MinMax)
        TransformPhase::Custom(_) => StackRule::Additive, // Default fallback
    }
}

/// Trait for stat transforms that modify stat values.
///
/// Transforms can read other stats (dependencies) and must declare
/// them explicitly. The resolver ensures dependencies are resolved
/// before applying the transform.
///
/// # Examples
///
/// ```rust
/// use zzstat::transform::{StatTransform, MultiplicativeTransform};
/// use zzstat::{StatContext, StatId};
/// use zzstat::numeric::StatNumeric;
/// use std::collections::HashMap;
///
/// let transform = MultiplicativeTransform::new(1.5);
/// let context = StatContext::new();
/// let deps = HashMap::new();
///
/// let result = transform.apply(100.0.into(), &deps, &context).unwrap();
/// assert_eq!(result.to_f64(), 150.0);
/// ```
pub trait StatTransform: Send + Sync {
    /// Get the list of stat IDs this transform depends on.
    ///
    /// These stats must be resolved before this transform can be applied.
    /// The resolver uses this information to build the dependency graph
    /// and determine resolution order.
    ///
    /// # Returns
    ///
    /// A vector of stat IDs that this transform depends on.
    fn depends_on(&self) -> Vec<StatId>;

    /// Get the phase this transform belongs to.
    ///
    /// Transforms are grouped by phase and applied in phase order.
    /// Within each phase, transforms are applied in registration order.
    ///
    /// Default implementation returns `Multiplicative` phase.
    ///
    /// # Returns
    ///
    /// The phase this transform should be applied in.
    fn phase(&self) -> TransformPhase {
        TransformPhase::Multiplicative
    }

    /// Apply the transform to an input value.
    ///
    /// # Arguments
    ///
    /// * `input` - The current stat value (after sources and previous transforms)
    /// * `dependencies` - Map of resolved dependency stats (keyed by StatId)
    /// * `context` - The stat context (for conditional transforms)
    ///
    /// # Returns
    ///
    /// The transformed value, or an error if the transform cannot be applied.
    fn apply(
        &self,
        input: StatValue,
        dependencies: &HashMap<StatId, StatValue>,
        context: &StatContext,
    ) -> Result<StatValue, StatError>;

    /// Get a human-readable description of this transform.
    ///
    /// Used for debugging and breakdown information in `ResolvedStat`.
    ///
    /// # Returns
    ///
    /// A string describing what this transform does.
    fn description(&self) -> String;
}

/// A multiplicative transform (percentage modifier).
///
/// Multiplies the input value by a constant factor.
///
/// # Examples
///
/// ```rust
/// use zzstat::transform::{StatTransform, MultiplicativeTransform};
/// use zzstat::StatContext;
/// use std::collections::HashMap;
///
/// let transform = MultiplicativeTransform::new(1.5);
/// let context = StatContext::new();
/// let deps = HashMap::new();
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
        _dependencies: &HashMap<StatId, StatValue>,
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
/// use std::collections::HashMap;
///
/// let transform = AdditiveTransform::new(25.0);
/// let context = StatContext::new();
/// let deps = HashMap::new();
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
        _dependencies: &HashMap<StatId, StatValue>,
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
/// use std::collections::HashMap;
///
/// // Clamp with both bounds
/// let transform = ClampTransform::with_bounds(
///     Some(StatValue::from_f64(0.0)),
///     Some(StatValue::from_f64(100.0)),
/// );
/// let context = StatContext::new();
/// let deps = HashMap::new();
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
        _dependencies: &HashMap<StatId, StatValue>,
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

/// A conditional transform that applies another transform based on a condition.
///
/// Only applies the inner transform if the condition function returns `true`
/// when called with the current `StatContext`. Otherwise, returns the input
/// value unchanged.
///
/// # Examples
///
/// ```rust
/// use zzstat::transform::{StatTransform, ConditionalTransform, MultiplicativeTransform};
/// use zzstat::StatContext;
/// use std::collections::HashMap;
///
/// let mut context = StatContext::new();
/// context.set("in_combat", true);
///
/// let inner_transform = Box::new(MultiplicativeTransform::new(1.2));
/// let transform = ConditionalTransform::new(
///     |ctx| ctx.get::<bool>("in_combat").unwrap_or(false),
///     inner_transform,
///     "combat bonus",
/// );
///
/// let deps = HashMap::new();
/// // In combat: 100 * 1.2 = 120
/// assert_eq!(transform.apply(100.0, &deps, &context).unwrap(), 120.0);
///
/// context.set("in_combat", false);
/// // Out of combat: 100 (unchanged)
/// assert_eq!(transform.apply(100.0, &deps, &context).unwrap(), 100.0);
/// ```
pub struct ConditionalTransform {
    condition: Box<dyn Fn(&StatContext) -> bool + Send + Sync>,
    transform: Box<dyn StatTransform>,
    description: String,
}

impl ConditionalTransform {
    /// Create a new conditional transform.
    ///
    /// # Arguments
    ///
    /// * `condition` - A function that takes `&StatContext` and returns `bool`
    /// * `transform` - The transform to apply when condition is `true`
    /// * `description` - Human-readable description for debugging
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::transform::{ConditionalTransform, MultiplicativeTransform};
    ///
    /// let inner = Box::new(MultiplicativeTransform::new(1.5));
    /// let transform = ConditionalTransform::new(
    ///     |ctx| ctx.get::<bool>("in_combat").unwrap_or(false),
    ///     inner,
    ///     "combat bonus +50%",
    /// );
    /// ```
    pub fn new<F>(
        condition: F,
        transform: Box<dyn StatTransform>,
        description: impl Into<String>,
    ) -> Self
    where
        F: Fn(&StatContext) -> bool + Send + Sync + 'static,
    {
        Self {
            condition: Box::new(condition),
            transform,
            description: description.into(),
        }
    }
}

impl StatTransform for ConditionalTransform {
    fn depends_on(&self) -> Vec<StatId> {
        self.transform.depends_on()
    }

    fn apply(
        &self,
        input: StatValue,
        dependencies: &HashMap<StatId, StatValue>,
        context: &StatContext,
    ) -> Result<StatValue, StatError> {
        if (self.condition)(context) {
            self.transform.apply(input, dependencies, context)
        } else {
            Ok(input)
        }
    }

    fn description(&self) -> String {
        self.description.clone()
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
/// use std::collections::HashMap;
///
/// let str_id = StatId::from_str("STR");
/// let transform = ScalingTransform::new(str_id.clone(), 2.0);
///
/// let mut deps = HashMap::new();
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
    /// let str_id = StatId::from_str("STR");
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
        dependencies: &HashMap<StatId, StatValue>,
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
    use super::*;

    #[test]
    fn test_multiplicative_transform() {
        let transform = MultiplicativeTransform::new(1.5);
        let context = StatContext::new();
        let deps = HashMap::new();

        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(150.0));
    }

    #[test]
    fn test_additive_transform() {
        let transform = AdditiveTransform::new(25.0);
        let context = StatContext::new();
        let deps = HashMap::new();

        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(125.0));
    }

    #[test]
    fn test_clamp_transform() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::new(0.0, 100.0);
        let context = StatContext::new();
        let deps = HashMap::new();

        assert_eq!(transform.apply(StatValue::from_f64(150.0), &deps, &context).unwrap().to_f64(), 100.0);
        assert_eq!(transform.apply(StatValue::from_f64(-10.0), &deps, &context).unwrap().to_f64(), 0.0);
        assert_eq!(transform.apply(StatValue::from_f64(50.0), &deps, &context).unwrap().to_f64(), 50.0);
    }

    #[test]
    fn test_clamp_transform_with_bounds() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::with_bounds(
            Some(StatValue::from_f64(0.0)),
            Some(StatValue::from_f64(100.0)),
        );
        let context = StatContext::new();
        let deps = HashMap::new();

        assert_eq!(transform.apply(StatValue::from_f64(150.0), &deps, &context).unwrap().to_f64(), 100.0);
        assert_eq!(transform.apply(StatValue::from_f64(-10.0), &deps, &context).unwrap().to_f64(), 0.0);
        assert_eq!(transform.apply(StatValue::from_f64(50.0), &deps, &context).unwrap().to_f64(), 50.0);
    }

    #[test]
    fn test_clamp_transform_min_only() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::with_min(StatValue::from_f64(100.0));
        let context = StatContext::new();
        let deps = HashMap::new();

        assert_eq!(transform.apply(StatValue::from_f64(50.0), &deps, &context).unwrap().to_f64(), 100.0);
        assert_eq!(transform.apply(StatValue::from_f64(150.0), &deps, &context).unwrap().to_f64(), 150.0);
        assert_eq!(transform.min(), Some(StatValue::from_f64(100.0)));
        assert_eq!(transform.max(), None);
    }

    #[test]
    fn test_clamp_transform_max_only() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::with_max(StatValue::from_f64(0.75));
        let context = StatContext::new();
        let deps = HashMap::new();

        assert_eq!(transform.apply(StatValue::from_f64(1.0), &deps, &context).unwrap().to_f64(), 0.75);
        assert_eq!(transform.apply(StatValue::from_f64(0.5), &deps, &context).unwrap().to_f64(), 0.5);
        assert_eq!(transform.min(), None);
        assert_eq!(transform.max(), Some(StatValue::from_f64(0.75)));
    }

    #[test]
    fn test_clamp_transform_no_bounds() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::with_bounds(None, None);
        let context = StatContext::new();
        let deps = HashMap::new();

        // No-op: should return input unchanged
        assert_eq!(transform.apply(StatValue::from_f64(50.0), &deps, &context).unwrap().to_f64(), 50.0);
        assert_eq!(transform.apply(StatValue::from_f64(150.0), &deps, &context).unwrap().to_f64(), 150.0);
        assert_eq!(transform.min(), None);
        assert_eq!(transform.max(), None);
    }

    #[test]
    fn test_clamp_bounds_trait() {
        use crate::numeric::StatValue;
        use crate::transform::ClampBounds;
        let clamp = ClampTransform::with_bounds(
            Some(StatValue::from_f64(0.0)),
            Some(StatValue::from_f64(100.0)),
        );

        assert_eq!(clamp.min_bound(), Some(StatValue::from_f64(0.0)));
        assert_eq!(clamp.max_bound(), Some(StatValue::from_f64(100.0)));

        let min_only = ClampTransform::with_min(StatValue::from_f64(10.0));
        assert_eq!(min_only.min_bound(), Some(StatValue::from_f64(10.0)));
        assert_eq!(min_only.max_bound(), None);

        let max_only = ClampTransform::with_max(StatValue::from_f64(75.0));
        assert_eq!(max_only.min_bound(), None);
        assert_eq!(max_only.max_bound(), Some(StatValue::from_f64(75.0)));
    }

    #[test]
    fn test_scaling_transform() {
        let str_id = StatId::from_str("STR");
        let transform = ScalingTransform::new(str_id.clone(), 2.0);
        let context = StatContext::new();
        let mut deps = HashMap::new();
        deps.insert(str_id.clone(), StatValue::from_f64(10.0));

        assert_eq!(transform.depends_on(), vec![str_id]);
        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(120.0));
    }

    #[test]
    fn test_scaling_transform_missing_dependency() {
        let str_id = StatId::from_str("STR");
        let transform = ScalingTransform::new(str_id, 2.0);
        let context = StatContext::new();
        let deps = HashMap::new();

        assert!(transform.apply(StatValue::from_f64(100.0), &deps, &context).is_err());
    }

    #[test]
    fn test_conditional_transform() {
        let mut context = StatContext::new();
        context.set("in_combat", true);

        let inner_transform = Box::new(MultiplicativeTransform::new(1.2));
        let transform = ConditionalTransform::new(
            |ctx| ctx.get::<bool>("in_combat").unwrap_or(false),
            inner_transform,
            "combat bonus",
        );

        let deps = HashMap::new();
        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(120.0));

        context.set("in_combat", false);
        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(100.0));
    }

    #[test]
    fn test_transform_phase_values() {
        assert_eq!(TransformPhase::Additive.value(), 0);
        assert_eq!(TransformPhase::Multiplicative.value(), 1);
        assert_eq!(TransformPhase::Final.value(), 2);
        assert_eq!(TransformPhase::Custom(5).value(), 5);
        assert_eq!(TransformPhase::Custom(2).value(), 3); // Custom phases must be >= 3
    }

    #[test]
    fn test_transform_phase_ordering() {
        assert!(TransformPhase::Additive < TransformPhase::Multiplicative);
        assert!(TransformPhase::Multiplicative < TransformPhase::Final);
        assert!(TransformPhase::Final < TransformPhase::Custom(10));
        assert!(TransformPhase::Custom(5) < TransformPhase::Custom(10));
    }

    #[test]
    fn test_multiplicative_transform_zero() {
        let transform = MultiplicativeTransform::new(0.0);
        let context = StatContext::new();
        let deps = HashMap::new();

        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(0.0));
    }

    #[test]
    fn test_multiplicative_transform_negative() {
        let transform = MultiplicativeTransform::new(-1.0);
        let context = StatContext::new();
        let deps = HashMap::new();

        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(-100.0));
    }

    #[test]
    fn test_additive_transform_negative() {
        let transform = AdditiveTransform::new(-50.0);
        let context = StatContext::new();
        let deps = HashMap::new();

        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(50.0));
    }

    #[test]
    fn test_clamp_transform_edge_cases() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::new(0.0, 100.0);
        let context = StatContext::new();
        let deps = HashMap::new();

        // Exactly at min
        assert_eq!(transform.apply(StatValue::from_f64(0.0), &deps, &context).unwrap().to_f64(), 0.0);
        // Exactly at max
        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap().to_f64(), 100.0);
        // Below min
        assert_eq!(transform.apply(StatValue::from_f64(-1.0), &deps, &context).unwrap().to_f64(), 0.0);
        // Above max
        assert_eq!(transform.apply(StatValue::from_f64(101.0), &deps, &context).unwrap().to_f64(), 100.0);
    }

    #[test]
    fn test_scaling_transform_zero_scale() {
        let str_id = StatId::from_str("STR");
        let transform = ScalingTransform::new(str_id.clone(), 0.0);
        let context = StatContext::new();
        let mut deps = HashMap::new();
        deps.insert(str_id.clone(), StatValue::from_f64(10.0));

        // 100 + 10 * 0 = 100
        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(100.0));
    }

    #[test]
    fn test_scaling_transform_negative_scale() {
        let str_id = StatId::from_str("STR");
        let transform = ScalingTransform::new(str_id.clone(), -2.0);
        let context = StatContext::new();
        let mut deps = HashMap::new();
        deps.insert(str_id.clone(), StatValue::from_f64(10.0));

        // 100 + 10 * -2 = 80
        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(80.0));
    }

    #[test]
    fn test_conditional_transform_with_dependencies() {
        let str_id = StatId::from_str("STR");
        let mut context = StatContext::new();
        context.set("enabled", true);

        let inner_transform = Box::new(ScalingTransform::new(str_id.clone(), 2.0));
        let transform = ConditionalTransform::new(
            |ctx| ctx.get::<bool>("enabled").unwrap_or(false),
            inner_transform,
            "conditional scaling",
        );

        let mut deps = HashMap::new();
        deps.insert(str_id.clone(), StatValue::from_f64(10.0));

        // When enabled: 100 + 10 * 2 = 120
        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(120.0));

        context.set("enabled", false);
        // When disabled: 100 (unchanged)
        assert_eq!(transform.apply(StatValue::from_f64(100.0), &deps, &context).unwrap(), StatValue::from_f64(100.0));

        // Check dependencies are forwarded
        assert_eq!(transform.depends_on(), vec![str_id]);
    }

    #[test]
    fn test_transform_descriptions() {
        let mult = MultiplicativeTransform::new(1.5);
        assert!(mult.description().contains("1.50"));

        let add = AdditiveTransform::new(25.0);
        assert!(add.description().contains("25.00"));

        let clamp = ClampTransform::new(0.0, 100.0);
        assert!(clamp.description().contains("clamp"));

        let str_id = StatId::from_str("STR");
        let scale = ScalingTransform::new(str_id.clone(), 2.0);
        let desc = scale.description();
        assert!(desc.contains("STR"));
        assert!(desc.contains("2.00"));
    }
}
