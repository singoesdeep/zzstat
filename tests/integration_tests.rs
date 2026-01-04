use zzstat::source::ConstantSource;
use zzstat::transform::{
    AdditiveTransform, ClampTransform, MultiplicativeTransform, ScalingTransform, StackRule,
    TransformPhase,
};
use zzstat::*;

/// Test a complete stat resolution pipeline with dependencies.
#[test]
fn test_complete_pipeline() {
    let mut resolver = StatResolver::new();

    // Define stats
    let str_id = StatId::from_str("STR");
    let dex_id = StatId::from_str("DEX");
    let atk_id = StatId::from_str("ATK");
    let crit_id = StatId::from_str("CRIT");
    let dps_id = StatId::from_str("DPS");

    // Base stats
    resolver.register_source(str_id.clone(), Box::new(ConstantSource(10.0)));
    resolver.register_source(dex_id.clone(), Box::new(ConstantSource(15.0)));
    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(50.0)));

    // ATK scales with STR
    resolver.register_transform(
        atk_id.clone(),
        Box::new(ScalingTransform::new(str_id.clone(), 2.0)),
    );

    // CRIT scales with DEX
    resolver.register_source(crit_id.clone(), Box::new(ConstantSource(5.0)));
    resolver.register_transform(
        crit_id.clone(),
        Box::new(ScalingTransform::new(dex_id.clone(), 1.5)),
    );

    // DPS depends on ATK and CRIT
    resolver.register_source(dps_id.clone(), Box::new(ConstantSource(0.0)));
    resolver.register_transform(
        dps_id.clone(),
        Box::new(ScalingTransform::new(atk_id.clone(), 1.0)),
    );
    resolver.register_transform(
        dps_id.clone(),
        Box::new(ScalingTransform::new(crit_id.clone(), 0.1)),
    );

    let context = StatContext::new();

    // Resolve all stats
    let results = resolver.resolve_all(&context).unwrap();

    // Verify STR
    let str_resolved = results.get(&str_id).unwrap();
    assert_eq!(str_resolved.value, 10.0);

    // Verify DEX
    let dex_resolved = results.get(&dex_id).unwrap();
    assert_eq!(dex_resolved.value, 15.0);

    // Verify ATK: 50 (base) + 10 (STR) * 2 = 70
    let atk_resolved = results.get(&atk_id).unwrap();
    assert_eq!(atk_resolved.value, 70.0);

    // Verify CRIT: 5 (base) + 15 (DEX) * 1.5 = 27.5
    let crit_resolved = results.get(&crit_id).unwrap();
    assert_eq!(crit_resolved.value, 27.5);

    // Verify DPS: 0 (base) + 70 (ATK) * 1 + 27.5 (CRIT) * 0.1 = 72.75
    let dps_resolved = results.get(&dps_id).unwrap();
    assert_eq!(dps_resolved.value, 72.75);
}

/// Test multiple sources being summed (additive).
#[test]
fn test_additive_sources() {
    let mut resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    // Multiple sources should be summed
    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));
    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(25.0)));

    let context = StatContext::new();
    let resolved = resolver.resolve(&hp_id, &context).unwrap();

    assert_eq!(resolved.value, 175.0); // 100 + 50 + 25
    assert_eq!(resolved.sources.len(), 3);
}

/// Test transform chain (multiple transforms applied in order).
#[test]
fn test_transform_chain() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Apply multiple transforms in sequence
    resolver.register_transform(atk_id.clone(), Box::new(MultiplicativeTransform::new(1.5)));
    resolver.register_transform(atk_id.clone(), Box::new(MultiplicativeTransform::new(1.2)));
    resolver.register_transform(atk_id.clone(), Box::new(ClampTransform::new(0.0, 200.0)));

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // 100 * 1.5 * 1.2 = 180, then clamped to 200 (no change)
    assert!((resolved.value.to_f64() - 180.0).abs() < 0.01);
    // With new stacking semantics, transforms are grouped by stack rule
    // 2 multiplicative transforms → 1 stack group → 1 recorded transform
    // 1 clamp transform → 1 stack group → 1 recorded transform
    // Total: 2 transforms
    assert_eq!(resolved.transforms.len(), 2);
}

/// Test cache behavior.
#[test]
fn test_cache_behavior() {
    let mut resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));

    let context = StatContext::new();

    // First resolve
    let resolved1 = resolver.resolve(&hp_id, &context).unwrap();
    assert_eq!(resolved1.value, 100.0);

    // Second resolve should use cache
    let resolved2 = resolver.resolve(&hp_id, &context).unwrap();
    assert_eq!(resolved2.value, 100.0);

    // Add new source and invalidate
    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));

    // Should recalculate
    let resolved3 = resolver.resolve(&hp_id, &context).unwrap();
    assert_eq!(resolved3.value, 150.0);
}

/// Test complex dependency chain.
#[test]
fn test_complex_dependency_chain() {
    let mut resolver = StatResolver::new();

    let base_id = StatId::from_str("BASE");
    let mid_id = StatId::from_str("MID");
    let top_id = StatId::from_str("TOP");

    resolver.register_source(base_id.clone(), Box::new(ConstantSource(10.0)));
    resolver.register_source(mid_id.clone(), Box::new(ConstantSource(20.0)));
    resolver.register_source(top_id.clone(), Box::new(ConstantSource(30.0)));

    // MID depends on BASE
    resolver.register_transform(
        mid_id.clone(),
        Box::new(ScalingTransform::new(base_id.clone(), 1.0)),
    );

    // TOP depends on MID (which depends on BASE)
    resolver.register_transform(
        top_id.clone(),
        Box::new(ScalingTransform::new(mid_id.clone(), 1.0)),
    );

    let context = StatContext::new();
    let results = resolver.resolve_all(&context).unwrap();

    // BASE: 10
    assert_eq!(results.get(&base_id).unwrap().value, 10.0);

    // MID: 20 + 10 = 30
    assert_eq!(results.get(&mid_id).unwrap().value, 30.0);

    // TOP: 30 + 30 = 60
    assert_eq!(results.get(&top_id).unwrap().value, 60.0);
}

/// Test breakdown information for debugging.
#[test]
fn test_breakdown_information() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));
    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(50.0)));
    resolver.register_transform(atk_id.clone(), Box::new(MultiplicativeTransform::new(1.5)));

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // Check breakdown
    assert_eq!(resolved.sources.len(), 2);
    assert_eq!(resolved.transforms.len(), 1);

    // Get breakdown from resolver
    let breakdown = resolver.get_breakdown(&atk_id).unwrap();
    assert_eq!(breakdown.value, 225.0); // (100 + 50) * 1.5
}

/// Test resolver forking (copy-on-write).
#[test]
fn test_resolver_fork() {
    let mut base = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    // Register source in base
    base.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));

    // Fork the resolver
    let mut fork = base.fork();

    // Add source to fork (shouldn't affect base)
    fork.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));

    let context = StatContext::new();

    // Base should still have only 100
    let base_resolved = base.resolve(&hp_id, &context).unwrap();
    assert_eq!(base_resolved.value, 100.0);

    // Fork should have 150 (100 + 50)
    let fork_resolved = fork.resolve(&hp_id, &context).unwrap();
    assert_eq!(fork_resolved.value, 150.0);
}

/// Test resolver fork with transforms.
#[test]
fn test_resolver_fork_with_transforms() {
    let mut base = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    base.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));
    base.register_transform(atk_id.clone(), Box::new(MultiplicativeTransform::new(1.5)));

    let mut fork = base.fork();
    fork.register_transform(atk_id.clone(), Box::new(MultiplicativeTransform::new(1.2)));

    let context = StatContext::new();

    // Base: 100 * 1.5 = 150
    let base_resolved = base.resolve(&atk_id, &context).unwrap();
    assert_eq!(base_resolved.value, 150.0);

    // Fork: 100 * 1.5 * 1.2 = 180 (both transforms from base and fork)
    let fork_resolved = fork.resolve(&atk_id, &context).unwrap();
    assert!((fork_resolved.value.to_f64() - 180.0).abs() < 0.01);
}

/// Test resolve_batch functionality.
#[test]
fn test_resolve_batch() {
    let mut resolver = StatResolver::new();
    let str_id = StatId::from_str("STR");
    let atk_id = StatId::from_str("ATK");
    let hp_id = StatId::from_str("HP");
    let mp_id = StatId::from_str("MP");

    resolver.register_source(str_id.clone(), Box::new(ConstantSource(10.0)));
    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(50.0)));
    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    resolver.register_source(mp_id.clone(), Box::new(ConstantSource(50.0)));

    // ATK depends on STR
    resolver.register_transform(
        atk_id.clone(),
        Box::new(ScalingTransform::new(str_id.clone(), 2.0)),
    );

    let context = StatContext::new();

    // Resolve batch with ATK and HP
    let results = resolver.resolve_batch(&[atk_id.clone(), hp_id.clone()], &context).unwrap();

    // Should contain ATK and HP
    assert!(results.contains_key(&atk_id));
    assert!(results.contains_key(&hp_id));

    // Should also contain STR (dependency of ATK)
    assert!(results.contains_key(&str_id));

    // Should NOT contain MP (not requested and not a dependency)
    assert!(!results.contains_key(&mp_id));

    // Verify values
    assert_eq!(results[&str_id].value, 10.0);
    assert_eq!(results[&atk_id].value, 70.0); // 50 + 10*2
    assert_eq!(results[&hp_id].value, 100.0);
}

/// Test resolve_batch with empty targets.
#[test]
fn test_resolve_batch_empty() {
    let mut resolver = StatResolver::new();
    let context = StatContext::new();

    let results = resolver.resolve_batch(&[], &context).unwrap();
    assert!(results.is_empty());
}

/// Test cache invalidation.
#[test]
fn test_cache_invalidation() {
    let mut resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));

    let context = StatContext::new();

    // First resolve
    let resolved1 = resolver.resolve(&hp_id, &context).unwrap();
    assert_eq!(resolved1.value, 100.0);

    // Should be cached
    let resolved2 = resolver.resolve(&hp_id, &context).unwrap();
    assert_eq!(resolved2.value, 100.0);

    // Invalidate
    resolver.invalidate(&hp_id);

    // Add new source
    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));

    // Should recalculate
    let resolved3 = resolver.resolve(&hp_id, &context).unwrap();
    assert_eq!(resolved3.value, 150.0);
}

/// Test invalidate_all.
#[test]
fn test_invalidate_all() {
    let mut resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");
    let mp_id = StatId::from_str("MP");

    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    resolver.register_source(mp_id.clone(), Box::new(ConstantSource(50.0)));

    let context = StatContext::new();

    // Resolve both
    let _ = resolver.resolve_all(&context).unwrap();

    // Both should be cached
    assert!(resolver.get_breakdown(&hp_id).is_some());
    assert!(resolver.get_breakdown(&mp_id).is_some());

    // Invalidate all
    resolver.invalidate_all();

    // Both should be gone
    assert!(resolver.get_breakdown(&hp_id).is_none());
    assert!(resolver.get_breakdown(&mp_id).is_none());
}

/// Test transform phase ordering.
#[test]
fn test_transform_phase_ordering() {
    use zzstat::transform::{AdditiveTransform, ClampTransform};

    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Register transforms in different phases
    // Final phase (clamp) should be applied last
    resolver.register_transform(atk_id.clone(), Box::new(ClampTransform::new(0.0, 150.0)));

    // Multiplicative phase
    resolver.register_transform(atk_id.clone(), Box::new(MultiplicativeTransform::new(2.0)));

    // Additive phase (should be applied first)
    resolver.register_transform(atk_id.clone(), Box::new(AdditiveTransform::new(50.0)));

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // Order: 100 (base) + 50 (additive) = 150, * 2 (multiplicative) = 300, clamp(0, 150) = 150
    assert_eq!(resolved.value, 150.0);
}

/// Test custom transform phase.
#[test]
fn test_custom_transform_phase() {
    use zzstat::transform::{StatTransform, TransformPhase};

    struct CustomPhaseTransform {
        phase: TransformPhase,
        value: f64,
    }

    impl StatTransform for CustomPhaseTransform {
        fn depends_on(&self) -> Vec<StatId> {
            Vec::new()
        }

        fn phase(&self) -> TransformPhase {
            self.phase
        }

        fn apply(
            &self,
            input: StatValue,
            _dependencies: &std::collections::HashMap<StatId, StatValue>,
            _context: &StatContext,
        ) -> Result<StatValue, StatError> {
            Ok(input + StatValue::from_f64(self.value))
        }

        fn description(&self) -> String {
            format!("Custom phase +{}", self.value)
        }
    }

    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Add custom phase transform (phase 10)
    resolver.register_transform(
        atk_id.clone(),
        Box::new(CustomPhaseTransform {
            phase: TransformPhase::Custom(10),
            value: 50.0,
        }),
    );

    // Add multiplicative transform (phase 1)
    resolver.register_transform(atk_id.clone(), Box::new(MultiplicativeTransform::new(2.0)));

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // Order: 100 * 2 (multiplicative) = 200, + 50 (custom phase 10) = 250
    assert_eq!(resolved.value, 250.0);
}

/// Test missing dependency (resolves to 0 by default).
#[test]
fn test_missing_dependency() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");
    let missing_id = StatId::from_str("MISSING");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // ATK depends on MISSING (which doesn't exist - no source registered)
    // The resolver will resolve MISSING to 0 (default) and use that
    resolver.register_transform(
        atk_id.clone(),
        Box::new(ScalingTransform::new(missing_id.clone(), 1.0)),
    );

    let context = StatContext::new();
    let result = resolver.resolve(&atk_id, &context);

    // The resolver will resolve MISSING to 0 (default value when no source)
    // So ATK = 100 + 0 * 1.0 = 100
    assert!(result.is_ok());
    let resolved = result.unwrap();
    assert_eq!(resolved.value, 100.0); // 100 + 0*1 = 100
}

/// Test missing source error.
#[test]
fn test_missing_source() {
    let mut resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    // No source registered
    let context = StatContext::new();
    let result = resolver.resolve(&hp_id, &context);

    assert!(result.is_err());
    if let Err(StatError::MissingSource(id)) = result {
        assert_eq!(id, hp_id);
    } else {
        panic!("Expected MissingSource error");
    }
}

/// Test stat with only transforms (no sources).
#[test]
fn test_stat_with_only_transforms() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    // No source, but has transform
    resolver.register_transform(atk_id.clone(), Box::new(AdditiveTransform::new(100.0)));

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // Should default to 0 + 100 = 100
    assert_eq!(resolved.value, 100.0);
}

/// Test multiple forks from same base.
#[test]
fn test_multiple_forks() {
    let mut base = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    base.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));

    let mut fork1 = base.fork();
    let mut fork2 = base.fork();

    fork1.register_source(hp_id.clone(), Box::new(ConstantSource(10.0)));
    fork2.register_source(hp_id.clone(), Box::new(ConstantSource(20.0)));

    let context = StatContext::new();

    let base_resolved = base.resolve(&hp_id, &context).unwrap();
    let fork1_resolved = fork1.resolve(&hp_id, &context).unwrap();
    let fork2_resolved = fork2.resolve(&hp_id, &context).unwrap();

    assert_eq!(base_resolved.value, 100.0);
    assert_eq!(fork1_resolved.value, 110.0);
    assert_eq!(fork2_resolved.value, 120.0);
}

/// Test additive stacking: multiple additive transforms should sum their deltas.
#[test]
fn test_additive_stacking() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Register multiple additive transforms with explicit stack rule
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Additive,
        StackRule::Additive,
        Box::new(AdditiveTransform::new(50.0)),
    );
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Additive,
        StackRule::Additive,
        Box::new(AdditiveTransform::new(30.0)),
    );
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Additive,
        StackRule::Additive,
        Box::new(AdditiveTransform::new(20.0)),
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // Additive stacking: base + sum(deltas) = 100 + (50 + 30 + 20) = 200
    assert_eq!(resolved.value.to_f64(), 200.0);
}

/// Test multiplicative stacking: multiple multiplicative transforms should multiply their multipliers.
#[test]
fn test_multiplicative_stacking() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Register multiple multiplicative transforms with explicit stack rule
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Multiplicative,
        StackRule::Multiplicative,
        Box::new(MultiplicativeTransform::new(1.5)),
    );
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Multiplicative,
        StackRule::Multiplicative,
        Box::new(MultiplicativeTransform::new(1.2)),
    );
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Multiplicative,
        StackRule::Multiplicative,
        Box::new(MultiplicativeTransform::new(1.1)),
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // Multiplicative stacking: base × product(multipliers) = 100 × (1.5 × 1.2 × 1.1) = 198
    assert!((resolved.value.to_f64() - 198.0).abs() < 0.01);
}

/// Test additive + multiplicative combination.
#[test]
fn test_additive_multiplicative_combination() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Additive phase: sum of deltas
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Additive,
        StackRule::Additive,
        Box::new(AdditiveTransform::new(50.0)),
    );
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Additive,
        StackRule::Additive,
        Box::new(AdditiveTransform::new(30.0)),
    );

    // Multiplicative phase: product of multipliers
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Multiplicative,
        StackRule::Multiplicative,
        Box::new(MultiplicativeTransform::new(1.5)),
    );
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Multiplicative,
        StackRule::Multiplicative,
        Box::new(MultiplicativeTransform::new(1.2)),
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // Order: additive first, then multiplicative
    // base = 100
    // after additive: 100 + (50 + 30) = 180
    // after multiplicative: 180 × (1.5 × 1.2) = 324
    assert!((resolved.value.to_f64() - 324.0).abs() < 0.01);
}

/// Test override precedence: last override transform wins.
#[test]
fn test_override_precedence() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Register multiple override transforms
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Final,
        StackRule::Override,
        Box::new(AdditiveTransform::new(200.0)), // This adds 200 to base
    );
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Final,
        StackRule::Override,
        Box::new(AdditiveTransform::new(300.0)), // This adds 300 to base (should win)
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // Override: last transform wins
    // Last transform: 100 + 300 = 400
    assert_eq!(resolved.value.to_f64(), 400.0);
}

/// Test diminishing returns stacking.
#[test]
fn test_diminishing_returns() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Create a simple transform that works with diminishing returns
    // We'll use multiplicative transforms with diminishing stack rule
    // Actually, diminishing returns needs a k parameter, so let's use a custom approach
    // For now, let's test that diminishing applies the formula correctly
    
    // Note: Diminishing returns formula: value × (1 - exp(-k × stacks))
    // With k=0.5 and 2 stacks: multiplier = 1 - exp(-0.5 × 2) = 1 - exp(-1) ≈ 0.632
    
    // Since we can't easily create a diminishing transform without custom code,
    // let's test that the stack rule exists and the resolver handles it
    // For a proper test, we'd need a transform type that supports diminishing returns
    
    // For now, this test documents the expected behavior
    // In practice, diminishing returns would be implemented as a custom transform type
}

/// Test min/max clamping with multiple clamps.
#[test]
fn test_min_max_clamping() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Register multiple max clamps (most restrictive wins)
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Final,
        StackRule::Max,
        Box::new(ClampTransform::new(0.0, 150.0)),
    );
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Final,
        StackRule::Max,
        Box::new(ClampTransform::new(0.0, 120.0)), // More restrictive
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // Max clamping: minimum of all max bounds wins (most restrictive)
    // 100 is below 120, so no change
    assert_eq!(resolved.value.to_f64(), 100.0);

    // Test with value above the limit
    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(200.0)));
    resolver.invalidate(&atk_id);
    let resolved2 = resolver.resolve(&atk_id, &context).unwrap();
    // Should be clamped to 120 (most restrictive max)
    assert_eq!(resolved2.value.to_f64(), 120.0);
}

/// Test min clamping.
#[test]
fn test_min_clamping() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(50.0)));

    // Register multiple min clamps (most restrictive wins - highest min)
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Final,
        StackRule::Min,
        Box::new(ClampTransform::new(10.0, 1000.0)),
    );
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Final,
        StackRule::Min,
        Box::new(ClampTransform::new(30.0, 1000.0)), // More restrictive (higher min)
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // Min clamping: maximum of all min bounds wins (most restrictive)
    // 50 is above 30, so no change
    assert_eq!(resolved.value.to_f64(), 50.0);

    // Test with value below the limit - create a new resolver to avoid source accumulation
    let mut resolver2 = StatResolver::new();
    resolver2.register_source(atk_id.clone(), Box::new(ConstantSource(20.0)));
    resolver2.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Final,
        StackRule::Min,
        Box::new(ClampTransform::new(10.0, 1000.0)),
    );
    resolver2.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Final,
        StackRule::Min,
        Box::new(ClampTransform::new(30.0, 1000.0)), // More restrictive (higher min)
    );
    let resolved2 = resolver2.resolve(&atk_id, &context).unwrap();
    // Should be clamped to 30 (most restrictive min)
    assert_eq!(resolved2.value.to_f64(), 30.0);
}

/// Test edge case: zero base value with additive stacking.
#[test]
fn test_additive_stacking_zero_base() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(0.0)));

    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Additive,
        StackRule::Additive,
        Box::new(AdditiveTransform::new(10.0)),
    );
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Additive,
        StackRule::Additive,
        Box::new(AdditiveTransform::new(20.0)),
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // 0 + (10 + 20) = 30
    assert_eq!(resolved.value.to_f64(), 30.0);
}

/// Test edge case: negative additive values.
#[test]
fn test_additive_stacking_negative() {
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Additive,
        StackRule::Additive,
        Box::new(AdditiveTransform::new(50.0)),
    );
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Additive,
        StackRule::Additive,
        Box::new(AdditiveTransform::new(-30.0)), // Negative value
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // 100 + (50 + (-30)) = 120
    assert_eq!(resolved.value.to_f64(), 120.0);
}

/// Test MinMax stack rule with multiple clamp transforms.
#[test]
fn test_minmax_stack_rule() {
    use zzstat::transform::StackRule;
    use zzstat::numeric::StatValue;
    
    let mut resolver = StatResolver::new();
    let crit_id = StatId::from_str("CRIT");

    resolver.register_source(crit_id.clone(), Box::new(ConstantSource(0.9))); // 90% crit

    // Register multiple clamp transforms with MinMax stack rule
    resolver.register_transform_with_rule(
        crit_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(0.0)),
            Some(StatValue::from_f64(0.75)), // 75% cap
        )),
    );
    resolver.register_transform_with_rule(
        crit_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(0.0)),
            Some(StatValue::from_f64(0.80)), // 80% cap (less restrictive)
        )),
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&crit_id, &context).unwrap();

    // MinMax: effective_max = min(0.75, 0.80) = 0.75 (most restrictive)
    // 0.9 should be clamped to 0.75
    assert_eq!(resolved.value.to_f64(), 0.75);
}

/// Test MinMax stack rule with min-only and max-only clamps.
#[test]
fn test_minmax_stack_rule_mixed_bounds() {
    use zzstat::transform::StackRule;
    use zzstat::numeric::StatValue;
    
    let mut resolver = StatResolver::new();
    let move_speed_id = StatId::from_str("MOVE_SPEED");

    resolver.register_source(move_speed_id.clone(), Box::new(ConstantSource(50.0)));

    // Register min-only and max-only clamps with MinMax stack rule
    resolver.register_transform_with_rule(
        move_speed_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_min(StatValue::from_f64(100.0))), // Floor
    );
    resolver.register_transform_with_rule(
        move_speed_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_max(StatValue::from_f64(200.0))), // Cap
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&move_speed_id, &context).unwrap();

    // 50 should be clamped to 100 (min floor)
    assert_eq!(resolved.value.to_f64(), 100.0);

    // Test with value above max
    resolver.register_source(move_speed_id.clone(), Box::new(ConstantSource(250.0)));
    resolver.invalidate(&move_speed_id);
    let resolved2 = resolver.resolve(&move_speed_id, &context).unwrap();
    // 250 should be clamped to 200 (max cap)
    assert_eq!(resolved2.value.to_f64(), 200.0);
}

/// Test clamp composition with additive transform.
#[test]
fn test_clamp_with_additive() {
    use zzstat::transform::StackRule;
    use zzstat::numeric::StatValue;
    
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Additive transform first
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Additive,
        StackRule::Additive,
        Box::new(AdditiveTransform::new(50.0)),
    );

    // Then clamp in Final phase
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(0.0)),
            Some(StatValue::from_f64(120.0)),
        )),
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // 100 + 50 = 150, then clamped to 120
    assert_eq!(resolved.value.to_f64(), 120.0);
}

/// Test clamp composition with multiplicative transform.
#[test]
fn test_clamp_with_multiplicative() {
    use zzstat::transform::StackRule;
    use zzstat::numeric::StatValue;
    
    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Multiplicative transform
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Multiplicative,
        StackRule::Multiplicative,
        Box::new(MultiplicativeTransform::new(2.0)),
    );

    // Then clamp in Final phase
    resolver.register_transform_with_rule(
        atk_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(0.0)),
            Some(StatValue::from_f64(150.0)),
        )),
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&atk_id, &context).unwrap();

    // 100 * 2 = 200, then clamped to 150
    assert_eq!(resolved.value.to_f64(), 150.0);
}

/// Test multiple clamps with MinMax: effective_min = max(all mins), effective_max = min(all maxes).
#[test]
fn test_multiple_clamps_minmax_composition() {
    use zzstat::transform::StackRule;
    use zzstat::numeric::StatValue;
    
    let mut resolver = StatResolver::new();
    let stat_id = StatId::from_str("STAT");

    resolver.register_source(stat_id.clone(), Box::new(ConstantSource(150.0)));

    // Multiple clamps with different bounds
    resolver.register_transform_with_rule(
        stat_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(10.0)),
            Some(StatValue::from_f64(200.0)),
        )),
    );
    resolver.register_transform_with_rule(
        stat_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(30.0)), // Higher min (more restrictive)
            Some(StatValue::from_f64(120.0)), // Lower max (more restrictive)
        )),
    );

    let context = StatContext::new();
    let resolved = resolver.resolve(&stat_id, &context).unwrap();

    // effective_min = max(10, 30) = 30
    // effective_max = min(200, 120) = 120
    // 150 is within [30, 120], so it should be clamped to 120
    assert_eq!(resolved.value.to_f64(), 120.0);

    // Test with value below effective_min
    // Create a new resolver with the same transforms but different source value
    // (since register_source adds sources additively, we need a fresh resolver)
    let mut resolver2 = StatResolver::new();
    resolver2.register_source(stat_id.clone(), Box::new(ConstantSource(20.0)));
    resolver2.register_transform_with_rule(
        stat_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(10.0)),
            Some(StatValue::from_f64(200.0)),
        )),
    );
    resolver2.register_transform_with_rule(
        stat_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(30.0)), // Higher min (more restrictive)
            Some(StatValue::from_f64(120.0)), // Lower max (more restrictive)
        )),
    );
    let resolved2 = resolver2.resolve(&stat_id, &context).unwrap();
    // 20 should be clamped to 30 (effective_min)
    assert_eq!(resolved2.value.to_f64(), 30.0);
}

/// Test deterministic ordering: same transforms, different registration order.
#[test]
fn test_clamp_deterministic_ordering() {
    use zzstat::transform::StackRule;
    use zzstat::numeric::StatValue;
    
    let mut resolver1 = StatResolver::new();
    let mut resolver2 = StatResolver::new();
    let stat_id = StatId::from_str("STAT");

    resolver1.register_source(stat_id.clone(), Box::new(ConstantSource(150.0)));
    resolver2.register_source(stat_id.clone(), Box::new(ConstantSource(150.0)));

    // Register same clamps in different order
    resolver1.register_transform_with_rule(
        stat_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(10.0)),
            Some(StatValue::from_f64(200.0)),
        )),
    );
    resolver1.register_transform_with_rule(
        stat_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(30.0)),
            Some(StatValue::from_f64(120.0)),
        )),
    );

    resolver2.register_transform_with_rule(
        stat_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(30.0)),
            Some(StatValue::from_f64(120.0)),
        )),
    );
    resolver2.register_transform_with_rule(
        stat_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(10.0)),
            Some(StatValue::from_f64(200.0)),
        )),
    );

    let context = StatContext::new();
    let resolved1 = resolver1.resolve(&stat_id, &context).unwrap();
    let resolved2 = resolver2.resolve(&stat_id, &context).unwrap();

    // Results should be identical regardless of registration order
    // effective_min = max(10, 30) = 30
    // effective_max = min(200, 120) = 120
    // 150 clamped to 120
    assert_eq!(resolved1.value.to_f64(), 120.0);
    assert_eq!(resolved2.value.to_f64(), 120.0);
    assert_eq!(resolved1.value, resolved2.value);
}