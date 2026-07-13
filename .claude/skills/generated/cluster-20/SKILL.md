---
name: cluster-20
description: "Skill for the Cluster_20 area of zzstat. 5 symbols across 1 files."
---

# Cluster_20

5 symbols | 1 files | Cohesion: 100%

## When to Use

- Working with code in `src/`
- Understanding how normalize, add, sub work
- Modifying cluster_20-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/numeric.rs` | normalize, add, sub, mul, div |

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `normalize` | Function | `src/numeric.rs` | 172 |
| `add` | Function | `src/numeric.rs` | 221 |
| `sub` | Function | `src/numeric.rs` | 234 |
| `mul` | Function | `src/numeric.rs` | 247 |
| `div` | Function | `src/numeric.rs` | 262 |

## How to Explore

1. `context({name: "normalize"})` — see callers and callees
2. `query({search_query: "cluster_20"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
