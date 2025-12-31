//! Dependency graph module.
//!
//! Provides the `StatGraph` type, which represents stat dependencies
//! as a directed acyclic graph (DAG). Used by the resolver to determine
//! the correct order for stat resolution.

use crate::error::StatError;
use crate::stat_id::StatId;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
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
    /// * `Err(StatError::CycleDetected)` with the cycle path if a cycle is found
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
        let mut cycle_path = Vec::new();

        for node_idx in self.graph.node_indices() {
            if !visited.contains(&node_idx) {
                if self.dfs_cycle_detect(
                    node_idx,
                    &mut visited,
                    &mut rec_stack,
                    &mut cycle_path,
                ) {
                    return Err(StatError::CycleDetected(cycle_path));
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
    ) -> bool {
        visited.insert(node);
        rec_stack.insert(node);
        cycle_path.push(self.graph[node].clone());

        for neighbor in self.graph.neighbors_directed(node, petgraph::Direction::Outgoing) {
            if !visited.contains(&neighbor) {
                if self.dfs_cycle_detect(neighbor, visited, rec_stack, cycle_path) {
                    return true;
                }
            } else if rec_stack.contains(&neighbor) {
                // Cycle detected
                cycle_path.push(self.graph[neighbor].clone());
                return true;
            }
        }

        rec_stack.remove(&node);
        cycle_path.pop();
        false
    }

    /// Get a topological sort of all nodes.
    ///
    /// This gives the order in which stats should be resolved.
    /// Dependencies are guaranteed to come before dependents.
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<StatId>)` - The resolution order (dependencies first)
    /// * `Err(StatError::CycleDetected)` - If a cycle is detected
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
            Ok(indices) => {
                Ok(indices
                    .into_iter()
                    .map(|idx| self.graph[idx].clone())
                    .collect())
            }
            Err(cycle) => {
                // This shouldn't happen if detect_cycles passed, but handle it anyway
                let cycle_path = vec![self.graph[cycle.node_id()].clone()];
                Err(StatError::CycleDetected(cycle_path))
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
}

