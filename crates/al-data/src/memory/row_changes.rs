//! The committed rows one store holds apart from another, row by row ([`MemStore::committed_row_changes_from`]), and
//! a store with such changes put on top ([`MemStore::with_committed_row_changes`]).
//!
//! ADR-066 amendment 5, decision 5 (al-runner issue 6924): a serial session on a worker thread starts from a baseline
//! a carry-chain member left. Written whole as bytes for every step that needed it, a start with about 65,000 rows of
//! test data cost the main thread about 78 ms each, thousands of times. Two baselines next to each other in the carry
//! chain differ in the few rows one member wrote; the persistent maps share every entry the member did not touch, so
//! the difference is found by comparing pointers, and only the rows that differ are compared by value.

use super::{Key, MemStore, PkStorage, Row, Slot, Table, table_storage};
use crate::{Store, Value};

/// One table's rows in a [`RowChanges`].
#[derive(Debug, Clone)]
pub struct TableRows {
    pub company: String,
    pub object: u32,
    /// The table is new (the other store holds no entry for it) or its keys order otherwise: `put` holds all of its
    /// rows and replaces what the other store holds.
    pub whole: bool,
    /// How the table's primary key fields are stored: the order of its keys (`memory/key.rs`).
    pub pk_storage: PkStorage,
    /// The rows that are new or hold other values, with their primary keys, in key order.
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
    /// The tables a DeleteAll without filters emptied, as this store has them (all of them, not a difference).
    pub emptied: Vec<(String, u32)>,
}

/// Whether two rows (or keys) are the same; `None` when one holds a value that is not plain.
pub type SameRows<'a> = dyn FnMut(&[Value], &[Value]) -> Option<bool> + 'a;

fn name(s: &Slot) -> (String, u32) {
    (s.0.to_string(), s.1)
}

impl MemStore {
    /// The committed rows of this store apart from `base`'s, row by row: the tables whose rows differ, with the rows
    /// that are new or changed and the keys of those that are gone, and the tables `base` holds that this store does
    /// not. A table with `base`'s stamp holds `base`'s rows (stamps are unique in the process) and is skipped; an entry
    /// both maps share is the same row. `same(a, b)` says whether two rows (or keys) are the same, as their bytes are;
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
            let Some(old) = base.committed.get(slot) else {
                out.tables.push(whole(slot, table));
                continue;
            };
            if table.ptr_eq(old) || self.committed_versions.get(slot) == base.committed_versions.get(slot) {
                continue;
            }
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
                    whole: false,
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
        out.emptied = self.committed_emptied.keys().map(name).collect();
        Some(out)
    }

    /// This store, with no change open, with `changes` put on top of its committed rows: each table of it replaced
    /// whole or changed row by row, the tables it names gone removed, and the tables a DeleteAll emptied as it has them.
    /// The changed tables get new stamps, as a write gives them. `None` when a table to change row by row is not held.
    pub fn with_committed_row_changes(mut self, changes: RowChanges) -> Option<MemStore> {
        self.rollback();
        for (company, object) in &changes.gone {
            let slot: Slot = (std::sync::Arc::from(company.as_str()), *object);
            self.tables.remove(&slot);
            self.versions.remove(&slot);
            self.shapes.remove(&slot);
        }
        for t in changes.tables {
            let slot: Slot = (std::sync::Arc::from(t.company.as_str()), t.object);
            let mut table: Table = if t.whole {
                Table::new()
            } else {
                self.tables.get(&slot)?.clone()
            };
            for k in t.gone {
                table.remove(&Key::in_table(k, &t.pk_storage));
            }
            for (k, r) in t.put {
                table.insert(Key::in_table(k, &t.pk_storage), r);
            }
            self.counter = super::next_stamp();
            self.versions.insert(slot.clone(), self.counter);
            self.shapes.insert(slot.clone(), self.counter);
            self.tables.insert(slot, table);
        }
        let mut emptied = crate::pmap::PMap::new();
        for (company, object) in changes.emptied {
            emptied.insert((std::sync::Arc::from(company.as_str()), object), ());
        }
        self.emptied = emptied;
        self.commit();
        Some(self)
    }
}

/// Every row of `table`, to replace whole.
fn whole(slot: &Slot, table: &Table) -> TableRows {
    let (company, object) = name(slot);
    TableRows {
        company,
        object,
        whole: true,
        pk_storage: table_storage(table),
        put: table.iter().map(|(k, r)| (k.0.clone(), r.clone())).collect(),
        gone: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::ExportedTable;
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
    fn rows_a_clone_shares_are_not_compared_and_a_copy_with_the_changes_holds_the_same_rows() {
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
        let copy = MemStore::from_exported(vec![table(2000)])
            .with_committed_row_changes(c)
            .unwrap();
        let rows = |s: &MemStore| format!("{:?}", s.export_committed().unwrap());
        assert_eq!(rows(&copy), rows(&later));
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
