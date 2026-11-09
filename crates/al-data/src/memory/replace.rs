//! Replacing every row of a table at once: what a `DeleteAll` without filters followed by one `Insert` per row does,
//! in one pass.
//!
//! The Code Coverage table is written this way on every read after recording (al-vm `platform/coverage.rs`): about
//! 200,000 rows for a posting test. Removing the old rows one by one and inserting the new ones one by one cost about
//! 0.3 s per refresh (Tests-SCM codeunit 137380 "SCM Performance Tests" refreshes 59 times). Building the table from
//! the sorted rows takes a fraction of that.

use super::{Key, MemStore, Row, Table, bump, slot};
use crate::{AccessContext, InsertOutcome, Store, StoreError, TableDef};

impl MemStore {
    /// Replaces every row of `t` with `rows`, as `delete_all(cx, t, &[])` followed by `insert(cx, t, row)` for each row
    /// in order would: a row whose primary key an earlier row already took is not inserted (Insert answers
    /// `AlreadyExists`). Returns how many rows were inserted.
    pub fn replace_all(&mut self, cx: &AccessContext, t: &TableDef, rows: Vec<Row>) -> Result<usize, StoreError> {
        // A virtual table, and a store that writes down changes to the backing's rows, take the general path: both act
        // per row.
        if self.virtual_of(t).is_some() || self.restored_log.is_some() {
            return self.replace_one_by_one(cx, t, rows);
        }
        // The DeleteAll.
        bump();
        self.unknown_read(cx, t, &[], &[])?;
        Self::code_storage_read(t, &[], &[])?;
        let had = self.table(cx, t).is_some_and(|m| !m.is_empty());
        let sl = slot(cx, t);
        if had {
            self.touch_shape(cx, t);
        }
        self.emptied.insert(sl.clone(), ());
        // The Inserts: each counts as one store operation, as an Insert does.
        let mut keyed: Vec<(Key, Row)> = Vec::with_capacity(rows.len());
        for row in rows {
            bump();
            self.note_unknown(t, &row);
            keyed.push((Key::of(t, t.primary_key(&row)), row));
        }
        // The first row with a key wins (a stable sort keeps the input order among equal keys).
        keyed.sort_by(|a, b| a.0.cmp(&b.0));
        keyed.dedup_by(|later, first| later.0 == first.0);
        let inserted = keyed.len();
        let table: Table = keyed.into_iter().collect();
        self.tables.insert(sl, table);
        if inserted > 0 {
            self.touch_shape(cx, t);
        }
        Ok(inserted)
    }

    fn replace_one_by_one(&mut self, cx: &AccessContext, t: &TableDef, rows: Vec<Row>) -> Result<usize, StoreError> {
        self.delete_all(cx, t, &[])?;
        let mut inserted = 0;
        for row in rows {
            if self.insert(cx, t, row)? == InsertOutcome::Inserted {
                inserted += 1;
            }
        }
        Ok(inserted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scan;
    use crate::contract::{cx, row, table};

    /// Every row a scan in key `key` returns, in order (written out: a value has no equality).
    fn scan(s: &MemStore, c: &AccessContext, t: &TableDef, key: usize, ascending: bool) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut after: Option<Vec<Value>> = None;
        while let Some(r) = s
            .next(
                c,
                t,
                &Scan {
                    key,
                    filters: &[],
                    ascending,
                    after: after.as_deref(),
                },
            )
            .unwrap()
        {
            after = Some(crate::position(t, key, &r));
            out.push(format!("{r:?}"));
        }
        out
    }

    use al_builtins::value::Value;

    /// A store with committed rows, an open change, and a cached order of the secondary key (a read sorted it).
    fn store() -> MemStore {
        let c = cx("C");
        let t = table();
        let mut s = MemStore::default();
        for (no, amount, group) in [("B", 2, 1), ("A", 1, 2), ("C", 3, 1)] {
            s.insert(&c, &t, row(no, amount, group)).unwrap();
        }
        s.commit();
        s.insert(&c, &t, row("D", 4, 0)).unwrap();
        scan(&s, &c, &t, 1, true);
        s
    }

    /// The new rows: out of key order, with a key that comes twice (the first one wins, as Insert answers AlreadyExists).
    fn new_rows() -> Vec<Row> {
        vec![
            row("Z", 9, 3),
            row("B", 5, 1),
            row("M", 7, 2),
            row("B", 6, 0),
            row("A", 1, 1),
        ]
    }

    #[test]
    fn replace_all_leaves_the_store_as_a_delete_all_and_one_insert_per_row_do() {
        let (c, t) = (cx("C"), table());
        let (mut general, mut fast) = (store(), store());
        let since = general.change_counter().min(fast.change_counter());
        let ops0 = super::super::store_ops();
        general.delete_all(&c, &t, &[]).unwrap();
        let mut inserted = 0;
        for r in new_rows() {
            if general.insert(&c, &t, r).unwrap() == InsertOutcome::Inserted {
                inserted += 1;
            }
        }
        let general_ops = super::super::store_ops() - ops0;
        let ops1 = super::super::store_ops();
        assert_eq!(fast.replace_all(&c, &t, new_rows()).unwrap(), inserted);
        assert_eq!(
            super::super::store_ops() - ops1,
            general_ops,
            "as many store operations"
        );
        for key in 0..t.keys.len() {
            for ascending in [true, false] {
                assert_eq!(
                    scan(&fast, &c, &t, key, ascending),
                    scan(&general, &c, &t, key, ascending),
                    "key {key}"
                );
            }
        }
        assert_eq!(fast.count(&c, &t, &[]).unwrap(), 4);
        assert!(fast.emptied(&c, &t));
        assert_eq!(fast.tables_changed_since(since), general.tables_changed_since(since));
        // A rollback brings back the committed rows in both.
        general.rollback();
        fast.rollback();
        assert_eq!(scan(&fast, &c, &t, 1, true), scan(&general, &c, &t, 1, true));
        assert_eq!(fast.count(&c, &t, &[]).unwrap(), 3);
    }

    #[test]
    fn replace_all_with_no_rows_empties_the_table_and_of_an_empty_table_changes_nothing() {
        let (c, t) = (cx("C"), table());
        let mut s = store();
        assert_eq!(s.replace_all(&c, &t, Vec::new()).unwrap(), 0);
        assert_eq!(s.count(&c, &t, &[]).unwrap(), 0);
        let before = s.change_counter();
        assert_eq!(s.replace_all(&c, &t, Vec::new()).unwrap(), 0);
        assert!(
            s.tables_changed_since(before).is_empty(),
            "nothing to delete, nothing inserted"
        );
    }
}
