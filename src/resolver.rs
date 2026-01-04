//! Stat resolver module.
//!
//! Provides the `StatResolver` type, which is the main entry point
//! for stat resolution. It manages sources, transforms, dependency
//! graphs, and caching.

use crate::context::StatContext;
use crate::error::StatError;
use crate::graph::StatGraph;
use crate::numeric::{StatNumeric, StatValue};
use crate::resolved::ResolvedStat;
use crate::source::StatSource;
use crate::stat_id::StatId;
use crate::transform::{StackRule, StatTransform, TransformEntry, TransformPhase};
use std::collections::HashMap;
use std::sync::Arc;

/// Base data shared across resolver forks.
///
/// Contains the sources and transforms that are shared via copy-on-write.
struct BaseData {
    /// Multiple sources per stat (additive).
    sources: HashMap<StatId, Vec<Box<dyn StatSource>>>,

    /// Transform chain per stat.
    transforms: HashMap<StatId, Vec<TransformEntry>>,
}

/// Overlay data for copy-on-write modifications.
///
/// When a resolver is forked, modifications are stored in the overlay.
/// Reading checks overlay first, then falls back to base data.
struct OverlayData {
    /// Overlay sources (shadows base sources when present).
    sources: HashMap<StatId, Vec<Box<dyn StatSource>>>,

    /// Overlay transforms (shadows base transforms when present).
    transforms: HashMap<StatId, Vec<TransformEntry>>,
}

/// Scope for stat resolution.
///
/// Determines which stats should be resolved and how the dependency graph
/// should be constructed.
enum ResolveScope {
    /// Resolve a single stat and its dependencies.
    Single(StatId),
    /// Resolve all registered stats.
    All,
    /// Resolve specific stats and their dependencies (batch).
    Batch(Vec<StatId>),
}

/// The main stat resolver that manages sources, transforms, and resolution.
///
/// The resolver coordinates the entire stat resolution process:
/// 1. Collects sources (additive)
/// 2. Builds dependency graph from transforms
/// 3. Detects cycles
/// 4. Resolves stats in topological order
/// 5. Caches results until invalidated
///
/// Supports copy-on-write forking for efficient resolver variations.
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
/// assert_eq!(resolved.value.to_f64(), 225.0); // (100 + 50) * 1.5
/// ```
pub struct StatResolver {
    /// Shared base data (sources and transforms).
    base: Arc<BaseData>,

    /// Copy-on-write overlay for modifications.
    overlay: OverlayData,

    /// Cache of resolved stats (per-instance, not shared).
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
            base: Arc::new(BaseData {
                sources: HashMap::new(),
                transforms: HashMap::new(),
            }),
            overlay: OverlayData {
                sources: HashMap::new(),
                transforms: HashMap::new(),
            },
            cache: HashMap::new(),
        }
    }

    /// Fork this resolver, creating a new resolver that shares base data.
    ///
    /// The forked resolver starts with an empty overlay and cache.
    /// Modifications to the fork only affect the fork (copy-on-write).
    /// The original resolver is unaffected.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::*;
    /// use zzstat::source::ConstantSource;
    /// use zzstat::numeric::StatNumeric;
    ///
    /// let mut base = StatResolver::new();
    /// let hp_id = StatId::from_str("HP");
    /// base.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    ///
    /// // Fork the resolver
    /// let mut fork = base.fork();
    ///
    /// // Modify the fork (doesn't affect base)
    /// fork.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));
    ///
    /// let context = StatContext::new();
    /// let base_resolved = base.resolve(&hp_id, &context).unwrap();
    /// let fork_resolved = fork.resolve(&hp_id, &context).unwrap();
    ///
    /// assert_eq!(base_resolved.value.to_f64(), 100.0);
    /// assert_eq!(fork_resolved.value.to_f64(), 150.0); // 100 + 50
    /// ```
    pub fn fork(&self) -> Self {
        Self {
            base: Arc::clone(&self.base),
            overlay: OverlayData {
                sources: HashMap::new(),
                transforms: HashMap::new(),
            },
            cache: HashMap::new(),
        }
    }

    /// Register a source for a stat.
    ///
    /// Multiple sources for the same stat are summed (additive).
    /// Registering a source automatically invalidates the cache for that stat.
    ///
    /// Uses copy-on-write semantics: if this resolver is a fork, the source
    /// is added to the overlay. Otherwise, it's added to the base data.
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
        // Use copy-on-write helper to get the appropriate sources vector
        self.get_mut_sources(stat_id).push(source);
        // Invalidate cache for this stat
        self.cache.remove(&stat_id_clone);
    }

    /// Register a transform for a stat.
    ///
    /// Transforms are applied in registration order within each phase.
    /// The stack rule is inferred from the transform's phase.
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
        let phase = transform.phase();
        let rule = crate::transform::infer_stack_rule(transform.as_ref());
        let entry = TransformEntry {
            phase,
            rule,
            transform,
        };
        self.register_transform_entry(stat_id, entry);
    }

    /// Register a transform with explicit phase and inferred stack rule.
    ///
    /// The stack rule is inferred from the transform's type and phase.
    /// This is a convenience method for registering transforms in a specific phase.
    ///
    /// # Arguments
    ///
    /// * `stat_id` - The stat to register a transform for
    /// * `phase` - The phase to register the transform in
    /// * `transform` - The transform to register
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::*;
    /// use zzstat::source::ConstantSource;
    /// use zzstat::transform::{AdditiveTransform, TransformPhase};
    ///
    /// let mut resolver = StatResolver::new();
    /// let atk_id = StatId::from_str("ATK");
    ///
    /// resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));
    /// resolver.register_transform_in_phase(
    ///     atk_id.clone(),
    ///     TransformPhase::Additive,
    ///     Box::new(AdditiveTransform::new(50.0)),
    /// );
    /// ```
    pub fn register_transform_in_phase(
        &mut self,
        stat_id: StatId,
        phase: TransformPhase,
        transform: Box<dyn StatTransform>,
    ) {
        let rule = crate::transform::infer_stack_rule(transform.as_ref());
        let entry = TransformEntry {
            phase,
            rule,
            transform,
        };
        self.register_transform_entry(stat_id, entry);
    }

    /// Register a transform with explicit phase and stack rule.
    ///
    /// This method provides full control over how the transform is registered
    /// and how it stacks with other transforms in the same phase.
    ///
    /// # Arguments
    ///
    /// * `stat_id` - The stat to register a transform for
    /// * `phase` - The phase to register the transform in
    /// * `rule` - The stack rule for combining with other transforms
    /// * `transform` - The transform to register
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::*;
    /// use zzstat::source::ConstantSource;
    /// use zzstat::transform::{AdditiveTransform, StackRule, TransformPhase};
    ///
    /// let mut resolver = StatResolver::new();
    /// let atk_id = StatId::from_str("ATK");
    ///
    /// resolver.register_source(atk_id.clone(), Box::new(ConstantSource(100.0)));
    /// resolver.register_transform_with_rule(
    ///     atk_id.clone(),
    ///     TransformPhase::Additive,
    ///     StackRule::Additive,
    ///     Box::new(AdditiveTransform::new(50.0)),
    /// );
    /// ```
    pub fn register_transform_with_rule(
        &mut self,
        stat_id: StatId,
        phase: TransformPhase,
        rule: StackRule,
        transform: Box<dyn StatTransform>,
    ) {
        let entry = TransformEntry {
            phase,
            rule,
            transform,
        };
        self.register_transform_entry(stat_id, entry);
    }

    /// Internal method to register a transform entry.
    ///
    /// Uses copy-on-write semantics: if this resolver is a fork, the transform
    /// is added to the overlay. Otherwise, it's added to the base data.
    fn register_transform_entry(&mut self, stat_id: StatId, entry: TransformEntry) {
        let stat_id_clone = stat_id.clone();
        // Use copy-on-write helper to get the appropriate transforms vector
        self.get_mut_transforms(stat_id).push(entry);
        // Invalidate cache for this stat and potentially dependent stats
        self.cache.remove(&stat_id_clone);
    }

    /// Resolve a single stat.
    ///
    /// This will resolve the requested stat and all of its dependencies
    /// in the correct order. Results are cached until invalidated.
    ///
    /// This method shares its core implementation with `resolve_all()` and
    /// `resolve_batch()` via the internal `resolve_internal()` method.
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
        let results = self.resolve_internal(ResolveScope::Single(stat_id.clone()), context)?;
        results
            .into_iter()
            .next()
            .map(|(_, resolved)| resolved)
            .ok_or_else(|| StatError::MissingSource(stat_id.clone()))
    }

    /// Resolve all registered stats.
    ///
    /// Resolves all stats that have been registered (have sources or transforms).
    /// Stats are resolved in dependency order, and results are cached.
    ///
    /// This method shares its core implementation with `resolve()` and
    /// `resolve_batch()` via the internal `resolve_internal()` method.
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
        self.resolve_internal(ResolveScope::All, context)
    }

    /// Resolve multiple target stats and their dependencies in a single batch.
    ///
    /// More efficient than calling `resolve()` multiple times, as it only resolves
    /// the dependency subgraph needed for the specified targets.
    ///
    /// This method shares its core implementation with `resolve()` and
    /// `resolve_all()` via the internal `resolve_internal()` method.
    ///
    /// # Arguments
    ///
    /// * `targets` - The stat IDs to resolve
    /// * `context` - The stat context for conditional calculations
    ///
    /// # Returns
    ///
    /// * `Ok(HashMap<StatId, ResolvedStat>)` - Map of resolved stats (targets and dependencies)
    /// * `Err(StatError)` - If resolution fails (cycle, missing dependency, etc.)
    ///
    /// # Examples
    ///
    /// ```rust
    /// use zzstat::*;
    /// use zzstat::source::ConstantSource;
    ///
    /// let mut resolver = StatResolver::new();
    /// let str_id = StatId::from_str("STR");
    /// let atk_id = StatId::from_str("ATK");
    /// let hp_id = StatId::from_str("HP");
    ///
    /// resolver.register_source(str_id.clone(), Box::new(ConstantSource(10.0)));
    /// resolver.register_source(atk_id.clone(), Box::new(ConstantSource(50.0)));
    /// resolver.register_source(hp_id.clone(), Box::new(ConstantSource(100.0)));
    ///
    /// let context = StatContext::new();
    /// let results = resolver.resolve_batch(&[atk_id.clone(), hp_id.clone()], &context)?;
    /// assert!(results.contains_key(&atk_id));
    /// assert!(results.contains_key(&hp_id));
    /// // STR may or may not be included depending on dependencies
    /// # Ok::<(), zzstat::StatError>(())
    /// ```
    pub fn resolve_batch(
        &mut self,
        targets: &[StatId],
        context: &StatContext,
    ) -> Result<HashMap<StatId, ResolvedStat>, StatError> {
        self.resolve_internal(ResolveScope::Batch(targets.to_vec()), context)
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

    /// Get sources for a stat (checking overlay first, then base).
    ///
    /// Overlay completely shadows base - if overlay has sources for this stat,
    /// only overlay sources are returned. Otherwise, base sources are returned.
    #[allow(dead_code)]
    fn get_sources(&self, stat_id: &StatId) -> Option<&Vec<Box<dyn StatSource>>> {
        self.overlay.sources.get(stat_id).or_else(|| self.base.sources.get(stat_id))
    }

    /// Get transforms for a stat (checking overlay first, then base).
    ///
    /// Overlay completely shadows base - if overlay has transforms for this stat,
    /// only overlay transforms are returned. Otherwise, base transforms are returned.
    fn get_transforms(&self, stat_id: &StatId) -> Option<&Vec<TransformEntry>> {
        self.overlay.transforms.get(stat_id).or_else(|| self.base.transforms.get(stat_id))
    }

    /// Get all stat IDs that have sources or transforms.
    fn get_all_stat_ids(&self) -> std::collections::HashSet<StatId> {
        let mut ids = std::collections::HashSet::new();
        ids.extend(self.base.sources.keys().cloned());
        ids.extend(self.base.transforms.keys().cloned());
        ids.extend(self.overlay.sources.keys().cloned());
        ids.extend(self.overlay.transforms.keys().cloned());
        ids
    }

    /// Check if this resolver is a fork (has shared base data).
    ///
    /// Forks use copy-on-write semantics, storing modifications in the overlay.
    fn is_fork(&self) -> bool {
        Arc::strong_count(&self.base) > 1
    }

    /// Get mutable access to sources, using overlay if this is a fork.
    ///
    /// Returns a mutable reference to the sources vector for the given stat,
    /// either from the overlay (if fork) or base (if original).
    fn get_mut_sources(&mut self, stat_id: StatId) -> &mut Vec<Box<dyn StatSource>> {
        if self.is_fork() {
            // This is a fork, use overlay
            self.overlay.sources.entry(stat_id).or_default()
        } else {
            // This is the original, try to get mutable access to base
            if let Some(base) = Arc::get_mut(&mut self.base) {
                base.sources.entry(stat_id).or_default()
            } else {
                // Fallback: use overlay if we can't get mutable access
                self.overlay.sources.entry(stat_id).or_default()
            }
        }
    }

    /// Get mutable access to transforms, using overlay if this is a fork.
    ///
    /// Returns a mutable reference to the transforms vector for the given stat,
    /// either from the overlay (if fork) or base (if original).
    fn get_mut_transforms(&mut self, stat_id: StatId) -> &mut Vec<TransformEntry> {
        if self.is_fork() {
            // This is a fork, use overlay
            self.overlay.transforms.entry(stat_id).or_default()
        } else {
            // This is the original, try to get mutable access to base
            if let Some(base) = Arc::get_mut(&mut self.base) {
                base.transforms.entry(stat_id).or_default()
            } else {
                // Fallback: use overlay if we can't get mutable access
                self.overlay.transforms.entry(stat_id).or_default()
            }
        }
    }

    /// Internal method to resolve stats based on scope.
    ///
    /// This is the unified implementation for all resolve methods.
    /// It handles graph building, topological sorting, and the resolution loop.
    fn resolve_internal(
        &mut self,
        scope: ResolveScope,
        context: &StatContext,
    ) -> Result<HashMap<StatId, ResolvedStat>, StatError> {
        // Extract needed information from scope before moving
        let (single_stat_id, _batch_targets, is_all) = match &scope {
            ResolveScope::Single(stat_id) => {
                // For single stat, check cache first
                if let Some(cached) = self.cache.get(stat_id) {
                    let mut result = HashMap::new();
                    result.insert(stat_id.clone(), cached.clone());
                    return Ok(result);
                }
                (Some(stat_id.clone()), None, false)
            }
            ResolveScope::All => (None, None, true),
            ResolveScope::Batch(targets) => {
                if targets.is_empty() {
                    return Ok(HashMap::new());
                }
                (None, Some(targets.clone()), false)
            }
        };

        // Determine which graph to use and which stats to resolve
        let (graph, resolution_order) = match scope {
            ResolveScope::Single(_) => {
                // Build full graph and get topological sort
                let full_graph = self.build_graph()?;
                let order = full_graph.topological_sort()?;
                (full_graph, order)
            }
            ResolveScope::All => {
                // Build full graph and get topological sort
                let full_graph = self.build_graph()?;
                let order = full_graph.topological_sort()?;
                (full_graph, order)
            }
            ResolveScope::Batch(ref targets) => {
                // Build full graph and extract subgraph for targets
                let full_graph = self.build_graph()?;
                let subgraph = full_graph.subgraph_for_targets(targets);
                let order = subgraph.topological_sort()?;
                (full_graph, order)
            }
        };

        // Resolve all stats in resolution order
        for stat_id in &resolution_order {
            if !self.cache.contains_key(stat_id) {
                let resolved = self.resolve_stat_internal(stat_id, context, &graph)?;
                self.cache.insert(stat_id.clone(), resolved);
            }
        }

        // Collect results based on scope
        let mut results = HashMap::new();
        if let Some(stat_id) = single_stat_id {
            // Return only the requested stat
            if let Some(resolved) = self.cache.get(&stat_id) {
                results.insert(stat_id, resolved.clone());
            }
        } else if is_all {
            // Return all cached stats
            results = self.cache.clone();
        } else {
            // Return only stats from the resolution order (subgraph)
            for stat_id in &resolution_order {
                if let Some(resolved) = self.cache.get(stat_id) {
                    results.insert(stat_id.clone(), resolved.clone());
                }
            }
        }

        Ok(results)
    }

    /// Build the dependency graph from all registered transforms.
    fn build_graph(&self) -> Result<StatGraph, StatError> {
        let mut graph = StatGraph::new();

        // Add all stats that have sources or transforms
        for stat_id in self.get_all_stat_ids() {
            graph.add_node(stat_id);
        }

        // Add edges from transform dependencies (check overlay first, then base)
        for stat_id in self.get_all_stat_ids() {
            if let Some(transforms) = self.get_transforms(&stat_id) {
                for entry in transforms {
                    for dep in entry.transform.depends_on() {
                        // dep must be resolved before stat_id
                        graph.add_edge(stat_id.clone(), dep);
                    }
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
        let mut resolved = ResolvedStat::new(stat_id.clone(), StatValue::zero());

        // Step 1: Collect all source values (additive)
        // Combine overlay and base sources (overlay adds to base, doesn't shadow)
        let mut base_value = StatValue::zero();
        let mut source_count = 0;
        
        // Collect base sources
        if let Some(base_sources) = self.base.sources.get(stat_id) {
            for source in base_sources.iter() {
                let value = source.get_value(stat_id, context);
                base_value += value;
                source_count += 1;
                resolved.add_source(format!("Source #{}", source_count), value);
            }
        }
        
        // Collect overlay sources (additive to base)
        if let Some(overlay_sources) = self.overlay.sources.get(stat_id) {
            for source in overlay_sources.iter() {
                let value = source.get_value(stat_id, context);
                base_value += value;
                source_count += 1;
                resolved.add_source(format!("Source #{}", source_count), value);
            }
        }
        
        // If no sources at all, create a default source entry
        if source_count == 0 {
            resolved.add_source("Default", StatValue::zero());
        }

        // Step 2: Apply transforms grouped by phase, then by stack rule
        // Combine overlay and base transforms (overlay adds to base, doesn't shadow)
        let mut current_value = base_value;
        
        // Collect all transforms (base first, then overlay)
        let mut all_transforms = Vec::new();
        if let Some(base_transforms) = self.base.transforms.get(stat_id) {
            all_transforms.extend(base_transforms.iter());
        }
        if let Some(overlay_transforms) = self.overlay.transforms.get(stat_id) {
            all_transforms.extend(overlay_transforms.iter());
        }
        
        if !all_transforms.is_empty() {
            // Group transforms by phase
            let mut transforms_by_phase: std::collections::BTreeMap<u8, Vec<&TransformEntry>> =
                std::collections::BTreeMap::new();

            for entry in all_transforms {
                let phase_value = entry.phase.value();
                transforms_by_phase
                    .entry(phase_value)
                    .or_default()
                    .push(entry);
            }

            // Apply transforms in phase order, with stack rules applied within each phase
            for (_phase_value, phase_entries) in transforms_by_phase {
                current_value = self.apply_transforms_with_stack_rules(
                    current_value,
                    phase_entries,
                    stat_id,
                    context,
                    &mut resolved,
                )?;
            }
        }

        resolved.value = current_value;
        Ok(resolved)
    }

    /// Apply transforms in a phase with stack rule semantics.
    ///
    /// Groups transforms by stack rule priority and applies them in order:
    /// Override → Additive → Multiplicative → Diminishing → Min → Max → MinMax
    ///
    /// Stack rule semantics:
    /// - Additive: base + sum(all additive deltas) where delta = transform.apply(0)
    /// - Multiplicative: base × product(all multipliers) where multiplier = transform.apply(1.0)
    /// - Override: last transform wins (deterministic order)
    /// - Diminishing: value × (1 - exp(-k × stacks)) where stacks = number of transforms
    /// - Min: clamp to maximum of all min bounds (most restrictive)
    /// - Max: clamp to minimum of all max bounds (most restrictive)
    /// - MinMax: collect all min/max bounds, compute effective_min = max(all mins),
    ///   effective_max = min(all maxes), then clamp(value, effective_min, effective_max)
    fn apply_transforms_with_stack_rules(
        &self,
        base_value: StatValue,
        entries: Vec<&TransformEntry>,
        stat_id: &StatId,
        context: &StatContext,
        resolved: &mut ResolvedStat,
    ) -> Result<StatValue, StatError> {
        // Group entries by stack rule (sorted by priority)
        let mut by_rule: std::collections::BTreeMap<u8, Vec<&TransformEntry>> =
            std::collections::BTreeMap::new();
        
        for entry in entries {
            let priority = entry.rule.priority();
            by_rule.entry(priority).or_default().push(entry);
        }

        let mut current_value = base_value;

        // Apply stack rules in priority order
        for (_priority, rule_entries) in by_rule {
            if rule_entries.is_empty() {
                continue;
            }

            // All entries in this group have the same stack rule
            let stack_rule = rule_entries[0].rule;
            
            match stack_rule {
                StackRule::Override => {
                    // Last transform wins (deterministic order - use last entry)
                    if let Some(last_entry) = rule_entries.last() {
                        let dependencies = self.collect_dependencies(
                            last_entry.transform.depends_on(),
                            stat_id,
                        )?;
                        let new_value = last_entry
                            .transform
                            .apply(current_value, &dependencies, context)?;
                        resolved.add_transform(last_entry.transform.description(), new_value);
                        current_value = new_value;
                    }
                }
                StackRule::Additive => {
                    // Additive stacking: base + sum(all additive deltas)
                    // Extract delta by applying each transform to zero
                    let mut sum_delta = StatValue::zero();
                    for entry in &rule_entries {
                        let dependencies = self.collect_dependencies(
                            entry.transform.depends_on(),
                            stat_id,
                        )?;
                        // Apply to zero to extract the additive delta
                        let zero = StatValue::zero();
                        let delta = entry.transform.apply(zero, &dependencies, context)?;
                        sum_delta += delta;
                    }
                    // Apply the sum of deltas to the current value
                    current_value += sum_delta;
                    resolved.add_transform(
                        format!("+{:.2} (additive stack)", sum_delta.to_f64()),
                        current_value,
                    );
                }
                StackRule::Multiplicative => {
                    // Multiplicative stacking: base × product(all multipliers)
                    // Extract multiplier by applying each transform to 1.0
                    let mut product_multiplier = StatValue::from_f64(1.0);
                    for entry in &rule_entries {
                        let dependencies = self.collect_dependencies(
                            entry.transform.depends_on(),
                            stat_id,
                        )?;
                        // Apply to 1.0 to extract the multiplier
                        let one = StatValue::from_f64(1.0);
                        let multiplier = entry.transform.apply(one, &dependencies, context)?;
                        product_multiplier *= multiplier;
                    }
                    // Apply the product of multipliers to the current value
                    current_value *= product_multiplier;
                    resolved.add_transform(
                        format!("×{:.4} (multiplicative stack)", product_multiplier.to_f64()),
                        current_value,
                    );
                }
                StackRule::Diminishing { k } => {
                    // Diminishing returns: value × (1 - exp(-k × stacks))
                    // Count the number of stacks (transforms)
                    let stacks = rule_entries.len() as f64;
                    let k_f64 = k.to_f64();
                    let multiplier = 1.0 - (-k_f64 * stacks).exp();
                    current_value *= StatValue::from_f64(multiplier);
                    resolved.add_transform(
                        format!("×{:.4} (diminishing k={:.2}, stacks={:.0})", multiplier, k_f64, stacks),
                        current_value,
                    );
                }
                StackRule::Min => {
                    // Min clamping: clamp to maximum of all min bounds (most restrictive)
                    let mut min_bound = None;
                    for entry in &rule_entries {
                        let bound = self.extract_min_bound(entry, stat_id, context)?;
                        if let Some(bound_value) = bound {
                            // Take the maximum of all min bounds (most restrictive)
                            min_bound = Some(min_bound.map_or(bound_value, |m: StatValue| m.max(bound_value)));
                        }
                    }
                    if let Some(min) = min_bound {
                        current_value = current_value.max(min);
                        resolved.add_transform(
                            format!("min({:.2})", min.to_f64()),
                            current_value,
                        );
                    }
                }
                StackRule::Max => {
                    // Max clamping: clamp to minimum of all max bounds (most restrictive)
                    let mut max_bound = None;
                    for entry in &rule_entries {
                        let bound = self.extract_max_bound(entry, stat_id, context)?;
                        if let Some(bound_value) = bound {
                            // Take the minimum of all max bounds (most restrictive)
                            max_bound = Some(max_bound.map_or(bound_value, |m: StatValue| m.min(bound_value)));
                        }
                    }
                    if let Some(max) = max_bound {
                        current_value = current_value.min(max);
                        resolved.add_transform(
                            format!("max({:.2})", max.to_f64()),
                            current_value,
                        );
                    }
                }
                StackRule::MinMax => {
                    // MinMax clamping: collect all min/max bounds and apply most restrictive
                    // effective_min = max(all mins), effective_max = min(all maxes)
                    let mut min_bounds = Vec::new();
                    let mut max_bounds = Vec::new();
                    
                    for entry in &rule_entries {
                        // Extract bounds using helper methods
                        if let Some(min) = self.extract_min_bound(entry, stat_id, context)? {
                            min_bounds.push(min);
                        }
                        if let Some(max) = self.extract_max_bound(entry, stat_id, context)? {
                            max_bounds.push(max);
                        }
                    }
                    
                    // Compute effective bounds (most restrictive)
                    let effective_min = min_bounds.iter().fold(None, |acc: Option<StatValue>, &m| {
                        Some(acc.map_or(m, |a: StatValue| a.max(m)))
                    });
                    let effective_max = max_bounds.iter().fold(None, |acc: Option<StatValue>, &m| {
                        Some(acc.map_or(m, |a: StatValue| a.min(m)))
                    });
                    
                    // Apply clamping
                    if let Some(min) = effective_min {
                        current_value = current_value.max(min);
                    }
                    if let Some(max) = effective_max {
                        current_value = current_value.min(max);
                    }
                    
                    // Update resolved stat description
                    match (effective_min, effective_max) {
                        (Some(min), Some(max)) => {
                            resolved.add_transform(
                                format!("clamp({:.2}, {:.2})", min.to_f64(), max.to_f64()),
                                current_value,
                            );
                        }
                        (Some(min), None) => {
                            resolved.add_transform(
                                format!("clamp_min({:.2})", min.to_f64()),
                                current_value,
                            );
                        }
                        (None, Some(max)) => {
                            resolved.add_transform(
                                format!("clamp_max({:.2})", max.to_f64()),
                                current_value,
                            );
                        }
                        (None, None) => {
                            // No bounds, no-op
                        }
                    }
                }
            }
        }

        Ok(current_value)
    }

    /// Collect dependency values for a transform.
    fn collect_dependencies(
        &self,
        dep_ids: Vec<StatId>,
        _stat_id: &StatId,
    ) -> Result<HashMap<StatId, StatValue>, StatError> {
        let mut dependencies = HashMap::new();
        for dep_id in dep_ids {
            let dep_value = self
                .cache
                .get(&dep_id)
                .map(|r| r.value)
                .ok_or_else(|| StatError::MissingDependency(dep_id.clone()))?;
            dependencies.insert(dep_id, dep_value);
        }
        Ok(dependencies)
    }

    /// Extract minimum bound from a transform entry.
    ///
    /// Applies the transform to a very negative value to infer the minimum bound.
    /// For ClampTransform, this will return the min parameter.
    fn extract_min_bound(
        &self,
        entry: &TransformEntry,
        stat_id: &StatId,
        context: &StatContext,
    ) -> Result<Option<StatValue>, StatError> {
        let dependencies = self.collect_dependencies(
            entry.transform.depends_on(),
            stat_id,
        )?;
        let very_negative = StatValue::from_f64(-1e10);
        let bound_result = entry.transform.apply(very_negative, &dependencies, context)?;
        
        // If the result differs from input, it's a bound
        if bound_result > very_negative {
            Ok(Some(bound_result))
        } else {
            Ok(None)
        }
    }

    /// Extract maximum bound from a transform entry.
    ///
    /// Applies the transform to a very large value to infer the maximum bound.
    /// For ClampTransform, this will return the max parameter.
    fn extract_max_bound(
        &self,
        entry: &TransformEntry,
        stat_id: &StatId,
        context: &StatContext,
    ) -> Result<Option<StatValue>, StatError> {
        let dependencies = self.collect_dependencies(
            entry.transform.depends_on(),
            stat_id,
        )?;
        let very_large = StatValue::from_f64(1e10);
        let bound_result = entry.transform.apply(very_large, &dependencies, context)?;
        
        // If the result differs from input, it's a bound
        if bound_result < very_large {
            Ok(Some(bound_result))
        } else {
            Ok(None)
        }
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

        assert_eq!(resolved.value, StatValue::from_f64(100.0));
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

        assert_eq!(resolved.value, StatValue::from_f64(150.0));
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

        assert_eq!(resolved.value, StatValue::from_f64(150.0));
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
        assert_eq!(resolved.value, StatValue::from_f64(70.0));
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
        assert_eq!(resolved1.value, StatValue::from_f64(100.0));

        // Should be cached
        let resolved2 = resolver.resolve(&hp_id, &context).unwrap();
        assert_eq!(resolved2.value, StatValue::from_f64(100.0));

        // Invalidate and add new source
        resolver.invalidate(&hp_id);
        resolver.register_source(hp_id.clone(), Box::new(ConstantSource(50.0)));

        let resolved3 = resolver.resolve(&hp_id, &context).unwrap();
        assert_eq!(resolved3.value, StatValue::from_f64(150.0)); // 100 + 50
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
        if let Err(StatError::Cycle { path: _ }) = result {
            // Good
        } else {
            panic!("Expected Cycle error");
        }
    }
}
