//! Row-version notes and the stale-record verdict (moved out of `records.rs`).

use super::*;

/// Where `Session::note_bulk_write` put its note: the index in `TxState::writes` and the store's change counter then.
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
        self.note_row_versions(t.object, &company, kind);
        self.tx.writes.push(crate::state::TxWrite {
            table: t.object,
            company,
            kind,
            pk: pk.to_vec(),
            before,
            after: self.store.change_counter(),
            bulk: None,
        });
    }

    /// What a database write does to `Database.LastUsedRowVersion` (`last_used_row_version`). BC keeps the rows an Insert
    /// wrote until a read of their table or a Modify sends them: the value does not grow after an Insert alone
    /// (record/TestProbeRecordRefContracts.al:620-640 `grows_after_insert` No, metadata/platform-tables/PlTDatabase.al:
    /// 220-245 `after_insert_greater` No), it covers the inserted row after a Get of its table (corpus master
    /// PlTRowVersionValue.al:157-172 TV_MinimumActiveRowVersion_AcrossInsertModifyCommit, `insert open last > start last` and
    /// `row version <= last` Yes) and grows after a Modify (PlTDatabase.al:238 `after_modify_greater`, PlTRowVersionValue.al
    /// `modify open row version <= last`). Not pinned, so refused: a Delete, Rename or bulk write after an Insert
    /// and row versions an Insert or a Modify does not account for.
    pub(super) fn note_row_versions(&mut self, table: u32, company: &Option<String>, kind: crate::state::TxWriteKind) {
        use crate::state::TxWriteKind;
        let rv = &mut self.tx.row_versions;
        // An Insert or a Modify hands out one row version; more since the last one accounted for came from another write.
        let new_versions = rv.handed_out != rv.accounted;
        if rv.handed_out > rv.accounted + 1 {
            rv.unpinned = Some("row versions handed out by a write other than an Insert or a Modify");
        }
        match kind {
            TxWriteKind::Insert if new_versions => {
                rv.accounted = rv.handed_out;
                let last = rv.last;
                match rv
                    .pending_inserts
                    .iter_mut()
                    .find(|(o, c, _)| *o == table && c == company)
                {
                    Some(p) => p.2 = last,
                    None => rv.pending_inserts.push((table, company.clone(), last)),
                }
            }
            TxWriteKind::Modify if new_versions => {
                rv.accounted = rv.handed_out;
                rv.sent = rv.sent.max(rv.last);
                rv.pending_inserts.retain(|(o, c, _)| !(*o == table && c == company));
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
            TxWriteKind::Delete
            | TxWriteKind::Rename
            | TxWriteKind::ModifyAll
            | TxWriteKind::DeleteAll
            | TxWriteKind::Truncate
            | TxWriteKind::Other => {
                if !rv.pending_inserts.is_empty() {
                    rv.unpinned = Some(
                        "a Delete, Rename, DeleteAll, ModifyAll or Truncate after an Insert in the open transaction",
                    );
                }
            }
        }
    }

    /// A read of a database table while the open transaction holds inserted rows nothing has sent yet
    /// (`note_row_versions`). `sends` is a Get, FindFirst, FindLast, FindSet, IsEmpty, Count or CalcSums: the first one of
    /// a table sends that table's rows and grows the value, a read of another table sends nothing (Get of the one table:
    /// PlTRowVersionValue.al:157-172 on corpus master; the rest: corpus branch al-runtime-rs-staging-pub-uk beab119c,
    /// PlTRowVersionValue.al UK_InsertThenFindSet, _FindFirst, _IsEmpty, _Count, _CalcSums, _ReadOtherTable,
    /// _ReadOtherThenSame, UK_InsertTwoTablesThenGetOne, UK_InsertReadInsertRead, UK_InsertThenModifyOtherTable). The rows
    /// get their row versions at the Insert here, so a read after a Modify handed out a higher one takes a new one to grow
    /// the value as BC does (UK_InsertThenModifyOtherTable `findfirst` greater). Other reads are not pinned.
    pub(crate) fn note_read_for_row_versions(&mut self, table: u32, company: &Option<String>, sends: bool) {
        let rv = &mut self.tx.row_versions;
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
            .position(|(o, c, _)| *o == table && c == company)
        else {
            return;
        };
        let (_, _, v) = rv.pending_inserts.remove(pos);
        let current = self.tx.committed_row_version.max(rv.sent);
        if v > current {
            rv.sent = v;
        } else {
            self.next_row_version += 1;
            self.tx.row_versions.sent = self.next_row_version;
        }
    }

    /// `Database.LastUsedRowVersion`: the committed value, raised by the rows the open transaction sent
    /// (`note_row_versions`). `Err` names why the value is not pinned.
    pub(crate) fn last_used_row_version(&self) -> Result<i64, &'static str> {
        let rv = &self.tx.row_versions;
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
        if rv.handed_out != rv.accounted {
            return Err("row versions handed out by a write other than an Insert or a Modify");
        }
        if !self.writes_all_noted() {
            return Err("a write of the open transaction the runtime does not track");
        }
        Ok(self.tx.committed_row_version.max(rv.sent).max(1))
    }

    /// Records a ModifyAll, DeleteAll, Rename or Truncate of a database table before it writes; `kind` says which (a Rename is
    /// `Other` until it turns out a plain one). Returns where the note is, for `note_bulk_end`.
    pub(crate) fn note_bulk_write(
        &mut self,
        rec: &Rec,
        t: &TableDef,
        kind: crate::state::TxWriteKind,
    ) -> Option<BulkMark> {
        if rec.borrow().temporary {
            return None;
        }
        let now = self.store.change_counter();
        self.note_write(rec, t, kind, &[], now);
        Some(BulkMark {
            at: self.tx.writes.len() - 1,
            before: now,
        })
    }

    /// A ModifyAll or DeleteAll noted by `note_bulk_write` ended without an error: how many notes its rows added after it and the
    /// store's change counter now (`TxWrite::bulk`, read by the stale-buffer rule). After a rollback inside it the note is gone.
    pub(crate) fn note_bulk_end(&mut self, mark: Option<BulkMark>) {
        let Some(BulkMark { at, before }) = mark else {
            return;
        };
        let (len, counter) = (self.tx.writes.len(), self.store.change_counter());
        if let Some(w) = self.tx.writes.get_mut(at)
            && w.before == before
            && w.bulk.is_none()
        {
            w.bulk = Some(crate::state::BulkEnd {
                entries: len - at - 1,
                counter,
            });
        }
    }

    /// Whether every Insert since the last Commit or rollback went through `SqlCounters::pre_insert`: every write to the
    /// database store since then is a Record write in `tx.writes` (sqlcount.rs).
    pub(crate) fn sql_pre_known(&self) -> bool {
        self.writes_all_noted()
    }

    /// Whether every write to the database store since the last Commit or rollback was a Record write in `tx.writes`: the
    /// counters chain from the Commit's to the store's current one.
    pub(crate) fn writes_all_noted(&self) -> bool {
        let mut at = self.tx.commit_counter;
        for w in &self.tx.writes {
            if w.before != at {
                return false;
            }
            at = w.after;
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

    pub(super) fn bump_row_version(&mut self, t: &TableDef, rec: &Rec) -> Option<(usize, Value)> {
        let i = t.fields.iter().position(|f| f.id == 2_000_000_005)?;
        if self.temp_never_stamps(t, rec) {
            return None;
        }
        self.next_row_version += 1;
        if !rec.borrow().temporary {
            self.tx.row_versions.handed_out += 1;
            self.tx.row_versions.last = self.next_row_version;
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
