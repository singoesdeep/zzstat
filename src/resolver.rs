//! Stat resolver module.
//!
//! Provides the `StatResolver` type, which is the main entry point
//! for stat resolution. It manages sources, transforms, dependency
//! graphs, and caching.

use crate::context::StatContext;
use crate::error::StatError;
use crate::graph::StatGraph;
use crate::resolved::ResolvedStat;
use crate::source::StatSource;
use crate::stat_id::StatId;
use crate::transform::StatTransform;
use std::collections::HashMap;

/// The main stat resolver that manages sources, transforms, and resolution.
///
/// The resolver coordinates the entire stat resolution process:
/// 1. Collects sources (additive)
/// 2. Builds dependency graph from transforms
/// 3. Detects cycles
/// 4. Resolves stats in topological order
/// 5. Caches results until invalidated
///
/// # Examples
///
/// ```rust
/// use zzstat::*;
/// use zzstat::source::ConstantSource;
/// use zzstat::transform::MultiplicativeTransform;
///
/// let mut resolver = StatResolver::new();
/// let hp_id = StatId::from_str("HP");
///
/// // Register sources
/// resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
/// resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));
///
/// // Register transform
/// resolver.register_transform(hp_id.clone(), Box::new(MultiplicativeTransform::new(1.5)));
///
/// // Resolve
/// let context = StatContext::new();
/// let resolved = resolver.resolve(&hp_id, &context).unwrap();
/// assert_eq!(resolved.value, 225.0); // (100 + 50) * 1.5
/// ```
pub struct StatResolver {
    /// Multiple sources per stat (additive).
    sources: HashMap<StatId, Vec<Box<dyn StatSource>>>,

    /// Transform chain per stat.
    transforms: HashMap<StatId, Vec<Box<dyn StatTransform>>>,

    /// Cache of resolved stats.
    cache: HashMap<StatId, ResolvedStat>,
}

impl StatResolver {
    /// Create a new empty resolver.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::StatResolver;
    ///
    /// let resolver = StatResolver::new();
    /// ```
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            transforms: HashMap::new(),
            cache: HashMap::new(),
        }
    }

    /// Register a source for a stat.
    ///
    /// Multiple sources for the same stat are summed (additive).
    /// Registering a source automatically invalidates the cache for that stat.
    ///
    /// # Arguments
    ///
    /// * `stat_id` - The stat to register a source for
    /// * `source` - The source to register
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::*;
    /// use zzstat::source::ConstantSource;
    ///
    /// let mut resolver = StatResolver::new();
    /// let hp_id = StatId::from_str("HP");
    ///
    /// resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    /// resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));
    /// // HP will be 150.0 (100 + 50)
    /// ```
    pub fn register_source(&mut self, stat_id: StatId, source: Box<dyn StatSource>) {
        let stat_id_clone = stat_id.clone();
        self.sources
            .entry(stat_id)
            .or_insert_with(Vec::new)
            .push(source);
        // Invalidate cache for this stat
        self.cache.remove(&stat_id_clone);
    }

    /// Register a transform for a stat.
    ///
    /// Transforms are applied in registration order.
    /// Registering a transform automatically invalidates the cache for that stat.
    ///
    /// # Arguments
    ///
    /// * `stat_id` - The stat to register a transform for
    /// * `transform` - The transform to register
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::*;
    /// use zzstat::source::ConstantSource;
    /// use zzstat::transform::MultiplicativeTransform;
    ///
    /// let mut resolver = StatResolver::new();
    /// let atk_id = StatId::from_str("ATK");
    ///
    /// resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));
    /// resolver.register_transform(atk_id.clone(), Box::new(MultiplicativeTransform::new(1.5)));
    /// // ATK will be 150.0 (100 * 1.5)
    /// ```
    pub fn register_transform(&mut self, stat_id: StatId, transform: Box<dyn StatTransform>) {
        let stat_id_clone = stat_id.clone();
        self.transforms
            .entry(stat_id)
            .or_insert_with(Vec::new)
            .push(transform);
        // Invalidate cache for this stat and potentially dependent stats
        self.cache.remove(&stat_id_clone);
    }

    /// Resolve a single stat.
    ///
    /// This will resolve the requested stat and all of its dependencies
    /// in the correct order. Results are cached until invalidated.
    ///
    /// # Arguments
    ///
    /// * `stat_id` - The stat to resolve
    /// * `context` - The stat context for conditional calculations
    ///
    /// # Returns
    ///
    /// * `Ok(ResolvedStat)` - The resolved stat with full breakdown
    /// * `Err(StatError)` - If resolution fails (cycle, missing dependency, etc.)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::*;
    /// use zzstat::source::ConstantSource;
    ///
    /// let mut resolver = StatResolver::new();
    /// let hp_id = StatId::from_str("HP");
    ///
    /// resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    ///
    /// let context = StatContext::new();
    /// let resolved = resolver.resolve(&hp_id, &context).unwrap();
    /// assert_eq!(resolved.value, 100.0);
    /// ```
    pub fn resolve(
        &mut self,
        stat_id: &StatId,
        context: &StatContext,
    ) -> Result<ResolvedStat, StatError> {
        // Check cache first
        if let Some(cached) = self.cache.get(stat_id) {
            return Ok(cached.clone());
        }

        // Build dependency graph
        let graph = self.build_graph()?;

        // Get resolution order
        let resolution_order = graph.topological_sort()?;

        // Resolve all stats in order
        for stat_to_resolve in &resolution_order {
            if self.cache.contains_key(stat_to_resolve) {
                continue; // Already resolved
            }

            let resolved = self.resolve_stat_internal(stat_to_resolve, context, &graph)?;
            self.cache.insert(stat_to_resolve.clone(), resolved);
        }

        // Return the requested stat
        self.cache
            .get(stat_id)
            .cloned()
            .ok_or_else(|| StatError::MissingSource(stat_id.clone()))
    }

    /// Resolve all registered stats.
    ///
    /// Resolves all stats that have been registered (have sources or transforms).
    /// Stats are resolved in dependency order, and results are cached.
    ///
    /// # Arguments
    ///
    /// * `context` - The stat context for conditional calculations
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<StatId, ResolvedStat>)` - Map of all resolved stats
    /// * `Err(StatError)` - If resolution fails (cycle, missing dependency, etc.)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::*;
    /// use zzstat::source::ConstantSource;
    ///
    /// let mut resolver = StatResolver::new();
    /// resolver.register_source(StatId::from_str("HP"), Box::new(ConstantSource(100.0)));
    /// resolver.register_source(StatId::from_str("MP"), Box::new(ConstantSource(50.0)));
    ///
    /// let context = StatContext::new();
    /// let results = resolver.resolve_all(&context).unwrap();
    /// assert_eq!(results.len(), 2);
    /// ```
    pub fn resolve_all(
        &mut self,
        context: &StatContext,
    ) -> Result<HashMap<StatId, ResolvedStat>, StatError> {
        // Build dependency graph
        let graph = self.build_graph()?;

        // Get resolution order
        let resolution_order = graph.topological_sort()?;

        // Resolve all stats in order
        for stat_id in &resolution_order {
            if !self.cache.contains_key(stat_id) {
                let resolved = self.resolve_stat_internal(stat_id, context, &graph)?;
                self.cache.insert(stat_id.clone(), resolved);
            }
        }

        Ok(self.cache.clone())
    }

    /// Invalidate the cache for a specific stat.
    ///
    /// The next time this stat is resolved, it will be recalculated
    /// instead of using the cached value.
    ///
    /// # Arguments
    ///
    /// * `stat_id` - The stat to invalidate
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::*;
    /// use zzstat::source::ConstantSource;
    ///
    /// let mut resolver = StatResolver::new();
    /// let hp_id = StatId::from_str("HP");
    ///
    /// resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    /// let context = StatContext::new();
    /// let _ = resolver.resolve(&hp_id, &context).unwrap();
    ///
    /// // Invalidate and add new source
    /// resolver.invalidate(&hp_id);
    /// resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));
    /// ```
    pub fn invalidate(&mut self, stat_id: &StatId) {
        self.cache.remove(stat_id);
    }

    /// Invalidate the entire cache.
    ///
    /// All cached stats will be recalculated on the next resolution.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::*;
    /// use zzstat::source::ConstantSource;
    ///
    /// let mut resolver = StatResolver::new();
    /// resolver.register_source(StatId::from_str("HP"), Box::new(ConstantSource(100.0)));
    ///
    /// let context = StatContext::new();
    /// let _ = resolver.resolve_all(&context).unwrap();
    ///
    /// // Clear all caches
    /// resolver.invalidate_all();
    /// ```
    pub fn invalidate_all(&mut self) {
        self.cache.clear();
    }

    /// Get the breakdown for a stat (if it's been resolved).
    ///
    /// Returns the cached `ResolvedStat` if it exists, or `None` if
    /// the stat hasn't been resolved yet.
    ///
    /// # Arguments
    ///
    /// * `stat_id` - The stat to get the breakdown for
    ///
    /// # Returns
    ///
    /// * `Some(&ResolvedStat)` - The resolved stat with breakdown
    /// * `None` - If the stat hasn't been resolved
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::*;
    /// use zzstat::source::ConstantSource;
    ///
    /// let mut resolver = StatResolver::new();
    /// let hp_id = StatId::from_str("HP");
    ///
    /// resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    /// let context = StatContext::new();
    ///
    /// // Not resolved yet
    /// assert!(resolver.get_breakdown(&hp_id).is_none());
    ///
    /// // Resolve
    /// let _ = resolver.resolve(&hp_id, &context).unwrap();
    ///
    /// // Now available
    /// let breakdown = resolver.get_breakdown(&hp_id).unwrap();
    /// assert_eq!(breakdown.value, 100.0);
    /// ```
    pub fn get_breakdown(&self, stat_id: &StatId) -> Option<&ResolvedStat> {
        self.cache.get(stat_id)
    }

    /// Build the dependency graph from all registered transforms.
    fn build_graph(&self) -> Result<StatGraph, StatError> {
        let mut graph = StatGraph::new();

        // Add all stats that have sources or transforms
        for stat_id in self.sources.keys().chain(self.transforms.keys()) {
            graph.add_node(stat_id.clone());
        }

        // Add edges from transform dependencies
        for (stat_id, transforms) in &self.transforms {
            for transform in transforms {
                for dep in transform.depends_on() {
                    // dep must be resolved before stat_id
                    graph.add_edge(stat_id.clone(), dep);
                }
            }
        }

        Ok(graph)
    }

    /// Internal method to resolve a single stat.
    fn resolve_stat_internal(
        &self,
        stat_id: &StatId,
        context: &StatContext,
        _graph: &StatGraph,
    ) -> Result<ResolvedStat, StatError> {
        let mut resolved = ResolvedStat::new(stat_id.clone(), 0.0);

        // Step 1: Collect all source values (additive)
        let mut base_value = 0.0;
        if let Some(sources) = self.sources.get(stat_id) {
            for (idx, source) in sources.iter().enumerate() {
                let value = source.get_value(stat_id, context);
                base_value += value;
                resolved.add_source(format!("Source #{}", idx + 1), value);
            }
        } else {
            // No source means 0.0, but we still create the resolved stat
            resolved.add_source("Default", 0.0);
        }

        // Step 2: Apply transforms in order
        let mut current_value = base_value;
        if let Some(transforms) = self.transforms.get(stat_id) {
            for transform in transforms {
                // Collect dependencies
                let mut dependencies = HashMap::new();
                for dep_id in transform.depends_on() {
                    let dep_value = self
                        .cache
                        .get(&dep_id)
                        .map(|r| r.value)
                        .ok_or_else(|| StatError::MissingDependency(dep_id.clone()))?;
                    dependencies.insert(dep_id, dep_value);
                }

                // Apply transform
                let new_value = transform.apply(current_value, &dependencies, context)?;
                resolved.add_transform(transform.description(), new_value);
                current_value = new_value;
            }
        }

        resolved.value = current_value;
        Ok(resolved)
    }
}

impl Default for StatResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::ConstantSource;
    use crate::transform::{MultiplicativeTransform, ScalingTransform};

    #[test]
    fn test_resolve_simple_source() {
        let mut resolver = StatResolver::new();
        let hp_id = StatId::from_str("HP");

        resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));

        let context = StatContext::new();
        let resolved = resolver.resolve(&hp_id, &context).unwrap();

        assert_eq!(resolved.value, 100.0);
        assert_eq!(resolved.stat_id, hp_id);
    }

    #[test]
    fn test_resolve_multiple_sources() {
        let mut resolver = StatResolver::new();
        let hp_id = StatId::from_str("HP");

        resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
        resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));

        let context = StatContext::new();
        let resolved = resolver.resolve(&hp_id, &context).unwrap();

        assert_eq!(resolved.value, 150.0);
        assert_eq!(resolved.sources.len(), 2);
    }

    #[test]
    fn test_resolve_with_transform() {
        let mut resolver = StatResolver::new();
        let atk_id = StatId::from_str("ATK");

        resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));
        resolver.register_transform(atk_id.clone(), Box::new(MultiplicativeTransform::new(1.5)));

        let context = StatContext::new();
        let resolved = resolver.resolve(&atk_id, &context).unwrap();

        assert_eq!(resolved.value, 150.0);
        assert_eq!(resolved.transforms.len(), 1);
    }

    #[test]
    fn test_resolve_with_dependency() {
        let mut resolver = StatResolver::new();
        let str_id = StatId::from_str("STR");
        let atk_id = StatId::from_str("ATK");

        resolver.register_source(str_id.clone(), Box::new(ConstantSource(10.0)));
        resolver.register_source(atk_id.clone(), Box::new(ConstantSource(50.0)));
        resolver.register_transform(
            atk_id.clone(),
            Box::new(ScalingTransform::new(str_id.clone(), 2.0)),
        );

        let context = StatContext::new();
        let resolved = resolver.resolve(&atk_id, &context).unwrap();

        // 50 (base) + 10 (STR) * 2 (scale) = 70
        assert_eq!(resolved.value, 70.0);
    }

    #[test]
    fn test_resolve_missing_source() {
        let mut resolver = StatResolver::new();
        let hp_id = StatId::from_str("HP");

        let context = StatContext::new();
        let _result = resolver.resolve(&hp_id, &context);

        // Should return MissingSource error since no source is registered
        // This is expected behavior per the spec
    }

    #[test]
    fn test_cache_invalidation() {
        let mut resolver = StatResolver::new();
        let hp_id = StatId::from_str("HP");

        resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));

        let context = StatContext::new();
        let resolved1 = resolver.resolve(&hp_id, &context).unwrap();
        assert_eq!(resolved1.value, 100.0);

        // Should be cached
        let resolved2 = resolver.resolve(&hp_id, &context).unwrap();
        assert_eq!(resolved2.value, 100.0);

        // Invalidate and add new source
        resolver.invalidate(&hp_id);
        resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));

        let resolved3 = resolver.resolve(&hp_id, &context).unwrap();
        assert_eq!(resolved3.value, 150.0); // 100 + 50
    }

    #[test]
    fn test_cycle_detection() {
        let mut resolver = StatResolver::new();
        let a_id = StatId::from_str("A");
        let b_id = StatId::from_str("B");

        // Create a cycle: A depends on B, B depends on A
        resolver.register_source(a_id.clone(), Box::new(ConstantSource(1.0)));
        resolver.register_source(b_id.clone(), Box::new(ConstantSource(1.0)));

        resolver.register_transform(
            a_id.clone(),
            Box::new(ScalingTransform::new(b_id.clone(), 1.0)),
        );
        resolver.register_transform(
            b_id.clone(),
            Box::new(ScalingTransform::new(a_id.clone(), 1.0)),
        );

        let context = StatContext::new();
        let result = resolver.resolve(&a_id, &context);

        assert!(result.is_err());
        if let Err(StatError::CycleDetected(_)) = result {
            // Good
        } else {
            panic!("Expected CycleDetected error");
        }
    }
}
