# zzstat

A deterministic stat calculation engine designed for RPG and MMORPG servers. zzstat provides pure math-based stat resolution with zero I/O, no async primitives, and no persistence layer dependencies.

## What is zzstat?

zzstat is a pure stat math engine. It calculates stat values deterministically from sources and transforms. The engine handles dependency resolution, phase ordering, stack rules, and caching, but does not handle inventory, equipment management, database operations, or game logic.

### What zzstat is NOT

- **Not an inventory system**: zzstat does not track items, equipment slots, or inventory state
- **Not a game engine**: zzstat has no game loop, rendering, or input handling
- **Not an ECS**: zzstat does not manage entities, components, or systems
- **Not a persistence layer**: zzstat has no database integration, serialization formats, or file I/O
- **Not async**: zzstat is synchronous and blocking by design

This focus on pure stat math makes zzstat suitable for integration into any RPG backend architecture, whether it uses Redis, PostgreSQL, or custom storage solutions.

## Core Concepts

### Stat

A stat represents a numeric attribute of a game entity (character, item, etc.). In RPG terms, this could be HP, ATTACK, DEFENSE, STRENGTH, or any calculated value. Stats are identified by string IDs, not hardcoded enums.

**Why**: Hardcoding stat names locks you into a specific design. String-based stat IDs allow game designers to define stats without code changes.

**When to use**: Define one stat ID per distinct numeric attribute that affects gameplay calculations.

### Derived Stat

A derived stat is calculated from other stats. For example, HP might be derived from VITALITY, or ATTACK from STRENGTH and DEXTERITY. Derived stats use transforms that depend on other stats.

**Why**: RPG systems commonly express stat relationships (e.g., "HP = VIT × 10"). Derived stats encode these formulas declaratively.

**When to use**: When a stat's value depends on other stats, use a derived stat with a scaling or dependency transform.

### Bonus / Modifier

A bonus is a gameplay-level concept: "this item adds +50 HP" or "this buff multiplies attack by 1.5x". Bonuses are defined using the bonus system API, compiled into transforms at setup time, and applied via resolver forks.

**Why**: Game designers think in terms of bonuses (items, buffs, auras). The bonus API provides a declarative way to express these that compiles to efficient transforms.

**When to use**: Use the bonus API when defining items, buffs, or temporary effects. Compile bonuses once at setup, apply them via resolver forks at runtime.

### Transform

A transform is an engine-level mathematical operation that modifies a stat value. Examples: add a flat value, multiply by a factor, scale based on another stat, clamp to bounds. Transforms can declare dependencies on other stats.

**Why**: Transforms provide the building blocks for stat formulas. They are composable and can depend on other stats, enabling complex dependency graphs.

**When to use**: Use transforms directly when defining base stat formulas. Use the bonus API (which compiles to transforms) when defining gameplay modifiers.

### Phase

A phase controls the order in which transforms are applied. Standard phases are Additive (0), Multiplicative (1), and Final (2). Custom phases (3+) allow fine-grained ordering. Within each phase, transforms are grouped by stack rule and applied in priority order.

**Why**: RPG systems require specific ordering. Equipment bonuses should apply before buff multipliers, and final caps should apply last. Phases enforce this ordering deterministically.

**When to use**: Use standard phases for common operations (additive bonuses, multipliers, clamping). Use custom phases for domain-specific ordering (e.g., Item phase 3, Buff phase 4, Aura phase 5).

### Resolver

A resolver manages stat sources, transforms, and resolution. It builds a dependency graph, resolves stats in topological order, caches results, and invalidates caches when sources or transforms change.

**Why**: Centralizing stat resolution provides caching, dependency management, and cycle detection. The resolver ensures stats are calculated efficiently and correctly.

**When to use**: Create one resolver per character or entity template. The resolver holds all stat formulas and base values.

### Resolver Fork

A resolver fork is a copy-on-write overlay of a base resolver. Forks share base data (sources and transforms) but can have their own overlays for temporary modifications. Forking is O(1) and does not clone the base data.

**Why**: In RPG systems, you frequently need to preview stats with different equipment or calculate temporary combat state. Forks allow this without mutating the base character or copying all data.

**When to use**: Fork the resolver when equipping items, applying buffs, previewing gear changes, or calculating combat snapshots. Each fork is independent and does not affect the base resolver.

## Basic RPG Character Example

This example shows a realistic RPG character with base attributes and derived stats:

```rust
use zzstat::*;
use zzstat::source::ConstantSource;
use zzstat::transform::ScalingTransform;

// Define stat IDs
let str_id = StatId::from_str("STR");
let dex_id = StatId::from_str("DEX");
let vit_id = StatId::from_str("VIT");
let atk_id = StatId::from_str("ATTACK");
let def_id = StatId::from_str("DEFENSE");
let hp_id = StatId::from_str("HP");

// Create resolver for character
let mut resolver = StatResolver::new();

// Register base attributes (sources)
resolver.register_source(str_id.clone(), Box::new(ConstantSource(10.0)));
resolver.register_source(dex_id.clone(), Box::new(ConstantSource(8.0)));
resolver.register_source(vit_id.clone(), Box::new(ConstantSource(12.0)));

// Define derived stats using transforms
// ATTACK = STR * 2 + DEX
resolver.register_source(atk_id.clone(), Box::new(ConstantSource(0.0)));
resolver.register_transform(atk_id.clone(), Box::new(ScalingTransform::new(str_id.clone(), 2.0)));
resolver.register_transform(atk_id.clone(), Box::new(ScalingTransform::new(dex_id.clone(), 1.0)));

// DEFENSE = VIT * 1.5
resolver.register_source(def_id.clone(), Box::new(ConstantSource(0.0)));
resolver.register_transform(def_id.clone(), Box::new(ScalingTransform::new(vit_id.clone(), 1.5)));

// HP = VIT * 10
resolver.register_source(hp_id.clone(), Box::new(ConstantSource(0.0)));
resolver.register_transform(hp_id.clone(), Box::new(ScalingTransform::new(vit_id.clone(), 10.0)));

// Resolve stats
let context = StatContext::new();
let stats = resolver.resolve_batch(&[atk_id, def_id, hp_id], &context)?;

// ATTACK = 10 * 2 + 8 = 28
// DEFENSE = 12 * 1.5 = 18
// HP = 12 * 10 = 120
```

This pattern separates base attributes (STR, DEX, VIT) from derived stats (ATTACK, DEFENSE, HP), allowing designers to adjust formulas without changing character data.

## Items and Equipment

Items modify stats through resolver forks. The base character resolver remains unchanged, and equipped items are applied via a fork:

```rust
use zzstat::bonus::{Bonus, compile_bonus, apply_compiled_bonuses};
use zzstat::transform::TransformPhase;

// Base character resolver (from previous example)
let base_resolver = /* ... */;

// Define items with bonuses
let sword_bonuses = vec![
    Bonus::add(atk_id.clone()).flat(25.0).in_phase(TransformPhase::Custom(3)),
    Bonus::mul(atk_id.clone()).percent(0.15).in_phase(TransformPhase::Custom(3)),
];

let armor_bonuses = vec![
    Bonus::add(hp_id.clone()).flat(100.0).in_phase(TransformPhase::Custom(3)),
    Bonus::add(def_id.clone()).flat(5.0).in_phase(TransformPhase::Custom(3)),
];

// Compile bonuses once (setup time)
let mut all_bonuses = Vec::new();
all_bonuses.extend(sword_bonuses);
all_bonuses.extend(armor_bonuses);
let compiled: Vec<_> = all_bonuses.iter().map(|b| compile_bonus::<f64>(b)).collect();

// Apply to resolver fork (runtime)
let mut equipped_resolver = base_resolver.fork();
apply_compiled_bonuses(&mut equipped_resolver, &compiled);

// Resolve equipped stats
let equipped_stats = equipped_resolver.resolve_batch(&[atk_id, def_id, hp_id], &context)?;
```

Items use Custom phase 3 (Item phase), which runs after base calculations but before buff phases. The base resolver is never mutated, allowing multiple equipment configurations to exist simultaneously.

## Resolver Fork

Resolver forks use copy-on-write semantics. Forking is O(1) and shares base data via `Arc`. Only modifications go into the fork's overlay.

**Real gameplay scenario**: A character has base stats. The player wants to preview how stats change when equipping different items, compare multiple equipment sets, or calculate combat stats with temporary buffs. Each scenario requires a separate resolver state without modifying the base character.

```rust
// Base character resolver
let base_resolver = /* character with base stats */;

// Fork 1: Preview gear change
let mut preview_fork = base_resolver.fork();
apply_compiled_bonuses(&mut preview_fork, &new_item_bonuses);
let preview_stats = preview_fork.resolve_batch(&target_stats, &context)?;

// Fork 2: Current equipped state
let mut equipped_fork = base_resolver.fork();
apply_compiled_bonuses(&mut equipped_fork, &current_item_bonuses);
let current_stats = equipped_fork.resolve_batch(&target_stats, &context)?;

// Fork 3: Combat snapshot (items + temporary buffs)
let mut combat_fork = base_resolver.fork();
apply_compiled_bonuses(&mut combat_fork, &item_bonuses);
apply_compiled_bonuses(&mut combat_fork, &buff_bonuses);
let combat_stats = combat_fork.resolve_batch(&target_stats, &context)?;

// Base resolver is unchanged - all forks are independent
```

Forks share the base resolver's sources and transforms, so memory usage is minimal. Only the overlay (modified sources/transforms) consumes additional memory.

## Batched Resolve

Combat calculations typically need multiple stats (HP, ATTACK, DEFENSE, CRIT_CHANCE, etc.) simultaneously. Resolving them individually is inefficient because dependencies may overlap.

Batch resolution resolves multiple target stats and their dependencies in a single pass, computing each dependency once and caching results:

```rust
// Combat snapshot: resolve all combat-relevant stats
let combat_stats = resolver.resolve_batch(
    &[hp_id, atk_id, def_id, crit_chance_id, move_speed_id],
    &context
)?;

// All stats are resolved with dependencies computed once
let damage = calculate_damage(
    combat_stats[&atk_id].value,
    combat_stats[&crit_chance_id].value,
);
```

This matches real combat snapshot patterns where you compute all relevant stats once per combat tick, not per-stat.

## Phase-Based Transforms

Phases enforce deterministic ordering of stat modifications. Each phase represents a category of modifiers that should apply at a specific point in the calculation pipeline.

### Standard Phases

- **Additive (0)**: Flat bonuses and penalties. Applied first.
- **Multiplicative (1)**: Percentage multipliers. Applied after additive.
- **Final (2)**: Caps, clamping, normalization. Applied last.

### Custom Phases

Custom phases (3+) allow domain-specific ordering:

- **Item phase (3)**: Equipment bonuses
- **Buff phase (4)**: Temporary buffs and debuffs
- **Aura phase (5)**: Area effects and party buffs
- **PvP phase (6)**: PvP-specific scaling
- **Final phase (2)**: Caps and bounds (always last)

Example with multiple phases:

```rust
// Base HP = 1000

// Item phase (3): +200 HP, +10% HP
let item_bonuses = vec![
    Bonus::add(hp_id.clone()).flat(200.0).in_phase(TransformPhase::Custom(3)),
    Bonus::mul(hp_id.clone()).percent(0.10).in_phase(TransformPhase::Custom(3)),
];

// Buff phase (4): +50% HP
let buff_bonuses = vec![
    Bonus::mul(hp_id.clone()).percent(0.50).in_phase(TransformPhase::Custom(4)),
];

// Calculation order:
// Base: 1000
// Item phase: (1000 + 200) * 1.10 = 1320
// Buff phase: 1320 * 1.50 = 1980
// Final: (no caps, so 1980)
```

Phases ensure that item bonuses are applied before buff multipliers, which matches RPG design expectations.

## Stack Rules

Stack rules determine how multiple transforms in the same phase combine. Transforms are grouped by stack rule and applied in priority order within each phase:

**Priority order**: Override → Additive → Multiplicative → Diminishing → Min → Max → MinMax

### Additive

Additive transforms sum together. Used for flat bonuses from items or multiple sources.

```rust
// Two items each add +50 HP
// Result: +100 HP total
Bonus::add(hp_id).flat(50.0).in_phase(TransformPhase::Custom(3));
Bonus::add(hp_id).flat(50.0).in_phase(TransformPhase::Custom(3));
```

### Multiplicative

Multiplicative transforms multiply together. Used for percentage bonuses from auras or buffs.

```rust
// Two auras each multiply by 1.20 (20% bonus)
// Result: 1.20 * 1.20 = 1.44x multiplier
Bonus::mul(atk_id).percent(0.20).in_phase(TransformPhase::Custom(5));
Bonus::mul(atk_id).percent(0.20).in_phase(TransformPhase::Custom(5));
```

### Override

Override transforms replace the value. Last override wins (deterministic by registration order). Used for polymorph effects, PvP normalization, or forced stat values.

```rust
// Polymorph effect: set HP to 1
Bonus::override(hp_id, 1.0).in_phase(TransformPhase::Custom(4));

// Override ignores all previous modifiers in this phase
```

### Clamp (MinMax)

Clamp transforms enforce bounds. Multiple clamps combine to find the most restrictive bounds (max of mins, min of maxes).

```rust
// Crit chance cap: 75%
Bonus::clamp_max(crit_chance_id, 0.75).in_phase(TransformPhase::Final);

// Movement speed floor: 100
Bonus::clamp_min(move_speed_id, 100.0).in_phase(TransformPhase::Final);
```

Stack rules ensure that gameplay logic (e.g., "crit chance is capped at 75%") is enforced correctly regardless of how many modifiers are active.

## Clamp / Cap Example

Caps are common in RPG systems to prevent values from exceeding gameplay boundaries. Crit chance caps prevent 100% crit builds, movement speed floors prevent characters from being completely immobile, and damage caps prevent overflow issues.

Clamps belong in the Final phase because they should apply after all other modifications:

```rust
use zzstat::bonus::Bonus;
use zzstat::transform::TransformPhase;

// Crit chance: base + bonuses, but capped at 75%
let crit_bonuses = vec![
    // Item and buff bonuses can push crit above 75%
    Bonus::add(crit_chance_id.clone()).flat(0.30).in_phase(TransformPhase::Custom(3)),
    Bonus::mul(crit_chance_id.clone()).percent(0.50).in_phase(TransformPhase::Custom(4)),
    // Final phase clamp ensures it never exceeds 75%
    Bonus::clamp_max(crit_chance_id.clone(), 0.75).in_phase(TransformPhase::Final),
];

// Movement speed: has a floor to prevent negative or zero speed
let speed_bonuses = vec![
    Bonus::add(move_speed_id.clone()).flat(-50.0).in_phase(TransformPhase::Custom(3)),
    // Final phase clamp ensures minimum of 100
    Bonus::clamp_min(move_speed_id.clone(), 100.0).in_phase(TransformPhase::Final),
];
```

Placing clamps in the Final phase ensures they apply last, after all bonuses and multipliers. This matches RPG design where caps are absolute limits, not modifiable values.

## Bonus Compilation Pattern

The bonus system separates gameplay-level bonus definitions from engine-level transforms. Bonuses are compiled once at setup time into transforms, and runtime resolution is pure math with zero branching.

**Why this matters for MMO performance**: In an MMO server handling thousands of players, stat resolution happens frequently (every combat tick, every equipment change, every buff application). If bonus logic contains branching or matching, this creates cache pressure and branch mispredictions at scale.

The compilation pattern moves all branching to setup time:

```rust
// Setup time: compile bonuses (branching happens here)
let item_bonuses = vec![
    Bonus::add(hp_id).flat(100.0).in_phase(TransformPhase::Custom(3)),
    Bonus::mul(atk_id).percent(0.20).in_phase(TransformPhase::Custom(3)),
];
let compiled: Vec<_> = item_bonuses.iter()
    .map(|b| compile_bonus::<f64>(b))
    .collect();

// Runtime: apply compiled bonuses (zero branching, pure math)
let mut fork = base_resolver.fork();
apply_compiled_bonuses(&mut fork, &compiled);
let stats = fork.resolve_batch(&target_stats, &context)?;
```

Compiled bonuses are reusable: compile once when loading item data, apply many times when equipping items. The resolver's transform application is branch-free and trivially inlinable.

## Determinism & Fixed-Point Math

Deterministic stat resolution is critical for MMO servers that use lockstep simulation, replay systems, or distributed stat calculations. Floating-point arithmetic can produce slightly different results across platforms or compiler optimizations, leading to desyncs.

zzstat supports an optional fixed-point numeric backend via the `fixed-point` feature:

```toml
[dependencies]
zzstat = { version = "0.1.5", features = ["fixed-point"] }
```

With fixed-point enabled, all calculations use integer arithmetic with a fixed decimal scale, ensuring identical results across all platforms:

```rust
use zzstat::numeric::FixedPoint;

// Fixed-point calculations are deterministic across platforms
let value = FixedPoint::from_f64(100.5);
let result = value * FixedPoint::from_f64(1.5);
// Always produces the same integer result, regardless of platform
```

Fixed-point math trades a small performance cost (integer operations vs floating-point) for absolute determinism. Use it when you need platform-independent stat calculations or when integrating with lockstep game clients.

## What zzstat Deliberately Does NOT Do

zzstat is intentionally limited to pure stat math. This focus enables integration into diverse architectures:

- **No Redis**: zzstat has no Redis dependencies. Use it with Redis, PostgreSQL, or any storage you prefer.
- **No async**: zzstat is synchronous. Integrate it into async runtimes (Tokio, async-std) by running resolution in blocking tasks.
- **No serialization format**: zzstat does not define how to serialize resolvers or stats. Use Serde, Protocol Buffers, or custom formats as needed.
- **No ECS**: zzstat does not manage entities or components. Integrate it into ECS systems by storing resolvers as components.
- **No game loop**: zzstat has no timing, frames, or ticks. Call resolution when your game logic requires stat values.

This design allows zzstat to serve as a stat calculation primitive that fits into any backend architecture. The engine provides stat math; your code handles persistence, networking, and game logic.

## Installation

Add zzstat to your `Cargo.toml`:

```toml
[dependencies]
zzstat = "0.1.5"
```

For fixed-point determinism:

```toml
[dependencies]
zzstat = { version = "0.1.5", features = ["fixed-point"] }
```

## Examples

The `examples/` directory contains comprehensive examples:

- **`rpg.rs`**: Complete RPG character system with items and equipment
- **`bonus_system.rs`**: Bonus API usage with compilation patterns
- **`dependencies.rs`**: Derived stats and dependency chains
- **`context.rs`**: Conditional calculations using StatContext
- **`complex.rs`**: Multi-phase stat system with clamping

Run examples with:

```bash
cargo run --example rpg
cargo run --example bonus_system
```

## License

This project is licensed under the MIT License.
