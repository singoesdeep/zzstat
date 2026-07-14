use crate::source::StatSource;
use crate::stat_id::StatId;
use crate::transform::TransformEntry;
use rustc_hash::{FxHashMap, FxHashSet};
use std::sync::Arc;

/// Base data shared across resolver forks.
///
/// Contains the sources and transforms that are shared via copy-on-write.
pub(crate) struct BaseData {
    /// Multiple sources per stat (additive).
    pub(crate) sources: FxHashMap<StatId, Vec<Box<dyn StatSource>>>,

    /// Transform chain per stat.
    pub(crate) transforms: FxHashMap<StatId, Vec<TransformEntry>>,
}

/// Overlay data for copy-on-write modifications.
///
/// When a resolver is forked, modifications are stored in the overlay.
/// Reading checks overlay first, then falls back to base data.
pub(crate) struct OverlayData {
    /// Overlay sources (shadows base sources when present).
    pub(crate) sources: FxHashMap<StatId, Vec<Box<dyn StatSource>>>,

    /// Overlay transforms (shadows base transforms when present).
    pub(crate) transforms: FxHashMap<StatId, Vec<TransformEntry>>,
}

/// Registry holding sources and transforms with COW semantics.
pub(crate) struct StatRegistry {
    pub(crate) base: Arc<BaseData>,
    pub(crate) overlay: OverlayData,
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
        }
    }

    pub(crate) fn fork(&self) -> Self {
        Self {
            base: Arc::clone(&self.base),
            overlay: OverlayData {
                sources: FxHashMap::default(),
                transforms: FxHashMap::default(),
            },
        }
    }

    pub(crate) fn is_fork(&self) -> bool {
        Arc::strong_count(&self.base) > 1
    }

    pub(crate) fn iter_sources<'a>(
        &'a self,
        stat_id: &StatId,
    ) -> impl Iterator<Item = &'a Box<dyn StatSource>> {
        let base_iter = self.base.sources.get(stat_id).into_iter().flatten();
        let overlay_iter = self.overlay.sources.get(stat_id).into_iter().flatten();
        base_iter.chain(overlay_iter)
    }

    pub(crate) fn iter_transforms<'a>(
        &'a self,
        stat_id: &StatId,
    ) -> impl Iterator<Item = &'a TransformEntry> {
        let base_iter = self.base.transforms.get(stat_id).into_iter().flatten();
        let overlay_iter = self.overlay.transforms.get(stat_id).into_iter().flatten();
        base_iter.chain(overlay_iter)
    }

    pub(crate) fn get_all_stat_ids(&self) -> FxHashSet<StatId> {
        let mut ids = FxHashSet::default();
        ids.extend(self.base.sources.keys().cloned());
        ids.extend(self.base.transforms.keys().cloned());
        ids.extend(self.overlay.sources.keys().cloned());
        ids.extend(self.overlay.transforms.keys().cloned());
        ids
    }

    pub(crate) fn get_mut_sources(&mut self, stat_id: StatId) -> &mut Vec<Box<dyn StatSource>> {
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

    pub(crate) fn get_mut_transforms(&mut self, stat_id: StatId) -> &mut Vec<TransformEntry> {
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
