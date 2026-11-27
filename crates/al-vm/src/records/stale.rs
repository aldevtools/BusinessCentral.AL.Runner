//! Row-version notes and the stale-record verdict (moved out of `records.rs`).
//!
//! row-version notes (`note_write`, `note_bulk_write`/`note_bulk_end` for ModifyAll, DeleteAll,
//! Rename, Truncate), `modify_row_version` and `bump_row_version`.

use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_data::AccessContext;

/// Why `Database.LastUsedRowVersion` is refused after a write of these kinds that follows an Insert (`note_row_versions`).
const AFTER_INSERT: &str = "a Delete, Rename, DeleteAll, ModifyAll or Truncate after an Insert in the open transaction";

/// Where `Session::note_bulk_write` put its note: the index in `OpenTx::writes` and the store's change counter then.
#[derive(Clone, Copy)]
pub(crate) struct BulkMark {
    at: usize,
    before: u64,
}

impl Session {
    /// Every write gets a new, increasing SystemRowVersion.
    /// Returns the previous value, so a refused write can give it back
    /// (record/TestRefusedWriteRowVersion.al).
    /// Records a Record write to the database store (not a temporary table's) for `stale_verdict`; `before` is the store's
    /// change counter before the write.
    pub(super) fn note_write(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        kind: crate::state::TxWriteKind,
        pk: &[Value],
        before: u64,
    ) {
        if rec.borrow().temporary {
            return;
        }
        let company = rec.borrow().company.clone();
        self.note_row_versions(t.object, &company, kind, pk);
        if kind == crate::state::TxWriteKind::Insert {
            self.note_row_version_buffer(rec, t.object, &company);
            if self.ret_used {
                self.send_checked_insert(rec, t.object, &company);
            }
        }
        self.push_write(t, company, kind, pk, before, Default::default());
    }

    /// Notes a Record write for the row locks (`note_row_lock_write`) and appends it to `OpenTx::writes` (`note_write`).
    fn push_write(
        &mut self,
        t: &TableDef,
        company: Option<String>,
        kind: crate::state::TxWriteKind,
        pk: &[Value],
        before: u64,
        bulk_shape: crate::state::BulkShape,
    ) {
        self.note_row_lock_write(t.object, &company, kind, pk);
        let verdict = crate::version_entry::write_verdict(kind, self.through_recordref);
        self.tx.open.table_writes.note(t.object, &company, verdict);
        let unmeasured = crate::version_entry::unmeasured_after(kind, self.through_recordref);
        self.tx.write_counts.note(t.object, &company, unmeasured);
        self.tx.open.writes.push(crate::state::TxWrite {
            table: t.object,
            company,
            kind,
            pk: pk.to_vec(),
            before,
            after: self.store.change_counter(),
            bulk: None,
            through_recordref: self.through_recordref,
            bulk_shape,
        });
    }

    /// What a database write does to `Database.LastUsedRowVersion` (`last_used_row_version`). BC keeps the rows an Insert
    /// wrote until a read of their table or a Modify sends them: the value does not grow after an Insert alone
    /// (record/TestProbeRecordRefContracts.al:620-640 `grows_after_insert` No, metadata/platform-tables/PlTDatabase.al:
    /// 220-245 `after_insert_greater` No), it covers the inserted row after a Get of its table (corpus master
    /// PlTRowVersionValue.al:157-172 TV_MinimumActiveRowVersion_AcrossInsertModifyCommit, `insert open last > start last` and
    /// `row version <= last` Yes) and grows after a Modify (PlTDatabase.al:238 `after_modify_greater`, PlTRowVersionValue.al
    /// `modify open row version <= last`). A Delete, Rename, ModifyAll or DeleteAll after an Insert moves it only in the shapes
    /// `row_version_after_write` lets through; every other one, a Truncate after an Insert and row versions an Insert or a
    /// Modify does not account for are not pinned, so refused. `row` is the key of the row a Delete or Rename
    /// writes (empty for other bulk writes).
    pub(super) fn note_row_versions(
        &mut self,
        table: u32,
        company: &Option<String>,
        kind: crate::state::TxWriteKind,
        row: &[Value],
    ) {
        use crate::state::TxWriteKind;
        let rv = &mut self.tx.open.row_versions;
        // An Insert or a Modify hands out one row version; more since the last one accounted for came from another write.
        let new_versions = rv.handed_out != rv.accounted;
        if rv.handed_out > rv.accounted + 1 {
            rv.unpinned = Some("row versions handed out by a write other than an Insert or a Modify");
        }
        match kind {
            TxWriteKind::Insert if new_versions => {
                rv.accounted = rv.handed_out;
                rv.sent_by.insert();
                let last = rv.last;
                match rv
                    .pending_inserts
                    .iter_mut()
                    .find(|(o, c, _, _)| *o == table && c == company)
                {
                    Some(p) => p.3 = last,
                    None => rv.pending_inserts.push((table, company.clone(), last, last)),
                }
            }
            TxWriteKind::Modify if new_versions => {
                // A row the ModifyAll that sent the inserted rows modifies: its row versions count as a Modify's
                // (`note_bulk_end` ends that ModifyAll).
                // The Modify sends the inserted rows of its table first. The send corpus PR 1613 measures
                // (RowVersionReReadShapes.al `Run`, `Other.Insert(); Other.Modify()`): it sends one entry of one row, and
                // modifies the row that Insert wrote and nothing wrote since.
                let one_row = matches!(
                    rv.pending_inserts
                        .iter()
                        .filter(|(o, c, _, _)| *o == table && c == company)
                        .map(|(_, _, first, last)| first == last)
                        .collect::<Vec<_>>()
                        .as_slice(),
                    [true]
                );
                let measured = (
                    one_row,
                    super::write_notes::inserted_unwritten(&self.tx.open.writes, table, company, row),
                ) == (true, true);
                while let Some(at) = self
                    .tx
                    .open
                    .row_versions
                    .pending_inserts
                    .iter()
                    .position(|(o, c, _, _)| *o == table && c == company)
                {
                    self.send_pending_insert(at, false);
                }
                let rv = &mut self.tx.open.row_versions;
                rv.accounted = rv.handed_out;
                rv.sent = rv.sent.max(rv.last);
                rv.sent_by.raise(measured);
                // A Modify grows the value while another table holds unsent inserted rows, and those stay unsent
                // (PlTRowVersionValue.al UK_InsertThenModifyOtherTable).
            }
            TxWriteKind::Insert | TxWriteKind::Modify => {
                if !rv.pending_inserts.is_empty() {
                    rv.unpinned = Some("an Insert or Modify without a new row version after an Insert");
                }
            }
            // A Delete or DeleteAll hands out no row version and leaves the value as it was, also after a Commit (corpus branch
            // al-runtime-rs-staging-pub-uk beab119c, PlTRowVersionValue.al UK_DeleteWithoutInsert, UK_DeleteAllWithoutInsert; corpus
            // session-system/TestSystemExtended.al:635-659, TestBCDatabaseContracts.al and PnBRecordMisc.al:205-235 read it after a
            // DeleteAll). A Rename or ModifyAll hands out row versions no Insert or Modify accounts for
            // (`last_used_row_version` refuses those).
            // A Delete or ModifyAll of the one table whose inserted rows nothing sent yet sends them first: the value grows to their
            // row version, and the Modify of each row a ModifyAll modifies grows it on; a ModifyAll that modifies no row is not
            // measured, so it stays in `write_sends` (`row_version_after_write`). A Rename (`Other` until it
            // turns out a plain one) of that table, or of a table the transaction inserted into and sent, grows it to the row
            // version it hands out (`end_row_version_write`).
            TxWriteKind::Delete | TxWriteKind::ModifyAll | TxWriteKind::DeleteAll | TxWriteKind::Other
                if Self::row_version_after_write(
                    rv,
                    self.tx.committed_row_version,
                    &self.tx.open.writes,
                    (table, company, row),
                    kind,
                ) =>
            {
                match kind {
                    TxWriteKind::DeleteAll => rv.assumed_delete_all = true,
                    _ => rv.assumed_write = true,
                }
                if matches!(kind, TxWriteKind::ModifyAll | TxWriteKind::Other) {
                    rv.write_sends = Some((table, company.clone(), kind));
                }
                if let Some(at) = rv.pending_inserts.len().checked_sub(1) {
                    let v = self.send_pending_insert(at, false);
                    let rv = &mut self.tx.open.row_versions;
                    rv.sent = rv.sent.max(v);
                    rv.sent_by.raise(false);
                }
            }
            TxWriteKind::Delete
            | TxWriteKind::Rename
            | TxWriteKind::ModifyAll
            | TxWriteKind::DeleteAll
            | TxWriteKind::Truncate
            | TxWriteKind::Other => {
                if !rv.pending_inserts.is_empty() {
                    rv.unpinned = Some(AFTER_INSERT);
                }
            }
        }
    }

    /// Whether a write of `kind` to `table` after an Insert in the open transaction moves `Database.LastUsedRowVersion` as corpus
    /// PR 1509 (merged, not pinned: session-system/row-version-writes/RowVersionWrites.al) measured it, through the assumption
    /// `row-version-write-after-insert`. Delete (:147) and ModifyAll (:218): the inserted rows nothing sent yet are exactly those
    /// of `table`. Rename (:192, `Other` when noted): the same, or (:517) none are unsent and the last write of `table` in the open
    /// transaction is an Insert a read sent. The probe sent no other row version before those rows (`own_unsent`: the first of
    /// them is above every row version sent), and the Delete (:147) and Rename (:192, :517) wrote a row the transaction inserted
    /// and did not write since (`row`, `write_notes::inserted_unwritten`); a ModifyAll's rows are checked when it ends
    /// (`note_bulk_end`). Every other shape keeps the refusal of `note_row_versions`: a Truncate (:267) is a store write
    /// `writes_all_noted` does not count, and a ModifyAll of a table whose rows were sent (:544) is main's Modify rule.
    /// A DeleteAll (RowVersionWriteShapes.al of corpus PR 1621, :316, :338, :363) sends the inserted rows of its table as a
    /// ModifyAll does, through the assumption `row-version-delete-all`.
    fn row_version_after_write(
        rv: &crate::state::RowVersionTx,
        committed: i64,
        writes: &[crate::state::TxWrite],
        (table, company, row): (u32, &Option<String>, &[Value]),
        kind: crate::state::TxWriteKind,
    ) -> bool {
        use crate::state::TxWriteKind;
        let own_unsent = matches!(rv.pending_inserts.as_slice(),
            [(o, c, first, _)] if *o == table && c == company && *first > committed.max(rv.sent));
        let inserted_row = super::write_notes::inserted_unwritten(writes, table, company, row);
        // The last write of `table` is an Insert and no inserted row is unsent: a read of `table` sent it (only a read or a
        // write of `table` takes its rows out of `pending_inserts`).
        let sent_insert = rv.pending_inserts.is_empty()
            && writes
                .iter()
                .rev()
                .find(|w| w.table == table && w.company == *company)
                .is_some_and(|w| w.kind == TxWriteKind::Insert);
        let none_running = rv.write_sends.is_none() && rv.handed_out == rv.accounted;
        match kind {
            TxWriteKind::Delete => own_unsent && inserted_row,
            TxWriteKind::ModifyAll => none_running && own_unsent,
            // RowVersionWriteShapes.al of corpus PR 1621 (open): :316 (unfiltered), :338 (a filter that matches no inserted
            // row) and :363 (committed rows too): the value becomes the last inserted row's (assumption `row-version-delete-all`).
            TxWriteKind::DeleteAll => none_running && own_unsent,
            TxWriteKind::Other => none_running && inserted_row && (own_unsent || sent_insert),
            _ => false,
        }
    }

    /// A checked Insert (`if not Rec.Insert() then`, the return value used) sends its row at once: `Database.LastUsedRowVersion`
    /// becomes that row's SystemRowVersion (corpus PR 1621, open, not pinned: session-system/row-version-writes/
    /// RowVersionWriteShapes.al:172 RowVersion_CheckedInsert, :524 RowVersion_IssueWriteOrder; assumption
    /// `row-version-checked-insert`). The measured shape: a Record (not a RecordRef) Insert into a database table whose
    /// statement-form Insert is queued, no LockTable in the open transaction, and the Insert just handed out the one row version
    /// of its table's only unsent entry, as its last row (earlier plain Inserts of that table and other tables may hold unsent
    /// rows, :524). Every other checked Insert is refused.
    fn send_checked_insert(&mut self, rec: &Rec, table: u32, company: &Option<String>) {
        let queued = self.insert_is_queued(rec);
        let rv = &self.tx.open.row_versions;
        let own: Vec<usize> = (0..rv.pending_inserts.len())
            .filter(|&i| rv.pending_inserts[i].0 == table && rv.pending_inserts[i].1 == *company)
            .collect();
        // The Insert's row is the last of its table's one unsent entry; earlier plain Inserts of the table may be in it (:524:
        // three Inserts, then two checked ones, `checked last/row` same).
        let one_row = matches!(own.as_slice(), [i] if rv.pending_inserts[*i].3 == rv.last);
        let measured = (
            queued,
            self.through_recordref,
            self.tx.open.locked_tables.is_empty(),
            one_row,
            rv.handed_out == rv.accounted,
            rv.unpinned.is_none(),
        ) == (true, false, true, true, true, true);
        if !measured {
            self.tx.open.row_versions.unpinned = Some("a checked Insert in a shape corpus PR 1621 does not measure");
            return;
        }
        let v = self.send_pending_insert(own[0], false);
        let rv = &mut self.tx.open.row_versions;
        rv.sent = rv.sent.max(v);
        rv.sent_by.raise(false);
        rv.assumed_checked_insert = true;
    }

    /// The end of a plain Rename of `table` that `note_row_versions` let send (`RowVersionTx::write_sends`): the row version it
    /// handed out counts, and the value grows to it (RowVersionWrites.al of corpus PR 1509: :192 and :517, the renamed row's
    /// SystemRowVersion equals the value).
    pub(crate) fn end_row_version_write(&mut self, table: u32, company: &Option<String>) {
        let rv = &mut self.tx.open.row_versions;
        if rv
            .write_sends
            .as_ref()
            .is_some_and(|(o, c, k)| *o == table && c == company && *k == crate::state::TxWriteKind::Other)
        {
            rv.write_sends = None;
            rv.accounted = rv.handed_out;
            rv.sent = rv.sent.max(rv.last);
            rv.sent_by.raise(false);
        }
    }

    /// A read of a database table while the open transaction holds inserted rows nothing has sent yet
    /// (`note_row_versions`). `sends` is a Get, FindFirst, FindLast, FindSet, IsEmpty, Count or CalcSums: the first one of
    /// a table sends that table's rows and grows the value, a read of another table sends nothing (Get of the one table:
    /// PlTRowVersionValue.al:157-172 on corpus master; the rest: corpus branch al-runtime-rs-staging-pub-uk beab119c,
    /// PlTRowVersionValue.al UK_InsertThenFindSet, _FindFirst, _IsEmpty, _Count, _CalcSums, _ReadOtherTable,
    /// _ReadOtherThenSame, UK_InsertTwoTablesThenGetOne, UK_InsertReadInsertRead, UK_InsertThenModifyOtherTable). The rows
    /// get their row versions at the Insert here, so a read after a Modify handed out a higher one takes a new one to grow
    /// the value as BC does (UK_InsertThenModifyOtherTable `findfirst` greater). Other reads are not pinned. `measured`: a read
    /// corpus PR 1613 sends the row with (`RestampedRows::insert_value_read`).
    pub(crate) fn note_read_for_row_versions(
        &mut self,
        table: u32,
        company: &Option<String>,
        sends: bool,
        measured: bool,
    ) {
        let rv = &mut self.tx.open.row_versions;
        if rv.pending_inserts.is_empty() {
            return;
        }
        if !sends {
            rv.unpinned = Some("a read other than Get, FindFirst, FindLast, FindSet, IsEmpty, Count or CalcSums");
            return;
        }
        let Some(pos) = rv
            .pending_inserts
            .iter()
            .position(|(o, c, _, _)| *o == table && c == company)
        else {
            return;
        };
        let current = self.tx.committed_row_version.max(rv.sent);
        let v = self.send_pending_insert(pos, measured);
        if v > current {
            self.tx.open.row_versions.sent = v;
            self.tx.open.row_versions.sent_by.raise(false);
        } else {
            self.next_row_version += 1;
            self.tx.open.row_versions.sent = self.next_row_version;
            self.tx.open.row_versions.sent_by.raise(false);
        }
    }

    /// `Database.LastUsedRowVersion`: the committed value, raised by the rows the open transaction sent
    /// (`note_row_versions`). `Err` names why the value is not pinned.
    pub(crate) fn last_used_row_version(&self) -> Result<i64, &'static str> {
        let rv = &self.tx.open.row_versions;
        // Test data whose last row version is not known (a configuration package, a `.bacpac`): its rows may hold row
        // versions above the session's counter (`Session::with_host`).
        if self.engine.test_data.is_some() && self.restored_row_version == 0 {
            return Err("test data whose last row version is not known");
        }
        // No row version handed out since the transaction began: the committed value, whatever else was written.
        if self.next_row_version == self.tx.committed_row_version {
            return Ok(self.tx.committed_row_version.max(1));
        }
        if let Some(why) = rv.unpinned {
            return Err(why);
        }
        if rv.write_sends.is_some() {
            return Err("a Rename that rewrote related rows, or a ModifyAll that modified no row, after an Insert");
        }
        if rv.handed_out != rv.accounted {
            return Err("row versions handed out by a write other than an Insert or a Modify");
        }
        // A DeleteAll writes its rows without a note each (`note_bulk_end`): over one in the shape corpus PR 1621 measures
        // (RowVersionWriteShapes.al :256, :276, :298 with no inserted row unsent; :316, :338, :363 after Inserts of its
        // table; :524) the chain goes on from where it ended.
        let measured = |w: &crate::state::TxWrite| {
            let b = w.bulk_shape;
            (
                w.kind,
                w.through_recordref,
                b.argument,
                b.marked_only,
                w.bulk.as_ref().map(|e| e.entries),
            ) == (crate::state::TxWriteKind::DeleteAll, false, false, false, Some(0))
        };
        // A DeleteAll that sent inserted rows (`row_version_after_write`) in another shape is not measured.
        let mut delete_alls = self
            .tx
            .open
            .writes
            .iter()
            .filter(|w| w.kind == crate::state::TxWriteKind::DeleteAll);
        if rv.assumed_delete_all && !delete_alls.all(measured) {
            return Err("a DeleteAll after an Insert in a shape corpus PR 1621 does not measure");
        }
        let over_delete_all = std::cell::Cell::new(false);
        let noted = self.writes_noted_over(|w| {
            measured(w).then(|| {
                over_delete_all.set(over_delete_all.get() || w.bulk.as_ref().is_some_and(|e| e.counter != w.after));
                w.bulk.as_ref().map_or(w.after, |e| e.counter)
            })
        });
        if !noted {
            return Err("a write of the open transaction the runtime does not track");
        }
        if over_delete_all.get() || rv.assumed_delete_all {
            assume(Assumption::RowVersionDeleteAll);
        }
        if rv.assumed_write {
            assume(Assumption::RowVersionWriteAfterInsert);
        }
        if rv.assumed_checked_insert {
            assume(Assumption::RowVersionCheckedInsert);
        }
        Ok(self.tx.committed_row_version.max(rv.sent).max(1))
    }

    /// Records a ModifyAll, DeleteAll, Rename or Truncate of a database table before it writes; `kind` says which (a Rename is
    /// `Other` until it turns out a plain one, `row` its old key; empty for the others). Returns where the note is, for
    /// `note_bulk_end`.
    pub(crate) fn note_bulk_write(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        kind: crate::state::TxWriteKind,
        row: &[Value],
    ) -> Option<BulkMark> {
        self.note_bulk_write_reset(rec, t, kind, row, false)
    }

    /// `note_bulk_write` of a DeleteAll; `argument`: the call passed one.
    pub(crate) fn note_delete_all_write(&mut self, rec: &Rec, t: &TableDef, argument: bool) -> Option<BulkMark> {
        self.note_bulk_write_reset(rec, t, crate::state::TxWriteKind::DeleteAll, &[], argument)
    }

    /// Before a filtered DeleteAll of `t` while the open transaction holds unsent inserted rows of `t`: those rows (their row
    /// versions, and how many of them `t` holds), for `end_filtered_delete_all`. `None` for every other DeleteAll.
    pub(crate) fn filtered_delete_all_inserted(
        &self,
        cx: &AccessContext,
        t: &TableDef,
        rec: &Rec,
    ) -> Result<Option<(i64, i64, usize)>, Signal> {
        let r = rec.borrow();
        let own = self
            .tx
            .open
            .row_versions
            .pending_inserts
            .iter()
            .find(|(o, c, ..)| *o == t.object && *c == r.company);
        let (Some(&(_, _, first, last)), true, false) = (own, !r.filters.is_empty(), r.temporary) else {
            return Ok(None);
        };
        drop(r);
        Ok(Some((first, last, self.rows_in_row_versions(cx, t, first, last)?)))
    }

    /// After the DeleteAll `filtered_delete_all_inserted` looked at: RowVersionWriteShapes.al:338 of corpus PR 1621 measures a
    /// filter that matches none of the inserted rows (assumption `row-version-delete-all`); one that deleted one of them is not
    /// measured, so what reads LastUsedRowVersion later is refused.
    pub(crate) fn end_filtered_delete_all(
        &mut self,
        cx: &AccessContext,
        t: &TableDef,
        inserted: Option<(i64, i64, usize)>,
    ) -> Result<(), Signal> {
        let Some((first, last, before)) = inserted else {
            return Ok(());
        };
        if self.rows_in_row_versions(cx, t, first, last)? != before {
            self.tx
                .open
                .row_versions
                .unpinned
                .get_or_insert("a filtered DeleteAll that deleted rows the open transaction inserted");
        }
        Ok(())
    }

    /// How many rows of `t` hold a SystemRowVersion in [`first`, `last`].
    fn rows_in_row_versions(&self, cx: &AccessContext, t: &TableDef, first: i64, last: i64) -> Result<usize, Signal> {
        let Some(pos) = t.fields.iter().position(|f| f.id == super::restamped::ROW_VERSION) else {
            return Ok(0);
        };
        let rows = self.all_rows(cx, t)?;
        Ok(rows
            .iter()
            .filter(|row| matches!(row.get(pos), Some(Value::BigInteger(v)) if first <= *v && *v <= last))
            .count())
    }

    /// `note_bulk_write` of a `Truncate(reset)`.
    pub(crate) fn note_truncate_write(&mut self, rec: &Rec, t: &TableDef, argument: bool) {
        self.note_bulk_write_reset(rec, t, crate::state::TxWriteKind::Truncate, &[], argument);
    }

    fn note_bulk_write_reset(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        kind: crate::state::TxWriteKind,
        row: &[Value],
        argument: bool,
    ) -> Option<BulkMark> {
        if rec.borrow().temporary {
            return None;
        }
        let now = self.store.change_counter();
        let company = rec.borrow().company.clone();
        self.note_row_versions(t.object, &company, kind, row);
        let bulk_shape = {
            let r = rec.borrow();
            crate::state::BulkShape {
                filtered: !r.filters.is_empty(),
                marked_only: r.marked_only,
                argument,
            }
        };
        self.push_write(t, company, kind, &[], now, bulk_shape);
        Some(BulkMark {
            at: self.tx.open.writes.len() - 1,
            before: now,
        })
    }

    /// A ModifyAll or DeleteAll noted by `note_bulk_write` ended without an error: how many notes its rows added after it and the
    /// store's change counter now (`TxWrite::bulk`, read by the stale-buffer rule). After a rollback inside it the note is gone.
    pub(crate) fn note_bulk_end(&mut self, mark: Option<BulkMark>) {
        let Some(BulkMark { at, before }) = mark else {
            return;
        };
        let (len, counter) = (self.tx.open.writes.len(), self.store.change_counter());
        if let Some(w) = self.tx.open.writes.get_mut(at)
            && w.before == before
            && w.bulk.is_none()
        {
            w.bulk = Some(crate::state::BulkEnd {
                entries: len - at - 1,
                counter,
            });
            if counter != before {
                let (table, company) = (w.table, w.company.clone());
                crate::version_entry::note_bulk_changed(&mut self.tx.open.table_writes, table, &company);
            }
            self.end_row_version_modify_all(at);
        }
    }

    /// The end of the ModifyAll noted at `at` in `OpenTx::writes`, when `note_row_versions` let it send the inserted rows of its
    /// table (`RowVersionTx::write_sends`). RowVersionWrites.al:218 of corpus PR 1509 measured a ModifyAll of the rows the
    /// transaction inserted and did not write since: each row it modified must be one (`write_notes::inserted_unwritten` over
    /// the writes before it), else main's refusal. One that modified no row stays in `write_sends` (not measured).
    fn end_row_version_modify_all(&mut self, at: usize) {
        use crate::state::TxWriteKind;
        let writes = &self.tx.open.writes;
        let w = &writes[at];
        let rv = &mut self.tx.open.row_versions;
        if w.kind != TxWriteKind::ModifyAll
            || !rv
                .write_sends
                .as_ref()
                .is_some_and(|(o, c, k)| *o == w.table && *c == w.company && *k == TxWriteKind::ModifyAll)
        {
            return;
        }
        let rows = &writes[at + 1..];
        if rows.is_empty() {
            return;
        }
        let inserted = rows.iter().all(|r| {
            r.table == w.table
                && r.company == w.company
                && r.kind == TxWriteKind::Modify
                && super::write_notes::inserted_unwritten(&writes[..at], w.table, &w.company, &r.pk)
        });
        rv.write_sends = None;
        if !inserted {
            rv.unpinned = Some(AFTER_INSERT);
        }
    }

    /// Whether every Insert since the last Commit or rollback went through `SqlCounters::pre_insert`: every write to the
    /// database store since then is a Record write in `tx.open.writes` (sqlcount.rs).
    pub(crate) fn sql_pre_known(&self) -> bool {
        self.writes_all_noted()
    }

    /// Whether every write to the database store since the last Commit or rollback was a Record write in `tx.open.writes`: the
    /// counters chain from the Commit's to the store's current one.
    pub(crate) fn writes_all_noted(&self) -> bool {
        self.writes_noted_over(|_| None)
    }

    /// `writes_all_noted`, where `over` may name the counter a note's write ended at (a bulk write whose rows have no note
    /// each); the chain goes on from there.
    pub(crate) fn writes_noted_over(&self, over: impl Fn(&crate::state::TxWrite) -> Option<u64>) -> bool {
        let mut at = self.tx.commit_counter;
        for w in &self.tx.open.writes {
            if w.before != at {
                return false;
            }
            at = over(w).unwrap_or(w.after);
        }
        at == self.store.change_counter()
    }

    /// The error of a write through a stale buffer (record/TestRecordStaleBufferWrite.al).
    pub(crate) fn record_changed(&self, t: &TableDef, rec: &Rec) -> Result<Signal, Signal> {
        let identification = self.identification(t, &rec.borrow().fields)?;
        Ok(Signal::Error(Box::new(AlError {
            message: format!(
                "The changes to the {} record cannot be saved because some information on the page is not up-to-date. Close the page, reopen it, and try again.{BR}{BR}Identification fields and values:{BR}{BR}{}",
                self.caption(t),
                identification
            ),
            code: "DB:RecordChanged".into(),
            call_stack: String::new(),
            ..Default::default()
        })))
    }

    /// An unchanged Modify keeps its row version, but ModifyAll gives it a new one
    /// (record/PriorityAssumptions.al:143-244; record/crud/ModifyWithoutChange.al:375-448).
    pub(super) fn modify_row_version(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        stored: Option<&[Value]>,
    ) -> Option<(usize, Value)> {
        // A Time held past the day compares as the midnight it reads (record/filters/TimeRolledPastMidnight.al:251-260).
        let rolled_read = stored.filter(|r| r.iter().any(crate::sqltime::is_rolled)).map(|r| {
            let mut r = r.to_vec();
            crate::sqltime::read_back(&mut r);
            r
        });
        let unchanged = match rolled_read.as_deref().or(stored) {
            Some(old) if !rec.borrow().temporary && Session::modify_changes_nothing(t, old, rec) => Some(old),
            _ => None,
        };
        match unchanged {
            Some(old) if !self.bulk_modify => {
                if let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_005) {
                    rec.borrow_mut().fields[i] = old[i].clone();
                }
                None
            }
            _ => self.bump_row_version(t, rec),
        }
    }

    pub(super) fn bump_row_version(&mut self, t: &TableDef, rec: &Rec) -> Option<(usize, Value)> {
        let i = t.fields.iter().position(|f| f.id == 2_000_000_005)?;
        if self.temp_never_stamps(t, rec) {
            return None;
        }
        self.next_row_version += 1;
        if !rec.borrow().temporary {
            self.tx.open.row_versions.handed_out += 1;
            self.tx.open.row_versions.last = self.next_row_version;
        }
        let old = std::mem::replace(
            &mut rec.borrow_mut().fields[i],
            Value::BigInteger(self.next_row_version),
        );
        Some((i, old))
    }

    /// A BigInteger field with `SQLTimestamp = true` reads the row version: above 0 after the Insert, greater after a Modify,
    /// the same without one, and an assignment to it is not stored (record/table-properties/XTablePropsTests.al:412-415).
    /// `stored` is the row before a Modify: the field gets its value back first, so an assignment to it is no change.
    /// A temporary record never stamps (see `temp_never_stamps`).
    pub(super) fn sql_timestamp_before(&self, t: &TableDef, rec: &Rec, stored: Option<&[Value]>) -> Result<(), Signal> {
        let fields = &self.engine.table_facts[t.object as usize].sql_timestamp;
        if fields.is_empty() {
            return Ok(());
        }
        if t.fields.iter().all(|f| f.id != 2_000_000_005) {
            return unsupported(format!(
                "an SQLTimestamp field of {}, a table without a row version",
                t.name
            ));
        }
        if rec.borrow().temporary {
            return Ok(());
        }
        if let Some(old) = stored {
            let mut r = rec.borrow_mut();
            for &i in fields {
                r.fields[i] = old[i].clone();
            }
        }
        Ok(())
    }

    /// A temporary record of a table with an SQLTimestamp field never stamps: the field and SystemRowVersion keep what the buffer
    /// holds through Insert, Modify and Rename (0 for a new row, the database row's values for a copy of one, a value code assigned
    /// to the field), and Init keeps them (record/temporary/QrTempTimestamp.al:76-85, :111-115, :160-170, :221-226).
    /// A temporary record of a table without one is not pinned and keeps its own row versions.
    pub(super) fn temp_never_stamps(&self, t: &TableDef, rec: &Rec) -> bool {
        rec.borrow().temporary && !self.engine.table_facts[t.object as usize].sql_timestamp.is_empty()
    }

    /// The SQLTimestamp fields take the row version the write gives the row (see `sql_timestamp_before`).
    pub(super) fn sql_timestamp_after(&self, t: &TableDef, rec: &Rec) {
        let fields = &self.engine.table_facts[t.object as usize].sql_timestamp;
        if fields.is_empty() || rec.borrow().temporary {
            return;
        }
        if let Some(v) = t.fields.iter().position(|f| f.id == 2_000_000_005) {
            let mut r = rec.borrow_mut();
            let version = r.fields[v].clone();
            for &i in fields {
                r.fields[i] = version.clone();
            }
        }
    }

    pub(super) fn restore_row_version(rec: &Rec, old: Option<(usize, Value)>) {
        if let Some((i, v)) = old {
            rec.borrow_mut().fields[i] = v;
        }
    }
}

/// The reads that send a pending Insert's row version, besides the ones `record_op_inner` already counted as `more`.
pub(crate) fn is_row_version_send_read(method: RecordOp) -> bool {
    matches!(
        method,
        RecordOp::Get
            | RecordOp::FindFirst
            | RecordOp::FindLast
            | RecordOp::FindSet
            | RecordOp::IsEmpty
            | RecordOp::Count
            | RecordOp::CalcSums
    )
}
