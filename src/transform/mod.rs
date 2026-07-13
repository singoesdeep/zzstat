//! Stat transforms module.
//!
//! Transforms modify stat values after sources are collected.
//! Transforms can read other stats (dependencies) and must declare
//! them explicitly via `depends_on()`.

pub mod core;
pub mod standard;
pub mod conditional;

pub use core::*;
pub use standard::*;
pub use conditional::*;

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;
    use crate::numeric::{StatNumeric, StatValue};
    use crate::context::StatContext;
    use crate::stat_id::StatId;

    #[test]
    fn test_multiplicative_transform() {
        let transform = MultiplicativeTransform::new(1.5);
        let context = StatContext::new();
        let deps = FxHashMap::default();

        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(150.0)
        );
    }

    #[test]
    fn test_additive_transform() {
        let transform = AdditiveTransform::new(25.0);
        let context = StatContext::new();
        let deps = FxHashMap::default();

        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(125.0)
        );
    }

    #[test]
    fn test_clamp_transform() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::new(0.0, 100.0);
        let context = StatContext::new();
        let deps = FxHashMap::default();

        assert_eq!(
            transform
                .apply(StatValue::from_f64(150.0), &deps, &context)
                .unwrap()
                .to_f64(),
            100.0
        );
        assert_eq!(
            transform
                .apply(StatValue::from_f64(-10.0), &deps, &context)
                .unwrap()
                .to_f64(),
            0.0
        );
        assert_eq!(
            transform
                .apply(StatValue::from_f64(50.0), &deps, &context)
                .unwrap()
                .to_f64(),
            50.0
        );
    }

    #[test]
    fn test_clamp_transform_with_bounds() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::with_bounds(
            Some(StatValue::from_f64(0.0)),
            Some(StatValue::from_f64(100.0)),
        );
        let context = StatContext::new();
        let deps = FxHashMap::default();

        assert_eq!(
            transform
                .apply(StatValue::from_f64(150.0), &deps, &context)
                .unwrap()
                .to_f64(),
            100.0
        );
        assert_eq!(
            transform
                .apply(StatValue::from_f64(-10.0), &deps, &context)
                .unwrap()
                .to_f64(),
            0.0
        );
        assert_eq!(
            transform
                .apply(StatValue::from_f64(50.0), &deps, &context)
                .unwrap()
                .to_f64(),
            50.0
        );
    }

    #[test]
    fn test_clamp_transform_min_only() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::with_min(StatValue::from_f64(100.0));
        let context = StatContext::new();
        let deps = FxHashMap::default();

        assert_eq!(
            transform
                .apply(StatValue::from_f64(50.0), &deps, &context)
                .unwrap()
                .to_f64(),
            100.0
        );
        assert_eq!(
            transform
                .apply(StatValue::from_f64(150.0), &deps, &context)
                .unwrap()
                .to_f64(),
            150.0
        );
        assert_eq!(transform.min(), Some(StatValue::from_f64(100.0)));
        assert_eq!(transform.max(), None);
    }

    #[test]
    fn test_clamp_transform_max_only() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::with_max(StatValue::from_f64(0.75));
        let context = StatContext::new();
        let deps = FxHashMap::default();

        assert_eq!(
            transform
                .apply(StatValue::from_f64(1.0), &deps, &context)
                .unwrap()
                .to_f64(),
            0.75
        );
        assert_eq!(
            transform
                .apply(StatValue::from_f64(0.5), &deps, &context)
                .unwrap()
                .to_f64(),
            0.5
        );
        assert_eq!(transform.min(), None);
        assert_eq!(transform.max(), Some(StatValue::from_f64(0.75)));
    }

    #[test]
    fn test_clamp_transform_no_bounds() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::with_bounds(None, None);
        let context = StatContext::new();
        let deps = FxHashMap::default();

        // No-op: should return input unchanged
        assert_eq!(
            transform
                .apply(StatValue::from_f64(50.0), &deps, &context)
                .unwrap()
                .to_f64(),
            50.0
        );
        assert_eq!(
            transform
                .apply(StatValue::from_f64(150.0), &deps, &context)
                .unwrap()
                .to_f64(),
            150.0
        );
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
        let str_id = StatId::from("STR");
        let transform = ScalingTransform::new(str_id.clone(), 2.0);
        let context = StatContext::new();
        let mut deps = FxHashMap::default();
        deps.insert(str_id.clone(), StatValue::from_f64(10.0));

        assert_eq!(transform.depends_on(), vec![str_id]);
        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(120.0)
        );
    }

    #[test]
    fn test_scaling_transform_missing_dependency() {
        let str_id = StatId::from("STR");
        let transform = ScalingTransform::new(str_id, 2.0);
        let context = StatContext::new();
        let deps = FxHashMap::default();

        assert!(transform
            .apply(StatValue::from_f64(100.0), &deps, &context)
            .is_err());
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

        let deps = FxHashMap::default();
        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(120.0)
        );

        context.set("in_combat", false);
        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(100.0)
        );
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
        let deps = FxHashMap::default();

        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(0.0)
        );
    }

    #[test]
    fn test_multiplicative_transform_negative() {
        let transform = MultiplicativeTransform::new(-1.0);
        let context = StatContext::new();
        let deps = FxHashMap::default();

        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(-100.0)
        );
    }

    #[test]
    fn test_additive_transform_negative() {
        let transform = AdditiveTransform::new(-50.0);
        let context = StatContext::new();
        let deps = FxHashMap::default();

        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(50.0)
        );
    }

    #[test]
    fn test_clamp_transform_edge_cases() {
        use crate::numeric::StatValue;
        let transform = ClampTransform::new(0.0, 100.0);
        let context = StatContext::new();
        let deps = FxHashMap::default();

        // Exactly at min
        assert_eq!(
            transform
                .apply(StatValue::from_f64(0.0), &deps, &context)
                .unwrap()
                .to_f64(),
            0.0
        );
        // Exactly at max
        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap()
                .to_f64(),
            100.0
        );
        // Below min
        assert_eq!(
            transform
                .apply(StatValue::from_f64(-1.0), &deps, &context)
                .unwrap()
                .to_f64(),
            0.0
        );
        // Above max
        assert_eq!(
            transform
                .apply(StatValue::from_f64(101.0), &deps, &context)
                .unwrap()
                .to_f64(),
            100.0
        );
    }

    #[test]
    fn test_scaling_transform_zero_scale() {
        let str_id = StatId::from("STR");
        let transform = ScalingTransform::new(str_id.clone(), 0.0);
        let context = StatContext::new();
        let mut deps = FxHashMap::default();
        deps.insert(str_id.clone(), StatValue::from_f64(10.0));

        // 100 + 10 * 0 = 100
        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(100.0)
        );
    }

    #[test]
    fn test_scaling_transform_negative_scale() {
        let str_id = StatId::from("STR");
        let transform = ScalingTransform::new(str_id.clone(), -2.0);
        let context = StatContext::new();
        let mut deps = FxHashMap::default();
        deps.insert(str_id.clone(), StatValue::from_f64(10.0));

        // 100 + 10 * -2 = 80
        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(80.0)
        );
    }

    #[test]
    fn test_conditional_transform_with_dependencies() {
        let str_id = StatId::from("STR");
        let mut context = StatContext::new();
        context.set("enabled", true);

        let inner_transform = Box::new(ScalingTransform::new(str_id.clone(), 2.0));
        let transform = ConditionalTransform::new(
            |ctx| ctx.get::<bool>("enabled").unwrap_or(false),
            inner_transform,
            "conditional scaling",
        );

        let mut deps = FxHashMap::default();
        deps.insert(str_id.clone(), StatValue::from_f64(10.0));

        // When enabled: 100 + 10 * 2 = 120
        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(120.0)
        );

        context.set("enabled", false);
        // When disabled: 100 (unchanged)
        assert_eq!(
            transform
                .apply(StatValue::from_f64(100.0), &deps, &context)
                .unwrap(),
            StatValue::from_f64(100.0)
        );

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

        let str_id = StatId::from("STR");
        let scale = ScalingTransform::new(str_id.clone(), 2.0);
        let desc = scale.description();
        assert!(desc.contains("STR"));
        assert!(desc.contains("2.00"));
    }
}
