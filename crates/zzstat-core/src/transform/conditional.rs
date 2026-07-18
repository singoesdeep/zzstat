use crate::context::StatContext;
use crate::error::StatError;
use crate::numeric::StatValue;
use crate::stat_id::StatId;
use crate::transform::core::StatTransform;
use rustc_hash::FxHashMap;

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
/// use rustc_hash::FxHashMap;
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
/// let deps = FxHashMap::default();
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
        dependencies: &FxHashMap<StatId, StatValue>,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::numeric::{StatNumeric, StatValue};
    use crate::transform::standard::MultiplicativeTransform;

    #[test]
    fn test_conditional_transform_applied() {
        let mut context = StatContext::new();
        context.set("is_night", true);

        let inner = Box::new(MultiplicativeTransform::new(2.0));
        let transform = ConditionalTransform::new(
            |ctx| ctx.get::<bool>("is_night").unwrap_or(false),
            inner,
            "night bonus",
        );

        let deps = FxHashMap::default();
        let result = transform
            .apply(StatValue::from_f64(10.0), &deps, &context)
            .unwrap();
        assert_eq!(result.to_f64(), 20.0);
    }

    #[test]
    fn test_conditional_transform_ignored() {
        let mut context = StatContext::new();
        context.set("is_night", false);

        let inner = Box::new(MultiplicativeTransform::new(2.0));
        let transform = ConditionalTransform::new(
            |ctx| ctx.get::<bool>("is_night").unwrap_or(false),
            inner,
            "night bonus",
        );

        let deps = FxHashMap::default();
        let result = transform
            .apply(StatValue::from_f64(10.0), &deps, &context)
            .unwrap();
        assert_eq!(result.to_f64(), 10.0);
    }
}
