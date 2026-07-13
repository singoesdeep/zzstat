import os
import re

with open('src/resolver.rs', 'r') as f:
    content = f.read()

# 1. Add StatRegistry and GraphCache structs
registry_structs = """
/// Registry holding sources and transforms with COW semantics.
struct StatRegistry {
    base: Arc<BaseData>,
    overlay: OverlayData,
}

impl StatRegistry {
    fn new() -> Self {
        Self {
            base: Arc::new(BaseData {
                sources: FxHashMap::default(),
                transforms: FxHashMap::default(),
            }),
            overlay: OverlayData {
                sources: FxHashMap::default(),
                transforms: FxHashMap::default(),
            },
        }
    }

    fn fork(&self) -> Self {
        Self {
            base: Arc::clone(&self.base),
            overlay: OverlayData {
                sources: FxHashMap::default(),
                transforms: FxHashMap::default(),
            },
        }
    }

    fn is_fork(&self) -> bool {
        Arc::strong_count(&self.base) > 1
    }

    fn get_sources(&self, stat_id: &StatId) -> Option<&Vec<Box<dyn StatSource>>> {
        self.overlay
            .sources
            .get(stat_id)
            .or_else(|| self.base.sources.get(stat_id))
    }

    fn get_transforms(&self, stat_id: &StatId) -> Option<&Vec<TransformEntry>> {
        self.overlay
            .transforms
            .get(stat_id)
            .or_else(|| self.base.transforms.get(stat_id))
    }

    fn get_all_stat_ids(&self) -> FxHashSet<StatId> {
        let mut ids = FxHashSet::default();
        ids.extend(self.base.sources.keys().cloned());
        ids.extend(self.base.transforms.keys().cloned());
        ids.extend(self.overlay.sources.keys().cloned());
        ids.extend(self.overlay.transforms.keys().cloned());
        ids
    }

    fn get_mut_sources(&mut self, stat_id: StatId) -> &mut Vec<Box<dyn StatSource>> {
        if self.is_fork() {
            self.overlay.sources.entry(stat_id).or_default()
        } else {
            if let Some(base) = Arc::get_mut(&mut self.base) {
                base.sources.entry(stat_id).or_default()
            } else {
                self.overlay.sources.entry(stat_id).or_default()
            }
        }
    }

    fn get_mut_transforms(&mut self, stat_id: StatId) -> &mut Vec<TransformEntry> {
        if self.is_fork() {
            self.overlay.transforms.entry(stat_id).or_default()
        } else {
            if let Some(base) = Arc::get_mut(&mut self.base) {
                base.transforms.entry(stat_id).or_default()
            } else {
                self.overlay.transforms.entry(stat_id).or_default()
            }
        }
    }
}

/// Cached dependency graph.
struct GraphCache {
    graph: StatGraph,
    toposort: Vec<StatId>,
}
"""

content = content.replace("pub struct StatResolver {\n    /// Shared base data (sources and transforms).\n    base: Arc<BaseData>,\n\n    /// Copy-on-write overlay for modifications.\n    overlay: OverlayData,\n", registry_structs + "\npub struct StatResolver {\n    registry: StatRegistry,\n    graph_cache: Option<GraphCache>,\n")

# 2. Update new() and fork()
new_func = """    pub fn new() -> Self {
        Self {
            registry: StatRegistry::new(),
            graph_cache: None,
            cache: FxHashMap::default(),
        }
    }"""
content = re.sub(r'    pub fn new\(\) -> Self \{[\s\S]*?cache: FxHashMap::default\(\),\n        \}', new_func, content)

fork_func = """    pub fn fork(&self) -> Self {
        Self {
            registry: self.registry.fork(),
            graph_cache: None,
            cache: FxHashMap::default(),
        }
    }"""
content = re.sub(r'    pub fn fork\(&self\) -> Self \{[\s\S]*?cache: FxHashMap::default\(\),\n        \}', fork_func, content)

# 3. Replace self.get_mut_sources -> self.registry.get_mut_sources
content = content.replace('self.get_mut_sources', 'self.registry.get_mut_sources')
content = content.replace('self.get_mut_transforms', 'self.registry.get_mut_transforms')
content = content.replace('self.get_all_stat_ids', 'self.registry.get_all_stat_ids')
content = content.replace('self.get_transforms', 'self.registry.get_transforms')
content = content.replace('self.get_sources', 'self.registry.get_sources')
content = content.replace('self.is_fork', 'self.registry.is_fork')
content = content.replace('self.base.', 'self.registry.base.')
content = content.replace('self.overlay.', 'self.registry.overlay.')

# Invalidate graph_cache in register_source and register_transform_entry
content = content.replace('self.cache.remove(&stat_id_clone);', 'self.graph_cache = None;\n        self.cache.remove(&stat_id_clone);')

# We need to remove the now-extracted methods from StatResolver
content = re.sub(r'    /// Get sources for a stat[\s\S]*?fn is_fork\(&self\) -> bool \{[\s\S]*?\}', '', content)
content = re.sub(r'    /// Get mutable access to sources[\s\S]*?fn get_mut_transforms\(&mut self, stat_id: StatId\) -> &mut Vec<TransformEntry> \{[\s\S]*?\}\n        \}\n    \}', '', content)

# 4. Refactor resolve_internal to use graph_cache
resolve_internal_replacement = """    fn resolve_internal(
        &mut self,
        scope: ResolveScope,
        context: &StatContext,
    ) -> Result<FxHashMap<StatId, ResolvedStat>, StatError> {
        let (single_stat_id, batch_targets, is_all) = match &scope {
            ResolveScope::Single(stat_id) => {
                if let Some(cached) = self.cache.get(stat_id) {
                    let mut result = FxHashMap::default();
                    result.insert(stat_id.clone(), cached.clone());
                    return Ok(result);
                }
                (Some(stat_id.clone()), None, false)
            }
            ResolveScope::All => (None, None, true),
            ResolveScope::Batch(targets) => {
                if targets.is_empty() {
                    return Ok(FxHashMap::default());
                }
                (None, Some(targets.clone()), false)
            }
        };

        if self.graph_cache.is_none() {
            let graph = self.build_graph()?;
            let toposort = graph.topological_sort()?;
            self.graph_cache = Some(GraphCache { graph, toposort });
        }

        let graph_cache = self.graph_cache.as_ref().unwrap();

        let resolution_order = if let Some(targets) = &batch_targets {
            let subgraph = graph_cache.graph.subgraph_for_targets(targets);
            subgraph.topological_sort()?
        } else {
            graph_cache.toposort.clone()
        };

        for stat_id in &resolution_order {
            if !self.cache.contains_key(stat_id) {
                let resolved = self.resolve_stat_internal(stat_id, context, &graph_cache.graph)?;
                self.cache.insert(stat_id.clone(), resolved);
            }
        }

        let mut results = FxHashMap::default();
        if let Some(stat_id) = single_stat_id {
            if let Some(resolved) = self.cache.get(&stat_id) {
                results.insert(stat_id, resolved.clone());
            }
        } else if is_all {
            results = self.cache.clone();
        } else {
            for stat_id in &resolution_order {
                if let Some(resolved) = self.cache.get(stat_id) {
                    results.insert(stat_id.clone(), resolved.clone());
                }
            }
        }

        Ok(results)
    }"""

content = re.sub(r'    fn resolve_internal\([\s\S]*?Ok\(results\)\n    \}', resolve_internal_replacement, content)

# 5. Fix build_graph method name in StatResolver
# It is fine because `self.build_graph()` just calls the method that remains in StatResolver, 
# which calls `self.registry.get_all_stat_ids()` internally now.

# Fix the method `build_graph` calling get_all_stat_ids
# Wait, `build_graph` should be using `self.registry.get_all_stat_ids()`. The previous replace fixed it.

with open('src/resolver.rs', 'w') as f:
    f.write(content)

print("resolver.rs refactored successfully")
