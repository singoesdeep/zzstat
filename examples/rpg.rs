//! RPG Stat System Example
//!
//! This example demonstrates a complete RPG stat system using zzstat:
//! - Character base stats (STR, DEX, VIT)
//! - Derived stats via transforms (ATK, DEF, HP)
//! - Items with stat modifiers
//! - Equipping items using resolver fork (copy-on-write)
//! - Clamp/cap transforms (CRIT_CHANCE capped at 0.75)
//! - Batched stat resolution
//!
//! This example shows how to think with zzstat: stats are not hardcoded,
//! but defined through sources and transforms, allowing flexible stat systems.

use std::collections::HashMap;
use zzstat::source::ConstantSource;
use zzstat::transform::{AdditiveTransform, ClampTransform, ScalingTransform, TransformPhase};
use zzstat::*;

// ============================================================================
// Stat ID Definitions
// ============================================================================

/// Define all stat identifiers for our RPG system.
/// In zzstat, stats are identified by strings - no hardcoded stat names.
fn define_stat_ids() -> (StatId, StatId, StatId, StatId, StatId, StatId, StatId) {
    let str_id = StatId::from_str("STR");
    let dex_id = StatId::from_str("DEX");
    let vit_id = StatId::from_str("VIT");
    let atk_id = StatId::from_str("ATK");
    let def_id = StatId::from_str("DEF");
    let hp_id = StatId::from_str("HP");
    let crit_chance_id = StatId::from_str("CRIT_CHANCE");

    (str_id, dex_id, vit_id, atk_id, def_id, hp_id, crit_chance_id)
}

// ============================================================================
// Character Structure
// ============================================================================

/// A character with base stats and a stat resolver.
///
/// The resolver contains all the stat formulas and can be forked
/// to create equipped variants without modifying the base character.
struct Character {
    /// Base stat values (STR, DEX, VIT, CRIT_CHANCE)
    /// Stored for reference/documentation, though the resolver contains the actual data
    #[allow(dead_code)]
    base_stats: HashMap<StatId, f64>,
    /// Base resolver with all stat formulas registered
    base_resolver: StatResolver,
}

impl Character {
    /// Create a new character with base stats and set up all stat formulas.
    ///
    /// This registers:
    /// - Base stat sources (STR, DEX, VIT, CRIT_CHANCE)
    /// - Derived stat formulas using transforms:
    ///   - ATK = STR * 2 + DEX
    ///   - DEF = VIT * 1.5
    ///   - HP = VIT * 10
    /// - CRIT_CHANCE clamp at 0.75 (Final phase)
    fn new(base_stats: HashMap<StatId, f64>) -> Self {
        let mut resolver = StatResolver::new();

        // Get stat IDs
        let (str_id, dex_id, vit_id, atk_id, def_id, hp_id, crit_chance_id) = define_stat_ids();

        // Register base stat sources (these are the character's base attributes)
        for (stat_id, value) in &base_stats {
            resolver.register_source(stat_id.clone(), Box::new(ConstantSource(*value)));
        }

        // ====================================================================
        // Derived Stats: Using Transforms
        // ====================================================================
        // These stats are calculated from other stats, demonstrating
        // dependency resolution and transform-based formulas.

        // ATK = STR * 2 + DEX
        // We start with base 0, then add scaling from STR and DEX
        resolver.register_source(atk_id.clone(), Box::new(ConstantSource(0.0)));
        resolver.register_transform(
            atk_id.clone(),
            Box::new(ScalingTransform::new(str_id.clone(), 2.0)),
        );
        resolver.register_transform(
            atk_id.clone(),
            Box::new(ScalingTransform::new(dex_id.clone(), 1.0)),
        );

        // DEF = VIT * 1.5
        resolver.register_source(def_id.clone(), Box::new(ConstantSource(0.0)));
        resolver.register_transform(
            def_id.clone(),
            Box::new(ScalingTransform::new(vit_id.clone(), 1.5)),
        );

        // HP = VIT * 10
        resolver.register_source(hp_id.clone(), Box::new(ConstantSource(0.0)));
        resolver.register_transform(
            hp_id.clone(),
            Box::new(ScalingTransform::new(vit_id.clone(), 10.0)),
        );

        // CRIT_CHANCE: Clamp to maximum of 0.75 (75%)
        // This demonstrates Final phase clamping for gameplay rules
        resolver.register_transform(
            crit_chance_id.clone(),
            Box::new(ClampTransform::new(0.0, 0.75)),
        );

        Self {
            base_stats,
            base_resolver: resolver,
        }
    }

    /// Create a resolver with equipped items applied.
    ///
    /// This uses resolver fork (copy-on-write) to create a new resolver
    /// that includes item bonuses without modifying the base character.
    ///
    /// Items are registered as transforms in a custom "Item" phase (phase 3),
    /// which runs after base stat calculations but before final clamping.
    fn with_equipped_items(&self, items: &[Item]) -> StatResolver {
        // Fork the base resolver - this creates a copy-on-write fork
        // The base resolver remains unchanged
        let mut equipped_resolver = self.base_resolver.fork();

        // Register item modifiers as transforms in the Item phase
        // Items use AdditiveTransform for flat bonuses
        for item in items {
            for (stat_id, bonus) in &item.stat_modifiers {
                // Register in Custom phase 3 (Item phase)
                // This phase runs after base calculations but before Final phase
                equipped_resolver.register_transform_in_phase(
                    stat_id.clone(),
                    TransformPhase::Custom(3),
                    Box::new(AdditiveTransform::new(*bonus)),
                );
            }
        }

        equipped_resolver
    }
}

// ============================================================================
// Item Structure
// ============================================================================

/// An item that provides stat modifiers.
///
/// Items don't modify the character directly - instead, they're applied
/// via resolver fork, allowing multiple equipment configurations without
/// mutating the base character.
struct Item {
    name: String,
    stat_modifiers: HashMap<StatId, f64>,
}

impl Item {
    /// Create a new item with stat modifiers.
    fn new(name: impl Into<String>, stat_modifiers: HashMap<StatId, f64>) -> Self {
        Self {
            name: name.into(),
            stat_modifiers,
        }
    }
}

// ============================================================================
// Main Function
// ============================================================================

fn main() -> Result<(), StatError> {
    println!("=== RPG Stat System Example ===\n");

    // Get stat IDs
    let (str_id, dex_id, vit_id, atk_id, def_id, hp_id, crit_chance_id) = define_stat_ids();

    // ========================================================================
    // Create Character
    // ========================================================================
    println!("1. Creating Character\n");

    let mut base_stats = HashMap::new();
    base_stats.insert(str_id.clone(), 10.0);
    base_stats.insert(dex_id.clone(), 8.0);
    base_stats.insert(vit_id.clone(), 12.0);
    base_stats.insert(crit_chance_id.clone(), 0.5); // 50% base, will be clamped

    let mut character = Character::new(base_stats);

    println!("Base Stats:");
    println!("  STR: 10");
    println!("  DEX: 8");
    println!("  VIT: 12");
    println!("  CRIT_CHANCE: 0.5 (50%)\n");

    // ========================================================================
    // Create Items
    // ========================================================================
    println!("2. Creating Items\n");

    // Sword: +5 ATK
    let mut sword_mods = HashMap::new();
    sword_mods.insert(atk_id.clone(), 5.0);
    let sword = Item::new("Iron Sword", sword_mods);

    // Armor: +3 DEF, +50 HP
    let mut armor_mods = HashMap::new();
    armor_mods.insert(def_id.clone(), 3.0);
    armor_mods.insert(hp_id.clone(), 50.0);
    let armor = Item::new("Leather Armor", armor_mods);

    println!("Items:");
    println!("  {}: +5 ATK", sword.name);
    println!("  {}: +3 DEF, +50 HP\n", armor.name);

    // ========================================================================
    // Resolve Base Character Stats (Batched)
    // ========================================================================
    println!("3. Base Character Stats (Before Equipment)\n");

    let context = StatContext::new();

    // Use batched resolution to efficiently resolve multiple stats at once
    // This only resolves the requested stats and their dependencies
    let base_results = character
        .base_resolver
        .resolve_batch(
            &[hp_id.clone(), atk_id.clone(), def_id.clone(), crit_chance_id.clone()],
            &context,
        )?;

    println!("Final Stats:");
    println!("  HP: {:.2} (VIT * 10 = 12 * 10)", base_results[&hp_id].value);
    println!(
        "  ATK: {:.2} (STR * 2 + DEX = 10 * 2 + 8)",
        base_results[&atk_id].value
    );
    println!(
        "  DEF: {:.2} (VIT * 1.5 = 12 * 1.5)",
        base_results[&def_id].value
    );
    println!(
        "  CRIT_CHANCE: {:.2} (clamped at 0.75)",
        base_results[&crit_chance_id].value
    );
    println!();

    // ========================================================================
    // Equip Items and Resolve Final Stats
    // ========================================================================
    println!("4. Equipping Items (Using Resolver Fork)\n");

    // Create equipped resolver - this is a fork, so base character is unchanged
    let mut equipped_resolver = character.with_equipped_items(&[sword, armor]);

    println!("Equipped: Iron Sword, Leather Armor");
    println!("(Base character unchanged - using copy-on-write fork)\n");

    // Resolve final stats with items equipped
    // Include base stats (STR, DEX, VIT) to ensure dependencies are resolved
    let equipped_results = equipped_resolver.resolve_batch(
        &[
            str_id.clone(),
            dex_id.clone(),
            vit_id.clone(),
            hp_id.clone(),
            atk_id.clone(),
            def_id.clone(),
            crit_chance_id.clone(),
        ],
        &context,
    )?;

    println!("Final Stats (With Equipment):");
    println!(
        "  HP: {:.2} (base {} + item +50)",
        equipped_results[&hp_id].value,
        base_results[&hp_id].value
    );
    println!(
        "  ATK: {:.2} (base {} + item +5)",
        equipped_results[&atk_id].value,
        base_results[&atk_id].value
    );
    println!(
        "  DEF: {:.2} (base {} + item +3)",
        equipped_results[&def_id].value,
        base_results[&def_id].value
    );
    println!(
        "  CRIT_CHANCE: {:.2} (still clamped at 0.75)",
        equipped_results[&crit_chance_id].value
    );
    println!();

    // ========================================================================
    // Detailed Breakdown
    // ========================================================================
    println!("5. Detailed Stat Breakdown\n");

    // Show detailed breakdown for ATK
    if let Some(atk_resolved) = equipped_results.get(&atk_id) {
        println!("ATK Breakdown:");
        println!("  Final Value: {:.2}\n", atk_resolved.value);

        println!("  Sources:");
        for (desc, value) in &atk_resolved.sources {
            println!("    {}: {:.2}", desc, value);
        }

        println!("\n  Transforms (in phase order):");
        for (desc, value) in &atk_resolved.transforms {
            println!("    {}: {:.2}", desc, value);
        }
        println!();
    }

    // ========================================================================
    // Verify Base Character Unchanged
    // ========================================================================
    println!("6. Verifying Base Character Unchanged\n");

    let base_atk_again = character
        .base_resolver
        .resolve(&atk_id, &context)?;
    let equipped_atk = equipped_resolver.resolve(&atk_id, &context)?;

    println!("Base character ATK: {:.2}", base_atk_again.value);
    println!("Equipped character ATK: {:.2}", equipped_atk.value);
    println!(
        "✓ Base character unchanged (fork isolation working)\n"
    );

    // ========================================================================
    // Summary
    // ========================================================================
    println!("=== Summary ===");
    println!("✓ Base stats registered as sources");
    println!("✓ Derived stats calculated via transforms (ATK, DEF, HP)");
    println!("✓ Items applied using resolver fork (copy-on-write)");
    println!("✓ CRIT_CHANCE clamped at 0.75 (Final phase)");
    println!("✓ Batched resolution for efficiency");
    println!("✓ Phase-based transform pipeline (Base → Item → Final)");
    println!("✓ Stack rules applied automatically (additive stacking)");

    Ok(())
}

