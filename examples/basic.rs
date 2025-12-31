//! Basic example: Simple stat resolution with sources and transforms
//!
//! This example demonstrates:
//! - Registering stat sources (additive)
//! - Applying transforms
//! - Resolving stats

use zzstat::source::ConstantSource;
use zzstat::transform::MultiplicativeTransform;
use zzstat::*;

fn main() -> Result<(), StatError> {
    // Create a new stat resolver
    let mut resolver = StatResolver::new();

    // Define a stat ID
    let hp_id = StatId::from_str("HP");

    // Register multiple sources (they will be summed)
    println!("Registering sources for HP:");
    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    println!("  - Base HP: 100");

    resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));
    println!("  - Item bonus: +50");

    // Register a transform (percentage modifier)
    println!("\nRegistering transform:");
    resolver.register_transform(hp_id.clone(), Box::new(MultiplicativeTransform::new(1.2)));
    println!("  - 20% bonus multiplier");

    // Create context (empty for this example)
    let context = StatContext::new();

    // Resolve the stat
    println!("\nResolving HP...");
    let resolved = resolver.resolve(&hp_id, &context)?;

    // Display results
    println!("\n=== Resolved Stat ===");
    println!("Stat ID: {}", resolved.stat_id);
    println!("Final Value: {:.2}", resolved.value);

    println!("\nSource Breakdown:");
    for (desc, value) in &resolved.sources {
        println!("  {}: {:.2}", desc, value);
    }

    println!("\nTransform Breakdown:");
    for (desc, value) in &resolved.transforms {
        println!("  {}: {:.2}", desc, value);
    }

    println!("\nCalculation: (100 + 50) * 1.2 = {:.2}", resolved.value);

    Ok(())
}
