---
name: cluster-16
description: "Skill for the Cluster_16 area of zzstat. 13 symbols across 2 files."
---

# Cluster_16

13 symbols | 2 files | Cohesion: 87%

## When to Use

- Working with code in `src/`
- Understanding how new, set, get work
- Modifying cluster_16-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/context.rs` | new, set, get, test_context_set_get, test_context_missing_key (+6) |
| `src/transform.rs` | test_conditional_transform, test_conditional_transform_with_dependencies |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `src/context.rs:45`
- **`set`** (Function) — `src/context.rs:64`
- **`get`** (Function) — `src/context.rs:89`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `src/context.rs` | 45 |
| `set` | Function | `src/context.rs` | 64 |
| `get` | Function | `src/context.rs` | 89 |
| `test_context_set_get` | Function | `src/context.rs` | 118 |
| `test_context_missing_key` | Function | `src/context.rs` | 127 |
| `test_context_different_types` | Function | `src/context.rs` | 134 |
| `test_context_type_mismatch` | Function | `src/context.rs` | 150 |
| `test_context_contains_key` | Function | `src/context.rs` | 160 |
| `test_context_overwrite` | Function | `src/context.rs` | 169 |
| `test_context_serialization` | Function | `src/context.rs` | 179 |
| `test_context_clone` | Function | `src/context.rs` | 201 |
| `test_conditional_transform` | Function | `src/transform.rs` | 1108 |
| `test_conditional_transform_with_dependencies` | Function | `src/transform.rs` | 1271 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Tests | 3 calls |
| Cluster_25 | 2 calls |

## How to Explore

1. `context({name: "new"})` — see callers and callees
2. `query({search_query: "cluster_16"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
