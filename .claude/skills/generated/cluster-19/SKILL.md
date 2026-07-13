---
name: cluster-19
description: "Skill for the Cluster_19 area of zzstat. 3 symbols across 1 files."
---

# Cluster_19

3 symbols | 1 files | Cohesion: 100%

## When to Use

- Working with code in `src/`
- Understanding how new work
- Modifying cluster_19-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/numeric.rs` | new, test_fixed_point_creation, test_fixed_point_different_scales |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `src/numeric.rs:109`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `src/numeric.rs` | 109 |
| `test_fixed_point_creation` | Function | `src/numeric.rs` | 331 |
| `test_fixed_point_different_scales` | Function | `src/numeric.rs` | 358 |

## How to Explore

1. `context({name: "new"})` — see callers and callees
2. `query({search_query: "cluster_19"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
