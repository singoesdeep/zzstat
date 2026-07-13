use crate::context::StatContext;
use crate::error::StatError;
use crate::numeric::{StatValue, StatNumeric};
use crate::stat_id::StatId;
use rustc_hash::FxHashMap;
use serde::{Serialize, Deserialize};

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
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
/// use rustc_hash::FxHashMap;
///
/// let transform = MultiplicativeTransform::new(1.5);
/// let context = StatContext::new();
/// let deps = FxHashMap::default();
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
        dependencies: &FxHashMap<StatId, StatValue>,
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

#[cfg(test)]
mod tests {
    use crate::numeric::StatNumeric;
    use super::*;

    #[test]
    fn test_transform_phase_values() {
        assert_eq!(TransformPhase::Additive.value(), 0);
        assert_eq!(TransformPhase::Multiplicative.value(), 1);
        assert_eq!(TransformPhase::Final.value(), 2);
        assert_eq!(TransformPhase::Custom(10).value(), 10);
        // Custom phases below 3 should be clamped to 3
        assert_eq!(TransformPhase::Custom(1).value(), 3);
        assert_eq!(TransformPhase::Custom(2).value(), 3);
    }

    #[test]
    fn test_stack_rule_priority() {
        let rules = vec![
            StackRule::Override,
            StackRule::Additive,
            StackRule::Multiplicative,
            StackRule::Diminishing { k: StatValue::from_f64(1.0) },
            StackRule::Min,
            StackRule::Max,
            StackRule::MinMax,
        ];
        
        // Ensure they are strictly ordered according to their priority
        for i in 0..rules.len() - 1 {
            assert!(rules[i] < rules[i+1], "Rule {:?} should be less than {:?}", rules[i], rules[i+1]);
        }
    }
}

