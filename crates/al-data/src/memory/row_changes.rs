//! The committed rows one store holds apart from another, row by row ([`MemStore::committed_row_changes_from`]).
//!
//! The reader is the safety net of an install footprint (`footprint.rs` `MemStore::unrecorded_writes`, ADR-079 step 1).
//! The persistent maps share every entry a session did not touch, so the difference is found by comparing pointers, and
//! only the rows that differ are compared by value. The lane starts of ADR-066 amendment 5 that replayed the
//! changes are gone, and the replay with them (ADR-070 step 3).

use super::{MemStore, PkStorage, Row, Slot, Table, table_storage};
use crate::Value;

/// One table's rows in a [`RowChanges`].
#[derive(Debug, Clone)]
pub struct TableRows {
    pub company: String,
    pub object: u32,
    /// How the table's primary key fields are stored: the order of its keys (`memory/key.rs`).
    pub pk_storage: PkStorage,
    /// The rows that are new or hold other values, with their primary keys, in key order. All of its rows when the table
    /// is new (the other store holds no entry for it) or its keys order otherwise.
    pub put: Vec<(Vec<Value>, Row)>,
    /// The primary keys of the rows that are gone.
    pub gone: Vec<Vec<Value>>,
}

/// What a store's committed rows hold apart from another store's ([`MemStore::committed_row_changes_from`]).
#[derive(Debug, Clone, Default)]
pub struct RowChanges {
    /// The tables whose rows differ.
    pub tables: Vec<TableRows>,
    /// The tables the other store holds an entry for and this one does not.
    pub gone: Vec<(String, u32)>,
}

/// Whether two rows (or keys) are the same; `None` when one holds a value that is not plain.
pub type SameRows<'a> = dyn FnMut(&[Value], &[Value]) -> Option<bool> + 'a;

fn name(s: &Slot) -> (String, u32) {
    (s.0.to_string(), s.1)
}

impl MemStore {
    /// The committed rows of this store apart from `base`'s, row by row: the tables whose rows differ, with the rows
    /// that are new or changed and the keys of those that are gone, and the tables `base` holds that this store does
    /// not. A table `base` holds no entry for is compared with the backing's rows `base` reads for it. A table with
    /// `base`'s stamp holds `base`'s rows (stamps are unique in the process) and is skipped; an entry both maps share is
    /// the same row. `same(a, b)` says whether two rows (or keys) are the same, as their bytes are;
    /// `None` from it (a value that is not plain) gives `None`. `None` too when the company copies differ.
    pub fn committed_row_changes_from(&self, base: &MemStore, same: &mut SameRows<'_>) -> Option<RowChanges> {
        let same_copies = self.committed_copies.len() == base.committed_copies.len()
            && self
                .committed_copies
                .iter()
                .all(|(k, v)| base.committed_copies.get(k) == Some(v));
        if !same_copies {
            return None;
        }
        let mut out = RowChanges::default();
        for (slot, table) in self.committed.iter() {
            // A table `base` holds no entry for reads the backing's rows there (`backing_rows`); the first write here
            // copied them in (`table_mut`), so they are the rows to compare with (al-runner issue 8954).
            let backed;
            let old = match base.committed.get(slot) {
                Some(old)
                    if table.ptr_eq(old) || self.committed_versions.get(slot) == base.committed_versions.get(slot) =>
                {
                    continue;
                }
                Some(old) => old,
                None => match base.backing_rows(&slot.0, slot.1) {
                    Some(rows) => {
                        backed = rows.0;
                        &backed
                    }
                    None => {
                        out.tables.push(whole(slot, table));
                        continue;
                    }
                },
            };
            let mut put = Vec::new();
            let mut gone = Vec::new();
            let (mut a, mut b) = (table.iter().peekable(), old.iter().peekable());
            loop {
                match (a.peek(), b.peek()) {
                    (None, None) => break,
                    (Some((k, r)), None) => {
                        put.push((k.0.clone(), (*r).clone()));
                        a.next();
                    }
                    (None, Some((k, _))) => {
                        gone.push(k.0.clone());
                        b.next();
                    }
                    (Some((ka, ra)), Some((kb, rb))) => match (*ka).cmp(*kb) {
                        std::cmp::Ordering::Less => {
                            put.push((ka.0.clone(), (*ra).clone()));
                            a.next();
                        }
                        std::cmp::Ordering::Greater => {
                            gone.push(kb.0.clone());
                            b.next();
                        }
                        std::cmp::Ordering::Equal => {
                            // One shared entry: the same key and row (the maps share entries a write did not touch).
                            let shared = std::ptr::eq(*ka, *kb) && std::ptr::eq(*ra, *rb);
                            if !shared && !(same(&ka.0, &kb.0)? && same(ra, rb)?) {
                                put.push((ka.0.clone(), (*ra).clone()));
                            }
                            a.next();
                            b.next();
                        }
                    },
                }
            }
            if !put.is_empty() || !gone.is_empty() {
                let (company, object) = name(slot);
                out.tables.push(TableRows {
                    company,
                    object,
                    pk_storage: table_storage(table),
                    put,
                    gone,
                });
            }
        }
        out.gone = base
            .committed
            .keys()
            .filter(|s| !self.committed.contains_key(*s))
            .map(name)
            .collect();
        Some(out)
    }
}

/// Every row of `table`, to replace whole.
fn whole(slot: &Slot, table: &Table) -> TableRows {
    let (company, object) = name(slot);
    TableRows {
        company,
        object,
        pk_storage: table_storage(table),
        put: table.iter().map(|(k, r)| (k.0.clone(), r.clone())).collect(),
        gone: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::{ExportedTable, Key};
    use super::*;

    fn table(n: i32) -> ExportedTable {
        ExportedTable {
            company: "CRONUS".into(),
            object: 18,
            pk_storage: None,
            rows: (1..=n)
                .map(|k| (vec![Value::Integer(k)], vec![Value::Integer(k), Value::Integer(0)]))
                .collect(),
        }
    }

    /// A store a write made from a clone of another shares every entry the write did not touch: the difference is
    /// found by pointers, and only the changed rows are compared by value (issue 6924: a 65,000-row start per step).
    #[test]
    fn rows_a_clone_shares_are_not_compared_and_the_changes_name_the_changed_rows() {
        let base = MemStore::from_exported(vec![table(2000)]);
        let mut later = base.clone();
        let slot: Slot = (std::sync::Arc::from("CRONUS"), 18);
        let mut t = later.committed.get(&slot).unwrap().clone();
        t.insert(
            Key::in_table(vec![Value::Integer(5)], &None),
            vec![Value::Integer(5), Value::Integer(9)],
        );
        t.remove(&Key::in_table(vec![Value::Integer(6)], &None));
        t.insert(
            Key::in_table(vec![Value::Integer(3000)], &None),
            vec![Value::Integer(3000), Value::Integer(1)],
        );
        later.counter = super::super::next_stamp();
        later.committed_versions.insert(slot.clone(), later.counter);
        later.committed.insert(slot, t);
        let mut compared = 0;
        let mut same = |a: &[Value], b: &[Value]| {
            compared += 1;
            Some(format!("{a:?}") == format!("{b:?}"))
        };
        let c = later.committed_row_changes_from(&base, &mut same).unwrap();
        assert!(compared <= 2, "only the changed row: {compared}");
        assert_eq!(c.tables.len(), 1);
        assert_eq!((c.tables[0].put.len(), c.tables[0].gone.len()), (2, 1));
        let keys: Vec<_> = c.tables[0].put.iter().map(|(k, _)| k).collect();
        assert_eq!(format!("{keys:?}"), "[[Integer(5)], [Integer(3000)]]");
        assert_eq!(format!("{:?}", c.tables[0].gone), "[[Integer(6)]]");
        // Nothing changed: nothing to put.
        let mut never = |_: &[Value], _: &[Value]| -> Option<bool> { panic!("no row compared") };
        assert!(
            base.clone()
                .committed_row_changes_from(&base, &mut never)
                .unwrap()
                .tables
                .is_empty()
        );
    }
}
