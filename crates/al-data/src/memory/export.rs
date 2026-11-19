//! The committed data of a store as plain values, and a store made from them: for a baseline on disk or on another
//! thread (al-vm `Baseline::to_bytes`, `Baseline::from_bytes`) and for the row deltas of a carry chain (al-vm
//! `baseline/row_delta.rs`). The rows and the tables a DeleteAll without filters emptied ([`MemStore::emptied`]) both
//! travel: a store made from the export holds what the exported one held, marks included.

use super::{Key, MemStore, PkStorage, Row, Slot, Table, next_stamp, table_storage};
use crate::{Store, Value};

/// One table of an exported store ([`MemStore::export_committed`]): its company (empty for a table that is not per
/// company), its object, and its rows with their primary keys, in key order.
#[derive(Debug, Clone)]
pub struct ExportedTable {
    pub company: String,
    pub object: u32,
    /// How the table's primary key fields are stored: the order of its keys (`memory/key.rs`).
    pub pk_storage: PkStorage,
    pub rows: Vec<(Vec<Value>, Row)>,
}

/// The slot of table `object` in `company`.
fn slot_of(company: &str, object: u32) -> Slot {
    (std::sync::Arc::from(company), object)
}

/// The emptied marks `tables` names, as a store holds them.
pub(super) fn emptied_marks(tables: &[(String, u32)]) -> crate::pmap::PMap<Slot, ()> {
    let mut marks = crate::pmap::PMap::new();
    for (company, object) in tables {
        marks.insert(slot_of(company, *object), ());
    }
    marks
}

impl MemStore {
    /// The committed rows of every table the store holds an entry for, for a snapshot on disk or for
    /// another thread (the rows are plain values; [`MemStore::from_exported`] is the way back, with
    /// [`MemStore::committed_emptied`]).
    /// A store with a backing (test data) exports the tables it holds: those a write reached, which
    /// carry the backing's rows with the changes, and an emptied one stays empty. The tables it has
    /// no entry for are the backing's and are not exported; the importer puts the same backing back
    /// (`with_backing`). The record of changes to restored rows is a log and is not exported.
    /// Changes that are not committed are not exported.
    pub fn export_committed(&self) -> Option<Vec<ExportedTable>> {
        Some(
            self.committed
                .iter()
                .map(|((company, object), table)| ExportedTable {
                    company: company.to_string(),
                    object: *object,
                    pk_storage: table_storage(table),
                    rows: table.iter().map(|(k, r)| (k.0.clone(), r.clone())).collect(),
                })
                .collect(),
        )
    }

    /// The committed tables (company, object) a DeleteAll without filters emptied ([`MemStore::emptied`]), sorted.
    pub fn committed_emptied(&self) -> Vec<(String, u32)> {
        let mut emptied: Vec<(String, u32)> = self.committed_emptied.keys().map(|s| (s.0.to_string(), s.1)).collect();
        emptied.sort();
        emptied
    }

    /// A store whose committed (and current) data is `tables`, with `emptied` the tables a DeleteAll without filters
    /// emptied ([`MemStore::committed_emptied`] of the exported store), and no provider and no backing. Its stamps
    /// are new ones: a stamp only says whether a table changed, and no cache of this store knows an older one.
    pub fn from_exported(tables: Vec<ExportedTable>, emptied: &[(String, u32)]) -> MemStore {
        let mut s = MemStore::default();
        for t in tables {
            let slot = slot_of(&t.company, t.object);
            let table: Table = t
                .rows
                .into_iter()
                .map(|(k, r)| (Key::in_table(k, &t.pk_storage), r))
                .collect();
            s.counter = next_stamp();
            s.versions.insert(slot.clone(), s.counter);
            s.shapes.insert(slot.clone(), s.counter);
            s.tables.insert(slot, table);
        }
        s.emptied = emptied_marks(emptied);
        s.committed = s.tables.clone();
        s.committed_versions = s.versions.clone();
        s.committed_shapes = s.shapes.clone();
        s.committed_emptied = s.emptied.clone();
        s.with_held_noted()
    }

    /// The committed rows of table `object` in `company` that the store holds, and whether a DeleteAll without filters
    /// emptied it ([`MemStore::emptied`]); `None` when the store holds no entry for it.
    pub fn committed_table(&self, company: &str, object: u32) -> Option<(ExportedTable, bool)> {
        let slot = slot_of(company, object);
        let table = self.committed.get(&slot)?;
        Some((
            ExportedTable {
                company: company.to_string(),
                object,
                pk_storage: table_storage(table),
                rows: table.iter().map(|(k, r)| (k.0.clone(), r.clone())).collect(),
            },
            self.committed_emptied.contains_key(&slot),
        ))
    }

    /// This store, with no change open, holding `tables` (each with whether a DeleteAll emptied it) in place of what it
    /// held for them and nothing for `gone`; the other tables keep their rows and stamps. The replaced tables get new
    /// stamps, as a write gives them.
    pub fn with_committed_tables(mut self, tables: Vec<(ExportedTable, bool)>, gone: &[(String, u32)]) -> MemStore {
        self.rollback();
        for (company, object) in gone {
            let slot = slot_of(company, *object);
            self.tables.remove(&slot);
            self.versions.remove(&slot);
            self.shapes.remove(&slot);
            self.emptied.remove(&slot);
        }
        for (t, emptied) in tables {
            let slot = slot_of(&t.company, t.object);
            let table: Table = t
                .rows
                .into_iter()
                .map(|(k, r)| (Key::in_table(k, &t.pk_storage), r))
                .collect();
            self.counter = next_stamp();
            self.versions.insert(slot.clone(), self.counter);
            self.shapes.insert(slot.clone(), self.counter);
            if emptied {
                self.emptied.insert(slot.clone(), ());
            } else {
                self.emptied.remove(&slot);
            }
            self.tables.insert(slot, table);
        }
        self.commit();
        self
    }

    /// The marks of the committed store besides its rows: the tables a DeleteAll emptied
    /// ([`MemStore::committed_emptied`]) and the company copies, sorted.
    #[allow(clippy::type_complexity)]
    pub fn committed_marks(&self) -> (Vec<(String, u32)>, Vec<(String, String)>) {
        let mut copies: Vec<(String, String)> = self
            .committed_copies
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        copies.sort();
        (self.committed_emptied(), copies)
    }
}
