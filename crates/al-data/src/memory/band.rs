//! Which rows of a table a read looks at: the band of the key the filters narrow most (ADR-035), shared by a scan
//! (`MemStore::alt_next`) and by a count or an existence check, which stops at the first rows that pass.
//!
//! Which rows of a table a read looks at: the narrowest band of a key the filters bound
//! (`narrowest_band`, `band_of`; ADR-035), shared by a scan (`alt_next`) and by `count_up_to`, which
//! counts the rows that pass up to a limit (`Store::count` has none; `exists` stops at the first).
//! Callers that only test for a row (IsEmpty, a unique key, a TableRelation check, a FlowField's
//! source rows) use `exists` or a limit. Entry points: `exists`, `count_up_to`, `narrowest_band`.
//! Unit tests: `memory/tests/count_tests.rs`.

use super::*;

/// Where the band of one key is: the primary key's rows between two keys, or a slice of the cached order.
pub(super) enum Band {
    Pk,
    Order(std::rc::Rc<Vec<OrderKey>>, std::ops::Range<usize>),
}

impl MemStore {
    /// Whether a row passes `filters`: [`MemStore::count_up_to`] with a limit of 1, so it stops at the first row that passes.
    pub fn exists(&self, cx: &AccessContext, t: &TableDef, filters: &[FieldFilter]) -> Result<bool, StoreError> {
        Ok(self.count_up_to(cx, t, filters, 1)? > 0)
    }

    /// The rows that pass `filters`, counted up to `limit` (`usize::MAX`: `Store::count`): the walk stops when it has
    /// seen `limit` of them. The refusals come first and each looks at the rows it needs itself (`unknown_read` and
    /// `collation_read` at every row of the table, `number_filters` at every row for a Variant Code filter), so how far
    /// the walk goes changes no refusal. The walk only tests rows (`passes` refuses nothing).
    pub fn count_up_to(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        filters: &[FieldFilter],
        limit: usize,
    ) -> Result<usize, StoreError> {
        bump();
        let _fp = self.footprint_note(cx, t, None, super::footprint::Touch::ReadExtent);
        if let Some(v) = self.virtual_of(t) {
            return Ok(v.count(self, cx, t, filters)?.min(limit));
        }
        let filters = &*self.at_sign_alone_read(t, filters, false);
        self.unknown_read(cx, t, filters, &[])?;
        Self::unpinned_match_read(filters)?;
        Self::code_storage_read(t, filters, &[])?;
        self.collation_read(cx, t, filters, None)?;
        let Some(table) = self.table(cx, t) else {
            return Ok(0);
        };
        let f = &*crate::code_storage::number_filters(t, filters, &mut table.values())?;
        Ok(self
            .band_rows(cx, t, &table, f)
            .filter(|r| passes(r, f))
            .take(limit)
            .count())
    }

    /// The rows of `table` that can pass `filters`, in no promised order: the band of another key when it is a clear gain
    /// over the primary key's band (as a scan in primary key order picks it, [`MemStore::narrowest_band`]), else the
    /// primary key's band. A row of the band may still fail the filters, so the caller checks `passes`.
    fn band_rows<'a>(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        table: &'a Table,
        filters: &[FieldFilter],
    ) -> Box<dyn Iterator<Item = &'a Row> + 'a> {
        let sample = table.values().next();
        if table.len() >= INDEX_MIN_ROWS && (1..t.keys.len()).any(|j| Seek::of(t, j, filters, sample).is_some()) {
            let slot = slot(cx, t);
            let own = self.band_of(&slot, 0, t, table, filters, false).map(|(_, n)| n);
            if let Some(Band::Order(order, range)) = self.narrowest_band(&slot, t, table, filters, 0, own) {
                return Box::new(range.filter_map(move |i| table.get(&order[i])));
            }
        }
        Box::new(pk_band(table, t, filters).map(|(_, r)| r))
    }

    /// The narrowest band of a key other than `own_key` that the filters bound, if it is a clear gain over `own` (the rows
    /// of the band of `own_key`; `None`: the filters do not bound it, so the whole table). Against a narrow band of the
    /// own key, a key is sorted only when that band holds at least a quarter of the table: a scan that runs after each
    /// write (an Insert of two rows, then a Find; the Insert test of Tests-SCM codeunit 137380) would sort every such key
    /// each time, which costs more than walking the band.
    pub(super) fn narrowest_band(
        &self,
        slot: &Slot,
        t: &TableDef,
        table: &Table,
        filters: &[FieldFilter],
        own_key: usize,
        own: Option<usize>,
    ) -> Option<Band> {
        let sort = own.is_none_or(|m| m * 4 >= table.len());
        let mut best: Option<(Band, usize)> = None;
        for j in (0..t.keys.len()).filter(|&j| j != own_key) {
            if let Some((b, n)) = self.band_of(slot, j, t, table, filters, sort) {
                if best.as_ref().is_none_or(|(_, m)| n < *m) {
                    best = Some((b, n));
                }
            }
        }
        best.filter(|(_, n)| *n * 4 <= own.unwrap_or(table.len()))
            .map(|(b, _)| b)
    }

    /// The band of key `j` that the filters leave, and how many rows it holds; `None` when the filters bound no
    /// leading field of the key or the order cannot be trusted, or (`sort` false) when no order of the key is cached.
    pub(super) fn band_of(
        &self,
        slot: &Slot,
        j: usize,
        t: &TableDef,
        table: &Table,
        filters: &[FieldFilter],
        sort: bool,
    ) -> Option<(Band, usize)> {
        let seek = Seek::of(t, j, filters, table.values().next())?;
        if j == 0 {
            let n = pk_band(table, t, filters).count();
            return Some((Band::Pk, n));
        }
        let order = if sort {
            self.order_of(slot, j, t, table)
        } else {
            self.cached_order(slot, j, t, table)?
        };
        if order.len() != table.len() || !order.iter().take(1).all(|k| table.get(k).is_some()) {
            return None;
        }
        let trusted = std::cell::Cell::new(true);
        let probe = |k: &Key, f: &dyn Fn(&[Value]) -> Option<bool>| match table.get(k).map(|r| f(r)) {
            Some(Some(b)) => b,
            _ => {
                trusted.set(false);
                false
            }
        };
        let start = order.partition_point(|k| probe(k, &|r| seek.before(r)));
        let end = order.partition_point(|k| !probe(k, &|r| seek.after(r)));
        (trusted.get() && start <= end).then(|| {
            let n = end - start;
            (Band::Order(order, start..end), n)
        })
    }
}
