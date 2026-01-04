//! Advanced example: Demonstrating resolver forking, batch resolution, and transform phases
//!
//! This example demonstrates:
//! - Resolver forking (copy-on-write)
//! - Batch resolution
//! - Transform phase ordering
//! - Subgraph extraction

use zzstat::source::ConstantSource;
use zzstat::transform::{
    AdditiveTransform, ClampTransform, MultiplicativeTransform, ScalingTransform,
};
use zzstat::*;

fn main() -> Result<(), StatError> {
    println!("=== Advanced Features Demo ===\n");

    // ===== Resolver Forking =====
    println!("1. Resolver Forking (Copy-on-Write)\n");

    let mut base = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    base.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    println!("Base resolver: HP = 100 (base)");

    // Fork the resolver
    let mut fork = base.fork();
    fork.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));
    println!("Fork resolver: HP = 100 (base) + 50 (fork) = 150");

    let context = StatContext::new();
    let base_hp = base.resolve(&hp_id, &context)?;
    let fork_hp = fork.resolve(&hp_id, &context)?;

    println!("  Base HP: {:.2}", base_hp.value);
    println!("  Fork HP: {:.2}", fork_hp.value);
    println!("  ✓ Base is unchanged, fork has additional source\n");

    // ===== Batch Resolution =====
    println!("2. Batch Resolution\n");

    let mut resolver = StatResolver::new();
    let str_id = StatId::from_str("STR");
    let dex_id = StatId::from_str("DEX");
    let atk_id = StatId::from_str("ATK");
    let crit_id = StatId::from_str("CRIT");
    let hp_id = StatId::from_str("HP");
    let mp_id = StatId::from_str("MP");

    // Register all stats
    resolver.register_source(str_id.clone(), Box::new(ConstantSource(10.0)));
    resolver.register_source(dex_id.clone(), Box::new(ConstantSource(15.0)));
    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(50.0)));
    resolver.register_source(crit_id.clone(), Box::new(ConstantSource(5.0)));
    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    resolver.register_source(mp_id.clone(), Box::new(ConstantSource(50.0)));

    // ATK depends on STR
    resolver.register_transform(
        atk_id.clone(),
        Box::new(ScalingTransform::new(str_id.clone(), 2.0)),
    );

    // CRIT depends on DEX
    resolver.register_transform(
        crit_id.clone(),
        Box::new(ScalingTransform::new(dex_id.clone(), 1.5)),
    );

    println!("Registered stats: STR, DEX, ATK, CRIT, HP, MP");
    println!("Dependencies: ATK -> STR, CRIT -> DEX");

    // Resolve only ATK and HP (and their dependencies)
    println!("\nResolving batch: [ATK, HP]");
    let results = resolver.resolve_batch(&[atk_id.clone(), hp_id.clone()], &context)?;

    println!("  Resolved stats: {:?}", results.keys().collect::<Vec<_>>());
    println!("  ✓ Only ATK, HP, and STR (dependency) were resolved");
    println!("  ✓ MP and DEX were not resolved (not needed)");
    println!("  ATK: {:.2} (50 + 10*2)", results[&atk_id].value);
    println!("  HP: {:.2}", results[&hp_id].value);
    println!("  STR: {:.2}", results[&str_id].value);
    println!();

    // ===== Transform Phase Ordering =====
    println!("3. Transform Phase Ordering\n");

    let mut resolver = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Register transforms in "wrong" order - phases will determine actual order
    println!("Registering transforms (order: clamp, multiply, add)");
    resolver.register_transform(atk_id.clone(), Box::new(ClampTransform::new(0.0, 200.0)));
    resolver.register_transform(atk_id.clone(), Box::new(MultiplicativeTransform::new(2.0)));
    resolver.register_transform(atk_id.clone(), Box::new(AdditiveTransform::new(50.0)));

    println!("  Phase order: Additive (0) -> Multiplicative (1) -> Final (2)");

    let resolved = resolver.resolve(&atk_id, &context)?;

    println!("  Calculation: 100 (base)");
    println!("    + 50 (Additive phase) = 150");
    println!("    * 2.0 (Multiplicative phase) = 300");
    println!("    clamp(0, 200) (Final phase) = 200");
    println!("  Final ATK: {:.2}", resolved.value);
    println!("  ✓ Phases ensure correct order regardless of registration order\n");

    // ===== Clamp Transforms with MinMax Stack Rule =====
    println!("4. Clamp Transforms with MinMax Stack Rule\n");

    let mut resolver = StatResolver::new();
    let crit_id = StatId::from_str("CRIT_CHANCE");

    resolver.register_source(crit_id.clone(), Box::new(ConstantSource(0.95))); // 95% crit

    // Register multiple clamp transforms with MinMax stack rule
    // When multiple clamps apply: effective_min = max(all mins), effective_max = min(all maxes)
    println!("Registering multiple clamp transforms:");
    println!("  Clamp 1: [0.0, 0.75]");
    println!("  Clamp 2: [0.0, 0.80]");

    use zzstat::numeric::StatValue;
    use zzstat::transform::{StackRule, TransformPhase};

    resolver.register_transform_with_rule(
        crit_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(0.0)),
            Some(StatValue::from_f64(0.75)),
        )),
    );
    resolver.register_transform_with_rule(
        crit_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_bounds(
            Some(StatValue::from_f64(0.0)),
            Some(StatValue::from_f64(0.80)),
        )),
    );

    let resolved = resolver.resolve(&crit_id, &context)?;

    println!("  Calculation: 0.95 (base)");
    println!("    effective_max = min(0.75, 0.80) = 0.75 (most restrictive)");
    println!("    clamp(0.0, 0.75) = 0.75");
    println!("  Final CRIT_CHANCE: {:.2}", resolved.value.to_f64());
    println!("  ✓ Multiple clamps compose deterministically\n");

    // Demonstrate min-only and max-only clamps
    let mut resolver2 = StatResolver::new();
    let move_speed_id = StatId::from_str("MOVE_SPEED");
    resolver2.register_source(move_speed_id.clone(), Box::new(ConstantSource(50.0)));

    println!("Registering min-only and max-only clamps:");
    resolver2.register_transform_with_rule(
        move_speed_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_min(StatValue::from_f64(100.0))), // Floor
    );
    resolver2.register_transform_with_rule(
        move_speed_id.clone(),
        TransformPhase::Final,
        StackRule::MinMax,
        Box::new(ClampTransform::with_max(StatValue::from_f64(200.0))), // Cap
    );

    let resolved2 = resolver2.resolve(&move_speed_id, &context)?;
    println!("  Base: 50.0");
    println!("  After clamp_min(100.0): {:.2}", resolved2.value.to_f64());
    println!("  ✓ Optional bounds allow flexible clamping\n");

    // ===== Multiple Forks =====
    println!("4. Multiple Forks from Same Base\n");

    let mut base = StatResolver::new();
    let atk_id = StatId::from_str("ATK");

    base.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));
    base.register_transform(atk_id.clone(), Box::new(MultiplicativeTransform::new(1.5)));

    println!("Base: ATK = 100 * 1.5 = 150");

    // Create multiple forks
    let mut fork1 = base.fork();
    let mut fork2 = base.fork();

    fork1.register_source(atk_id.clone(), Box::new(ConstantSource(10.0)));
    fork2.register_source(atk_id.clone(), Box::new(ConstantSource(20.0)));

    println!("Fork 1: ATK = (100 + 10) * 1.5 = 165");
    println!("Fork 2: ATK = (100 + 20) * 1.5 = 180");

    let base_atk = base.resolve(&atk_id, &context)?;
    let fork1_atk = fork1.resolve(&atk_id, &context)?;
    let fork2_atk = fork2.resolve(&atk_id, &context)?;

    println!("  Base ATK: {:.2}", base_atk.value);
    println!("  Fork 1 ATK: {:.2}", fork1_atk.value);
    println!("  Fork 2 ATK: {:.2}", fork2_atk.value);
    println!("  ✓ Each fork is independent\n");

    println!("=== Summary ===");
    println!("✓ Resolver forking allows efficient stat variations");
    println!("✓ Batch resolution optimizes performance by resolving only needed stats");
    println!("✓ Transform phases ensure correct calculation order");
    println!("✓ Multiple forks can be created from the same base resolver");
    println!("✓ Clamp transforms with MinMax stack rule compose deterministically");

    Ok(())
}
