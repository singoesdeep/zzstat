---
name: cluster-17
description: "Skill for the Cluster_17 area of zzstat. 30 symbols across 1 files."
---

# Cluster_17

30 symbols | 1 files | Cohesion: 85%

## When to Use

- Working with code in `src/`
- Understanding how new, add_node, add_edge work
- Modifying cluster_17-related functionality

## Key Files

| File | Symbols |
|------|---------|
| `src/graph.rs` | new, add_node, add_edge, detect_cycles, dfs_cycle_detect (+25) |

## Entry Points

Start here when exploring this area:

- **`new`** (Function) — `src/graph.rs:51`
- **`add_node`** (Function) — `src/graph.rs:70`
- **`add_edge`** (Function) — `src/graph.rs:103`
- **`detect_cycles`** (Function) — `src/graph.rs:136`
- **`topological_sort`** (Function) — `src/graph.rs:234`

## Key Symbols

| Symbol | Type | File | Line |
|--------|------|------|------|
| `new` | Function | `src/graph.rs` | 51 |
| `add_node` | Function | `src/graph.rs` | 70 |
| `add_edge` | Function | `src/graph.rs` | 103 |
| `detect_cycles` | Function | `src/graph.rs` | 136 |
| `topological_sort` | Function | `src/graph.rs` | 234 |
| `nodes` | Function | `src/graph.rs` | 271 |
| `subgraph_for_targets` | Function | `src/graph.rs` | 338 |
| `dfs_cycle_detect` | Function | `src/graph.rs` | 155 |
| `default` | Function | `src/graph.rs` | 377 |
| `test_graph_add_nodes` | Function | `src/graph.rs` | 387 |
| `test_graph_add_edge` | Function | `src/graph.rs` | 400 |
| `test_graph_no_cycle` | Function | `src/graph.rs` | 413 |
| `test_graph_detect_cycle` | Function | `src/graph.rs` | 427 |
| `test_topological_sort` | Function | `src/graph.rs` | 442 |
| `test_subgraph_for_targets` | Function | `src/graph.rs` | 466 |
| `test_subgraph_for_multiple_targets` | Function | `src/graph.rs` | 494 |
| `test_subgraph_for_targets_with_shared_dependency` | Function | `src/graph.rs` | 519 |
| `test_subgraph_for_targets_empty` | Function | `src/graph.rs` | 549 |
| `test_subgraph_for_targets_nonexistent` | Function | `src/graph.rs` | 556 |
| `test_graph_nodes` | Function | `src/graph.rs` | 571 |

## Execution Flows

| Flow | Type | Steps |
|------|------|-------|
| `Main → Add_node` | cross_community | 5 |
| `Test_minmax_stack_rule_mixed_bounds → Add_node` | cross_community | 5 |
| `Test_multiple_clamps_minmax_composition → Add_node` | cross_community | 5 |
| `Main → Add_node` | cross_community | 5 |
| `Test_min_clamping → Add_node` | cross_community | 5 |

## Connected Areas

| Area | Connections |
|------|-------------|
| Tests | 20 calls |

## How to Explore

1. `context({name: "new"})` — see callers and callees
2. `query({search_query: "cluster_17"})` — find related execution flows
3. Read key files listed above for implementation details
4. `explain({target: "<file or symbol>"})` — persisted taint findings (source→sink data flows), when indexed with `--pdg`
