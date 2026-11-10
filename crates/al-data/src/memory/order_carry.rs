//! Keeps a cached key order (`MemStore::order_of`) through a write that changes the place of one row: an Insert, a
//! Delete, or a Modify that moves the row in that key. The order is not sorted again: the row is taken out of, or put
//! into, the order at the place a binary search finds, and the result is cached under the table's new stamps.
//!
//! Al-runner issue 6924: a test that creates a No. Series (one No. Series Line Insert) and then takes a number from it
//! (a FindLast in the key "Series Code", "Starting Date") sorted the whole No. Series Line table again on every
//! cycle; on a company with thousands of lines that was the self time of `No. Series - Impl..GetNoSeriesLine`.
//!
//! Only an order that a read used since it was cached is carried (`OrderEntry::used`): a run of writes with no read in
//! between carries once and leaves the next read to sort, so writing never costs more than the reads would.

use super::{Key, MemStore, ORDERS_PER_KEY, OrderEntry, OrderKey, Row, Slot, Table, cmp_rows, slot};
use crate::{AccessContext, TableDef};
use std::cmp::Ordering;

/// What the write did to the row with key `key`.
#[derive(Clone, Copy)]
pub(super) enum Change<'a> {
    /// The row is new.
    Insert,
    /// The row was removed; it held `old`.
    Delete { old: &'a Row },
    /// The row stays, with new values in fields of the key; it held `old`.
    Move { old: &'a Row },
}

impl MemStore {
    /// After an Insert or a Delete of the row `key` (the table's shape was `from_shape` before it): carries the cached
    /// orders of every secondary key.
    pub(super) fn carry_orders(&self, cx: &AccessContext, t: &TableDef, key: &Key, change: Change, from_shape: u64) {
        let sl = slot(cx, t);
        if !self.has_orders(&sl, t) {
            return;
        }
        let Some(table) = self.tables.get(&sl) else { return };
        let to_shape = self.shapes.get(&sl).copied().unwrap_or(0);
        for j in 1..t.keys.len() {
            let moved = self.moves.get(&(sl.clone(), j)).copied().unwrap_or(0);
            self.carry_order(&sl, j, t, table, key, change, (from_shape, moved), (to_shape, moved));
        }
    }

    /// After a Modify moved the row `key` (it held `old`) in key `j`, whose move stamp was `from_moved` before it.
    pub(super) fn carry_moved(&self, sl: &Slot, j: usize, t: &TableDef, key: &Key, old: &Row, from_moved: u64) {
        let Some(table) = self.tables.get(sl) else { return };
        let shape = self.shapes.get(sl).copied().unwrap_or(0);
        let to_moved = self.moves.get(&(sl.clone(), j)).copied().unwrap_or(0);
        self.carry_order(
            sl,
            j,
            t,
            table,
            key,
            Change::Move { old },
            (shape, from_moved),
            (shape, to_moved),
        );
    }

    /// Carries the cached order of key `j` of the table in `slot` over a write to the row `key`. `table` holds the rows
    /// after the write; `from` and `to` are the (shape, moved) stamps of the key before and after it.
    #[allow(clippy::too_many_arguments)]
    fn carry_order(
        &self,
        slot: &Slot,
        j: usize,
        t: &TableDef,
        table: &Table,
        key: &Key,
        change: Change,
        from: (u64, u64),
        to: (u64, u64),
    ) {
        let ck = (slot.clone(), j);
        let mut cache = self.order_cache.borrow_mut();
        let Some(kept) = cache.get_mut(&ck) else { return };
        let rows_before = match change {
            Change::Insert => table.len().wrapping_sub(1),
            Change::Delete { .. } => table.len() + 1,
            Change::Move { .. } => table.len(),
        };
        let Some(i) = kept.iter().position(|e| {
            e.used && e.shape == from.0 && e.moved == from.1 && e.rows == rows_before && e.fields == t.keys[j]
        }) else {
            return;
        };
        let Some(order) = carried(&kept[i].order, t, j, table, key, &change) else {
            return;
        };
        kept[i].used = false;
        kept.insert(
            0,
            OrderEntry {
                shape: to.0,
                moved: to.1,
                rows: table.len(),
                fields: t.keys[j].clone(),
                order: std::rc::Rc::new(order),
                used: false,
            },
        );
        kept.truncate(ORDERS_PER_KEY);
    }

    /// The key indexes, other than `moved`, whose cached orders a Modify from `old` to `new` leaves out of place: an
    /// order of a sort that `t` does not list under that index (an ad-hoc key of a record) and some field of it changed.
    pub(super) fn stale_adhoc_orders(
        &self,
        sl: &Slot,
        t: &TableDef,
        moved: &[usize],
        old: Option<&Row>,
        new: &Row,
    ) -> Vec<usize> {
        let Some(old) = old else { return Vec::new() };
        let cache = self.order_cache.borrow();
        let mut out: Vec<usize> = Vec::new();
        for ((s, j), kept) in cache.iter() {
            if s != sl || moved.contains(j) || out.contains(j) {
                continue;
            }
            let foreign = |f: &Vec<usize>| t.keys.get(*j) != Some(f);
            if kept
                .iter()
                .any(|e| foreign(&e.fields) && super::key_fields_differ(&e.fields, old, new))
            {
                out.push(*j);
            }
        }
        out
    }

    /// Whether some order of table `slot` is cached, so a write needs to look at `carry_order` at all.
    pub(super) fn has_orders(&self, slot: &Slot, t: &TableDef) -> bool {
        let cache = self.order_cache.borrow();
        !cache.is_empty() && (1..t.keys.len()).any(|j| cache.contains_key(&(slot.clone(), j)))
    }
}

/// `order` (the keys of the rows before the write, sorted as `order_of` sorts them: by `cmp_rows`, then by key) with
/// the write applied. `None` when the order does not fit the table, so the next read sorts.
fn carried(
    order: &[OrderKey],
    t: &TableDef,
    j: usize,
    table: &Table,
    key: &Key,
    change: &Change,
) -> Option<Vec<OrderKey>> {
    let mut out = order.to_vec();
    let old = match change {
        Change::Insert => None,
        Change::Delete { old } | Change::Move { old } => Some(*old),
    };
    if let Some(old) = old {
        // The row stood where its old values sort; the rows around it are read as they are now.
        let at = place(&out, t, j, table, key, old, Some(old))?;
        if out.get(at).map(|k| &**k) != Some(key) {
            return None;
        }
        out.remove(at);
    }
    if !matches!(change, Change::Delete { .. }) {
        let new = table.get(key)?;
        let at = place(&out, t, j, table, key, new, None)?;
        out.insert(at, std::rc::Rc::new(key.clone()));
    }
    Some(out)
}

/// The first place in `order` whose row sorts at or after the row `key` holding `row`. `stale`: the values `key`
/// had before the write, for the one probe that meets `key` itself while it is still in `order`.
fn place(
    order: &[OrderKey],
    t: &TableDef,
    j: usize,
    table: &Table,
    key: &Key,
    row: &Row,
    stale: Option<&Row>,
) -> Option<usize> {
    let missing = std::cell::Cell::new(false);
    let at = order.partition_point(|k| {
        let k: &Key = k;
        let r = if k == key {
            stale.or_else(|| table.get(k))
        } else {
            table.get(k)
        };
        match r {
            Some(r) => cmp_rows(t, j, r, row).then_with(|| k.cmp(key)) == Ordering::Less,
            None => {
                missing.set(true);
                false
            }
        }
    });
    (!missing.get()).then_some(at)
}

#[cfg(test)]
mod tests {
    use super::super::{MemStore, order_builds};
    use crate::{Scan, Store};

    /// The No. Series case: a table of 1,000 rows read in a secondary key, with an Insert, a Delete or a Modify that
    /// moves a row between the reads.
    fn store() -> (MemStore, crate::TableDef, crate::AccessContext) {
        let mut t = crate::contract::table();
        t.keys = vec![vec![0], vec![2], vec![1]];
        let cx = crate::contract::cx("C");
        let mut s = MemStore::default();
        for i in 0..1000 {
            s.insert(&cx, &t, crate::contract::row(&format!("{i:04}"), i, i % 7))
                .unwrap();
        }
        s.commit();
        (s, t, cx)
    }

    /// Every row in key `key`, walked from the store as a FindSet/Next loop does.
    fn walk(s: &MemStore, t: &crate::TableDef, cx: &crate::AccessContext, key: usize) -> Vec<String> {
        crate::contract::scan_all(s, cx, t, key, &[], true)
    }

    fn last(s: &MemStore, t: &crate::TableDef, cx: &crate::AccessContext, key: usize) -> Option<String> {
        let scan = Scan {
            key,
            filters: &[],
            ascending: false,
            after: None,
        };
        s.next(cx, t, &scan).unwrap().map(|r| format!("{:?}", r[0]))
    }

    #[test]
    fn an_insert_between_reads_does_not_sort_the_table_again() {
        let (mut s, t, cx) = store();
        assert_eq!(last(&s, &t, &cx, 1).as_deref(), Some("Code(\"0993\")"));
        let before = order_builds();
        for i in 0..50 {
            s.insert(&cx, &t, crate::contract::row(&format!("N{i:03}"), 5000 + i, 6))
                .unwrap();
            assert_eq!(last(&s, &t, &cx, 1), Some(format!("Code(\"N{i:03}\")")));
        }
        assert_eq!(order_builds() - before, 0, "each Insert carries the order of key 1");
    }

    #[test]
    fn carried_orders_match_a_fresh_sort() {
        let (mut s, t, cx) = store();
        for key in [1, 2] {
            walk(&s, &t, &cx, key);
        }
        // Inserts, deletes and modifies that move rows in key 1, key 2 or both; a read between each, as GetNoSeriesLine
        // reads between the writes.
        for i in 0..60i32 {
            match i % 4 {
                0 => {
                    s.insert(
                        &cx,
                        &t,
                        crate::contract::row(&format!("{:04}", 2000 + i), i * 37 % 900, i % 5),
                    )
                    .unwrap();
                }
                1 => {
                    s.delete(&cx, &t, &crate::contract::row(&format!("{:04}", i * 13), 0, 0)[..1])
                        .unwrap();
                }
                2 => {
                    s.modify(
                        &cx,
                        &t,
                        crate::contract::row(&format!("{:04}", i * 7 + 1), i * 11 % 900, (i + 3) % 7),
                    )
                    .unwrap();
                }
                _ => {
                    s.modify(
                        &cx,
                        &t,
                        crate::contract::row(&format!("{:04}", i * 5 + 2), i * 5 + 2, 0),
                    )
                    .unwrap();
                }
            }
            last(&s, &t, &cx, 1);
            last(&s, &t, &cx, 2);
        }
        // The same rows in a store with no cached order: every order is sorted from the rows.
        let mut plain = MemStore::default();
        for no in crate::contract::scan_all(&s, &cx, &t, 0, &[], true) {
            let r = s
                .get(&cx, &t, &[crate::Value::Code(no.as_str().into())])
                .unwrap()
                .unwrap();
            plain.insert(&cx, &t, r).unwrap();
        }
        for key in [1, 2] {
            assert_eq!(walk(&s, &t, &cx, key), walk(&plain, &t, &cx, key), "key {key}");
        }
    }

    #[test]
    fn a_rollback_after_carried_inserts_reads_the_committed_order() {
        let (mut s, t, cx) = store();
        let committed = walk(&s, &t, &cx, 1);
        for i in 0..5 {
            s.insert(&cx, &t, crate::contract::row(&format!("R{i}"), 9000 + i, 6))
                .unwrap();
            last(&s, &t, &cx, 1);
        }
        s.rollback();
        assert_eq!(walk(&s, &t, &cx, 1), committed);
    }
}
