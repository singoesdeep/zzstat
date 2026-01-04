//! Tests for the bonus system API.
//!
//! These tests verify:
//! - Bonus builder API correctness
//! - Compilation to transforms
//! - Application to resolvers and forks
//! - Override semantics (critical)
//! - Integration with resolver

use zzstat::bonus::{apply_compiled_bonus, apply_compiled_bonuses, compile_bonus, Bonus};
use zzstat::source::ConstantSource;
use zzstat::transform::{StackRule, TransformPhase};
use zzstat::*;

// ============================================================================
// Unit Tests: Bonus Builder API
// ============================================================================

#[test]
fn test_bonus_add_flat() {
    let hp_id = StatId::from_str("HP");
    let bonus = Bonus::add(hp_id.clone())
        .flat(50.0)
        .in_phase(TransformPhase::Custom(3));

    assert_eq!(bonus.target, hp_id);
    assert_eq!(bonus.operation, zzstat::bonus::BonusOp::Add);
    assert_eq!(bonus.value, zzstat::bonus::BonusValue::Flat(50.0));
    assert_eq!(bonus.phase, TransformPhase::Custom(3));
}

#[test]
fn test_bonus_add_percent() {
    let hp_id = StatId::from_str("HP");
    let bonus = Bonus::add(hp_id.clone())
        .percent(0.10)
        .in_phase(TransformPhase::Custom(3));

    assert_eq!(bonus.target, hp_id);
    assert_eq!(bonus.operation, zzstat::bonus::BonusOp::Add);
    assert_eq!(bonus.value, zzstat::bonus::BonusValue::Percent(0.10));
}

#[test]
fn test_bonus_multiply() {
    let atk_id = StatId::from_str("ATK");
    let bonus = Bonus::mul(atk_id.clone())
        .percent(0.20)
        .in_phase(TransformPhase::Custom(3));

    assert_eq!(bonus.target, atk_id);
    assert_eq!(bonus.operation, zzstat::bonus::BonusOp::Multiply);
    assert_eq!(bonus.value, zzstat::bonus::BonusValue::Percent(0.20));
}

#[test]
fn test_bonus_override() {
    let hp_id = StatId::from_str("HP");
    let bonus = Bonus::r#override(hp_id.clone(), 500.0).in_phase(TransformPhase::Custom(4));

    assert_eq!(bonus.target, hp_id);
    assert_eq!(bonus.operation, zzstat::bonus::BonusOp::Override);
    assert_eq!(bonus.value, zzstat::bonus::BonusValue::Flat(500.0));
}

#[test]
fn test_bonus_clamp_min() {
    let hp_id = StatId::from_str("HP");
    let bonus = Bonus::clamp_min(hp_id.clone(), 100.0).in_phase(TransformPhase::Final);

    assert_eq!(bonus.target, hp_id);
    assert_eq!(bonus.operation, zzstat::bonus::BonusOp::ClampMin);
    assert_eq!(bonus.value, zzstat::bonus::BonusValue::Flat(100.0));
}

#[test]
fn test_bonus_clamp_max() {
    let crit_id = StatId::from_str("CRIT_CHANCE");
    let bonus = Bonus::clamp_max(crit_id.clone(), 0.75).in_phase(TransformPhase::Final);

    assert_eq!(bonus.target, crit_id);
    assert_eq!(bonus.operation, zzstat::bonus::BonusOp::ClampMax);
    assert_eq!(bonus.value, zzstat::bonus::BonusValue::Flat(0.75));
}

// ============================================================================
// Compilation Tests: Verify Correct Transforms Produced
// ============================================================================

#[test]
fn test_compile_add_flat() {
    let hp_id = StatId::from_str("HP");
    let bonus = Bonus::add(hp_id.clone())
        .flat(50.0)
        .in_phase(TransformPhase::Custom(3));

    let compiled = compile_bonus::<f64>(&bonus);

    assert_eq!(compiled.stat, hp_id);
    assert_eq!(compiled.phase, TransformPhase::Custom(3));
    assert_eq!(compiled.stack_rule, StackRule::Additive);
}

#[test]
fn test_compile_add_percent() {
    let hp_id = StatId::from_str("HP");
    let bonus = Bonus::add(hp_id.clone())
        .percent(0.10)
        .in_phase(TransformPhase::Custom(3));

    let compiled = compile_bonus::<f64>(&bonus);

    assert_eq!(compiled.stat, hp_id);
    assert_eq!(compiled.phase, TransformPhase::Custom(3));
    assert_eq!(compiled.stack_rule, StackRule::Additive);
}

#[test]
fn test_compile_multiply() {
    let atk_id = StatId::from_str("ATK");
    let bonus = Bonus::mul(atk_id.clone())
        .percent(0.20)
        .in_phase(TransformPhase::Custom(3));

    let compiled = compile_bonus::<f64>(&bonus);

    assert_eq!(compiled.stat, atk_id);
    assert_eq!(compiled.phase, TransformPhase::Custom(3));
    assert_eq!(compiled.stack_rule, StackRule::Multiplicative);
}

#[test]
fn test_compile_override() {
    let hp_id = StatId::from_str("HP");
    let bonus = Bonus::r#override(hp_id.clone(), 500.0).in_phase(TransformPhase::Custom(4));

    let compiled = compile_bonus::<f64>(&bonus);

    assert_eq!(compiled.stat, hp_id);
    assert_eq!(compiled.phase, TransformPhase::Custom(4));
    assert_eq!(compiled.stack_rule, StackRule::Override);
}

#[test]
fn test_compile_clamp_min() {
    let hp_id = StatId::from_str("HP");
    let bonus = Bonus::clamp_min(hp_id.clone(), 100.0).in_phase(TransformPhase::Final);

    let compiled = compile_bonus::<f64>(&bonus);

    assert_eq!(compiled.stat, hp_id);
    assert_eq!(compiled.phase, TransformPhase::Final);
    assert_eq!(compiled.stack_rule, StackRule::MinMax);
}

#[test]
fn test_compile_clamp_max() {
    let crit_id = StatId::from_str("CRIT_CHANCE");
    let bonus = Bonus::clamp_max(crit_id.clone(), 0.75).in_phase(TransformPhase::Final);

    let compiled = compile_bonus::<f64>(&bonus);

    assert_eq!(compiled.stat, crit_id);
    assert_eq!(compiled.phase, TransformPhase::Final);
    assert_eq!(compiled.stack_rule, StackRule::MinMax);
}

// ============================================================================
// Integration Tests: Resolver Forks
// ============================================================================

#[test]
fn test_apply_compiled_bonus_to_resolver() {
    let mut resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));

    let bonus = Bonus::add(hp_id.clone())
        .flat(50.0)
        .in_phase(TransformPhase::Additive);
    let compiled = compile_bonus::<f64>(&bonus);

    apply_compiled_bonus(&mut resolver, &compiled);

    let context = StatContext::new();
    let resolved = resolver.resolve(&hp_id, &context).unwrap();
    assert_eq!(resolved.value.to_f64(), 150.0); // 100 + 50
}

#[test]
fn test_apply_compiled_bonuses_to_fork() {
    let mut base_resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");
    let atk_id = StatId::from_str("ATK");

    base_resolver.register_source(hp_id.clone(), Box::new(ConstantSource(1000.0)));
    base_resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    let bonuses = vec![
        Bonus::add(hp_id.clone())
            .flat(50.0)
            .in_phase(TransformPhase::Custom(3)),
        Bonus::mul(atk_id.clone())
            .percent(0.20)
            .in_phase(TransformPhase::Custom(3)),
    ];

    let compiled: Vec<_> = bonuses.iter().map(|b| compile_bonus::<f64>(b)).collect();

    let mut fork = base_resolver.fork();
    apply_compiled_bonuses(&mut fork, &compiled);

    let context = StatContext::new();
    let stats = fork
        .resolve_batch(&[hp_id.clone(), atk_id.clone()], &context)
        .unwrap();

    assert_eq!(stats[&hp_id].value.to_f64(), 1050.0); // 1000 + 50
    assert_eq!(stats[&atk_id].value.to_f64(), 120.0); // 100 * 1.20
}

#[test]
fn test_fork_isolation() {
    let mut base_resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    base_resolver.register_source(hp_id.clone(), Box::new(ConstantSource(1000.0)));

    let bonus = Bonus::add(hp_id.clone())
        .flat(200.0)
        .in_phase(TransformPhase::Custom(3));
    let compiled = compile_bonus::<f64>(&bonus);

    let mut fork = base_resolver.fork();
    apply_compiled_bonus(&mut fork, &compiled);

    let context = StatContext::new();

    // Base resolver should be unaffected
    let base_resolved = base_resolver.resolve(&hp_id, &context).unwrap();
    assert_eq!(base_resolved.value.to_f64(), 1000.0);

    // Fork should have the bonus
    let fork_resolved = fork.resolve(&hp_id, &context).unwrap();
    assert_eq!(fork_resolved.value.to_f64(), 1200.0); // 1000 + 200
}

// ============================================================================
// Override-Specific Tests (Critical)
// ============================================================================

#[test]
fn test_override_resets_value_in_phase() {
    // Test that override resets the value within its phase (ignores input)
    let mut resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    // Base HP = 1000
    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(1000.0)));

    // Item phase: +200 HP, +10% HP
    let item_bonuses = vec![
        Bonus::add(hp_id.clone())
            .flat(200.0)
            .in_phase(TransformPhase::Custom(3)),
        Bonus::mul(hp_id.clone())
            .percent(0.10)
            .in_phase(TransformPhase::Custom(3)),
    ];
    let item_compiled: Vec<_> = item_bonuses
        .iter()
        .map(|b| compile_bonus::<f64>(b))
        .collect();

    let mut item_fork = resolver.fork();
    apply_compiled_bonuses(&mut item_fork, &item_compiled);

    let context = StatContext::new();
    let item_stats = item_fork.resolve(&hp_id, &context).unwrap();
    // Item phase: 1000 + 200 = 1200, then 1200 * 1.10 = 1320
    assert_eq!(item_stats.value.to_f64(), 1320.0);

    // Buff phase: Override HP = 500, then +50% HP
    let buff_bonuses = vec![
        Bonus::r#override(hp_id.clone(), 500.0).in_phase(TransformPhase::Custom(4)),
        Bonus::mul(hp_id.clone())
            .percent(0.50)
            .in_phase(TransformPhase::Custom(4)),
    ];
    let buff_compiled: Vec<_> = buff_bonuses
        .iter()
        .map(|b| compile_bonus::<f64>(b))
        .collect();

    let mut buff_fork = item_fork.fork();
    apply_compiled_bonuses(&mut buff_fork, &buff_compiled);

    let buff_stats = buff_fork.resolve(&hp_id, &context).unwrap();
    // Buff phase: Override resets to 500 (ignores 1320), then 500 * 1.50 = 750
    assert_eq!(buff_stats.value.to_f64(), 750.0);
}

#[test]
fn test_override_does_not_affect_previous_phases() {
    // Test that override does not affect previous phases
    let mut resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    // Base HP = 1000
    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(1000.0)));

    // Phase 3: +200 HP
    let phase3_bonus = Bonus::add(hp_id.clone())
        .flat(200.0)
        .in_phase(TransformPhase::Custom(3));
    let mut phase3_fork = resolver.fork();
    apply_compiled_bonus(&mut phase3_fork, &compile_bonus::<f64>(&phase3_bonus));

    let context = StatContext::new();
    let phase3_stats = phase3_fork.resolve(&hp_id, &context).unwrap();
    assert_eq!(phase3_stats.value.to_f64(), 1200.0); // 1000 + 200

    // Phase 4: Override to 500
    let phase4_bonus = Bonus::r#override(hp_id.clone(), 500.0).in_phase(TransformPhase::Custom(4));
    let mut phase4_fork = phase3_fork.fork();
    apply_compiled_bonus(&mut phase4_fork, &compile_bonus::<f64>(&phase4_bonus));

    let phase4_stats = phase4_fork.resolve(&hp_id, &context).unwrap();
    // Override resets to 500, but phase 3 result (1200) is the input to phase 4
    // Since override ignores input, result is 500
    assert_eq!(phase4_stats.value.to_f64(), 500.0);
}

#[test]
fn test_override_composes_with_other_transforms() {
    // Test that override composes correctly with additive/multiplicative transforms
    let mut resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(1000.0)));

    // Same phase: Override to 500, then +50% HP
    let bonuses = vec![
        Bonus::r#override(hp_id.clone(), 500.0).in_phase(TransformPhase::Custom(4)),
        Bonus::mul(hp_id.clone())
            .percent(0.50)
            .in_phase(TransformPhase::Custom(4)),
    ];
    let compiled: Vec<_> = bonuses.iter().map(|b| compile_bonus::<f64>(b)).collect();

    let mut fork = resolver.fork();
    apply_compiled_bonuses(&mut fork, &compiled);

    let context = StatContext::new();
    let resolved = fork.resolve(&hp_id, &context).unwrap();
    // Override resets to 500 (ignores 1000), then 500 * 1.50 = 750
    assert_eq!(resolved.value.to_f64(), 750.0);
}

#[test]
fn test_multiple_overrides_last_wins() {
    // Test that multiple overrides in same phase: last one wins (deterministic)
    let mut resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(1000.0)));

    // Multiple overrides in same phase
    let bonuses = vec![
        Bonus::r#override(hp_id.clone(), 200.0).in_phase(TransformPhase::Custom(4)),
        Bonus::r#override(hp_id.clone(), 300.0).in_phase(TransformPhase::Custom(4)),
        Bonus::r#override(hp_id.clone(), 400.0).in_phase(TransformPhase::Custom(4)),
    ];
    let compiled: Vec<_> = bonuses.iter().map(|b| compile_bonus::<f64>(b)).collect();

    let mut fork = resolver.fork();
    apply_compiled_bonuses(&mut fork, &compiled);

    let context = StatContext::new();
    let resolved = fork.resolve(&hp_id, &context).unwrap();
    // Last override wins (based on registration order with StackRule::Override)
    // Override stack rule: last transform wins, so 400 is the final value
    assert_eq!(resolved.value.to_f64(), 400.0);
}

#[test]
fn test_override_does_not_mutate_resolver_state() {
    // Test that override does not mutate resolver state (no transforms removed)
    let mut resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(1000.0)));

    // Add some transforms first
    let add_bonus = Bonus::add(hp_id.clone())
        .flat(200.0)
        .in_phase(TransformPhase::Custom(3));
    let mul_bonus = Bonus::mul(hp_id.clone())
        .percent(0.10)
        .in_phase(TransformPhase::Custom(3));

    let mut fork = resolver.fork();
    apply_compiled_bonus(&mut fork, &compile_bonus::<f64>(&add_bonus));
    apply_compiled_bonus(&mut fork, &compile_bonus::<f64>(&mul_bonus));

    let context = StatContext::new();
    let before_override = fork.resolve(&hp_id, &context).unwrap();
    assert_eq!(before_override.value.to_f64(), 1320.0); // (1000 + 200) * 1.10

    // Add override in different phase
    let override_bonus =
        Bonus::r#override(hp_id.clone(), 500.0).in_phase(TransformPhase::Custom(4));
    apply_compiled_bonus(&mut fork, &compile_bonus::<f64>(&override_bonus));

    let after_override = fork.resolve(&hp_id, &context).unwrap();
    // Override resets to 500 in phase 4 (ignores phase 3 result of 1320)
    assert_eq!(after_override.value.to_f64(), 500.0);

    // Verify transforms are still registered (resolution still works)
    let still_works = fork.resolve(&hp_id, &context).unwrap();
    assert_eq!(still_works.value.to_f64(), 500.0);
}

#[test]
fn test_override_works_with_resolver_forks() {
    // Test that override works correctly with resolver forks (fork isolation maintained)
    let mut base_resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");

    base_resolver.register_source(hp_id.clone(), Box::new(ConstantSource(1000.0)));

    // Fork 1: Add +200 HP
    let add_bonus = Bonus::add(hp_id.clone())
        .flat(200.0)
        .in_phase(TransformPhase::Custom(3));
    let mut fork1 = base_resolver.fork();
    apply_compiled_bonus(&mut fork1, &compile_bonus::<f64>(&add_bonus));

    // Fork 2: Override to 500
    let override_bonus =
        Bonus::r#override(hp_id.clone(), 500.0).in_phase(TransformPhase::Custom(4));
    let mut fork2 = base_resolver.fork();
    apply_compiled_bonus(&mut fork2, &compile_bonus::<f64>(&override_bonus));

    let context = StatContext::new();

    // Fork 1 should have 1200 HP
    let fork1_resolved = fork1.resolve(&hp_id, &context).unwrap();
    assert_eq!(fork1_resolved.value.to_f64(), 1200.0);

    // Fork 2 should have 500 HP (override)
    let fork2_resolved = fork2.resolve(&hp_id, &context).unwrap();
    assert_eq!(fork2_resolved.value.to_f64(), 500.0);

    // Base resolver should be unaffected
    let base_resolved = base_resolver.resolve(&hp_id, &context).unwrap();
    assert_eq!(base_resolved.value.to_f64(), 1000.0);
}

// ============================================================================
// Additional Integration Tests
// ============================================================================

// Note: Additive percent bonuses that depend on the stat itself create cycles
// and are not supported. Use multiplicative bonuses instead for percentage-based modifications.

#[test]
fn test_clamp_bonuses() {
    // Test clamp bonuses work correctly
    let mut resolver = StatResolver::new();
    let crit_id = StatId::from_str("CRIT_CHANCE");

    // Set crit chance to 100% (will be clamped)
    resolver.register_source(crit_id.clone(), Box::new(ConstantSource(1.0)));

    let clamp_bonus = Bonus::clamp_max(crit_id.clone(), 0.75).in_phase(TransformPhase::Final);
    let compiled = compile_bonus::<f64>(&clamp_bonus);

    let mut fork = resolver.fork();
    apply_compiled_bonus(&mut fork, &compiled);

    let context = StatContext::new();
    let resolved = fork.resolve(&crit_id, &context).unwrap();
    assert_eq!(resolved.value.to_f64(), 0.75); // Clamped to 75%
}

#[test]
fn test_complete_item_system() {
    // Test a complete item system with multiple bonuses
    let mut base_resolver = StatResolver::new();
    let hp_id = StatId::from_str("HP");
    let atk_id = StatId::from_str("ATK");

    base_resolver.register_source(hp_id.clone(), Box::new(ConstantSource(1000.0)));
    base_resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));

    // Define item bonuses
    let sword_bonuses = vec![
        Bonus::add(atk_id.clone())
            .flat(25.0)
            .in_phase(TransformPhase::Custom(3)),
        Bonus::mul(atk_id.clone())
            .percent(0.15)
            .in_phase(TransformPhase::Custom(3)),
    ];

    let armor_bonuses = vec![Bonus::add(hp_id.clone())
        .flat(100.0)
        .in_phase(TransformPhase::Custom(3))];

    // Compile all bonuses
    let mut all_bonuses = Vec::new();
    all_bonuses.extend(sword_bonuses);
    all_bonuses.extend(armor_bonuses);
    let all_compiled: Vec<_> = all_bonuses
        .iter()
        .map(|b| compile_bonus::<f64>(b))
        .collect();

    // Apply to character
    let mut equipped_fork = base_resolver.fork();
    apply_compiled_bonuses(&mut equipped_fork, &all_compiled);

    let context = StatContext::new();
    let stats = equipped_fork
        .resolve_batch(&[hp_id.clone(), atk_id.clone()], &context)
        .unwrap();

    // HP: 1000 + 100 = 1100
    assert_eq!(stats[&hp_id].value.to_f64(), 1100.0);

    // ATK: (100 + 25) * 1.15 = 143.75
    assert_eq!(stats[&atk_id].value.to_f64(), 143.75);
}
