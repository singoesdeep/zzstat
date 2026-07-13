---
name: cluster-22
description: "Skill for the Cluster_22 area of zzstat. 5 symbols across 1 files."
---

# Cluster_22

5 symbols | 1 files | Cohesion: 30%

## When to Use

- Working with code in `src/`
- Understanding how new work
- Modifying cluster_22-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/resolver.rs` | new, default, test_resolve_simple_source, test_resolve_multiple_sources, test_resolve_missing_source |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `src/resolver.rs:107`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `src/resolver.rs` | 107 |
| `default` | Function | `src/resolver.rs` | 1096 |
| `test_resolve_simple_source` | Function | `src/resolver.rs` | 1108 |
| `test_resolve_multiple_sources` | Function | `src/resolver.rs` | 1122 |
| `test_resolve_missing_source` | Function | `src/resolver.rs` | 1172 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → BaseData` | cross_community | 5 |
| `Main → BaseData` | cross_community | 4 |
| `Main → OverlayData` | cross_community | 4 |
| `Main → BaseData` | cross_community | 4 |
| `Main → OverlayData` | cross_community | 4 |
| `Apply_transforms_with_stack_rules → BaseData` | cross_community | 4 |
| `Apply_transforms_with_stack_rules → OverlayData` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Tests | 9 calls |

## How to Explore

1. `context({name: "new"})` — see callers and callees
2. `query({search_query: "cluster_22"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
