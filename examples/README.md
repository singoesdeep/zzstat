# zzstat Examples

This directory contains example programs demonstrating how to use the `zzstat` stat engine in various real-world game development scenarios. The examples are numbered in order of complexity.

## Examples

### `01_template_loader.rs`
**Data-Driven Initialization via JSON**

Demonstrates:
- Loading a complete character archetype from JSON without hardcoding any Rust logic.
- Using `StatContext` to apply conditional logic dynamically (e.g., adding DEF only when in `DEFENSIVE` stance).
- Converting parsed JSON templates into a `StatResolver`.

Run with: `cargo run --example 01_template_loader`

### `02_resource_pool.rs`
**State Management and Event Triggers**

Demonstrates:
- Tracking stateful resources like Current HP/MP against a max value resolved by `StatResolver`.
- Using `TimeEffect` to apply Damage-over-Time (e.g., Poison).
- Firing `ThresholdTrigger` events when a condition is met (e.g., HP falls to 0 triggering `DEATH`).

Run with: `cargo run --example 02_resource_pool`

### `03_status_effects.rs`
**Temporary Buffs and Debuffs (Copy-on-Write)**

Demonstrates:
- Using `StatusManager` to manage temporary stat overlays.
- How buffs dynamically compose on top of the base stats using efficient `fork()` mechanics.
- Dealing with stacks (`Accumulate`, `Refresh`) and expiring effects over ticks.

Run with: `cargo run --example 03_status_effects`

### `04_combat_engine.rs`
**AST-based Combat Formula Evaluation**

Demonstrates:
- Parsing complex attack actions defined as an Abstract Syntax Tree (AST) in JSON.
- Passing `attacker` and `defender` resolvers into a single context.
- Handling stochastic (random) mechanics deterministically through closures (e.g., Dodge Chance, Critical Hit Chance).

Run with: `cargo run --example 04_combat_engine`

### `metin2/`
**Full MMORPG Damage Simulator**

A comprehensive simulation mimicking the damage pipeline of the classic MMORPG, Metin2.
Demonstrates:
- Creating custom entity wrappers (`Metin2Player`, `Metin2Monster`, `Weapon`).
- Simulating the complete damage cycle: `Base Damage -> Critical -> Piercing -> Defensive mitigation -> Resistances`.
- Combining multiple `zzstat` concepts together into a fully structured codebase.

Run with: `cargo run --example metin2`

---

## Running Examples

To run any single-file example:

```bash
cargo run --example <example_name>
```

For the Metin2 full simulation:
```bash
cargo run --example metin2
```

## Building All Examples

To compile all examples without running them, which is useful for checking compilation errors:

```bash
cargo build --examples
```
