---
name: tests
description: "Skill for the Tests area of zzstat. 129 symbols across 16 files."
---

# Tests

129 symbols | 16 files | Cohesion: 47%

## When to Use

- Working with code in `src/`
- Understanding how clamp_min, new, empty work
- Modifying tests-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `tests/integration_tests.rs` | test_diminishing_returns, test_cache_invalidation, test_additive_stacking, test_multiplicative_stacking, test_additive_multiplicative_combination (+29) |
| `tests/bonus_tests.rs` | test_bonus_add_flat, test_bonus_add_percent, test_bonus_multiply, test_bonus_clamp_min, test_compile_add_flat (+18) |
| `src/resolver.rs` | register_source, register_transform_with_rule, invalidate, test_cache_invalidation, register_transform (+18) |
| `src/source.rs` | new, empty, insert, test_constant_source, test_map_source (+7) |
| `src/bonus.rs` | clamp_min, to_transform, new, depends_on, apply_compiled_bonuses (+5) |
| `src/transform.rs` | with_bounds, with_min, with_max, test_clamp_bounds_trait, infer_stack_rule (+3) |
| `src/stat_id.rs` | deserialize, from_str, from, test_stat_id_creation, test_stat_id_ordering |
| `src/numeric.rs` | from_f64, from_f64_with_scale, from, test_fixed_point_from_f64, test_fixed_point_arithmetic |
| `src/error.rs` | test_error_display, test_cycle_error_display |
| `examples/advanced.rs` | main |

## Entry Points

Start here when exploring this area:

- **`clamp_min`** (Function) — `src/bonus.rs:148`
- **`new`** (Function) — `src/source.rs:112`
- **`empty`** (Function) — `src/source.rs:126`
- **`insert`** (Function) — `src/source.rs:142`
- **`from_str`** (Function) — `src/stat_id.rs:65`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `clamp_min` | Function | `src/bonus.rs` | 148 |
| `new` | Function | `src/source.rs` | 112 |
| `empty` | Function | `src/source.rs` | 126 |
| `insert` | Function | `src/source.rs` | 142 |
| `from_str` | Function | `src/stat_id.rs` | 65 |
| `register_source` | Function | `src/resolver.rs` | 188 |
| `register_transform_with_rule` | Function | `src/resolver.rs` | 305 |
| `invalidate` | Function | `src/resolver.rs` | 490 |
| `with_bounds` | Function | `src/transform.rs` | 570 |
| `with_min` | Function | `src/transform.rs` | 589 |
| `with_max` | Function | `src/transform.rs` | 611 |
| `register_transform` | Function | `src/resolver.rs` | 221 |
| `register_transform_in_phase` | Function | `src/resolver.rs` | 260 |
| `resolve` | Function | `src/resolver.rs` | 365 |
| `get_breakdown` | Function | `src/resolver.rs` | 553 |
| `infer_stack_rule` | Function | `src/transform.rs` | 226 |
| `apply_compiled_bonuses` | Function | `src/bonus.rs` | 475 |
| `fork` | Function | `src/resolver.rs` | 151 |
| `from_f64_with_scale` | Function | `src/numeric.rs` | 137 |
| `resolve_all` | Function | `src/resolver.rs` | 409 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → Add_node` | cross_community | 5 |
| `Main → Get_transforms` | cross_community | 5 |
| `Main → Is_fork` | intra_community | 5 |
| `Main → Is_fork` | intra_community | 5 |
| `Main → Is_fork` | cross_community | 5 |
| `Test_complete_pipeline → Is_fork` | cross_community | 5 |
| `Test_minmax_stack_rule_mixed_bounds → Is_fork` | cross_community | 5 |
| `Test_minmax_stack_rule_mixed_bounds → Add_node` | cross_community | 5 |
| `Test_minmax_stack_rule_mixed_bounds → Get_transforms` | cross_community | 5 |
| `Test_minmax_stack_rule_mixed_bounds → Add_source` | cross_community | 5 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Cluster_16 | 8 calls |
| Cluster_22 | 8 calls |
| Cluster_25 | 3 calls |
| Cluster_17 | 2 calls |
| Cluster_18 | 1 calls |

## How to Explore

1. `context({name: "clamp_min"})` — see callers and callees
2. `query({search_query: "tests"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
