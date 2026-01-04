# zzstat Examples

This directory contains example programs demonstrating how to use the zzstat stat engine.

## Examples

### `basic.rs`
**Basic stat resolution with sources and transforms**

Demonstrates:
- Registering multiple stat sources (additive)
- Applying transforms (percentage modifiers)
- Resolving stats and viewing breakdowns

Run with: `cargo run --example basic`

### `dependencies.rs`
**Stats that depend on other stats**

Demonstrates:
- Creating derived stats
- Dependency chains (STR → ATK, DEX → CRIT)
- Automatic resolution order using topological sort

Run with: `cargo run --example dependencies`

### `complex.rs`
**Complex character stat system**

Demonstrates:
- Multiple sources per stat
- Transform chains (multiple transforms in sequence)
- Multi-level dependencies (BASE → MID → TOP)
- Clamping values
- Real-world character stat calculations

Run with: `cargo run --example complex`

### `cycle_detection.rs`
**Error handling for circular dependencies**

Demonstrates:
- What happens when you create circular dependencies
- How the system detects and reports cycles
- Comparison with valid dependency chains

Run with: `cargo run --example cycle_detection`

### `context.rs`
**Using StatContext for conditional calculations**

Demonstrates:
- Using StatContext to pass game state
- Conditional transforms based on context
- Different stat values in different contexts (combat, PvP zones, etc.)
- Cache invalidation when context changes

Run with: `cargo run --example context`

### `advanced.rs`
**Advanced features: forking, batch resolution, and transform phases**

Demonstrates:
- Resolver forking (copy-on-write semantics)
- Batch resolution for efficient stat resolution
- Transform phase ordering (Additive → Multiplicative → Final)
- Multiple forks from the same base resolver
- Subgraph extraction for targeted resolution

Run with: `cargo run --example advanced`

## Running Examples

To run any example:

```bash
cargo run --example <example_name>
```

For example:
```bash
cargo run --example basic
cargo run --example dependencies
cargo run --example complex
```

## Building All Examples

To build all examples without running them:

```bash
cargo build --examples
```

