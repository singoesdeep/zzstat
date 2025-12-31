# zzstat

A deterministic, hardcode-free stat calculation engine designed for MMORPGs.

## Features

- **Deterministic** stat resolution (same input → same output)
- **Hardcode-free** design (no built-in stat names like "HP" or "ATK")
- **Event-driven** resolution (only resolves when invalidated)
- **Phase-based** transformation pipeline
- **Dependency Graph**: Automatically resolves dependencies in correct order
- **Cycle Detection**: Prevents circular dependencies
- **Caching**: Resolved stats are cached until invalidated
- **Context-Aware**: Supports conditional calculations via `StatContext`
- **Debug-Friendly**: Full breakdown of sources and transforms

## Core Concepts

### Stat Pipeline

Stats flow through a simple pipeline:

```
[StatSource] → [StatTransform] → [ResolvedStat]
```

1. **Sources** produce base values (additive)
2. **Transforms** modify values (can depend on other stats)
3. **ResolvedStat** contains the final value with full breakdown

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
zzstat = "0.1.0"
```

## Example

```rust
use zzstat::*;
use zzstat::source::ConstantSource;
use zzstat::transform::MultiplicativeTransform;

let mut resolver = StatResolver::new();
let hp_id = StatId::from_str("HP");

// Register sources (additive)
resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));

// Register transform
resolver.register_transform(hp_id.clone(), Box::new(MultiplicativeTransform::new(1.5)));

// Resolve
let context = StatContext::new();
let resolved = resolver.resolve(&hp_id, &context).unwrap();
assert_eq!(resolved.value, 225.0); // (100 + 50) * 1.5
```

## Examples

The `examples/` directory contains several example programs:

- **`basic.rs`** - Basic stat resolution with sources and transforms
- **`dependencies.rs`** - Stats that depend on other stats
- **`complex.rs`** - Complex character stat system
- **`cycle_detection.rs`** - Error handling for circular dependencies
- **`context.rs`** - Using StatContext for conditional calculations

Run examples with:

```bash
cargo run --example basic
cargo run --example dependencies
cargo run --example complex
```

See `examples/README.md` for more details.

## Modules

- `stat_id` - Stat identifier type
- `source` - Stat sources (produce base values)
- `transform` - Stat transforms (modify values)
- `resolver` - Main stat resolver
- `resolved` - Resolved stat results
- `context` - Context for conditional calculations
- `graph` - Dependency graph management
- `error` - Error types

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) file for details.

