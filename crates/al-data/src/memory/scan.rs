//! How a scan steps through a table in the order of one of its keys (ADR-035), in one place (issue 7671).
//!
//! - [`KeyOrder`] is the order of a key and the direction of a scan: how two rows compare, how a row stands to a
//!   position, whether a row lies past the scan's position, which of two rows the scan meets first, the first row of a
//!   set, and the part of a sorted list past a position. No scan path compares rows or tests the direction itself.
//! - `MemStore::step` picks the path of one step: the band of another key the filters bound (`alt_next`, with the
//!   lists `alt_cache` keeps), the primary key's order (`pk_step`), the cached order of the scan's key
//!   (`ordered_step`), or a look at every row of a small table. Each reads its band through `memory/band.rs`.
//!
//! One step of a scan (`MemStore::step`, called by `next`) and the order of a key with the scan's
//! direction (`KeyOrder`), in one place (issue 7671). Entry points: `KeyOrder`, `MemStore::step`.
//! Unit tests: `memory/tests/scan_tests.rs`, `memory/tests/seek_tests.rs`.

use super::*;
use std::ops::Range;

/// The order of key `key` of table `t` as a scan in one direction reads it. A row's place is its position
/// ([`crate::position`]: the key's fields, then the primary key's), compared field by field with
/// [`TableDef::cmp_key_field`], so a row is never copied into a position to be compared.
#[derive(Clone, Copy)]
pub(super) struct KeyOrder<'a> {
    t: &'a TableDef,
    key: usize,
    ascending: bool,
}

impl<'a> KeyOrder<'a> {
    /// The order the scan `scan` reads: its key, in its direction.
    pub(super) fn of(t: &'a TableDef, scan: &Scan) -> KeyOrder<'a> {
        KeyOrder {
            t,
            key: scan.key,
            ascending: scan.ascending,
        }
    }

    /// The order of key `key`, ascending: the order the store sorts the rows of a table into.
    pub(super) fn sorting(t: &'a TableDef, key: usize) -> KeyOrder<'a> {
        KeyOrder {
            t,
            key,
            ascending: true,
        }
    }

    /// The fields of a position in this key.
    fn fields(&self) -> impl Iterator<Item = &'a usize> + use<'a> {
        self.t.keys[self.key].iter().chain(&self.t.keys[0])
    }

    /// `key_cmp(position(a), position(b))` without building the positions.
    pub(super) fn cmp_rows(&self, a: &[Value], b: &[Value]) -> Ordering {
        for &i in self.fields() {
            match self.t.cmp_key_field(i, &a[i], &b[i]) {
                Ordering::Equal => continue,
                o => return o,
            }
        }
        Ordering::Equal
    }

    /// `key_cmp(position(row), pos)` without building the position: field by field, then the shorter first.
    pub(super) fn cmp_to_position(&self, row: &[Value], pos: &[Value]) -> Ordering {
        for (&i, x) in self.fields().zip(pos) {
            match self.t.cmp_key_field(i, &row[i], x) {
                Ordering::Equal => continue,
                o => return o,
            }
        }
        (self.t.keys[self.key].len() + self.t.keys[0].len()).cmp(&pos.len())
    }

    /// Whether a row that stands `o` to the scan's position lies past it in the scan's direction.
    fn past(&self, o: Ordering) -> bool {
        if self.ascending {
            o == Ordering::Greater
        } else {
            o == Ordering::Less
        }
    }

    /// Whether `row` lies past the position `after` in the scan's direction; every row does when there is none.
    pub(super) fn after_ok(&self, row: &[Value], after: Option<&[Value]>) -> bool {
        after.is_none_or(|a| self.past(self.cmp_to_position(row, a)))
    }

    /// Whether the scan meets row `a` before row `b`.
    pub(super) fn better(&self, a: &[Value], b: &[Value]) -> bool {
        self.past(self.cmp_rows(a, b).reverse())
    }

    /// The row the scan returns of `rows` (in any order): the first it meets past the position `after`.
    pub(super) fn first_of<'r>(&self, rows: impl Iterator<Item = &'r Row>, after: Option<&[Value]>) -> Option<&'r Row> {
        let mut best: Option<&Row> = None;
        for row in rows {
            if self.after_ok(row, after) && best.is_none_or(|b| self.better(row, b)) {
                best = Some(row);
            }
        }
        best
    }

    /// The part of `sorted` (items in this key's order) that lies past the position `after` in the scan's direction:
    /// the items after it ascending, those before it descending; all of them without a position. `place` tells how an
    /// item stands to a position; one it cannot place (`None`, a row the table no longer holds) counts as past the
    /// position ascending and as not past it descending.
    pub(super) fn past_in<T>(
        &self,
        sorted: &[T],
        after: Option<&[Value]>,
        place: impl Fn(&T, &[Value]) -> Option<Ordering>,
    ) -> Range<usize> {
        let Some(a) = after else {
            return 0..sorted.len();
        };
        if self.ascending {
            sorted.partition_point(|x| place(x, a).is_some_and(|o| o != Ordering::Greater))..sorted.len()
        } else {
            0..sorted.partition_point(|x| place(x, a).is_some_and(|o| o == Ordering::Less))
        }
    }

    /// The first item of `slice` (in this key's order) that the scan meets and `pick` takes: from the front ascending,
    /// from the back descending.
    pub(super) fn first_in<'s, T, R>(&self, slice: &'s [T], pick: impl FnMut(&'s T) -> Option<R>) -> Option<R> {
        if self.ascending {
            slice.iter().find_map(pick)
        } else {
            slice.iter().rev().find_map(pick)
        }
    }
}

/// The part two ranges of one list share; empty (at the end of the shared part) when they share none.
pub(super) fn meet(a: Range<usize>, b: Range<usize>) -> Range<usize> {
    let end = a.end.min(b.end);
    a.start.max(b.start).min(end)..end
}

impl MemStore {
    /// The row the scan returns: the first row past its position, in the order of its key, that passes its filters.
    /// `table` is the table as the store reads it, and `scan` holds the filters as the store reads them (`next` made
    /// its refusals first).
    pub(super) fn step(&self, cx: &AccessContext, t: &TableDef, scan: &Scan, table: &Table) -> Option<Row> {
        if table.len() >= INDEX_MIN_ROWS {
            if let Some(found) = self.alt_next(cx, t, scan, table, None) {
                return found;
            }
        }
        if scan.key == 0 {
            return pk_step(t, scan, table);
        }
        if table.len() >= INDEX_MIN_ROWS {
            if let Some(found) = self.ordered_step(cx, t, scan, table) {
                return found;
            }
        }
        // A small table in another key: every row is looked at, without building its position (a copy of the key fields).
        let rows = table.values().filter(|r| passes(r, scan.filters));
        KeyOrder::of(t, scan).first_of(rows, scan.after).cloned()
    }

    /// A big table in another key: steps through the table's rows in that key's order (`order_of`), in the band the
    /// filters on the leading fields of the key leave (ADR-035). `None` when the order does not hold the table's rows.
    fn ordered_step(&self, cx: &AccessContext, t: &TableDef, scan: &Scan, table: &Table) -> Option<Option<Row>> {
        let order = self.order_of(&slot(cx, t), scan.key, t, table);
        if !order_fits(&order, table) {
            return None;
        }
        let row_at = |k: &Key| table.get(k);
        let mut band = 0..order.len();
        if let Some(seek) = Seek::of(t, scan.key, scan.filters, table.values().next()) {
            // Rows that do not compare: the order cannot be trusted, so the whole of it is looked at.
            if let Some(b) = band::band(&order, |k| row_at(k), &seek) {
                band = b;
            }
            // A wide band of this key: another key the filters bound may hold far fewer rows
            // (Tests-SCM codeunit 137380 TenThousandSerialNos: "Reservation Entry" by source line and serial number).
            if band.len() >= INDEX_MIN_ROWS {
                if let Some(found) = self.alt_next(cx, t, scan, table, Some(band.len())) {
                    return Some(found);
                }
            }
        }
        let by = KeyOrder::of(t, scan);
        let past = by.past_in(&order, scan.after, |k, a| row_at(k).map(|r| by.cmp_to_position(r, a)));
        let found = by.first_in(&order[meet(past, band)], |k| {
            row_at(k).filter(|r| passes(r, scan.filters))
        });
        Some(found.cloned())
    }

    /// A scan whose own key is not bounded by the filters, but another key is (an index the filters narrow, as SQL
    /// picks one): the rows that pass are found in that key's band and put in the order of the scan's key, once
    /// per table version and set of filters (the cache `alt_cache`). `None`: the caller walks.
    ///
    /// The first step of a scan reads the band; the second builds the sorted list; later steps search it. The step
    /// right after a write walks (a loop that writes in every step must not pay for a band in every step).
    ///
    /// `own`: the number of rows of the band of the scan's own key, when the filters bound it (`None`: they do not). Another
    /// key's band is then read only when it is a clear gain over that band.
    fn alt_next(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        scan: &Scan,
        table: &Table,
        own: Option<usize>,
    ) -> Option<Option<Row>> {
        if scan.filters.is_empty()
            || (own.is_none() && Seek::of(t, scan.key, scan.filters, table.values().next()).is_some())
        {
            return None;
        }
        let sample = table.values().next();
        if !(0..t.keys.len()).any(|j| j != scan.key && Seek::of(t, j, scan.filters, sample).is_some()) {
            return None;
        }
        let slot = slot(cx, t);
        let stamp = self.stamp_of(&slot, &self.versions);
        let by = KeyOrder::of(t, scan);
        let calls = match self.alt_step(&slot, t, scan.key, scan.filters, stamp) {
            alt_cache::AltStep::Walk => return None,
            alt_cache::AltStep::Sorted(keys) => return Some(search_sorted(table, &keys, scan, by)),
            alt_cache::AltStep::Band(calls) => calls,
        };
        // When `own` is `None` the filters do not bound the scan's key (checked above), so it has no band to leave out.
        let Some(band) = self.narrowest_band(&slot, t, table, scan.filters, scan.key, own) else {
            self.alt_keep(&slot, t, scan.key, scan.filters, None);
            return None;
        };
        let rows = band
            .into_rows(table, t, scan.filters)
            .filter(|(_, r)| passes(r, scan.filters));
        if calls < 2 {
            // The first step: the best row of the band.
            return Some(by.first_of(rows.map(|(_, r)| r), scan.after).cloned());
        }
        let mut rows: Vec<(&Key, &Row)> = rows.collect();
        rows.sort_by(|a, b| by.cmp_rows(a.1, b.1));
        let keys = std::rc::Rc::new(rows.into_iter().map(|(k, _)| k.clone()).collect::<Vec<_>>());
        self.alt_keep(&slot, t, scan.key, scan.filters, Some(keys.clone()));
        Some(search_sorted(table, &keys, scan, by))
    }
}

/// In primary key order the table is already in the order of the scan: steps to the first row past the position and
/// looks at the rows from there on until one passes the filters.
fn pk_step(t: &TableDef, scan: &Scan, table: &Table) -> Option<Row> {
    use std::ops::Bound::{Excluded, Included, Unbounded};
    let k = t.keys[0].len();
    let after = scan.after.map(|a| Key::of(t, a[..k.min(a.len())].to_vec()));
    if scan.ascending {
        let seek = Seek::of(t, 0, scan.filters, table.values().next());
        let from = seek
            .as_ref()
            .filter(|s| !s.lo.is_empty())
            .map(|s| Key::of(t, s.lo.clone()));
        let lower = match (after, from) {
            (Some(a), Some(f)) if a.cmp(&f) == Ordering::Less => Included(f),
            (Some(a), _) => Excluded(a),
            (None, Some(f)) => Included(f),
            (None, None) => Unbounded,
        };
        return table
            .range((lower, Unbounded))
            .map(|(_, r)| r)
            // Past the last row the filters allow (a row that does not compare is looked at).
            .take_while(|r| seek.as_ref().is_none_or(|s| s.after(r) != Some(true)))
            .find(|r| passes(r, scan.filters))
            .cloned();
    }
    let upper = after.map_or(Unbounded, Excluded);
    table
        .range((Unbounded, upper))
        .rev()
        .map(|(_, r)| r)
        .find(|r| passes(r, scan.filters))
        .cloned()
}

/// The row after the position of `scan` in `keys` (the rows that pass, in the order of the scan's key `by`).
fn search_sorted(table: &Table, keys: &[Key], scan: &Scan, by: KeyOrder) -> Option<Row> {
    let past = by.past_in(keys, scan.after, |k, a| table.get(k).map(|r| by.cmp_to_position(r, a)));
    by.first_in(&keys[past], Some).and_then(|k| table.get(k).cloned())
}
