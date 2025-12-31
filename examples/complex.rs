//! Complex example: Multiple sources, transforms, and dependency chains
//!
//! This example demonstrates:
//! - Complex stat calculations
//! - Multiple sources per stat
//! - Transform chains
//! - Multi-level dependencies

use zzstat::source::ConstantSource;
use zzstat::transform::{ClampTransform, MultiplicativeTransform, ScalingTransform};
use zzstat::*;

fn main() -> Result<(), StatError> {
    let mut resolver = StatResolver::new();

    // Define a character's stats
    let str_id = StatId::from_str("STR");
    let dex_id = StatId::from_str("DEX");
    let int_id = StatId::from_str("INT");
    let hp_id = StatId::from_str("HP");
    let mp_id = StatId::from_str("MP");
    let atk_id = StatId::from_str("ATK");
    let crit_id = StatId::from_str("CRIT");
    let dps_id = StatId::from_str("DPS");

    println!("=== Character Stat System ===\n");

    // Base attributes
    println!("Base Attributes:");
    resolver.register_source(str_id.clone(), Box::new(ConstantSource(20.0)));
    println!("  STR: 20");

    resolver.register_source(dex_id.clone(), Box::new(ConstantSource(15.0)));
    println!("  DEX: 15");

    resolver.register_source(int_id.clone(), Box::new(ConstantSource(25.0)));
    println!("  INT: 25");

    // HP: Base + STR scaling + item bonuses
    println!("\nHP Calculation:");
    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    println!("  Base: 100");

    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));
    println!("  Item bonus: +50");

    resolver.register_transform(
        hp_id.clone(),
        Box::new(ScalingTransform::new(str_id.clone(), 5.0)),
    );
    println!("  STR scaling: +STR * 5");

    resolver.register_transform(hp_id.clone(), Box::new(MultiplicativeTransform::new(1.1)));
    println!("  Passive: +10%");

    // MP: Base + INT scaling
    println!("\nMP Calculation:");
    resolver.register_source(mp_id.clone(), Box::new(ConstantSource(50.0)));
    println!("  Base: 50");

    resolver.register_transform(
        mp_id.clone(),
        Box::new(ScalingTransform::new(int_id.clone(), 3.0)),
    );
    println!("  INT scaling: +INT * 3");

    // ATK: Base + STR scaling + DEX scaling
    println!("\nATK Calculation:");
    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(30.0)));
    println!("  Base: 30");

    resolver.register_transform(
        atk_id.clone(),
        Box::new(ScalingTransform::new(str_id.clone(), 2.0)),
    );
    println!("  STR scaling: +STR * 2");

    resolver.register_transform(
        atk_id.clone(),
        Box::new(ScalingTransform::new(dex_id.clone(), 1.0)),
    );
    println!("  DEX scaling: +DEX * 1");

    resolver.register_transform(atk_id.clone(), Box::new(ClampTransform::new(0.0, 200.0)));
    println!("  Clamp: [0, 200]");

    // CRIT: Base + DEX scaling
    println!("\nCRIT Calculation:");
    resolver.register_source(crit_id.clone(), Box::new(ConstantSource(5.0)));
    println!("  Base: 5");

    resolver.register_transform(
        crit_id.clone(),
        Box::new(ScalingTransform::new(dex_id.clone(), 2.0)),
    );
    println!("  DEX scaling: +DEX * 2");

    // DPS: Depends on ATK and CRIT
    println!("\nDPS Calculation:");
    resolver.register_source(dps_id.clone(), Box::new(ConstantSource(0.0)));
    println!("  Base: 0");

    resolver.register_transform(
        dps_id.clone(),
        Box::new(ScalingTransform::new(atk_id.clone(), 1.0)),
    );
    println!("  ATK contribution: +ATK * 1");

    resolver.register_transform(
        dps_id.clone(),
        Box::new(ScalingTransform::new(crit_id.clone(), 0.5)),
    );
    println!("  CRIT contribution: +CRIT * 0.5");

    let context = StatContext::new();

    println!("\n=== Resolving All Stats ===\n");
    let results = resolver.resolve_all(&context)?;

    // Display final stats
    println!("=== Final Character Stats ===\n");

    let stats = vec![
        ("STR", &str_id),
        ("DEX", &dex_id),
        ("INT", &int_id),
        ("HP", &hp_id),
        ("MP", &mp_id),
        ("ATK", &atk_id),
        ("CRIT", &crit_id),
        ("DPS", &dps_id),
    ];

    for (name, id) in stats {
        if let Some(resolved) = results.get(id) {
            println!("{}: {:.2}", name, resolved.value);
        }
    }

    println!("\n=== Detailed Breakdown ===\n");

    // Show detailed breakdown for DPS
    if let Some(dps) = results.get(&dps_id) {
        println!("DPS Breakdown:");
        println!("  Final Value: {:.2}", dps.value);

        println!("\n  Sources:");
        for (desc, value) in &dps.sources {
            println!("    {}: {:.2}", desc, value);
        }

        println!("\n  Transforms:");
        for (desc, value) in &dps.transforms {
            println!("    {}: {:.2}", desc, value);
        }
    }

    Ok(())
}
