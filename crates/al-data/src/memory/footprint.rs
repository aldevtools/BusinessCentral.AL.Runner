//! The rows part of an install session's footprint (ADR-079 decision 1): every row and table extent the store reads or
//! writes while a footprint is recorded ([`MemStore::footprint_start`]). Every read and write of a row passes the
//! `Store` methods of `MemStore`, so this record has no gap for stored tables. A table served by a provider
//! (`VirtualTables`) is noted apart ([`StoreFootprint::served`]): the store does not know what its rows depend on, so
//! the caller classifies it or counts it as touching everything.
//!
//! Which reads see which writes ([`Resource::read_sees_write`]) is decided here, the one owner of that rule; al-vm
//! `install_footprint.rs` builds the rest of the footprint on it.

use super::MemStore;
use crate::{AccessContext, TableDef};
use al_builtins::value::Value;
use std::collections::BTreeSet;
use std::sync::Arc;

/// A lasting resource of the store.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Resource {
    /// One row: company (empty for `DataPerCompany = false`), table object, encoded primary key.
    Row(Arc<str>, u32, Vec<u8>),
    /// Which rows a table holds, and through a filtered read what they hold: company, table object.
    Extent(Arc<str>, u32),
}

impl Resource {
    /// The company and table of the resource.
    pub fn table(&self) -> (&str, u32) {
        match self {
            Resource::Row(c, t, _) | Resource::Extent(c, t) => (c, *t),
        }
    }

    /// Whether a read of `self` may see the write `w`. A row read sees a write of the same row and a write of the extent
    /// (a DeleteAll removes rows it does not name). An extent read (`Find*`, `Next`, `Count`, `IsEmpty`, `DeleteAll`)
    /// sees every write of the table: a `Modify` changes what a filtered read returns, so a row write counts for it too
    /// (ADR-079 decision 1 lists `Modify` under rows only; reading it here as well keeps every gap a conflict).
    pub fn read_sees_write(&self, w: &Resource) -> bool {
        match (self, w) {
            (Resource::Row(..), Resource::Row(..)) => self == w,
            _ => self.table() == w.table(),
        }
    }

    /// Whether two writes collide: both write the same row. Two writes of an extent (two inserts of different keys)
    /// commute; a write that also reads the extent is caught by [`Resource::read_sees_write`].
    pub fn writes_collide(&self, other: &Resource) -> bool {
        matches!(self, Resource::Row(..)) && self == other
    }
}

/// What the store read and wrote while a footprint was recorded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoreFootprint {
    pub reads: BTreeSet<Resource>,
    pub writes: BTreeSet<Resource>,
    /// Tables served by a provider that were read or written (object ids): the caller classifies them.
    pub served: BTreeSet<u32>,
}

/// How a store method touches a row or the extent of its table.
#[derive(Clone, Copy)]
pub(super) enum Touch {
    /// `Get`.
    ReadRow,
    /// `Next`, `Count`.
    ReadExtent,
    /// `Insert`, `Delete`: reads the row (whether it exists decides the outcome), writes it and the extent.
    InsertOrDelete,
    /// `Modify`: reads and writes the row.
    Modify,
    /// `DeleteAll`: reads and writes the extent.
    DeleteAll,
}

fn encoded(pk: &[Value]) -> Vec<u8> {
    let mut out = Vec::new();
    if !al_builtins::value_codec::encode_row(pk, &mut out) {
        // A key value with no encoding: its debug form still tells two keys apart.
        out = format!("{pk:?}").into_bytes();
    }
    out
}

impl MemStore {
    /// Starts recording the footprint, replacing one that was recorded. Shared by the clones of the store, so a
    /// snapshot put back or a rollback does not lose it.
    pub fn footprint_start(&mut self) {
        self.footprint = Some(std::rc::Rc::new(std::cell::RefCell::new(StoreFootprint::default())));
    }

    /// Stops recording and gives what was recorded (`None` when nothing was).
    pub fn footprint_take(&mut self) -> Option<StoreFootprint> {
        self.footprint.take().map(|f| f.take())
    }

    /// Notes `touch` of table `t` (row `pk` when the touch names a row).
    pub(super) fn footprint_note(&self, cx: &AccessContext, t: &TableDef, pk: Option<&[Value]>, touch: Touch) {
        let Some(f) = &self.footprint else {
            return;
        };
        let mut f = f.borrow_mut();
        if self.virtual_of(t).is_some() {
            f.served.insert(t.object);
            return;
        }
        let company = super::slot(cx, t).0;
        let extent = Resource::Extent(company.clone(), t.object);
        let row = pk.map(|pk| Resource::Row(company, t.object, encoded(pk)));
        match (touch, row) {
            (Touch::ReadRow, Some(r)) => {
                f.reads.insert(r);
            }
            (Touch::Modify, Some(r)) => {
                f.reads.insert(r.clone());
                f.writes.insert(r);
            }
            (Touch::InsertOrDelete, Some(r)) => {
                f.reads.insert(r.clone());
                f.writes.insert(r);
                f.writes.insert(extent);
            }
            (Touch::DeleteAll, _) => {
                f.reads.insert(extent.clone());
                f.writes.insert(extent);
            }
            // A row touch without a key: count it for the whole table.
            (Touch::ReadExtent | Touch::ReadRow, _) => {
                f.reads.insert(extent);
            }
            (Touch::Modify | Touch::InsertOrDelete, None) => {
                f.reads.insert(extent.clone());
                f.writes.insert(extent);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contract::{cx, row, table};
    use crate::{SnapshotStore, Store};

    #[test]
    fn the_store_records_rows_and_extents_read_and_written_and_keeps_them_over_a_restore() {
        let (c, t) = (cx("C"), table());
        let mut s = MemStore::default();
        s.insert(&c, &t, row("B", 1, 1)).unwrap();
        let snap = s.snapshot();
        s.footprint_start();
        s.insert(&c, &t, row("A", 1, 1)).unwrap();
        s.restore(snap);
        s.count(&c, &t, &[]).unwrap();
        let f = s.footprint_take().unwrap();
        let extent = Resource::Extent("C".into(), t.object);
        assert!(f.writes.contains(&extent), "an insert writes the extent");
        assert!(f.reads.contains(&extent), "a count reads the extent");
        let rows: Vec<_> = f.writes.iter().filter(|r| matches!(r, Resource::Row(..))).collect();
        assert_eq!(rows.len(), 1, "the inserted row, kept over the restore");
        assert!(
            f.reads.contains(rows[0]),
            "an insert reads its row: whether it exists decides the outcome"
        );
        assert!(s.footprint_take().is_none(), "take stops the recording");
    }

    #[test]
    fn an_extent_read_sees_every_write_of_its_table_and_a_row_read_only_its_row_or_the_extent() {
        let row = |k: u8| Resource::Row("C".into(), 7, vec![k]);
        let extent = Resource::Extent("C".into(), 7);
        let other = Resource::Extent("C".into(), 8);
        assert!(
            extent.read_sees_write(&row(1)),
            "a Modify changes what a filtered read returns"
        );
        assert!(row(1).read_sees_write(&extent));
        assert!(row(1).read_sees_write(&row(1)));
        assert!(!row(1).read_sees_write(&row(2)));
        assert!(!extent.read_sees_write(&other));
        assert!(row(1).writes_collide(&row(1)));
        assert!(!extent.writes_collide(&extent), "two inserts of different keys commute");
    }
}
