---
name: cluster-18
description: "Skill for the Cluster_18 area of zzstat. 7 symbols across 2 files."
---

# Cluster_18

7 symbols | 2 files | Cohesion: 67%

## When to Use

- Working with code in `src/`
- Understanding how zero, test_stat_numeric_trait, resolve_stat_internal work
- Modifying cluster_18-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/resolver.rs` | resolve_stat_internal, apply_transforms_with_stack_rules, collect_dependencies, extract_min_bound, extract_max_bound |
| `src/numeric.rs` | zero, test_stat_numeric_trait |

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `zero` | Function | `src/numeric.rs` | 29 |
| `test_stat_numeric_trait` | Function | `src/numeric.rs` | 368 |
| `resolve_stat_internal` | Function | `src/resolver.rs` | 742 |
| `apply_transforms_with_stack_rules` | Function | `src/resolver.rs` | 836 |
| `collect_dependencies` | Function | `src/resolver.rs` | 1031 |
| `extract_min_bound` | Function | `src/resolver.rs` | 1052 |
| `extract_max_bound` | Function | `src/resolver.rs` | 1076 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Test_minmax_stack_rule_mixed_bounds → Add_source` | cross_community | 5 |
| `Test_multiple_clamps_minmax_composition → Add_source` | cross_community | 5 |
| `Main → Add_source` | cross_community | 5 |
| `Test_min_clamping → Add_source` | cross_community | 5 |
| `Apply_transforms_with_stack_rules → BaseData` | cross_community | 4 |
| `Apply_transforms_with_stack_rules → OverlayData` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_21 | 12 calls |
| Cluster_22 | 3 calls |
| Tests | 3 calls |

## How to Explore

1. `context({name: "zero"})` — see callers and callees
2. `query({search_query: "cluster_18"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
