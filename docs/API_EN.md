# zzstat - Complete API Documentation

Welcome to the **zzstat** API documentation! This guide provides detailed examples on how to use the core modules of the engine to build MMORPG or RPG stat systems.

## Table of Contents
1. [Core Concepts](#1-core-concepts)
2. [StatResolver & Registration](#2-statresolver--registration)
3. [BonusAction API (Items & Modifiers)](#3-bonusaction-api)
4. [ResourcePool (HP, Mana & DoT/HoT)](#4-resourcepool)
5. [StatusEffectManager (Buffs, Debuffs & Triggers)](#5-statuseffectmanager)
6. [Combat Engine & Bytecode VM](#6-combat-engine--bytecode-vm)
7. [Hierarchical Environments](#7-hierarchical-environments)

---

## 1. Core Concepts

At the heart of `zzstat` are three main structures:
- `StatId`: A unique string wrapper that represents a stat (e.g., `"MAX_HP"`, `"STR"`, `"ATK"`). Constructed using `StatId::from("STR")`.
- `StatValue`: An alias for the raw numeric value (either `f64` or a fixed-point decimal if the `fixed-point` feature is enabled).
- `StatContext`: A key-value storage used to evaluate conditional values and dynamic calculations.

```rust
use zzstat::{StatId, StatContext};

let atk_id = StatId::from("ATK");
let def_id = StatId::from("DEF");

let mut ctx = StatContext::new();
ctx.set("STANCE", "DEFENSIVE");
```

---

## 2. StatResolver & Registration

The `StatResolver` manages sources, transforms, dependency graphs, and caching. 

### Example: Defining Base Stats and Dependencies
```rust
use zzstat::resolver::StatResolver;
use zzstat::source::ConstantSource;
use zzstat::transform::standard::ScalingTransform;
use zzstat::{StatId, StatContext};

let mut resolver = StatResolver::new();

// 1. Give character 50 Base STR
let str_id = StatId::from("STR");
resolver.register_source(str_id.clone(), Box::new(ConstantSource(50.0)));

// 2. Define ATK. It has no base source, it only scales with STR! (1 STR = 2 ATK)
let atk_id = StatId::from("ATK");
let str_to_atk = ScalingTransform::new(str_id.clone(), 2.0);
resolver.register_transform(atk_id.clone(), Box::new(str_to_atk));

// 3. Resolve the values
let ctx = StatContext::new();
let resolved_str = resolver.resolve(&str_id, &ctx).unwrap(); // 50.0
let resolved_atk = resolver.resolve(&atk_id, &ctx).unwrap(); // 100.0 (50 * 2)

println!("STR: {}, ATK: {}", resolved_str.value.to_f64(), resolved_atk.value.to_f64());
```

---

## 3. BonusAction API

The `Bonus` structure provides a safe, builder-style declarative way to define modifiers (e.g., equipment stats, temporary buffs) and compile them into transforms.

### Available Actions:
- `Bonus::add(target).flat(value)`: Adds a flat amount.
- `Bonus::scale(target, source).factor(value)`: Scales the target stat based on another stat.
- `Bonus::mul(target).percent(value)`: Applies a multiplicative increase (e.g. `0.20` for +20%).
- `Bonus::r#override(target, value)`: Overrides the stat value.
- `Bonus::clamp_min(target, value)`: Clamps the stat to a minimum value.
- `Bonus::clamp_max(target, value)`: Clamps the stat to a maximum value.

### Conditional Bonuses
You can chain `.with_condition(condition)` to make any bonus conditional on context state.

```rust
use zzstat::bonus::{Bonus, compile_bonus, apply_compiled_bonus};
use zzstat::transform::TransformPhase;
use zzstat::condition::ConditionDef;

// 1. Define a conditional bonus (+50 DEF only when in DEFENSIVE stance)
let condition = ConditionDef::Equals {
    key: "STANCE".to_string(),
    value: serde_json::json!("DEFENSIVE"),
};

let bonus = Bonus::add(StatId::from("DEF"))
    .flat(50.0)
    .in_phase(TransformPhase::Additive)
    .with_condition(condition);

// 2. Compile and apply
let compiled = compile_bonus::<f64>(&bonus);
let mut fork = resolver.fork();
apply_compiled_bonus(&mut fork, &compiled);
```

---

## 4. ResourcePool

Use `ResourcePool` to track stateful vitals like HP and MP. It automatically binds to a maximum limit (e.g., `MAX_HP` from the resolver) and correctly clamps values when max stats drop.

```rust
use zzstat::resource::{ResourcePool, TimeEffect, ThresholdTrigger, TriggerCondition};

// Create an HP pool linked to the MAX_HP stat
let max_hp_id = StatId::from("MAX_HP");
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

## 5. StatusEffectManager

`StatusEffectManager` manages active status effects and provides an `O(1)` copy-on-write `fork()` mechanism. It overlay-applies temporary effects without modifying the base stats resolver.

### Event-Driven Reactive Triggers
You can register `EffectTrigger`s that apply `StatusEffect`s automatically when specific game events occur under custom conditions (e.g. low HP).

```rust
use zzstat::status_effect::{StatusEffectManager, StatusEffect, EffectTrigger, StackBehavior};
use zzstat::bonus::Bonus;
use zzstat::transform::TransformPhase;

let mut manager = StatusEffectManager::new();

// 1. Create a warcry effect (+50 ATK)
let warcry = StatusEffect {
    id: "WARCRY".to_string(),
    name: "Warcry".to_string(),
    bonuses: vec![Bonus::add(StatId::from("ATK"))
        .flat(50.0)
        .in_phase(TransformPhase::Additive)],
    max_stacks: 1,
    stack_behavior: StackBehavior::Refresh,
};

// 2. Register a trigger: apply warcry on event "on_combat_start"
let trigger = EffectTrigger {
    event: "on_combat_start".to_string(),
    condition: None,
    effect: warcry,
    duration_ticks: Some(5),
    stacks: 1,
};
manager.register_trigger(trigger);

// 3. Fire event
let ctx = StatContext::new();
manager.fire_event("on_combat_start", &ctx);

// 4. Get active resolver to retrieve buffed stats
let mut active_resolver = manager.get_active_resolver(&base_resolver);
let resolved = active_resolver.resolve(&StatId::from("ATK"), &ctx).unwrap();
```

---

## 6. Combat Engine & Bytecode VM

The `CombatEngine` evaluates AST-based combat formulas. You can evaluate formulas recursively on the AST or pre-compile them into flat bytecode to run them inside a stack-based VM for maximum speed.

### Compiling and Running in the VM
```rust
use zzstat::combat::{CombatEngine, CombatFormula, CombatExpression, CombatTarget};

// 1. Define formula (usually parsed from a JSON file)
let formula = CombatFormula {
    name: "Crit Hit".to_string(),
    expression: CombatExpression::Multiply {
        left: Box::new(CombatExpression::Stat {
            target: CombatTarget::Attacker,
            stat: "ATK".to_string(),
        }),
        right: Box::new(CombatExpression::Constant { value: 2.0 }),
    },
};

// 2. Compile formula into flat bytecode
let compiled = formula.compile();

// 3. Run inside the stack VM
let mut rng = || 0.5; // RNG closure
let damage = CombatEngine::evaluate_compiled(
    &compiled,
    &mut attacker_resolver,
    &attacker_ctx,
    &mut defender_resolver,
    &defender_ctx,
    &mut rng,
).unwrap();

println!("Dealt {} damage!", damage);
```

---

## 7. Hierarchical Environments

The environment system allows multiple resolvers to be chained in a parent-child hierarchy (e.g., `Weather -> Zone -> Party -> Character`) using `.fork()`.

Any modifers registered on parent resolvers are inherited by child resolvers. If the parent resolver is updated dynamically, the child automatically resolves the updated values.

```rust
// 1. Weather Resolver (Base layer)
let mut weather = StatResolver::new();
weather.register_source(StatId::from("HP"), Box::new(ConstantSource(100.0)));

// 2. Zone Resolver (forked from weather)
let mut zone = weather.fork();
zone.register_transform(StatId::from("ATK"), Box::new(MultiplicativeTransform::new(1.2)));

// 3. Character Resolver (forked from zone)
let mut character = zone.fork();
character.register_source(StatId::from("HP"), Box::new(ConstantSource(50.0)));

// Resolve stats (inherits all parent modifiers!)
let ctx = StatContext::new();
let resolved_hp = character.resolve(&StatId::from("HP"), &ctx).unwrap(); // 100 + 50 = 150
```
