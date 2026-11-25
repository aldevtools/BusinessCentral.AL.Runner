//! The rows part of an install session's footprint (ADR-079 decision 1): every row and table extent the store reads or
//! writes while a footprint is recorded ([`MemStore::footprint_start`]). The `Store` methods note precisely what they
//! touch ([`MemStore::footprint_note`]). Every other read or write of the store is caught where the store reads a table
//! (`seen`, `seen_all`, `table_mut`) or by one note in the method, and counts for the whole table in every company
//! ([`Resource::Table`]) or for everything ([`StoreFootprint::everything`]), so the record has no silent gap. A table
//! served by a provider (`VirtualTables`) is noted apart ([`StoreFootprint::served`]): the store does not know what its
//! rows depend on, so the caller classifies it or counts it as touching everything.
//!
//! Which reads see which writes ([`Resource::read_sees_write`]) and which writes collide is decided here, the one owner
//! of those rules; al-vm `install_footprint.rs` builds the rest of the footprint on them.

use super::{Key, MemStore};
use crate::{AccessContext, TableDef};
use al_builtins::value::Value;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;
use std::sync::Arc;

/// The primary key of a [`Resource::Row`]: two keys are one when the store finds one row by both, so it compares as
/// the store compares the keys of the table (`memory/key.rs`; trailing blanks of a text key do not count,
/// `collation::text_key_order`). Not hashable: equal keys may differ in their values.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct RowKey(Key);

impl RowKey {
    /// The key of the primary key values `pk` of table `t`.
    pub fn of(t: &TableDef, pk: Vec<Value>) -> RowKey {
        RowKey(Key::of(t, pk))
    }
}

/// A lasting resource of the store.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Resource {
    /// One row: company (empty for `DataPerCompany = false`), table object, primary key. Rows of one table object carry
    /// keys of one order, so the derived order compares two keys only when they belong to the same table.
    Row(Arc<str>, u32, RowKey),
    /// Every row of a table in a company: read by a filtered read (which rows the table holds and what they hold),
    /// written by a write that may change any row (`DeleteAll`, a write the store cannot place by key).
    Extent(Arc<str>, u32),
    /// Every row of a table object in every company: what a read or write the store cannot place more precisely touches.
    Table(u32),
}

impl Resource {
    /// The table object of the resource.
    pub fn object(&self) -> u32 {
        match self {
            Resource::Row(_, t, _) | Resource::Extent(_, t) | Resource::Table(t) => *t,
        }
    }

    /// Whether `self` and `other` may name rows of the same table in the same company.
    fn same_table(&self, other: &Resource) -> bool {
        let company = |r: &Resource| match r {
            Resource::Row(c, ..) | Resource::Extent(c, _) => Some(c.clone()),
            Resource::Table(_) => None,
        };
        self.object() == other.object()
            && match (company(self), company(other)) {
                (Some(a), Some(b)) => a == b,
                _ => true,
            }
    }

    /// Whether a read of `self` may see the write `w`. A row read sees a write of the same row and a write of the extent
    /// (a DeleteAll removes rows it does not name). An extent read (`Find*`, `Next`, `Count`, `IsEmpty`, `DeleteAll`)
    /// sees every write of the table: a `Modify` changes what a filtered read returns, so a row write counts for it too
    /// (ADR-079 decision 1 lists `Modify` under rows only; reading it here as well keeps every gap a conflict). A
    /// [`Resource::Table`] read or write meets every resource of its table object.
    pub fn read_sees_write(&self, w: &Resource) -> bool {
        match (self, w) {
            (Resource::Row(..), Resource::Row(..)) => self == w,
            _ => self.same_table(w),
        }
    }

    /// Whether two writes collide: both write the same row, or one is a write the store could not place
    /// ([`Resource::Table`]) of the other's table. Two writes of an extent (two inserts of different keys) commute; a
    /// write that also reads the extent is caught by [`Resource::read_sees_write`].
    pub fn writes_collide(&self, other: &Resource) -> bool {
        match (self, other) {
            (Resource::Table(_), _) | (_, Resource::Table(_)) => self.same_table(other),
            (Resource::Row(..), Resource::Row(..)) => self == other,
            _ => false,
        }
    }
}

/// The writes of one footprint, indexed for [`Resource::read_sees_write`] and [`Resource::writes_collide`]: a lookup per
/// read or write instead of a scan of every write (the review of PR 8865, finding L7). It answers what the two rules
/// answer against every write of the set; `an_index_answers_what_the_rules_answer` checks that on every pair of kinds.
#[derive(Debug, Default)]
pub struct WriteIndex<'a> {
    rows: BTreeSet<&'a Resource>,
    /// (company, table) of every row or extent write.
    in_company: BTreeSet<(&'a str, u32)>,
    /// (company, table) of every extent write.
    extents: BTreeSet<(&'a str, u32)>,
    /// Tables written as a whole ([`Resource::Table`]).
    whole: BTreeSet<u32>,
    /// Tables of every write.
    objects: BTreeSet<u32>,
}

impl<'a> WriteIndex<'a> {
    pub fn of(writes: impl IntoIterator<Item = &'a Resource>) -> WriteIndex<'a> {
        let mut x = WriteIndex::default();
        for w in writes {
            x.objects.insert(w.object());
            match w {
                Resource::Row(c, t, _) => {
                    x.rows.insert(w);
                    x.in_company.insert((c, *t));
                }
                Resource::Extent(c, t) => {
                    x.in_company.insert((c, *t));
                    x.extents.insert((c, *t));
                }
                Resource::Table(t) => {
                    x.whole.insert(*t);
                }
            }
        }
        x
    }

    /// Whether the read `r` sees one of the writes ([`Resource::read_sees_write`]).
    pub fn seen_by(&self, r: &Resource) -> bool {
        match r {
            Resource::Row(c, t, _) => {
                self.rows.contains(r) || self.extents.contains(&(&**c, *t)) || self.whole.contains(t)
            }
            Resource::Extent(c, t) => self.in_company.contains(&(&**c, *t)) || self.whole.contains(t),
            Resource::Table(t) => self.objects.contains(t),
        }
    }

    /// Whether the write `w` collides with one of the writes ([`Resource::writes_collide`]).
    pub fn collides_with(&self, w: &Resource) -> bool {
        match w {
            Resource::Row(_, t, _) => self.rows.contains(w) || self.whole.contains(t),
            Resource::Extent(_, t) => self.whole.contains(t),
            Resource::Table(t) => self.objects.contains(t),
        }
    }
}

/// What the store read and wrote while a footprint was recorded.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct StoreFootprint {
    pub reads: BTreeSet<Resource>,
    pub writes: BTreeSet<Resource>,
    /// Tables served by a provider that were read or written (object ids): the caller classifies them.
    pub served: BTreeSet<u32>,
    /// Why the session counts as touching everything (a read of every table, a company copy, a whole-store export, a
    /// read of changes from before the recording): empty when it does not.
    pub everything: BTreeSet<&'static str>,
}

/// The recording shared by the clones of a store.
#[derive(Debug)]
pub(super) struct Recorder {
    data: RefCell<StoreFootprint>,
    /// How many `Store` methods that note precisely what they touch are running: while one is, the catch-all
    /// notes of `seen`, `seen_all` and `table_mut` stay out (they are that method's own table reads).
    precise: Cell<u32>,
    /// The change counter when the recording started: a read of the changes since an older counter reads writes of
    /// earlier sessions.
    start_counter: u64,
    /// Whether a write was pending when the recording started: then a read of "a write is pending" reads more than the
    /// session's own writes.
    start_pending: bool,
}

/// Held while a `Store` method that noted its touch runs ([`MemStore::footprint_note`]).
pub(super) struct Precise(Option<Rc<Recorder>>);

impl Drop for Precise {
    fn drop(&mut self) {
        if let Some(r) = &self.0 {
            r.precise.set(r.precise.get() - 1);
        }
    }
}

/// How a store method touches a row or the extent of its table.
#[derive(Clone, Copy)]
pub(super) enum Touch {
    /// `Get`.
    ReadRow,
    /// `Next`, `Count`, and the reads of whether or when a table changed.
    ReadExtent,
    /// `Insert`, `Delete`: reads the row (whether it exists decides the outcome) and writes it.
    InsertOrDelete,
    /// `Modify`: reads and writes the row.
    Modify,
    /// `DeleteAll`: reads and writes the extent.
    DeleteAll,
}

impl MemStore {
    /// The safety net of a footprint (ADR-079 step 1): `Ok` when every committed row this store holds apart from
    /// `before` (new, changed or gone) is covered by a write of `writes`: a write of that row, of its table's extent in its
    /// company, or of the whole table. Else the reason, which counts as touching everything: the recorder missed a write;
    /// with the table (object index) whose row changed or that was emptied, when one is known.
    /// Rows are compared by their bytes; a row that is not a plain value counts as changed.
    pub fn unrecorded_writes(
        &self,
        before: &MemStore,
        writes: &BTreeSet<Resource>,
    ) -> Result<(), (&'static str, Option<u32>)> {
        let (mut x, mut y) = (Vec::new(), Vec::new());
        let mut same = |a: &[Value], b: &[Value]| {
            x.clear();
            y.clear();
            let plain =
                al_builtins::value_codec::encode_row(a, &mut x) && al_builtins::value_codec::encode_row(b, &mut y);
            Some(plain && x == y)
        };
        let Some(changes) = self.committed_row_changes_from(before, &mut same) else {
            return Err(("the companies copied changed", None));
        };
        let covers_table = |company: &str, object: u32| {
            writes.contains(&Resource::Table(object)) || writes.contains(&Resource::Extent(company.into(), object))
        };
        for t in &changes.tables {
            if covers_table(&t.company, t.object) {
                continue;
            }
            let keys = t
                .put
                .iter()
                .map(|(k, _)| k.as_slice())
                .chain(t.gone.iter().map(Vec::as_slice));
            for k in keys {
                let key = RowKey(Key::in_table(k.to_vec(), &t.pk_storage));
                if !writes.contains(&Resource::Row(t.company.as_str().into(), t.object, key)) {
                    return Err(("a row changed that no recorded write names", Some(t.object)));
                }
            }
        }
        if let Some((_, o)) = changes.gone.iter().find(|(c, o)| !covers_table(c, *o)) {
            return Err(("a table emptied that no recorded write names", Some(*o)));
        }
        Ok(())
    }
}

impl MemStore {
    /// Starts recording the footprint, replacing one that was recorded. Shared by the clones of the store, so a
    /// snapshot put back or a rollback does not lose it.
    pub fn footprint_start(&mut self) {
        let start_pending = self.has_uncommitted() || self.virtual_write_pending();
        self.footprint = Some(Rc::new(Recorder {
            data: RefCell::new(StoreFootprint::default()),
            precise: Cell::new(0),
            start_counter: self.counter,
            start_pending,
        }));
    }

    /// Stops recording and gives what was recorded (`None` when nothing was).
    pub fn footprint_take(&mut self) -> Option<StoreFootprint> {
        self.footprint.take().map(|f| f.data.take())
    }

    /// Notes `touch` of table `t` (row `pk` when the touch names a row). The method keeps the returned guard while it
    /// runs, so its own table reads are not noted again by the catch-all.
    #[must_use]
    pub(super) fn footprint_note(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        pk: Option<&[Value]>,
        touch: Touch,
    ) -> Precise {
        let Some(rec) = &self.footprint else {
            return Precise(None);
        };
        rec.precise.set(rec.precise.get() + 1);
        let guard = Precise(Some(rec.clone()));
        let mut f = rec.data.borrow_mut();
        if self.virtual_of(t).is_some() {
            f.served.insert(t.object);
            return guard;
        }
        let company = super::slot(cx, t).0;
        let extent = Resource::Extent(company.clone(), t.object);
        let row = pk.map(|pk| Resource::Row(company, t.object, RowKey::of(t, pk.to_vec())));
        match (touch, row) {
            (Touch::ReadRow, Some(r)) => {
                f.reads.insert(r);
            }
            (Touch::Modify, Some(r)) => {
                f.reads.insert(r.clone());
                f.writes.insert(r);
            }
            // An insert or delete changes its row only. A filtered read sees it as a write of a row of its table
            // (`read_sees_write`), so no extent write is needed: that one stands for a write of every row.
            (Touch::InsertOrDelete, Some(r)) => {
                f.reads.insert(r.clone());
                f.writes.insert(r);
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
        drop(f);
        guard
    }

    /// The recorder, when one records and no precise `Store` method is running.
    fn footprint_open(&self) -> Option<&Recorder> {
        self.footprint.as_deref().filter(|r| r.precise.get() == 0)
    }

    /// The catch-all of a read of table `object` (`seen`): outside a precise method it reads the whole table.
    pub(super) fn footprint_seen(&self, object: u32) {
        if let Some(r) = self.footprint_open() {
            r.data.borrow_mut().reads.insert(Resource::Table(object));
        }
    }

    /// The catch-all of a write of table `object` (`table_mut`): outside a precise method it reads and writes the whole
    /// table.
    pub(super) fn footprint_written(&self, object: u32) {
        if let Some(r) = self.footprint_open() {
            let mut f = r.data.borrow_mut();
            f.reads.insert(Resource::Table(object));
            f.writes.insert(Resource::Table(object));
        }
    }

    /// Marks the recorded session as touching everything, for `why` (a read of every table, a company copy, a
    /// whole-store export).
    pub(super) fn footprint_everything(&self, why: &'static str) {
        if let Some(r) = &self.footprint {
            r.data.borrow_mut().everything.insert(why);
        }
    }

    /// A read of the changes made after change counter `counter`: one older than the recording reads writes of earlier
    /// sessions.
    pub(super) fn footprint_changes_since(&self, counter: u64) {
        if self.footprint.as_ref().is_some_and(|r| counter < r.start_counter) {
            self.footprint_everything("a read of the changes from before the install session");
        }
    }

    /// A read of whether a write is pending: when one was pending at the start, it reads more than the session's own.
    pub(super) fn footprint_pending_read(&self) {
        if self.footprint.as_ref().is_some_and(|r| r.start_pending) {
            self.footprint_everything("a read of the writes pending from before the install session");
        }
    }

    /// Runs `f` on `view` (a copy of this store) without recording, for a copy whose own writes are no session write
    /// (`without_rows`).
    pub(super) fn footprint_unrecorded<R>(view: &mut MemStore, f: impl FnOnce(&mut MemStore) -> R) -> R {
        let rec = view.footprint.take();
        let r = f(view);
        view.footprint = rec;
        r
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
        assert!(f.everything.is_empty());
        let extent = Resource::Extent("C".into(), t.object);
        assert!(
            !f.writes.contains(&extent),
            "an insert writes its row only; a filtered read sees that"
        );
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
        let t = crate::contract::table();
        let row = |k: i32| Resource::Row("C".into(), t.object, RowKey::of(&t, vec![Value::Integer(k)]));
        let extent = Resource::Extent("C".into(), t.object);
        let other = Resource::Extent("C".into(), t.object + 1);
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

    #[test]
    fn an_index_answers_what_the_rules_answer() {
        let t = crate::contract::table();
        let row = |c: &str, o: u32, k: i32| Resource::Row(c.into(), o, RowKey::of(&t, vec![Value::Integer(k)]));
        let all = [
            row("C", 1, 1),
            row("C", 1, 2),
            row("D", 1, 1),
            row("C", 2, 1),
            Resource::Extent("C".into(), 1),
            Resource::Extent("D".into(), 1),
            Resource::Extent("C".into(), 2),
            Resource::Table(1),
            Resource::Table(2),
        ];
        // Every write set of up to two resources, against every read and write.
        let mut sets: Vec<Vec<&Resource>> = vec![vec![]];
        for (i, a) in all.iter().enumerate() {
            sets.push(vec![a]);
            for b in &all[i + 1..] {
                sets.push(vec![a, b]);
            }
        }
        for set in &sets {
            let x = WriteIndex::of(set.iter().copied());
            for r in &all {
                let seen = set.iter().any(|w| r.read_sees_write(w));
                assert_eq!(x.seen_by(r), seen, "read {r:?} of {set:?}");
                let collide = set.iter().any(|w| r.writes_collide(w));
                assert_eq!(x.collides_with(r), collide, "write {r:?} of {set:?}");
            }
        }
    }
}
