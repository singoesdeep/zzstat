---
name: cluster-21
description: "Skill for the Cluster_21 area of zzstat. 9 symbols across 1 files."
---

# Cluster_21

9 symbols | 1 files | Cohesion: 63%

## When to Use

- Working with code in `src/`
- Understanding how new, add_source, add_transform work
- Modifying cluster_21-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/resolved.rs` | new, add_source, add_transform, test_resolved_stat_creation, test_resolved_stat_breakdown (+4) |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `src/resolved.rs:66`
- **`add_source`** (Function) — `src/resolved.rs:95`
- **`add_transform`** (Function) — `src/resolved.rs:118`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `src/resolved.rs` | 66 |
| `add_source` | Function | `src/resolved.rs` | 95 |
| `add_transform` | Function | `src/resolved.rs` | 118 |
| `test_resolved_stat_creation` | Function | `src/resolved.rs` | 129 |
| `test_resolved_stat_breakdown` | Function | `src/resolved.rs` | 138 |
| `test_resolved_stat_multiple_sources` | Function | `src/resolved.rs` | 149 |
| `test_resolved_stat_multiple_transforms` | Function | `src/resolved.rs` | 162 |
| `test_resolved_stat_clone` | Function | `src/resolved.rs` | 173 |
| `test_resolved_stat_empty_breakdown` | Function | `src/resolved.rs` | 188 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Test_minmax_stack_rule_mixed_bounds → Add_source` | cross_community | 5 |
| `Test_multiple_clamps_minmax_composition → Add_source` | cross_community | 5 |
| `Main → Add_source` | cross_community | 5 |
| `Test_min_clamping → Add_source` | cross_community | 5 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Tests | 12 calls |

## How to Explore

1. `context({name: "new"})` — see callers and callees
2. `query({search_query: "cluster_21"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
