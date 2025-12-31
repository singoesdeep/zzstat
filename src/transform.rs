//! Stat transforms module.
//!
//! Transforms modify stat values after sources are collected.
//! Transforms can read other stats (dependencies) and must declare
//! them explicitly via `depends_on()`.

use crate::context::StatContext;
use crate::error::StatError;
use crate::stat_id::StatId;
use std::collections::HashMap;

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
/// use std::collections::HashMap;
///
/// let transform = MultiplicativeTransform::new(1.5);
/// let context = StatContext::new();
/// let deps = HashMap::new();
///
/// let result = transform.apply(100.0, &deps, &context).unwrap();
/// assert_eq!(result, 150.0);
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
        input: f64,
        dependencies: &HashMap<StatId, f64>,
        context: &StatContext,
    ) -> Result<f64, StatError>;

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
}

impl StatTransform for MultiplicativeTransform {
    fn depends_on(&self) -> Vec<StatId> {
        Vec::new()
    }

    fn apply(
        &self,
        input: f64,
        _dependencies: &HashMap<StatId, f64>,
        _context: &StatContext,
    ) -> Result<f64, StatError> {
        Ok(input * self.multiplier)
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
}

impl StatTransform for AdditiveTransform {
    fn depends_on(&self) -> Vec<StatId> {
        Vec::new()
    }

    fn apply(
        &self,
        input: f64,
        _dependencies: &HashMap<StatId, f64>,
        _context: &StatContext,
    ) -> Result<f64, StatError> {
        Ok(input + self.bonus)
    }

    fn description(&self) -> String {
        format!("+{:.2}", self.bonus)
    }
}

/// A clamp transform that restricts values to a range.
///
/// Ensures the output value is between `min` and `max` (inclusive).
///
/// # Examples
///
/// ```rust
/// use zzstat::transform::{StatTransform, ClampTransform};
/// use zzstat::StatContext;
/// use std::collections::HashMap;
///
/// let transform = ClampTransform::new(0.0, 100.0);
/// let context = StatContext::new();
/// let deps = HashMap::new();
///
/// assert_eq!(transform.apply(150.0, &deps, &context).unwrap(), 100.0);
/// assert_eq!(transform.apply(-10.0, &deps, &context).unwrap(), 0.0);
/// assert_eq!(transform.apply(50.0, &deps, &context).unwrap(), 50.0);
/// ```
#[derive(Debug, Clone)]
pub struct ClampTransform {
    min: f64,
    max: f64,
}

impl ClampTransform {
    /// Create a new clamp transform.
    ///
    /// # Arguments
    ///
    /// * `min` - Minimum allowed value (inclusive)
    /// * `max` - Maximum allowed value (inclusive)
    ///
    /// # Panics
    ///
    /// This function does not panic, but if `min > max`, the behavior
    /// is undefined (values will never pass the clamp).
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
        Self { min, max }
    }
}

impl StatTransform for ClampTransform {
    fn depends_on(&self) -> Vec<StatId> {
        Vec::new()
    }

    fn apply(
        &self,
        input: f64,
        _dependencies: &HashMap<StatId, f64>,
        _context: &StatContext,
    ) -> Result<f64, StatError> {
        Ok(input.clamp(self.min, self.max))
    }

    fn description(&self) -> String {
        format!("clamp({:.2}, {:.2})", self.min, self.max)
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
        input: f64,
        dependencies: &HashMap<StatId, f64>,
        context: &StatContext,
    ) -> Result<f64, StatError> {
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

    fn apply(
        &self,
        input: f64,
        dependencies: &HashMap<StatId, f64>,
        _context: &StatContext,
    ) -> Result<f64, StatError> {
        let dep_value = dependencies
            .get(&self.dependency)
            .ok_or_else(|| StatError::MissingDependency(self.dependency.clone()))?;
        Ok(input + (dep_value * self.scale_factor))
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

        assert_eq!(transform.apply(100.0, &deps, &context).unwrap(), 150.0);
    }

    #[test]
    fn test_additive_transform() {
        let transform = AdditiveTransform::new(25.0);
        let context = StatContext::new();
        let deps = HashMap::new();

        assert_eq!(transform.apply(100.0, &deps, &context).unwrap(), 125.0);
    }

    #[test]
    fn test_clamp_transform() {
        let transform = ClampTransform::new(0.0, 100.0);
        let context = StatContext::new();
        let deps = HashMap::new();

        assert_eq!(transform.apply(150.0, &deps, &context).unwrap(), 100.0);
        assert_eq!(transform.apply(-10.0, &deps, &context).unwrap(), 0.0);
        assert_eq!(transform.apply(50.0, &deps, &context).unwrap(), 50.0);
    }

    #[test]
    fn test_scaling_transform() {
        let str_id = StatId::from_str("STR");
        let transform = ScalingTransform::new(str_id.clone(), 2.0);
        let context = StatContext::new();
        let mut deps = HashMap::new();
        deps.insert(str_id.clone(), 10.0);

        assert_eq!(transform.depends_on(), vec![str_id]);
        assert_eq!(transform.apply(100.0, &deps, &context).unwrap(), 120.0);
    }

    #[test]
    fn test_scaling_transform_missing_dependency() {
        let str_id = StatId::from_str("STR");
        let transform = ScalingTransform::new(str_id, 2.0);
        let context = StatContext::new();
        let deps = HashMap::new();

        assert!(transform.apply(100.0, &deps, &context).is_err());
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
        assert_eq!(transform.apply(100.0, &deps, &context).unwrap(), 120.0);

        context.set("in_combat", false);
        assert_eq!(transform.apply(100.0, &deps, &context).unwrap(), 100.0);
    }
}
