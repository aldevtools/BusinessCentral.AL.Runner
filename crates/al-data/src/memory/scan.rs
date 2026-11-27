//! How a scan steps through a table in the order of one of its keys (ADR-035), in one place (issue 7671). The order of
//! a key and the direction of a scan are [`KeyOrder`]: how two rows compare, how a row stands to a position, whether a
//! row lies past the scan's position, which of two rows the scan meets first, the first row of a set, and the part of a
//! sorted list past a position. Every scan path asks it; none compares rows or tests the direction itself.
//!
//! The order of a key and a scan's direction in one place (`KeyOrder`, issue 7671): every scan path
//! (`MemStore::next`, `alt_next`, the order cache) compares and steps through it. Entry point: `KeyOrder`.
//! Unit tests: `memory/tests/scan_tests.rs`, `memory/tests/seek_tests.rs`.

use crate::{Row, Scan, TableDef, Value};
use std::cmp::Ordering;
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

/// The row after the position of `scan` in `keys` (the rows that pass, in the order of the scan's key `by`).
pub(super) fn search_sorted(table: &super::Table, keys: &[super::Key], scan: &Scan, by: KeyOrder) -> Option<Row> {
    let past = by.past_in(keys, scan.after, |k, a| table.get(k).map(|r| by.cmp_to_position(r, a)));
    by.first_in(&keys[past], Some).and_then(|k| table.get(k).cloned())
}
