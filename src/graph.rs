//! Dependency graph module.
//!
//! Provides the `StatGraph` type, which represents stat dependencies
//! as a directed acyclic graph (DAG). Used by the resolver to determine
//! the correct order for stat resolution.

use crate::error::StatError;
use crate::stat_id::StatId;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// A directed acyclic graph (DAG) representing stat dependencies.
///
/// Nodes are `StatId`s, edges represent dependencies. If stat A depends
/// on stat B, then B must be resolved before A.
///
/// The graph automatically detects cycles and provides topological sorting
/// to determine resolution order.
///
/// # Examples
///
/// ```rust
/// use zzstat::graph::StatGraph;
/// use zzstat::StatId;
///
/// let mut graph = StatGraph::new();
/// let str_id = StatId::from_str("STR");
/// let atk_id = StatId::from_str("ATK");
///
/// // ATK depends on STR
/// graph.add_edge(atk_id, str_id);
///
/// // Get resolution order (STR before ATK)
/// let order = graph.topological_sort().unwrap();
/// ```
pub struct StatGraph {
    graph: DiGraph<StatId, ()>,
    node_map: HashMap<StatId, NodeIndex>,
}

impl StatGraph {
    /// Create a new empty graph.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::graph::StatGraph;
    ///
    /// let graph = StatGraph::new();
    /// ```
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_map: HashMap::new(),
        }
    }

    /// Add a node to the graph if it doesn't exist.
    ///
    /// If the node already exists, returns the existing node index.
    /// Otherwise, creates a new node and returns its index.
    ///
    /// # Arguments
    ///
    /// * `stat_id` - The stat ID to add as a node
    ///
    /// # Returns
    ///
    /// The node index for this stat ID.
    pub fn add_node(&mut self, stat_id: StatId) -> NodeIndex {
        if let Some(&idx) = self.node_map.get(&stat_id) {
            idx
        } else {
            let idx = self.graph.add_node(stat_id.clone());
            self.node_map.insert(stat_id, idx);
            idx
        }
    }

    /// Add an edge representing a dependency.
    ///
    /// `from` depends on `to` (to must be resolved before from).
    /// Both nodes are automatically added to the graph if they don't exist.
    ///
    /// # Arguments
    ///
    /// * `from` - The stat that depends on `to`
    /// * `to` - The stat that `from` depends on
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::graph::StatGraph;
    /// use zzstat::StatId;
    ///
    /// let mut graph = StatGraph::new();
    /// let atk_id = StatId::from_str("ATK");
    /// let str_id = StatId::from_str("STR");
    ///
    /// // ATK depends on STR
    /// graph.add_edge(atk_id, str_id);
    /// ```
    pub fn add_edge(&mut self, from: StatId, to: StatId) {
        let from_idx = self.add_node(from);
        let to_idx = self.add_node(to);
        self.graph.add_edge(to_idx, from_idx, ());
    }

    /// Detect cycles in the graph.
    ///
    /// Uses depth-first search to detect any circular dependencies.
    ///
    /// # Returns
    ///
    /// * `Ok(())` if no cycles are detected
    /// * `Err(StatError::Cycle)` with the cycle path if a cycle is found
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::graph::StatGraph;
    /// use zzstat::StatId;
    ///
    /// let mut graph = StatGraph::new();
    /// let a = StatId::from_str("A");
    /// let b = StatId::from_str("B");
    ///
    /// // No cycle
    /// graph.add_edge(b.clone(), a.clone());
    /// assert!(graph.detect_cycles().is_ok());
    ///
    /// // Create cycle: A -> B -> A
    /// graph.add_edge(a.clone(), b.clone());
    /// assert!(graph.detect_cycles().is_err());
    /// ```
    pub fn detect_cycles(&self) -> Result<(), StatError> {
        // Use DFS to detect cycles
        let mut visited = std::collections::HashSet::new();
        let mut rec_stack = std::collections::HashSet::new();

        for node_idx in self.graph.node_indices() {
            if !visited.contains(&node_idx) {
                let mut cycle_path = Vec::new();
                if let Some(cycle) = self.dfs_cycle_detect(node_idx, &mut visited, &mut rec_stack, &mut cycle_path) {
                    return Err(cycle);
                }
            }
        }

        Ok(())
    }

    fn dfs_cycle_detect(
        &self,
        node: NodeIndex,
        visited: &mut std::collections::HashSet<NodeIndex>,
        rec_stack: &mut std::collections::HashSet<NodeIndex>,
        cycle_path: &mut Vec<StatId>,
    ) -> Option<StatError> {
        visited.insert(node);
        rec_stack.insert(node);
        cycle_path.push(self.graph[node].clone());

        for neighbor in self
            .graph
            .neighbors_directed(node, petgraph::Direction::Outgoing)
        {
            if !visited.contains(&neighbor) {
                if let Some(cycle) = self.dfs_cycle_detect(neighbor, visited, rec_stack, cycle_path) {
                    return Some(cycle);
                }
            } else if rec_stack.contains(&neighbor) {
                // Cycle detected - extract the cycle portion from the path
                let neighbor_stat = self.graph[neighbor].clone();
                
                // Find where the cycle starts (where neighbor first appears)
                if let Some(cycle_start_pos) = cycle_path.iter().position(|stat| stat == &neighbor_stat) {
                    // Extract only the cycle portion
                    let mut cycle: Vec<StatId> = cycle_path[cycle_start_pos..].to_vec();
                    // Close the loop by adding the neighbor again
                    cycle.push(neighbor_stat);
                    return Some(StatError::Cycle { path: cycle });
                } else {
                    // Fallback: create cycle with current node and neighbor
                    return Some(StatError::Cycle {
                        path: vec![self.graph[node].clone(), neighbor_stat.clone(), neighbor_stat],
                    });
                }
            }
        }

        rec_stack.remove(&node);
        cycle_path.pop();
        None
    }

    /// Get a topological sort of all nodes.
    ///
    /// This gives the order in which stats should be resolved.
    /// Dependencies are guaranteed to come before dependents.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<StatId>)` - The resolution order (dependencies first)
    /// * `Err(StatError::Cycle)` - If a cycle is detected
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::graph::StatGraph;
    /// use zzstat::StatId;
    ///
    /// let mut graph = StatGraph::new();
    /// let str_id = StatId::from_str("STR");
    /// let atk_id = StatId::from_str("ATK");
    ///
    /// graph.add_edge(atk_id.clone(), str_id.clone());
    ///
    /// let order = graph.topological_sort().unwrap();
    /// // STR will come before ATK in the order
    /// let str_pos = order.iter().position(|s| s == &str_id).unwrap();
    /// let atk_pos = order.iter().position(|s| s == &atk_id).unwrap();
    /// assert!(str_pos < atk_pos);
    /// ```
    pub fn topological_sort(&self) -> Result<Vec<StatId>, StatError> {
        // First check for cycles
        self.detect_cycles()?;

        // Use petgraph's toposort
        match toposort(&self.graph, None) {
            Ok(indices) => Ok(indices
                .into_iter()
                .map(|idx| self.graph[idx].clone())
                .collect()),
            Err(cycle) => {
                // This shouldn't happen if detect_cycles passed, but handle it anyway
                let cycle_path = vec![self.graph[cycle.node_id()].clone()];
                Err(StatError::Cycle { path: cycle_path })
            }
        }
    }

    /// Get all nodes in the graph.
    ///
    /// # Returns
    ///
    /// A vector of all stat IDs in the graph.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::graph::StatGraph;
    /// use zzstat::StatId;
    ///
    /// let mut graph = StatGraph::new();
    /// graph.add_node(StatId::from_str("HP"));
    /// graph.add_node(StatId::from_str("ATK"));
    ///
    /// let nodes = graph.nodes();
    /// assert_eq!(nodes.len(), 2);
    /// ```
    pub fn nodes(&self) -> Vec<StatId> {
        self.graph
            .node_indices()
            .map(|idx| self.graph[idx].clone())
            .collect()
    }

    /// Check if a node exists in the graph.
    ///
    /// # Arguments
    ///
    /// * `stat_id` - The stat ID to check
    ///
    /// # Returns
    ///
    /// `true` if the node exists, `false` otherwise.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::graph::StatGraph;
    /// use zzstat::StatId;
    ///
    /// let mut graph = StatGraph::new();
    /// let hp_id = StatId::from_str("HP");
    /// graph.add_node(hp_id.clone());
    ///
    /// assert!(graph.contains_node(&hp_id));
    /// assert!(!graph.contains_node(&StatId::from_str("ATK")));
    /// ```
    pub fn contains_node(&self, stat_id: &StatId) -> bool {
        self.node_map.contains_key(stat_id)
    }

    /// Extract a subgraph containing only the specified targets and their dependencies.
    ///
    /// Performs a reverse DFS from the target nodes to find all dependencies.
    /// Only nodes reachable from the targets are included in the subgraph.
    ///
    /// # Arguments
    ///
    /// * `targets` - The target stat IDs to include in the subgraph
    ///
    /// # Returns
    ///
    /// A new `StatGraph` containing only the targets and their dependencies.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::graph::StatGraph;
    /// use zzstat::StatId;
    ///
    /// let mut graph = StatGraph::new();
    /// let str_id = StatId::from_str("STR");
    /// let atk_id = StatId::from_str("ATK");
    /// let hp_id = StatId::from_str("HP");
    ///
    /// // ATK depends on STR
    /// graph.add_edge(atk_id.clone(), str_id.clone());
    ///
    /// // Extract subgraph for ATK (includes STR as dependency)
    /// let subgraph = graph.subgraph_for_targets(&[atk_id.clone()]);
    /// assert!(subgraph.contains_node(&atk_id));
    /// assert!(subgraph.contains_node(&str_id));
    /// assert!(!subgraph.contains_node(&hp_id)); // HP not reachable from ATK
    /// ```
    pub fn subgraph_for_targets(&self, targets: &[StatId]) -> StatGraph {
        let mut subgraph = StatGraph::new();
        let mut visited = std::collections::HashSet::new();

        // Reverse DFS from targets to find all dependencies
        let mut stack: Vec<StatId> = targets.to_vec();

        while let Some(stat_id) = stack.pop() {
            if visited.contains(&stat_id) {
                continue;
            }
            visited.insert(stat_id.clone());

            // Add node to subgraph
            if let Some(&node_idx) = self.node_map.get(&stat_id) {
                subgraph.add_node(stat_id.clone());

                // Find all dependencies (nodes that this stat depends on)
                // In our graph, edges go from dependency to dependent
                // So we need to find incoming edges (dependencies of stat_id)
                for neighbor_idx in self
                    .graph
                    .neighbors_directed(node_idx, petgraph::Direction::Incoming)
                {
                    let dep_stat_id = self.graph[neighbor_idx].clone();
                    if !visited.contains(&dep_stat_id) {
                        stack.push(dep_stat_id.clone());
                    }
                    // Add edge to subgraph (dependency -> dependent)
                    subgraph.add_edge(stat_id.clone(), dep_stat_id);
                }
            }
        }

        subgraph
    }
}

impl Default for StatGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graph_add_nodes() {
        let mut graph = StatGraph::new();
        let hp = StatId::from_str("HP");
        let atk = StatId::from_str("ATK");

        graph.add_node(hp.clone());
        graph.add_node(atk.clone());

        assert!(graph.contains_node(&hp));
        assert!(graph.contains_node(&atk));
    }

    #[test]
    fn test_graph_add_edge() {
        let mut graph = StatGraph::new();
        let atk = StatId::from_str("ATK");
        let str = StatId::from_str("STR");

        // ATK depends on STR
        graph.add_edge(atk.clone(), str.clone());

        assert!(graph.contains_node(&atk));
        assert!(graph.contains_node(&str));
    }

    #[test]
    fn test_graph_no_cycle() {
        let mut graph = StatGraph::new();
        let str = StatId::from_str("STR");
        let atk = StatId::from_str("ATK");
        let dps = StatId::from_str("DPS");

        // STR -> ATK -> DPS (linear chain, no cycle)
        graph.add_edge(atk.clone(), str.clone());
        graph.add_edge(dps.clone(), atk.clone());

        assert!(graph.detect_cycles().is_ok());
    }

    #[test]
    fn test_graph_detect_cycle() {
        let mut graph = StatGraph::new();
        let a = StatId::from_str("A");
        let b = StatId::from_str("B");
        let c = StatId::from_str("C");

        // Create cycle: A -> B -> C -> A
        graph.add_edge(b.clone(), a.clone());
        graph.add_edge(c.clone(), b.clone());
        graph.add_edge(a.clone(), c.clone());

        assert!(graph.detect_cycles().is_err());
    }

    #[test]
    fn test_topological_sort() {
        let mut graph = StatGraph::new();
        let str = StatId::from_str("STR");
        let dex = StatId::from_str("DEX");
        let atk = StatId::from_str("ATK");
        let crit = StatId::from_str("CRIT");

        // STR -> ATK, DEX -> CRIT
        graph.add_edge(atk.clone(), str.clone());
        graph.add_edge(crit.clone(), dex.clone());

        let sorted = graph.topological_sort().unwrap();

        // STR and DEX should come before ATK and CRIT
        let str_pos = sorted.iter().position(|s| s == &str).unwrap();
        let dex_pos = sorted.iter().position(|s| s == &dex).unwrap();
        let atk_pos = sorted.iter().position(|s| s == &atk).unwrap();
        let crit_pos = sorted.iter().position(|s| s == &crit).unwrap();

        assert!(str_pos < atk_pos);
        assert!(dex_pos < crit_pos);
    }

    #[test]
    fn test_subgraph_for_targets() {
        let mut graph = StatGraph::new();
        let str_id = StatId::from_str("STR");
        let dex_id = StatId::from_str("DEX");
        let atk_id = StatId::from_str("ATK");
        let crit_id = StatId::from_str("CRIT");
        let hp_id = StatId::from_str("HP");

        // ATK depends on STR
        graph.add_edge(atk_id.clone(), str_id.clone());
        // CRIT depends on DEX
        graph.add_edge(crit_id.clone(), dex_id.clone());
        // HP has no dependencies

        // Extract subgraph for ATK
        let subgraph = graph.subgraph_for_targets(&[atk_id.clone()]);

        // Should contain ATK and STR
        assert!(subgraph.contains_node(&atk_id));
        assert!(subgraph.contains_node(&str_id));

        // Should NOT contain CRIT, DEX, or HP
        assert!(!subgraph.contains_node(&crit_id));
        assert!(!subgraph.contains_node(&dex_id));
        assert!(!subgraph.contains_node(&hp_id));
    }

    #[test]
    fn test_subgraph_for_multiple_targets() {
        let mut graph = StatGraph::new();
        let str_id = StatId::from_str("STR");
        let atk_id = StatId::from_str("ATK");
        let dps_id = StatId::from_str("DPS");
        let hp_id = StatId::from_str("HP");

        // ATK depends on STR
        graph.add_edge(atk_id.clone(), str_id.clone());
        // DPS depends on ATK (which depends on STR)
        graph.add_edge(dps_id.clone(), atk_id.clone());

        // Extract subgraph for ATK and DPS
        let subgraph = graph.subgraph_for_targets(&[atk_id.clone(), dps_id.clone()]);

        // Should contain all three (STR is dependency of both)
        assert!(subgraph.contains_node(&atk_id));
        assert!(subgraph.contains_node(&dps_id));
        assert!(subgraph.contains_node(&str_id));

        // Should NOT contain HP
        assert!(!subgraph.contains_node(&hp_id));
    }

    #[test]
    fn test_subgraph_for_targets_with_shared_dependency() {
        let mut graph = StatGraph::new();
        let base_id = StatId::from_str("BASE");
        let mid1_id = StatId::from_str("MID1");
        let mid2_id = StatId::from_str("MID2");
        let top1_id = StatId::from_str("TOP1");
        let top2_id = StatId::from_str("TOP2");

        // Both MID1 and MID2 depend on BASE
        graph.add_edge(mid1_id.clone(), base_id.clone());
        graph.add_edge(mid2_id.clone(), base_id.clone());

        // TOP1 depends on MID1, TOP2 depends on MID2
        graph.add_edge(top1_id.clone(), mid1_id.clone());
        graph.add_edge(top2_id.clone(), mid2_id.clone());

        // Extract subgraph for TOP1 only
        let subgraph = graph.subgraph_for_targets(&[top1_id.clone()]);

        // Should contain TOP1, MID1, and BASE
        assert!(subgraph.contains_node(&top1_id));
        assert!(subgraph.contains_node(&mid1_id));
        assert!(subgraph.contains_node(&base_id));

        // Should NOT contain TOP2 or MID2
        assert!(!subgraph.contains_node(&top2_id));
        assert!(!subgraph.contains_node(&mid2_id));
    }

    #[test]
    fn test_subgraph_for_targets_empty() {
        let graph = StatGraph::new();
        let subgraph = graph.subgraph_for_targets(&[]);
        assert_eq!(subgraph.nodes().len(), 0);
    }

    #[test]
    fn test_subgraph_for_targets_nonexistent() {
        let mut graph = StatGraph::new();
        let existing_id = StatId::from_str("EXISTING");
        let nonexistent_id = StatId::from_str("NONEXISTENT");

        graph.add_node(existing_id.clone());

        // Extract subgraph for non-existent node
        let subgraph = graph.subgraph_for_targets(&[nonexistent_id.clone()]);

        // Should not contain the non-existent node
        assert!(!subgraph.contains_node(&nonexistent_id));
    }

    #[test]
    fn test_graph_nodes() {
        let mut graph = StatGraph::new();
        let hp = StatId::from_str("HP");
        let atk = StatId::from_str("ATK");
        let mp = StatId::from_str("MP");

        graph.add_node(hp.clone());
        graph.add_node(atk.clone());
        graph.add_node(mp.clone());

        let nodes = graph.nodes();
        assert_eq!(nodes.len(), 3);
        assert!(nodes.contains(&hp));
        assert!(nodes.contains(&atk));
        assert!(nodes.contains(&mp));
    }

    #[test]
    fn test_graph_duplicate_nodes() {
        let mut graph = StatGraph::new();
        let hp = StatId::from_str("HP");

        let idx1 = graph.add_node(hp.clone());
        let idx2 = graph.add_node(hp.clone());

        // Should return the same node index
        assert_eq!(idx1, idx2);
        assert_eq!(graph.nodes().len(), 1);
    }

    #[test]
    fn test_graph_complex_cycle() {
        let mut graph = StatGraph::new();
        let a = StatId::from_str("A");
        let b = StatId::from_str("B");
        let c = StatId::from_str("C");
        let d = StatId::from_str("D");

        // Create cycle: A -> B -> C -> D -> A
        graph.add_edge(b.clone(), a.clone());
        graph.add_edge(c.clone(), b.clone());
        graph.add_edge(d.clone(), c.clone());
        graph.add_edge(a.clone(), d.clone());

        assert!(graph.detect_cycles().is_err());
    }

    #[test]
    fn test_graph_self_cycle() {
        let mut graph = StatGraph::new();
        let a = StatId::from_str("A");

        // Self-cycle: A depends on itself
        graph.add_edge(a.clone(), a.clone());

        assert!(graph.detect_cycles().is_err());
    }

    #[test]
    fn test_graph_multiple_independent_cycles() {
        let mut graph = StatGraph::new();
        let a1 = StatId::from_str("A1");
        let b1 = StatId::from_str("B1");
        let a2 = StatId::from_str("A2");
        let b2 = StatId::from_str("B2");

        // Two independent cycles
        graph.add_edge(b1.clone(), a1.clone());
        graph.add_edge(a1.clone(), b1.clone());

        graph.add_edge(b2.clone(), a2.clone());
        graph.add_edge(a2.clone(), b2.clone());

        assert!(graph.detect_cycles().is_err());
    }

    #[test]
    fn test_cycle_path_simple_2_node() {
        let mut graph = StatGraph::new();
        let a = StatId::from_str("A");
        let b = StatId::from_str("B");

        // Create cycle: A -> B -> A
        graph.add_edge(b.clone(), a.clone());
        graph.add_edge(a.clone(), b.clone());

        let result = graph.detect_cycles();
        assert!(result.is_err());
        if let Err(StatError::Cycle { path }) = result {
            // Should be [A, B, A] or [B, A, B] depending on DFS start
            assert_eq!(path.len(), 3);
            assert_eq!(path[0], path[2]); // First and last should be same
            assert!(path.contains(&a));
            assert!(path.contains(&b));
        } else {
            panic!("Expected Cycle error");
        }
    }

    #[test]
    fn test_cycle_path_3_node() {
        let mut graph = StatGraph::new();
        let a = StatId::from_str("A");
        let b = StatId::from_str("B");
        let c = StatId::from_str("C");

        // Create cycle: A -> B -> C -> A
        graph.add_edge(b.clone(), a.clone());
        graph.add_edge(c.clone(), b.clone());
        graph.add_edge(a.clone(), c.clone());

        let result = graph.detect_cycles();
        assert!(result.is_err());
        if let Err(StatError::Cycle { path }) = result {
            // Should be [A, B, C, A] or similar
            assert_eq!(path.len(), 4);
            assert_eq!(path[0], path[3]); // First and last should be same
            assert!(path.contains(&a));
            assert!(path.contains(&b));
            assert!(path.contains(&c));
        } else {
            panic!("Expected Cycle error");
        }
    }

    #[test]
    fn test_cycle_path_4_node() {
        let mut graph = StatGraph::new();
        let a = StatId::from_str("A");
        let b = StatId::from_str("B");
        let c = StatId::from_str("C");
        let d = StatId::from_str("D");

        // Create cycle: A -> B -> C -> D -> A
        graph.add_edge(b.clone(), a.clone());
        graph.add_edge(c.clone(), b.clone());
        graph.add_edge(d.clone(), c.clone());
        graph.add_edge(a.clone(), d.clone());

        let result = graph.detect_cycles();
        assert!(result.is_err());
        if let Err(StatError::Cycle { path }) = result {
            // Should be [A, B, C, D, A] or similar
            assert_eq!(path.len(), 5);
            assert_eq!(path[0], path[4]); // First and last should be same
            assert!(path.contains(&a));
            assert!(path.contains(&b));
            assert!(path.contains(&c));
            assert!(path.contains(&d));
        } else {
            panic!("Expected Cycle error");
        }
    }

    #[test]
    fn test_cycle_path_self_cycle() {
        let mut graph = StatGraph::new();
        let a = StatId::from_str("A");

        // Self-cycle: A depends on itself
        graph.add_edge(a.clone(), a.clone());

        let result = graph.detect_cycles();
        assert!(result.is_err());
        if let Err(StatError::Cycle { path }) = result {
            // Should be [A, A]
            assert_eq!(path.len(), 2);
            assert_eq!(path[0], a);
            assert_eq!(path[1], a);
        } else {
            panic!("Expected Cycle error");
        }
    }

    #[test]
    fn test_cycle_path_excludes_non_cycle_nodes() {
        let mut graph = StatGraph::new();
        let x = StatId::from_str("X");
        let y = StatId::from_str("Y");
        let a = StatId::from_str("A");
        let b = StatId::from_str("B");
        let c = StatId::from_str("C");

        // X -> Y -> A -> B -> C -> A (cycle)
        // X and Y are not part of the cycle
        graph.add_edge(y.clone(), x.clone());
        graph.add_edge(a.clone(), y.clone());
        graph.add_edge(b.clone(), a.clone());
        graph.add_edge(c.clone(), b.clone());
        graph.add_edge(a.clone(), c.clone()); // Creates cycle A -> B -> C -> A

        let result = graph.detect_cycles();
        assert!(result.is_err());
        if let Err(StatError::Cycle { path }) = result {
            // Should only contain A, B, C (not X, Y)
            assert!(!path.contains(&x));
            assert!(!path.contains(&y));
            assert!(path.contains(&a));
            assert!(path.contains(&b));
            assert!(path.contains(&c));
            // Path should be closed loop
            assert_eq!(path[0], path[path.len() - 1]);
        } else {
            panic!("Expected Cycle error");
        }
    }

    #[test]
    fn test_cycle_path_deterministic() {
        let mut graph = StatGraph::new();
        let a = StatId::from_str("A");
        let b = StatId::from_str("B");
        let c = StatId::from_str("C");

        // Create cycle: A -> B -> C -> A
        graph.add_edge(b.clone(), a.clone());
        graph.add_edge(c.clone(), b.clone());
        graph.add_edge(a.clone(), c.clone());

        // Run multiple times to ensure deterministic
        let result1 = graph.detect_cycles();
        let result2 = graph.detect_cycles();

        if let (Err(StatError::Cycle { path: path1 }), Err(StatError::Cycle { path: path2 })) = (result1, result2) {
            // Paths should be the same (deterministic)
            assert_eq!(path1, path2);
        } else {
            panic!("Expected Cycle errors");
        }
    }
}
