---
name: examples
description: "Skill for the Examples area of zzstat. 4 symbols across 1 files."
---

# Examples

4 symbols | 1 files | Cohesion: 63%

## When to Use

- Working with code in `examples/`
- Understanding how define_stat_ids, new, with_equipped_items work
- Modifying examples-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `examples/rpg.rs` | define_stat_ids, new, with_equipped_items, main |

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `define_stat_ids` | Function | `examples/rpg.rs` | 24 |
| `new` | Function | `examples/rpg.rs` | 71 |
| `with_equipped_items` | Function | `examples/rpg.rs` | 134 |
| `main` | Function | `examples/rpg.rs` | 185 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → Add_node` | cross_community | 5 |
| `Main → Get_transforms` | cross_community | 5 |
| `Main → Add_source` | cross_community | 5 |
| `Main → BaseData` | cross_community | 5 |
| `Main → Clone` | cross_community | 4 |
| `Main → OverlayData` | cross_community | 4 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Tests | 15 calls |

## How to Explore

1. `context({name: "define_stat_ids"})` — see callers and callees
2. `query({search_query: "examples"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
