---
name: cluster-25
description: "Skill for the Cluster_25 area of zzstat. 17 symbols across 1 files."
---

# Cluster_25

17 symbols | 1 files | Cohesion: 65%

## When to Use

- Working with code in `src/`
- Understanding how new work
- Modifying cluster_25-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/transform.rs` | depends_on, description, new, test_multiplicative_transform, test_additive_transform (+12) |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `src/transform.rs:351`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `src/transform.rs` | 351 |
| `depends_on` | Function | `src/transform.rs` | 266 |
| `description` | Function | `src/transform.rs` | 307 |
| `test_multiplicative_transform` | Function | `src/transform.rs` | 891 |
| `test_additive_transform` | Function | `src/transform.rs` | 905 |
| `test_clamp_transform` | Function | `src/transform.rs` | 919 |
| `test_clamp_transform_min_only` | Function | `src/transform.rs` | 982 |
| `test_clamp_transform_no_bounds` | Function | `src/transform.rs` | 1032 |
| `test_scaling_transform` | Function | `src/transform.rs` | 1079 |
| `test_scaling_transform_missing_dependency` | Function | `src/transform.rs` | 1096 |
| `test_multiplicative_transform_zero` | Function | `src/transform.rs` | 1154 |
| `test_multiplicative_transform_negative` | Function | `src/transform.rs` | 1168 |
| `test_additive_transform_negative` | Function | `src/transform.rs` | 1182 |
| `test_clamp_transform_edge_cases` | Function | `src/transform.rs` | 1196 |
| `test_scaling_transform_zero_scale` | Function | `src/transform.rs` | 1237 |
| `test_scaling_transform_negative_scale` | Function | `src/transform.rs` | 1254 |
| `test_transform_descriptions` | Function | `src/transform.rs` | 1308 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Tests | 12 calls |

## How to Explore

1. `context({name: "new"})` — see callers and callees
2. `query({search_query: "cluster_25"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
