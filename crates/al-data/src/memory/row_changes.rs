//! The committed rows one store holds apart from another, row by row ([`MemStore::committed_row_changes_from`]).
//!
//! Two readers, each naming what a table `base` holds no entry for is compared with ([`BeforeRows`]): the safety net of
//! an install footprint (`footprint.rs` `MemStore::unrecorded_writes`, ADR-079 step 1) and the login image (al-vm
//! `login_image/rows.rs`). The persistent maps share every entry a session did not touch, so the difference is found by
//! comparing pointers, and only the rows that differ are compared by value. The lane starts of ADR-066 amendment 5 that replayed the
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
    /// The rows that are new or hold other values, with their primary keys, in key order. All of its rows when its keys
    /// order otherwise, or when the other store holds no entry for it and the caller asked for [`BeforeRows::Empty`].
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

/// What a table the other store holds no entry for is compared with ([`MemStore::committed_row_changes_from`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeforeRows {
    /// The backing's rows the other store reads for it (when it has some), so only the rows that differ from those are
    /// put: the install footprint's safety net, which asks which rows a write changed (al-runner issue 8954).
    Backing,
    /// No rows, so every row of the table is put: the login image, which replaces such a table whole.
    Empty,
}

/// Whether two rows (or keys) are the same; `None` when one holds a value that is not plain.
pub type SameRows<'a> = dyn FnMut(&[Value], &[Value]) -> Option<bool> + 'a;

fn name(s: &Slot) -> (String, u32) {
    (s.0.to_string(), s.1)
}

impl MemStore {
    /// The committed rows of this store apart from `base`'s, row by row: the tables whose rows differ, with the rows
    /// that are new or changed and the keys of those that are gone, and the tables `base` holds that this store does
    /// not. A table `base` holds no entry for is compared with what `before` names ([`BeforeRows`]). A table with
    /// `base`'s stamp holds `base`'s rows (stamps are unique in the process) and is skipped; an entry both maps share is
    /// the same row. `same(a, b)` says whether two rows (or keys) are the same, as their bytes are;
    /// `None` from it (a value that is not plain) gives `None`. `None` too when a committed mark the rows do not show
    /// differs ([`MemStore::same_unshown_marks`]).
    pub fn committed_row_changes_from(
        &self,
        base: &MemStore,
        before: BeforeRows,
        same: &mut SameRows<'_>,
    ) -> Option<RowChanges> {
        if !self.same_unshown_marks(base) {
            return None;
        }
        let mut out = RowChanges::default();
        for (slot, table) in self.committed.iter() {
            // A table `base` holds no entry for reads the backing's rows there; the first write here copied them in
            // (`table_mut`). See [`BeforeRows`].
            let backed;
            let old = match base.committed.get(slot) {
                Some(old)
                    if table.ptr_eq(old) || self.committed_versions.get(slot) == base.committed_versions.get(slot) =>
                {
                    continue;
                }
                Some(old) => old,
                None => match (before == BeforeRows::Backing)
                    .then(|| base.backing_rows(&slot.0, slot.1))
                    .flatten()
                {
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

impl MemStore {
    /// Whether this store's committed company copies and deleted companies are `base`'s: they change what a table's
    /// rows answer, and a row change does not show them (maintainer's review of al-runner PR 8665: a login that deletes a
    /// company; al-runner issue 9039: an install order that deletes one). Every field is named, so a new mark of the
    /// store does not compile here until it says what a row change does with it.
    pub fn same_unshown_marks(&self, base: &MemStore) -> bool {
        let MemStore {
            // The open transaction: a row change reads the committed store only.
            tables: _,
            versions: _,
            shapes: _,
            moves: _,
            emptied: _,
            copies: _,
            dropped_backed_companies: _,
            source_stamps: _,
            virtual_write_pending: _,
            // The rows themselves: compared row by row.
            committed: _,
            // Stamps: a store the rows are put on takes stamps of its own.
            committed_versions: _,
            committed_shapes: _,
            committed_moves: _,
            committed_source_stamps: _,
            counter: _,
            // A DeleteAll without filters is a write of its table: the callers read the marks (`committed_marks`,
            // `committed_table`).
            committed_emptied: _,
            // What the rows answer besides themselves: compared here.
            committed_copies,
            committed_dropped_backed_companies,
            // Caches, notes of what was held and read, and the store's set-up: no committed content.
            order_cache: _,
            alt_cache: _,
            virtuals: _,
            backing: _,
            restored_log: _,
            unknown_cols: _,
            accent_cols: _,
            ae_cols: _,
            collation_verdicts: _,
            fold_indexes: _,
            modify_runs: _,
            watch: _,
            footprint: _,
            counts: _,
            database: _,
            connection: _,
        } = self;
        committed_copies.iter().eq(base.committed_copies.iter())
            && committed_dropped_backed_companies
                .keys()
                .eq(base.committed_dropped_backed_companies.keys())
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
        let c = later
            .committed_row_changes_from(&base, BeforeRows::Empty, &mut same)
            .unwrap();
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
                .committed_row_changes_from(&base, BeforeRows::Empty, &mut never)
                .unwrap()
                .tables
                .is_empty()
        );
    }

    /// A company deleted with its rows takes the rows a backing holds in it (`drop_company`), which no row shows: a store
    /// whose deleted companies differ from `base`'s has no row changes from it, as with other company copies (maintainer's
    /// review of al-runner PR 8665).
    #[test]
    fn another_set_of_deleted_companies_gives_no_row_changes() {
        use crate::Store;
        let base = MemStore::from_exported(vec![table(2)]);
        let mut later = base.clone();
        later.drop_company("OTHER");
        later.commit();
        let mut same = |a: &[Value], b: &[Value]| Some(format!("{a:?}") == format!("{b:?}"));
        assert!(
            later
                .committed_row_changes_from(&base, BeforeRows::Empty, &mut same)
                .is_none()
        );
        assert!(
            base.committed_row_changes_from(&later, BeforeRows::Empty, &mut same)
                .is_none()
        );
    }
}
