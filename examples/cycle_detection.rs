//! Cycle detection example: Demonstrating error handling for circular dependencies
//!
//! This example shows:
//! - What happens when you create circular dependencies
//! - How the system detects and reports cycles

use zzstat::*;
use zzstat::source::ConstantSource;
use zzstat::transform::ScalingTransform;

fn main() {
    let mut resolver = StatResolver::new();
    
    // Create stats that depend on each other in a cycle
    let a_id = StatId::from_str("A");
    let b_id = StatId::from_str("B");
    let c_id = StatId::from_str("C");
    
    println!("=== Setting up circular dependencies ===\n");
    
    // A depends on B
    resolver.register_source(a_id.clone(), Box::new(ConstantSource(10.0)));
    resolver.register_transform(
        a_id.clone(),
        Box::new(ScalingTransform::new(b_id.clone(), 1.0)),
    );
    println!("A: 10 (base) + B * 1.0");
    
    // B depends on C
    resolver.register_source(b_id.clone(), Box::new(ConstantSource(20.0)));
    resolver.register_transform(
        b_id.clone(),
        Box::new(ScalingTransform::new(c_id.clone(), 1.0)),
    );
    println!("B: 20 (base) + C * 1.0");
    
    // C depends on A (creates cycle: A -> B -> C -> A)
    resolver.register_source(c_id.clone(), Box::new(ConstantSource(30.0)));
    resolver.register_transform(
        c_id.clone(),
        Box::new(ScalingTransform::new(a_id.clone(), 1.0)),
    );
    println!("C: 30 (base) + A * 1.0");
    
    println!("\n=== Attempting to resolve (should detect cycle) ===\n");
    
    let context = StatContext::new();
    match resolver.resolve(&a_id, &context) {
        Err(StatError::CycleDetected(cycle)) => {
            println!("✓ Cycle detected successfully!");
            println!("\nCycle path:");
            for (i, stat_id) in cycle.iter().enumerate() {
                if i < cycle.len() - 1 {
                    print!("{} -> ", stat_id);
                } else {
                    println!("{}", stat_id);
                }
            }
        }
        Err(e) => {
            println!("✗ Unexpected error: {}", e);
        }
        Ok(_) => {
            println!("✗ ERROR: Cycle was not detected! This should not happen.");
        }
    }
    
    println!("\n=== Valid dependency chain (no cycle) ===\n");
    
    // Reset resolver
    let mut resolver2 = StatResolver::new();
    
    let x_id = StatId::from_str("X");
    let y_id = StatId::from_str("Y");
    let z_id = StatId::from_str("Z");
    
    // Linear chain: X -> Y -> Z (no cycle)
    resolver2.register_source(x_id.clone(), Box::new(ConstantSource(10.0)));
    println!("X: 10 (base)");
    
    resolver2.register_source(y_id.clone(), Box::new(ConstantSource(20.0)));
    resolver2.register_transform(
        y_id.clone(),
        Box::new(ScalingTransform::new(x_id.clone(), 1.0)),
    );
    println!("Y: 20 (base) + X * 1.0");
    
    resolver2.register_source(z_id.clone(), Box::new(ConstantSource(30.0)));
    resolver2.register_transform(
        z_id.clone(),
        Box::new(ScalingTransform::new(y_id.clone(), 1.0)),
    );
    println!("Z: 30 (base) + Y * 1.0");
    
    println!("\n=== Resolving valid chain ===\n");
    
    let results = resolver2.resolve_all(&context).unwrap();
    
    println!("Results:");
    println!("  X: {:.2}", results[&x_id].value);
    println!("  Y: {:.2} (20 + 10)", results[&y_id].value);
    println!("  Z: {:.2} (30 + 30)", results[&z_id].value);
    
    println!("\n✓ Valid dependency chain resolved successfully!");
}

