//! What `MemStore::alt_next` keeps per scan: per table and key, the last few sets of filters a scan used, each with the
//! rows they leave in the order of the key, as of the table's stamp. The filters themselves tell the scans apart
//! ([`same_filters`]): a lookup compares a few filters, where a key made of their text formatted every filter at every
//! step (issue 8603).
//!
//! What `alt_next` keeps per scan: per table and key, the last `ALT_SCANS_PER_KEY` sets of filters,
//! told apart by `same_filters` (the filters themselves, not their text; issue 8603), each with the
//! rows they leave in the order of the key as of the table's stamp. Entry points: `alt_step`,
//! `alt_keep`. Unit tests: `memory/alt_cache_tests.rs`.

use super::*;

/// How many sets of filters one table and key keep, the one used last first.
pub(super) const ALT_SCANS_PER_KEY: usize = 8;

/// The scans kept per table and key.
pub(super) type AltCache = std::collections::HashMap<(Slot, usize), Vec<AltEntry>>;

/// What `alt_next` knows about one scan (a table, a key and a set of filters) as of a table stamp.
#[derive(Clone)]
pub(super) struct AltEntry {
    /// The filters of the scan, as the store reads them (after `number_filters`).
    filters: Vec<FieldFilter>,
    stamp: u64,
    /// How many scan steps came at this stamp: the first is answered by a pass over the band, the second builds
    /// `keys`, the later ones search it.
    calls: u32,
    /// No other key narrows this scan: the caller walks.
    not_applicable: bool,
    /// The primary keys of the rows that pass the filters, in the order of the scan's key.
    keys: Option<std::rc::Rc<Vec<Key>>>,
}

/// What the next step of a scan does, from what is kept for it.
pub(super) enum AltStep {
    /// Walk the table: no other key narrows the scan, or this is the first step after a write.
    Walk,
    /// Search the rows kept in the order of the scan's key.
    Sorted(std::rc::Rc<Vec<Key>>),
    /// Read the band: the `n`-th step at this stamp.
    Band(u32),
}

impl MemStore {
    /// The step of the scan in key `key` of `slot` through `filters` at the table's `stamp`, counting it.
    pub(super) fn alt_step(&self, slot: &Slot, key: usize, filters: &[FieldFilter], stamp: u64) -> AltStep {
        let mut cache = self.alt_cache.borrow_mut();
        if cache.len() > 256 {
            cache.clear();
        }
        let kept = cache.entry((slot.clone(), key)).or_default();
        let Some(i) = kept.iter().position(|e| same_filters(&e.filters, filters)) else {
            kept.insert(
                0,
                AltEntry {
                    filters: filters.to_vec(),
                    stamp,
                    calls: 1,
                    not_applicable: false,
                    keys: None,
                },
            );
            kept.truncate(ALT_SCANS_PER_KEY);
            return AltStep::Band(1);
        };
        kept[..=i].rotate_right(1);
        let e = &mut kept[0];
        if e.stamp != stamp {
            // The first step after a write: walk.
            *e = AltEntry {
                filters: std::mem::take(&mut e.filters),
                stamp,
                calls: 1,
                not_applicable: false,
                keys: None,
            };
            return AltStep::Walk;
        }
        if e.not_applicable {
            return AltStep::Walk;
        }
        if let Some(keys) = &e.keys {
            return AltStep::Sorted(keys.clone());
        }
        e.calls += 1;
        AltStep::Band(e.calls)
    }

    /// Notes what the band of the scan gave: no band (`None`), or the rows that pass in the order of its key.
    pub(super) fn alt_keep(
        &self,
        slot: &Slot,
        key: usize,
        filters: &[FieldFilter],
        keys: Option<std::rc::Rc<Vec<Key>>>,
    ) {
        let mut cache = self.alt_cache.borrow_mut();
        let Some(e) = cache
            .get_mut(&(slot.clone(), key))
            .and_then(|kept| kept.iter_mut().find(|e| same_filters(&e.filters, filters)))
        else {
            return;
        };
        match keys {
            None => e.not_applicable = true,
            Some(keys) => e.keys = Some(keys),
        }
    }

    #[cfg(test)]
    pub(super) fn alt_entries(&self, cx: &AccessContext, t: &TableDef, key: usize) -> usize {
        self.alt_cache.borrow().get(&(slot(cx, t), key)).map_or(0, Vec::len)
    }

    #[cfg(test)]
    pub(super) fn alt_entry_kept(&self, cx: &AccessContext, t: &TableDef, key: usize, filters: &[FieldFilter]) -> bool {
        self.alt_cache
            .borrow()
            .get(&(slot(cx, t), key))
            .is_some_and(|kept| kept.iter().any(|e| same_filters(&e.filters, filters)))
    }
}
