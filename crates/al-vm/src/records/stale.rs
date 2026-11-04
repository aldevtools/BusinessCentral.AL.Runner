//! Row-version notes and the stale-record verdict (moved out of `records.rs`).

use super::*;

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
        });
    }

    /// What a database write does to `Database.LastUsedRowVersion` (`last_used_row_version`). BC keeps the rows an Insert
    /// wrote until a read of their table or a Modify sends them: the value does not grow after an Insert alone
    /// (record/TestProbeRecordRefContracts.al:620-640 `grows_after_insert` No, platform/lt-platform-members/PlTDatabase.al:
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
            // session/TestSystemExtended.al:635-659, TestBCDatabaseContracts.al and PnBRecordMisc.al:205-235 read it after a
            // DeleteAll). A Rename or ModifyAll hands out row versions no Insert or Modify accounts for
            // (`last_used_row_version` refuses those).
            TxWriteKind::Delete | TxWriteKind::Rename | TxWriteKind::Other => {
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

    /// Records a ModifyAll, DeleteAll, Rename or Truncate of a database table: `stale_verdict` does not tell its writes apart.
    pub(crate) fn note_other_write(&mut self, rec: &Rec, t: &TableDef) {
        let now = self.store.change_counter();
        self.note_write(rec, t, crate::state::TxWriteKind::Other, &[], now);
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

    /// Whether a Modify or Delete through `rec` of the stored row `pk` (whose row version is now `theirs`) may write.
    ///
    /// The buffer is stale when the last Commit left the row newer than the buffer. A buffer newer than the row (a rollback to
    /// the last Commit after the buffer's own Modify, as an asserterror does) is not stale (corpus
    /// record/lt-record-conversion-assumptions/StaleBuffer.al: P98_Stale_AfterAsserterror_SameBuffer,
    /// P98_Stale_CommitThenErrorThenModify_Rec). A stale buffer is refused (record/TestRecordStaleBufferWrite.al), with one
    /// exception (TestRecordStaleBufferWrite.al:395-690, AL Runner issue 5805): a buffer from the variable's own Insert in the
    /// transaction the last Commit closed, or from its own Modify or Rename after its Insert or Get in that transaction
    /// (record/lt-sb-stale-own-modify/SbStaleOwnModify.al, AL Runner issue 5823), is written, the stale buffer winning, once the new transaction Modified the row
    /// (changed or not, through any variable: :495, :513, :567, Delete :676) or read it by Get after a LockTable (:533). It is
    /// refused with nothing in between (:481, Delete :662), after a plain Get (:551), and after a further Commit (:586). A
    /// buffer read by Get, with no own write since, is refused whatever the new transaction did (:604, :623, :642). Not pinned, refused: other contact
    /// with the table in the new transaction (another write, another lock, a rollback), an Insert buffer older than one
    /// Commit that the new transaction touched. A stale buffer of another origin whose row the new transaction wrote is written
    /// under the assumption stale-buffer-other-origin-written. The three shapes of al-runner issue 6028 (`stale_6028_shape`) are
    /// pinned by record/lt-sb-stale-own-modify/SbStaleOwnModify.al TE_*.
    pub(crate) fn stale_verdict(
        &mut self,
        t: &TableDef,
        rec: &Rec,
        cx: &al_data::AccessContext,
        pk: &[Value],
        theirs: i64,
        deleting: bool,
    ) -> Result<StaleVerdict, Signal> {
        let Some(i) = t.fields.iter().position(|f| f.id == 2_000_000_005) else {
            return Ok(StaleVerdict::Fresh);
        };
        let (mine, origin, temporary, company) = {
            let r = rec.borrow();
            let v = if let Value::BigInteger(n) = &r.fields[i] { *n } else { 0 };
            (v, r.row_origin, r.temporary, r.company.clone())
        };
        if temporary || mine == 0 {
            return Ok(StaleVerdict::Fresh);
        }
        let committed = self
            .store_for(rec)
            .committed_get(cx, t, pk)
            .and_then(|row| match row.get(i) {
                Some(Value::BigInteger(n)) => Some(*n),
                _ => None,
            });
        // Stale: the row changed since the buffer was read, and the last Commit already held a newer row than the buffer.
        if mine >= theirs || !committed.is_some_and(|c| mine < c) {
            return Ok(StaleVerdict::Fresh);
        }
        let written_now = theirs > self.tx.committed_row_version;
        let touched = self.tx.rows_touched.iter().any(|(o, c, k)| {
            *o == t.object
                && *c == company
                && k.len() == pk.len()
                && k.iter()
                    .zip(pk)
                    .all(|(a, b)| al_builtins::ops::compare(a, b).is_ok_and(|o| o == std::cmp::Ordering::Equal))
        });
        // Four Commits after the buffer's own write, a read under ReadIsolation UpdLock (Find, as No. Series takes a number) of
        // another table only, beside writes of other tables only, decides as the same writes after a plain Get: DB:RecordChanged
        // (record/lt-wb-6287/WbStaleUpdLockTests.al:16-25 UpdLockReadOfAnotherTableFourCommitsLaterRecordChanged, contrast
        // :29-38; al-runner issue 6287). Other Commit counts and reads of the buffer's own table keep the lock as contact.
        // The lock then comes from ReadIsolation alone (no LockTable), on other tables only.
        let read_of_others_only = self.tx.lock_unpinned == Some(UNPINNED_LOCKING_READ)
            && origin.is_some_and(|o| o.version == mine && o.own_write && o.commits + 4 == self.commits)
            && self.tx.locked_tables.is_empty()
            && (!self.tx.lock_tx || self.tx.isolation_lock)
            && self
                .tx
                .isolation_lock_tables
                .iter()
                .all(|(o, c)| *o != t.object || *c != company)
            && self.writes_all_noted()
            && self.tx.writes.iter().all(|w| w.table != t.object);
        let contact = touched
            || self.tx.rolled_back
            || (self.tx.lock_tx && !read_of_others_only)
            || (self.tx.lock_unpinned.is_some() && !read_of_others_only)
            || self.store_for(rec).written_since_commit(cx, t);
        match origin.filter(|o| o.version == mine) {
            Some(o) if o.unpinned_read => Err(Self::stale_unpinned_find(t)),
            // A buffer that only got TransferFields from a record (Y76StaleBuffers.al Y76_Part1_Copy_TransferFromRecord).
            Some(o) if o.own_write && o.lock_only => {
                self.stale_lock_only_verdict(t, &company, pk, o.commits, touched, contact)
            }
            // A buffer whose version came from its own Modify or Rename after its own Insert or Get in that transaction follows the
            // same rule (record/lt-sb-stale-own-modify/SbStaleOwnModify.al:132-448, al-runner issue 5823).
            // One or two Commits after the buffer's own write, and the new transaction inserted another row of the table and did
            // nothing else to the row or the table's locks: written (record/lt-sb-stale-own-modify/SbStaleOwnModify.al:666-745,
            // SH_OwnModify_*); an Insert into another table only is refused. Also for an own Insert without the Modify, a Modify
            // of another row beside the Insert and three Commits (SbStaleOwnModify.al:2484-2580, Y2_SH_*). Other writes beside
            // the Insert (a Delete or a Rename, a write of another table) are not told apart (assumption
            // `stale-own-insert-two-commits-other-insert`).
            Some(o)
                if o.own_write
                    && self.stale_other_insert_in_reach(t, &company, pk, o)
                    && !touched
                    && !self.tx.rolled_back
                    && !self.tx.lock_tx
                    && self.tx.lock_unpinned.is_none()
                    && self
                        .tx
                        .tables_inserted
                        .iter()
                        .any(|(ob, c)| *ob == t.object && *c == company) =>
            {
                let pinned = self.stale_other_insert_pinned(t, &company, pk, self.commits - o.commits, o.own_modified);
                if !pinned {
                    al_builtins::assumptions::assume(
                        al_builtins::assumptions::Assumption::StaleOwnInsertTwoCommitsOtherInsert,
                    );
                }
                Ok(StaleVerdict::Written)
            }
            // Writers TE and TH measured the same neighbours at 1 to 10 Commits after the buffer's own write (al-runner issue 6040,
            // record/lt-sb-stale-own-modify/SbStaleOwnModify.al TE_* and TH_*), and writer Y2 every form at 4, 5, 10 and 12 Commits
            // (SbStaleOwnModify.al:2096-2484, Y2_4C_*, Y2_5C_*, Y2_10C_*, Y2_12C_*): the same rule at every count. See
            // `stale_neighbour_shape`. The three shapes of 6028 keep their own assumption below.
            Some(o)
                if o.own_write
                    && !touched
                    && contact
                    && self.stale_6028_shape(t, &company, pk, o.commits, deleting).is_none()
                    && self.stale_neighbour_shape(t, &company, pk).is_some() =>
            {
                Ok(self.stale_neighbour_shape(t, &company, pk).expect("checked"))
            }
            Some(o) if o.own_write && o.commits + 1 == self.commits => {
                // A buffer that read the row by Get, GetBySystemId or FindFirst in the transaction that wrote it, with no own write
                // since, follows the same rule (record/lt-sb-stale-own-modify/SbStaleOwnModify.al:467-640, the SF_ tests; al-runner
                // issue 5913).
                if touched && !self.tx.rolled_back {
                    return Ok(StaleVerdict::Written);
                }
                if contact {
                    return unsupported(format!(
                        "a stale {} buffer from its own Insert, written after the new transaction touched the table in a way no test pins",
                        t.name
                    ));
                }
                Ok(StaleVerdict::Refused)
            }
            // Two or three Commits after the buffer's own write, in the three shapes of the reporter's real-BC repro of al-runner
            // issue 6028 (pinned by record/lt-sb-stale-own-modify/SbStaleOwnModify.al
            // TE_2C_*, TE_3C_*). The neighbours writers TE and TH measured are decided above
            // (`stale_neighbour_shape`); every other mix is refused below.
            Some(o)
                if o.own_write && !touched && self.stale_6028_shape(t, &company, pk, o.commits, deleting).is_some() =>
            {
                Ok(self
                    .stale_6028_shape(t, &company, pk, o.commits, deleting)
                    .expect("checked"))
            }
            // Two or three Commits after the buffer's own write, another variable wrote the row in the transaction the last
            // Commit closed, and the new transaction's only writes are Modifies of the buffer's row through other variables
            // (changed or not, once or twice), with no lock: written whole, the stale buffer winning, OnModify once with xRec the
            // buffer (record/lt-sb-stale-own-modify/Y27StaleOtherVarTests.al Y27_2C_*, Y27_3C_*, al-runner issue 6522). Y27_3C_* writes the row in both transactions between, so three Commits need it written in
            // the one before too; a transaction between that left the row alone stays refused below, as does a row last written
            // in the buffer's own transaction with an empty Commit after it.
            Some(o)
                if o.own_write
                    && (o.commits + 2 == self.commits
                        || (o.commits + 3 == self.commits
                            && self.row_written_before_last_closed_tx(t, &company, pk)))
                    && touched
                    && committed.is_some_and(|c| c > self.tx.prior_committed_row_version)
                    && self.stale_row_modified_only(t, &company, pk) =>
            {
                Ok(StaleVerdict::Written)
            }
            Some(o) if o.own_write && contact => unsupported(format!(
                "a stale {} buffer from an Insert older than the last Commit but one, written after the new transaction touched the table",
                t.name
            )),
            Some(_) => Ok(StaleVerdict::Refused),
            // Not pinned; Microsoft's tests need it (assumption stale-buffer-other-origin-written): the last writer wins, as
            // before any of this was probed.
            None if written_now => {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::StaleBufferOtherOriginWritten);
                Ok(StaleVerdict::Written)
            }
            None => Ok(StaleVerdict::Refused),
        }
    }

    /// The verdict on a stale buffer from the variable's own write `commits` Commits into the session, when the new
    /// transaction has one of the three shapes al-runner issue 6028 measured on real BC 28.5 (reporter's repro, 3/3 green):
    /// - two Commits later, LockTable on other tables only, Inserts into those tables only and a FindSet of the buffer's table:
    ///   a Modify raises DB:RecordChanged (the text of record/TestRecordStaleBufferWrite.al);
    /// - three Commits later, Modifies of other rows of the table as the only writes: a Modify writes the whole buffer;
    /// - three Commits later, Deletes of other rows of the table as the only writes: a Delete deletes the row.
    ///
    /// Each needs no rollback, no lock from a ReadIsolation or a RecordRef and no write left out of `tx.writes`. `None` for
    /// anything else.
    pub(super) fn stale_6028_shape(
        &self,
        t: &TableDef,
        company: &Option<String>,
        pk: &[Value],
        commits: u64,
        deleting: bool,
    ) -> Option<StaleVerdict> {
        let tx = &self.tx;
        if tx.rolled_back
            || tx.lock_unpinned.is_some()
            || tx.isolation_lock
            || tx.writes.is_empty()
            || !self.writes_all_noted()
        {
            return None;
        }
        let same_key = |k: &[Value]| {
            k.len() == pk.len()
                && k.iter()
                    .zip(pk)
                    .all(|(a, b)| al_builtins::ops::compare(a, b).is_ok_and(|o| o == std::cmp::Ordering::Equal))
        };
        let other_rows_only = |kind: crate::state::TxWriteKind| {
            tx.writes
                .iter()
                .all(|w| w.kind == kind && w.table == t.object && w.company == *company && !same_key(&w.pk))
        };
        if commits + 2 == self.commits {
            let locked_others = tx.lock_tx && !tx.locked_tables.is_empty() && !tx.locked_tables.contains(&t.object);
            let inserts_into_locked = tx
                .writes
                .iter()
                .all(|w| w.kind == crate::state::TxWriteKind::Insert && tx.locked_tables.contains(&w.table));
            let read = tx.find_set_tables.iter().any(|(o, c)| *o == t.object && c == company);
            return (!deleting && locked_others && inserts_into_locked && read).then_some(StaleVerdict::Refused);
        }
        if commits + 3 != self.commits || tx.lock_tx || !tx.locked_tables.is_empty() {
            return None;
        }
        match deleting {
            false if other_rows_only(crate::state::TxWriteKind::Modify) => Some(StaleVerdict::Written),
            true if other_rows_only(crate::state::TxWriteKind::Delete) => Some(StaleVerdict::Written),
            _ => None,
        }
    }

    /// The verdict on a stale buffer from the variable's own write, whatever the number of Commits since, when the new transaction
    /// did not write the buffer's row and has one of the forms writers TE and TH measured on real BC (record/lt-sb-stale-own-modify/
    /// SbStaleOwnModify.al TE_1C_* to TE_3C_*, TH_4C_*, TH_5C_*, TH_10C_*, TH_8C_*, TH_7C_* on branch al-runtime-rs-staging-pub-th;
    /// al-runner issue 6040):
    /// - LockTable on other tables only and no write (TeLockOtherOnly), or Inserts into other tables only, locked or not, and a
    ///   FindSet of the buffer's table (TeLockOtherInsertRead, TeOtherTableInsertRead; the reporter's shape 1 adds Inserts into a
    ///   table it did not lock): a Modify or Delete raises DB:RecordChanged;
    /// - LockTable on the buffer's table only and no write (TeLockSame): written;
    /// - only Modifies, only Deletes, only Inserts or only Renames of other rows of the table, and no lock (TeModifyOther,
    ///   TeDeleteOther, TeInsertOther, TeRenameOther): written, the stale buffer winning.
    ///
    /// Each needs no rollback, no lock from a ReadIsolation or a RecordRef and no write left out of `tx.writes`. `None` for
    /// anything else (a ModifyAll or DeleteAll, a mix of kinds, a write beside a lock).
    pub(super) fn stale_neighbour_shape(
        &self,
        t: &TableDef,
        company: &Option<String>,
        pk: &[Value],
    ) -> Option<StaleVerdict> {
        use crate::state::TxWriteKind;
        let tx = &self.tx;
        if tx.rolled_back || tx.lock_unpinned.is_some() || tx.isolation_lock || !self.writes_all_noted() {
            return None;
        }
        let same_key = |k: &[Value]| {
            k.len() == pk.len()
                && k.iter()
                    .zip(pk)
                    .all(|(a, b)| al_builtins::ops::compare(a, b).is_ok_and(|o| o == std::cmp::Ordering::Equal))
        };
        let locks_table = tx.locked_tables.contains(&t.object);
        let read = tx.find_set_tables.iter().any(|(o, c)| *o == t.object && c == company);
        if !locks_table
            && tx
                .writes
                .iter()
                .all(|w| w.kind == TxWriteKind::Insert && w.table != t.object)
        {
            let lock_only = tx.lock_tx && !tx.locked_tables.is_empty() && tx.writes.is_empty() && !read;
            let inserts_and_read = !tx.writes.is_empty() && read;
            return (lock_only || inserts_and_read).then_some(StaleVerdict::Refused);
        }
        if tx.writes.is_empty() {
            let lock_same = tx.lock_tx && locks_table && tx.locked_tables.iter().all(|o| *o == t.object) && !read;
            return lock_same.then_some(StaleVerdict::Written);
        }
        if tx.lock_tx || !tx.locked_tables.is_empty() {
            return None;
        }
        let kind = tx.writes[0].kind;
        let other_rows_of_one_kind = matches!(
            kind,
            TxWriteKind::Insert | TxWriteKind::Modify | TxWriteKind::Delete | TxWriteKind::Rename
        ) && tx
            .writes
            .iter()
            .all(|w| w.kind == kind && w.table == t.object && w.company == *company && !same_key(&w.pk));
        // An Insert and a Rename of another row are measured at one to three Commits only (TE_*_InsertOtherRow_*,
        // TE_*_RenameOtherRow_*).
        other_rows_of_one_kind.then_some(StaleVerdict::Written)
    }

    /// Whether the transaction the Commit before the last one closed wrote the row `pk` of `t` by a Record write.
    pub(super) fn row_written_before_last_closed_tx(
        &self,
        t: &TableDef,
        company: &Option<String>,
        pk: &[Value],
    ) -> bool {
        self.tx.closed_tx_rows_before.as_ref().is_some_and(|rows| {
            rows.iter().any(|(table, c, k)| {
                *table == t.object
                    && c == company
                    && k.len() == pk.len()
                    && k.iter()
                        .zip(pk)
                        .all(|(a, b)| al_builtins::ops::compare(a, b).is_ok_and(|o| o == std::cmp::Ordering::Equal))
            })
        })
    }

    /// Whether the open transaction's only writes are Modifies of the row `pk` of `t`, with no rollback and no lock of any kind.
    pub(super) fn stale_row_modified_only(&self, t: &TableDef, company: &Option<String>, pk: &[Value]) -> bool {
        let tx = &self.tx;
        !tx.rolled_back
            && !tx.lock_tx
            && tx.lock_unpinned.is_none()
            && !tx.isolation_lock
            && tx.locked_tables.is_empty()
            && self.writes_all_noted()
            && !tx.writes.is_empty()
            && tx.writes.iter().all(|w| {
                w.kind == crate::state::TxWriteKind::Modify
                    && w.table == t.object
                    && w.company == *company
                    && w.pk.len() == pk.len()
                    && w.pk
                        .iter()
                        .zip(pk)
                        .all(|(a, b)| al_builtins::ops::compare(a, b).is_ok_and(|o| o == std::cmp::Ordering::Equal))
            })
    }

    /// Whether the open transaction's only writes are Deletes of other rows of `t` and it took no lock: the neighbour shape
    /// `stale_neighbour_shape` decides as written (record/lt-sb-stale-own-modify/SbStaleOwnModify.al TeDeleteOther).
    pub(super) fn stale_after_other_deletes_only(&self, t: &TableDef, company: &Option<String>, pk: &[Value]) -> bool {
        !self.tx.writes.is_empty()
            && self
                .tx
                .writes
                .iter()
                .all(|w| w.kind == crate::state::TxWriteKind::Delete && w.table == t.object && w.company == *company)
            && self.stale_neighbour_shape(t, company, pk) == Some(StaleVerdict::Written)
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
    /// the same without one, and an assignment to it is not stored (record/lt-x-table-props/XTablePropsTests.al:412-415).
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
    /// to the field), and Init keeps them (record/lt-qr-temp-sqltimestamp/QrTempTimestamp.al:76-85, :111-115, :160-170, :221-226).
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
