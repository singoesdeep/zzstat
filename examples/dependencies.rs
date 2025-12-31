//! Dependencies example: Stats that depend on other stats
//!
//! This example demonstrates:
//! - Creating derived stats
//! - Dependency chains
//! - Automatic resolution order

use zzstat::source::ConstantSource;
use zzstat::transform::ScalingTransform;
use zzstat::*;

fn main() -> Result<(), StatError> {
    let mut resolver = StatResolver::new();

    // Define stat IDs
    let str_id = StatId::from_str("STR");
    let dex_id = StatId::from_str("DEX");
    let atk_id = StatId::from_str("ATK");
    let crit_id = StatId::from_str("CRIT");

    println!("=== Setting up base stats ===");

    // Base stats (no dependencies)
    resolver.register_source(str_id.clone(), Box::new(ConstantSource(10.0)));
    println!("STR: 10 (base)");

    resolver.register_source(dex_id.clone(), Box::new(ConstantSource(15.0)));
    println!("DEX: 15 (base)");

    println!("\n=== Setting up derived stats ===");

    // ATK depends on STR
    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(50.0)));
    resolver.register_transform(
        atk_id.clone(),
        Box::new(ScalingTransform::new(str_id.clone(), 2.0)),
    );
    println!("ATK: 50 (base) + STR * 2.0");

    // CRIT depends on DEX
    resolver.register_source(crit_id.clone(), Box::new(ConstantSource(5.0)));
    resolver.register_transform(
        crit_id.clone(),
        Box::new(ScalingTransform::new(dex_id.clone(), 1.5)),
    );
    println!("CRIT: 5 (base) + DEX * 1.5");

    let context = StatContext::new();

    println!("\n=== Resolving all stats ===");
    let results = resolver.resolve_all(&context)?;

    // Display results
    println!("\n=== Results ===");
    for (stat_id, resolved) in &results {
        println!("\n{}:", stat_id);
        println!("  Final Value: {:.2}", resolved.value);

        if !resolved.sources.is_empty() {
            println!("  Sources:");
            for (desc, value) in &resolved.sources {
                println!("    {}: {:.2}", desc, value);
            }
        }

        if !resolved.transforms.is_empty() {
            println!("  Transforms:");
            for (desc, value) in &resolved.transforms {
                println!("    {}: {:.2}", desc, value);
            }
        }
    }

    println!("\n=== Verification ===");
    println!("STR: {:.2} (expected: 10.00)", results[&str_id].value);
    println!("DEX: {:.2} (expected: 15.00)", results[&dex_id].value);
    println!(
        "ATK: {:.2} (expected: 70.00 = 50 + 10*2)",
        results[&atk_id].value
    );
    println!(
        "CRIT: {:.2} (expected: 27.50 = 5 + 15*1.5)",
        results[&crit_id].value
    );

    Ok(())
}
