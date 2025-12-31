//! Context example: Using StatContext for conditional stat calculations
//!
//! This example demonstrates:
//! - Using StatContext to pass game state
//! - Conditional transforms based on context
//! - Different stat values in different contexts

use zzstat::*;
use zzstat::source::ConstantSource;
use zzstat::transform::{ConditionalTransform, MultiplicativeTransform};

fn main() -> Result<(), StatError> {
    let mut resolver = StatResolver::new();
    
    let atk_id = StatId::from_str("ATK");
    let def_id = StatId::from_str("DEF");
    
    println!("=== Setting up stats with context-dependent transforms ===\n");
    
    // Base ATK
    resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));
    println!("ATK base: 100");
    
    // ATK gets +50% bonus in combat
    let combat_bonus = ConditionalTransform::new(
        |ctx| ctx.get::<bool>("in_combat").unwrap_or(false),
        Box::new(MultiplicativeTransform::new(1.5)),
        "Combat bonus +50%",
    );
    resolver.register_transform(atk_id.clone(), Box::new(combat_bonus));
    println!("ATK: +50% when in combat");
    
    // Base DEF
    resolver.register_source(def_id.clone(), Box::new(ConstantSource(80.0)));
    println!("DEF base: 80");
    
    // DEF gets +25% bonus in PvP zones
    let pvp_bonus = ConditionalTransform::new(
        |ctx| ctx.get::<String>("zone_type").map(|z| z == "pvp").unwrap_or(false),
        Box::new(MultiplicativeTransform::new(1.25)),
        "PvP zone bonus +25%",
    );
    resolver.register_transform(def_id.clone(), Box::new(pvp_bonus));
    println!("DEF: +25% in PvP zones");
    
    // Scenario 1: Out of combat, normal zone
    println!("\n=== Scenario 1: Out of combat, normal zone ===");
    let mut context1 = StatContext::new();
    context1.set("in_combat", false);
    context1.set("zone_type", "normal");
    
    let atk1 = resolver.resolve(&atk_id, &context1)?;
    let def1 = resolver.resolve(&def_id, &context1)?;
    
    println!("ATK: {:.2} (no bonuses)", atk1.value);
    println!("DEF: {:.2} (no bonuses)", def1.value);
    
    // Scenario 2: In combat, normal zone
    println!("\n=== Scenario 2: In combat, normal zone ===");
    let mut context2 = StatContext::new();
    context2.set("in_combat", true);
    context2.set("zone_type", "normal");
    
    // Invalidate cache to force recalculation
    resolver.invalidate(&atk_id);
    resolver.invalidate(&def_id);
    
    let atk2 = resolver.resolve(&atk_id, &context2)?;
    let def2 = resolver.resolve(&def_id, &context2)?;
    
    println!("ATK: {:.2} (combat bonus: 100 * 1.5)", atk2.value);
    println!("DEF: {:.2} (no bonuses)", def2.value);
    
    // Scenario 3: Out of combat, PvP zone
    println!("\n=== Scenario 3: Out of combat, PvP zone ===");
    let mut context3 = StatContext::new();
    context3.set("in_combat", false);
    context3.set("zone_type", "pvp");
    
    resolver.invalidate(&atk_id);
    resolver.invalidate(&def_id);
    
    let atk3 = resolver.resolve(&atk_id, &context3)?;
    let def3 = resolver.resolve(&def_id, &context3)?;
    
    println!("ATK: {:.2} (no bonuses)", atk3.value);
    println!("DEF: {:.2} (PvP bonus: 80 * 1.25)", def3.value);
    
    // Scenario 4: In combat, PvP zone
    println!("\n=== Scenario 4: In combat, PvP zone ===");
    let mut context4 = StatContext::new();
    context4.set("in_combat", true);
    context4.set("zone_type", "pvp");
    
    resolver.invalidate(&atk_id);
    resolver.invalidate(&def_id);
    
    let atk4 = resolver.resolve(&atk_id, &context4)?;
    let def4 = resolver.resolve(&def_id, &context4)?;
    
    println!("ATK: {:.2} (combat bonus: 100 * 1.5)", atk4.value);
    println!("DEF: {:.2} (PvP bonus: 80 * 1.25)", def4.value);
    
    println!("\n=== Summary ===");
    println!("Context allows stats to vary based on game state:");
    println!("  - Combat state affects ATK");
    println!("  - Zone type affects DEF");
    println!("  - Stats are recalculated when context changes");
    
    Ok(())
}

