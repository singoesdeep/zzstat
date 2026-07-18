use crate::source::StatSource;
use crate::stat_id::StatId;
use crate::transform::TransformEntry;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;

pub(crate) type SharedSource = Arc<Box<dyn StatSource>>;
pub(crate) type SharedTransform = Arc<TransformEntry>;

/// Base data shared across resolver forks.
///
/// Contains the sources and transforms that are shared via copy-on-write.
pub(crate) struct BaseData {
    /// Multiple sources per stat (additive).
    pub(crate) sources: FxHashMap<StatId, Vec<SharedSource>>,

    /// Transform chain per stat.
    pub(crate) transforms: FxHashMap<StatId, Vec<SharedTransform>>,
}

/// Overlay data for copy-on-write modifications.
///
/// When a resolver is forked, modifications are stored in the overlay.
/// Reading checks overlay first, then falls back to base data.
pub(crate) struct OverlayData {
    /// Overlay sources (shadows base sources when present).
    pub(crate) sources: FxHashMap<StatId, Vec<SharedSource>>,

    /// Overlay transforms (shadows base transforms when present).
    pub(crate) transforms: FxHashMap<StatId, Vec<SharedTransform>>,
}

/// Registry holding sources and transforms with COW semantics and hierarchical parent inheritance.
pub(crate) struct StatRegistry {
    pub(crate) base: Arc<BaseData>,
    pub(crate) overlay: OverlayData,
    pub(crate) parent: Option<Arc<std::sync::RwLock<StatRegistry>>>,
}

impl StatRegistry {
    pub(crate) fn new() -> Self {
        Self {
            base: Arc::new(BaseData {
                sources: FxHashMap::default(),
                transforms: FxHashMap::default(),
            }),
            overlay: OverlayData {
                sources: FxHashMap::default(),
                transforms: FxHashMap::default(),
            },
            parent: None,
        }
    }

    pub(crate) fn fork_with_parent(parent: Arc<std::sync::RwLock<StatRegistry>>) -> Self {
        let base = {
            let guard = parent.read().unwrap();
            Arc::clone(&guard.base)
        };
        Self {
            base,
            overlay: OverlayData {
                sources: FxHashMap::default(),
                transforms: FxHashMap::default(),
            },
            parent: Some(parent),
        }
    }

    pub(crate) fn get_sources(&self, stat_id: &StatId) -> Vec<SharedSource> {
        let mut sources = Vec::new();
        self.collect_sources(stat_id, &mut sources);
        sources
    }

    fn collect_sources(&self, stat_id: &StatId, out: &mut Vec<SharedSource>) {
        if let Some(ref parent) = self.parent {
            let guard = parent.read().unwrap();
            guard.collect_sources(stat_id, out);
        } else {
            if let Some(base_sources) = self.base.sources.get(stat_id) {
                out.extend(base_sources.iter().cloned());
            }
        }
        if let Some(overlay_sources) = self.overlay.sources.get(stat_id) {
            out.extend(overlay_sources.iter().cloned());
        }
    }

    pub(crate) fn get_transforms(&self, stat_id: &StatId) -> Vec<SharedTransform> {
        let mut transforms = Vec::new();
        self.collect_transforms(stat_id, &mut transforms);
        transforms
    }

    fn collect_transforms(&self, stat_id: &StatId, out: &mut Vec<SharedTransform>) {
        if let Some(ref parent) = self.parent {
            let guard = parent.read().unwrap();
            guard.collect_transforms(stat_id, out);
        } else {
            if let Some(base_transforms) = self.base.transforms.get(stat_id) {
                out.extend(base_transforms.iter().cloned());
            }
        }
        if let Some(overlay_transforms) = self.overlay.transforms.get(stat_id) {
            out.extend(overlay_transforms.iter().cloned());
        }
    }

    pub(crate) fn get_all_stat_ids(&self) -> FxHashSet<StatId> {
        let mut ids = FxHashSet::default();
        self.collect_stat_ids(&mut ids);
        ids
    }

    fn collect_stat_ids(&self, out: &mut FxHashSet<StatId>) {
        if let Some(ref parent) = self.parent {
            let guard = parent.read().unwrap();
            guard.collect_stat_ids(out);
        } else {
            out.extend(self.base.sources.keys().cloned());
            out.extend(self.base.transforms.keys().cloned());
        }
        out.extend(self.overlay.sources.keys().cloned());
        out.extend(self.overlay.transforms.keys().cloned());
    }

    pub(crate) fn get_mut_sources(&mut self, stat_id: StatId) -> &mut Vec<SharedSource> {
        if self.parent.is_some() {
            self.overlay.sources.entry(stat_id).or_default()
        } else {
            if let Some(base) = Arc::get_mut(&mut self.base) {
                base.sources.entry(stat_id).or_default()
            } else {
                self.overlay.sources.entry(stat_id).or_default()
            }
        }
    }

    pub(crate) fn get_mut_transforms(&mut self, stat_id: StatId) -> &mut Vec<SharedTransform> {
        if self.parent.is_some() {
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
