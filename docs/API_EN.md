# zzstat - Complete API Documentation

Welcome to the **zzstat** API documentation! This guide provides detailed examples on how to use the core modules of the engine to build MMORPG or RPG stat systems.

## Table of Contents
1. [Core Concepts](#1-core-concepts)
2. [StatRegistry & StatResolver](#2-statregistry--statresolver)
3. [BonusAction API (Items & Modifiers)](#3-bonusaction-api)
4. [ResourcePool (HP, Mana & Clamping)](#4-resourcepool)
5. [StatusManager (Buffs & Debuffs)](#5-statusmanager)
6. [Combat Engine (Formulas & AST)](#6-combat-engine)

---

## 1. Core Concepts

At the heart of `zzstat` are three main identifiers:
- `StatId`: A unique string wrapper that represents a stat (e.g., `"MAX_HP"`, `"STR"`, `"ATK"`).
- `StatValue`: An alias for `f64` (or fixed-point if enabled) representing the raw number.
- `StatContext`: An empty struct placeholder for future contextual resolution (like environment variables).

```rust
use zzstat::{StatId, StatValue, StatContext};

let atk_id = StatId::new("ATK");
let def_id = StatId::new("DEF");
let ctx = StatContext;
```

---

## 2. StatRegistry & StatResolver

The engine uses a separated data-logic architecture. 
- **`StatRegistry`**: Stores the raw definitions (Base sources, Transforms).
- **`StatResolver`**: Wraps the registry and executes the math, resolving dependencies dynamically and caching the results.

### Example: Defining Base Stats and Dependencies
```rust
use zzstat::registry::StatRegistry;
use zzstat::resolver::StatResolver;
use zzstat::source::ConstantSource;
use zzstat::transform::standard::ScalingTransform;
use zzstat::transform::core::{TransformEntry, StackRule, TransformPhase};
use zzstat::{StatId, StatContext};
use std::sync::Arc;

let mut registry = StatRegistry::new();

// 1. Give character 50 Base STR
let str_id = StatId::new("STR");
registry.add_source(str_id.clone(), Box::new(ConstantSource::new(50.0)));

// 2. Define ATK. It has no base source, it only scales with STR!
let atk_id = StatId::new("ATK");
let str_to_atk = ScalingTransform::new(str_id.clone(), 2.0); // 1 STR = 2 ATK

registry.add_transform(
    atk_id.clone(),
    TransformEntry::new(
        TransformPhase::Base, 
        StackRule::Additive, 
        Box::new(str_to_atk)
    )
);

// 3. Resolve the values
let mut resolver = StatResolver::new(registry);
let resolved_str = resolver.resolve(&str_id, &StatContext); // 50.0
let resolved_atk = resolver.resolve(&atk_id, &StatContext); // 100.0 (50 * 2)

println!("STR: {}, ATK: {}", resolved_str, resolved_atk);
```

---

## 3. BonusAction API

The `BonusAction` enum is the safest and most idiomatic way to handle Item modifiers. Instead of dealing with complex `TransformEntry` structures, you can create bonuses using helper methods and "compile" them into the resolver.

### Available Actions:
- `Bonus::add_flat()`: Adds a flat amount (+50 HP).
- `Bonus::scale()`: Multiplies by a stat dependency (+50% of STR).
- `Bonus::multiply()`: Multiplies the base value (+20% Overall ATK).
- `Bonus::override_value()`: Hard overrides the stat to a specific number.

### Example: Wearing a Sword
```rust
use zzstat::bonus::{Bonus, apply_compiled_bonus, compile_bonus};

// Define an item's stats
let sword_bonuses = vec![
    Bonus::add_flat(StatId::new("ATK"), 120.0),      // +120 ATK
    Bonus::multiply(StatId::new("ATK"), 0.15),       // +15% Total ATK
];

// Compile and apply the bonuses to the resolver
for bonus in sword_bonuses {
    let compiled = compile_bonus::<f64>(&bonus);
    apply_compiled_bonus(&mut resolver, &compiled);
}

// ATK is now updated!
```

---

## 4. ResourcePool

Use `ResourcePool` to track stateful vitals like HP and MP. It automatically binds to a maximum limit (e.g., `MAX_HP` from the resolver) and correctly clamps values when max stats drop.

```rust
use zzstat::resource::{ResourcePool, TimeEffect, ThresholdTrigger, TriggerCondition};

// Create an HP pool linked to the MAX_HP stat
let max_hp_id = StatId::new("MAX_HP");
let mut hp_pool = ResourcePool::new(max_hp_id.clone());

// Full heal at start
hp_pool.fill(&resolver, &ctx);

// Apply a Poison DoT (-20 HP per tick for 3 ticks)
hp_pool.add_effect(TimeEffect {
    name: "Poison".to_string(),
    amount_per_tick: -20.0,
    ticks_remaining: 3,
});

// Add a Death Trigger
hp_pool.add_trigger(ThresholdTrigger {
    condition: TriggerCondition::Empty,
    event_name: "DEATH".to_string(),
});

// Run the game loop
for _ in 0..3 {
    let events = hp_pool.tick(&resolver, &ctx);
    println!("Current HP: {}", hp_pool.current_value());
    
    if events.contains(&"DEATH".to_string()) {
        println!("Character died!");
    }
}
```

---

## 5. StatusManager

StatusManager provides an `O(1)` copy-on-write `fork()` mechanism. This allows you to apply temporary buffs to a character without mutating their base `StatResolver`!

```rust
use zzstat::status::{StatusManager, StatusEffect, StackBehavior};

let mut status_manager = StatusManager::new();

// Create a buff
let warcry = StatusEffect {
    id: "WARCRY".to_string(),
    name: "Warcry (+50 ATK)".to_string(),
    bonuses: vec![Bonus::add_flat(StatId::new("ATK"), 50.0)],
    max_stacks: 1,
    stack_behavior: StackBehavior::Refresh,
};

// Apply buff for 5 ticks, with 1 stack
status_manager.add_status(warcry, Some(5), 1);

// Generate a forked resolver containing the base stats + buff stats
let mut active_resolver = status_manager.get_active_resolver(&base_resolver);

// This resolution includes the +50 ATK!
let buffed_atk = active_resolver.resolve(&StatId::new("ATK"), &StatContext);

// In your game loop:
status_manager.tick(); // Reduces duration, removes buff when it expires
```

---

## 6. Combat Engine

A fully decoupled AST (Abstract Syntax Tree) for executing combat formulas and attack skills based on JSON definitions.

```rust
use zzstat::combat::{CombatEngine, Node, CombatContext};

// An AST that subtracts Defender's DEF from Attacker's ATK
let damage_formula = Node::Subtract(
    Box::new(Node::Stat { target: "attacker".to_string(), stat: "ATK".to_string() }),
    Box::new(Node::Stat { target: "defender".to_string(), stat: "DEF".to_string() }),
);

// We need an attacker and a defender resolver
let mut combat = CombatEngine::new(damage_formula);
let mut combat_ctx = CombatContext::new(&attacker_resolver);
combat_ctx.add_target("defender", &defender_resolver);

// Calculate final damage (Deterministically! No RNG involved during evaluation)
let mut rand_generator = || 0.5; // Mock random number generator (for testing chances)
let damage = combat.evaluate(&combat_ctx, &mut rand_generator).unwrap();

println!("Dealt {} damage!", damage);
```
