//! Example 03: Temporary Buffs & Debuffs (StatusManager)
//!
//! Demonstrates how `zzstat` handles temporary effects using the zero-cost
//! `.fork()` overlay system. Base stats are never mutated, and buffs are
//! applied dynamically.

use zzstat::bonus::Bonus;
use zzstat::context::StatContext;
use zzstat::resolver::StatResolver;
use zzstat::source::ConstantSource;
use zzstat::stat_id::StatId;
use zzstat::status::{StackBehavior, StatusEffect, StatusManager};
use zzstat::StatNumeric;

fn main() {
    let mut base_resolver = StatResolver::new();
    let def_id = StatId::from("DEF");

    // Base defense is 100
    base_resolver.register_source(def_id.clone(), Box::new(ConstantSource(100.0)));

    let mut status_manager = StatusManager::new();
    let ctx = StatContext::new();

    // 1. Check Initial Base Defense
    let initial_def = status_manager
        .get_active_resolver(&base_resolver)
        .resolve(&def_id, &ctx)
        .unwrap();
    println!("Initial DEF: {}", initial_def.value.to_f64()); // Expect: 100

    // 2. Create an Accumulating Debuff: "Sunder Armor" (-15 DEF per stack, max 3 stacks)
    let sunder_armor = StatusEffect {
        id: "SUNDER_ARMOR".to_string(),
        name: "Sunder Armor".to_string(),
        bonuses: vec![Bonus::add(def_id.clone())
            .flat(-15.0)
            .in_phase(zzstat::transform::TransformPhase::Additive)],
        max_stacks: 3,
        stack_behavior: StackBehavior::Accumulate {
            reset_duration: true,
        },
    };

    // 3. Apply Stack 1 (Duration: 3 ticks)
    println!("--- Applying Sunder Armor (Stack 1) ---");
    status_manager.add_status(sunder_armor.clone(), Some(3), 1);

    let def_1 = status_manager
        .get_active_resolver(&base_resolver)
        .resolve(&def_id, &ctx)
        .unwrap();
    println!("DEF after Stack 1: {}", def_1.value.to_f64()); // Expect: 85

    // 4. Apply Stack 2
    println!("--- Applying Sunder Armor (Stack 2) ---");
    status_manager.add_status(sunder_armor.clone(), Some(3), 1);

    let def_2 = status_manager
        .get_active_resolver(&base_resolver)
        .resolve(&def_id, &ctx)
        .unwrap();
    println!("DEF after Stack 2: {}", def_2.value.to_f64()); // Expect: 70

    // 5. Let it expire
    println!("--- Waiting 3 ticks for debuff to expire ---");
    status_manager.tick();
    status_manager.tick();
    status_manager.tick();

    let def_final = status_manager
        .get_active_resolver(&base_resolver)
        .resolve(&def_id, &ctx)
        .unwrap();
    println!("DEF after expiration: {}", def_final.value.to_f64()); // Expect: 100
}
