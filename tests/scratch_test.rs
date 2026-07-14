
use zzstat::resource::*;
use zzstat::source::ConstantSource;
use zzstat::transform::TransformPhase;
use zzstat::*;

#[test]
fn test_resource_pool_clamp_on_tick() {
    let mut resolver = StatResolver::new();
    let context = StatContext::new();
    let max_hp_id = StatId::from("MAX_HP");

    // Start with 100 MAX_HP
    resolver.register_source(max_hp_id.clone(), Box::new(ConstantSource(100.0)));
    let mut pool = ResourcePool::new(max_hp_id.clone(), &mut resolver, &context);

    assert_eq!(pool.current_value, 100.0);

    // Now reduce MAX_HP to 50 using an override
    let override_bonus =
        zzstat::bonus::Bonus::r#override(max_hp_id.clone(), 50.0).in_phase(TransformPhase::Final);
    zzstat::bonus::apply_compiled_bonus(
        &mut resolver,
        &zzstat::bonus::compile_bonus::<f64>(&override_bonus),
    );

    // Call tick
    pool.tick(&mut resolver, &context);

    // Check if current_value is clamped to 50
    assert_eq!(
        pool.current_value, 50.0,
        "Current value should be clamped to max_val on tick!"
    );
}
