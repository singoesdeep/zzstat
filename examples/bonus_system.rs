//! Bonus System API Example
//!
//! This example demonstrates the generic bonus system API for zzstat:
//! - Declarative bonus definition using the builder API
//! - Compilation of bonuses to transforms (branch-free)
//! - Application of compiled bonuses to resolver forks
//! - Override semantics (reset value within phase)
//!
//! This shows how game code can use a simple, declarative API while
//! maintaining zero branching during stat resolution.

use zzstat::bonus::{apply_compiled_bonuses, compile_bonus, Bonus};
use zzstat::source::ConstantSource;
use zzstat::transform::TransformPhase;
use zzstat::*;

fn main() -> Result<(), StatError> {
    println!("=== Bonus System API Example ===\n");

    // Define stat IDs
    let hp_id = StatId::from_str("HP");
    let attack_id = StatId::from_str("ATTACK");
    let crit_chance_id = StatId::from_str("CRIT_CHANCE");

    // ========================================================================
    // Setup: Create base resolver with character stats
    // ========================================================================

    let mut base_resolver = StatResolver::new();

    // Register base HP source
    base_resolver.register_source(hp_id.clone(), Box::new(ConstantSource(1000.0)));
    base_resolver.register_source(attack_id.clone(), Box::new(ConstantSource(100.0)));
    base_resolver.register_source(crit_chance_id.clone(), Box::new(ConstantSource(0.5)));

    // ========================================================================
    // Example 1: Basic Bonus Usage
    // ========================================================================

    println!("--- Example 1: Basic Bonus Usage ---");

    // Define bonuses declaratively
    let bonuses = vec![
        Bonus::add(hp_id.clone()).flat(50.0).in_phase(TransformPhase::Custom(3)),
        Bonus::mul(attack_id.clone())
            .percent(0.20)
            .in_phase(TransformPhase::Custom(3)),
    ];

    // Compile once (all branching happens here)
    let compiled: Vec<_> = bonuses.iter().map(|b| compile_bonus::<f64>(b)).collect();

    // Apply to resolver fork
    let mut fork = base_resolver.fork();
    apply_compiled_bonuses(&mut fork, &compiled);

    // Resolve (pure math, no branching)
    let context = StatContext::new();
    let stats = fork.resolve_batch(&[hp_id.clone(), attack_id.clone()], &context)?;

    println!("Base HP: 1000, Attack: 100");
    println!(
        "After bonuses (+50 HP, +20% Attack): HP = {:.2}, Attack = {:.2}",
        stats[&hp_id].value.to_f64(),
        stats[&attack_id].value.to_f64()
    );
    println!();

    // ========================================================================
    // Example 2: Override Semantics
    // ========================================================================

    println!("--- Example 2: Override Semantics ---");
    println!("Demonstrates how override resets the value within its phase");

    // Setup: Base HP = 1000, Item phase adds +200 HP and +10% HP
    let mut override_resolver = StatResolver::new();
    override_resolver.register_source(hp_id.clone(), Box::new(ConstantSource(1000.0)));

    let item_bonuses = vec![
        Bonus::add(hp_id.clone()).flat(200.0).in_phase(TransformPhase::Custom(3)),
        Bonus::mul(hp_id.clone())
            .percent(0.10)
            .in_phase(TransformPhase::Custom(3)),
    ];
    let item_compiled: Vec<_> = item_bonuses.iter().map(|b| compile_bonus::<f64>(b)).collect();

    let mut item_fork = override_resolver.fork();
    apply_compiled_bonuses(&mut item_fork, &item_compiled);

    let item_stats = item_fork.resolve(&hp_id, &context)?;
    println!(
        "Item phase (base 1000 + 200 + 10%): HP = {:.2}",
        item_stats.value.to_f64()
    );

    // Buff phase: Override HP = 500, then +50% HP
    let buff_bonuses = vec![
        Bonus::r#override(hp_id.clone(), 500.0).in_phase(TransformPhase::Custom(4)),
        Bonus::mul(hp_id.clone())
            .percent(0.50)
            .in_phase(TransformPhase::Custom(4)),
    ];
    let buff_compiled: Vec<_> = buff_bonuses.iter().map(|b| compile_bonus::<f64>(b)).collect();

    let mut buff_fork = item_fork.fork();
    apply_compiled_bonuses(&mut buff_fork, &buff_compiled);

    let buff_stats = buff_fork.resolve(&hp_id, &context)?;
    println!(
        "Buff phase (override 500 + 50%): HP = {:.2}",
        buff_stats.value.to_f64()
    );
    println!("Note: Override resets the value to 500, then +50% is applied");
    println!();

    // ========================================================================
    // Example 3: Clamp Bonuses
    // ========================================================================

    println!("--- Example 3: Clamp Bonuses ---");

    // Crit chance cap at 75%
    let clamp_bonus = Bonus::clamp_max(crit_chance_id.clone(), 0.75)
        .in_phase(TransformPhase::Final);

    let mut clamp_fork = base_resolver.fork();
    apply_compiled_bonus(&mut clamp_fork, &compile_bonus::<f64>(&clamp_bonus));

    // Test with high crit chance
    let mut high_crit_resolver = StatResolver::new();
    high_crit_resolver.register_source(crit_chance_id.clone(), Box::new(ConstantSource(1.0))); // 100% crit
    let mut high_crit_fork = high_crit_resolver.fork();
    apply_compiled_bonus(&mut high_crit_fork, &compile_bonus::<f64>(&clamp_bonus));

    let clamped_stats = high_crit_fork.resolve(&crit_chance_id, &context)?;
    println!(
        "Crit chance capped at 75%: {:.2}%",
        clamped_stats.value.to_f64() * 100.0
    );
    println!();

    // ========================================================================
    // Example 4: Complete Item System
    // ========================================================================

    println!("--- Example 4: Complete Item System ---");

    struct Item {
        name: &'static str,
        bonuses: Vec<Bonus>,
    }

    let sword = Item {
        name: "Steel Sword",
        bonuses: vec![
            Bonus::add(attack_id.clone()).flat(25.0).in_phase(TransformPhase::Custom(3)),
            Bonus::mul(attack_id.clone())
                .percent(0.15)
                .in_phase(TransformPhase::Custom(3)),
        ],
    };

    let armor = Item {
        name: "Plate Armor",
        bonuses: vec![Bonus::add(hp_id.clone()).flat(100.0).in_phase(TransformPhase::Custom(3))],
    };

    // Compile all item bonuses once
    let mut all_bonuses = Vec::new();
    all_bonuses.extend(sword.bonuses);
    all_bonuses.extend(armor.bonuses);
    let all_compiled: Vec<_> = all_bonuses.iter().map(|b| compile_bonus::<f64>(b)).collect();

    // Apply to character
    let mut equipped_fork = base_resolver.fork();
    apply_compiled_bonuses(&mut equipped_fork, &all_compiled);

    let equipped_stats = equipped_fork.resolve_batch(&[hp_id.clone(), attack_id.clone()], &context)?;
    println!("Base stats: HP = 1000, Attack = 100");
    println!(
        "Equipped {} and {}: HP = {:.2}, Attack = {:.2}",
        sword.name,
        armor.name,
        equipped_stats[&hp_id].value.to_f64(),
        equipped_stats[&attack_id].value.to_f64()
    );

    Ok(())
}

