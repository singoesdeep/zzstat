use zzstat::source::ConstantSource;
use zzstat::transform::{ClampTransform, MultiplicativeTransform, ScalingTransform};
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
    assert_eq!(resolved.value, 180.0);
    assert_eq!(resolved.transforms.len(), 3);
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
