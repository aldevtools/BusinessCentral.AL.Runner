//! Stale buffers by where their row version came from (corpus record/crud/Y76StaleBuffers.al); the rule itself is
//! `records/stale_rule.rs`.

use crate::Session;
use crate::records::StaleVerdict;
use al_builtins::native::Signal;
use al_builtins::value::{RowOrigin, TableWritten, Value};
use al_data::TableDef;

use crate::records::same_key;

type Pk = [Value];
type Rec = std::rc::Rc<std::cell::RefCell<al_builtins::value::RecordState>>;

/// The origin of the version the variable's own Modify gave a buffer whose origin `o` was its own Insert or Get in the
/// same transaction: its own write (record/crud/SbStaleOwnModify.al:132-264, :410-448). The Modify wrote the table;
/// that counts after a failed Insert only after the own Insert (corpus PR 1523,
/// record/crud/TestRecordFailedInsertMovedKey.al:561 Q7).
pub(crate) fn own_modify_origin(o: RowOrigin, now: i64) -> RowOrigin {
    let table_written = match o.table_written {
        TableWritten::ByRecord => TableWritten::ByRecord,
        _ => TableWritten::Otherwise,
    };
    RowOrigin {
        own_write: true,
        version: now,
        read_in_write_tx: false,
        own_modified: true,
        table_written,
        ..o
    }
}

/// The origin of the version the variable's own Rename gave a buffer of origin `o`
/// (record/crud/SbStaleOwnModify.al:268-298). The Rename wrote the table; a version from it is not measured after a
/// failed Insert.
pub(crate) fn renamed_origin(o: RowOrigin, now: i64) -> RowOrigin {
    RowOrigin {
        version: now,
        table_written: TableWritten::Otherwise,
        ..o
    }
}

impl Session {
    /// Gives `rec` the origin of the version `v` the variable's own Insert into `t` gave it. The Insert itself wrote
    /// the table (corpus PR 1523, record/crud/TestRecordFailedInsertMovedKey.al:283 P6, :528 Q6); after another kind of
    /// write of the table, or through a RecordRef, it is not measured.
    pub(crate) fn set_own_insert_origin(&self, t: &TableDef, rec: &Rec, v: i64) {
        let table_written = match self.table_written(t.object, &rec.borrow().company) {
            TableWritten::ByRecord if !self.through_recordref => TableWritten::ByRecord,
            _ => TableWritten::Otherwise,
        };
        rec.borrow_mut().row_origin = Some(RowOrigin {
            table_written,
            ..RowOrigin::new(true, v, self.commits)
        });
    }

    /// The origin of a row version a load just read. `how` is [Get, GetBySystemId, a Find].
    /// A version written in the open transaction (above the last Commit's) read by Get, GetBySystemId or a Find follows the
    /// rule of the buffer that wrote it (record/crud/SbStaleOwnModify.al:467-640; record/crud/StaleRuleMatrix.al
    /// GetUncommitted_2C). A Get after the Commit that followed the write keeps a Get origin
    /// (record/TestRecordStaleBufferWrite.al:604-660).
    /// `table_written`: what the open transaction had written to the table at the read (`table_written`).
    pub(crate) fn load_origin(
        &self,
        version: Option<&Value>,
        how: [bool; 3],
        table_written: TableWritten,
    ) -> Option<RowOrigin> {
        let [via_get, via_sysid, via_find] = how;
        match version {
            Some(Value::BigInteger(v)) if (via_get || via_sysid || via_find) && *v > self.tx.committed_row_version => {
                Some(RowOrigin {
                    read_in_write_tx: true,
                    by_get: via_get,
                    table_written,
                    ..RowOrigin::new(true, *v, self.commits)
                })
            }
            Some(Value::BigInteger(v)) if via_get || via_find => Some(RowOrigin {
                from_find: !via_get,
                by_get: via_get,
                table_written,
                ..RowOrigin::new(false, *v, self.commits)
            }),
            _ => None,
        }
    }

    /// What the open transaction had written to `t` when a load into `rec` read its row (`table_written`). A temporary
    /// record's writes are not in the transaction; a read through a RecordRef after a write of the table is not
    /// measured (`check_failed_insert_stale`).
    pub(crate) fn load_table_written(&self, t: &TableDef, rec: &Rec) -> TableWritten {
        let r = rec.borrow();
        match self.table_written(t.object, &r.company) {
            _ if r.temporary => TableWritten::No,
            TableWritten::ByRecord if self.through_recordref => TableWritten::Otherwise,
            w => w,
        }
    }

    /// The `table_written` of a version TransferFields copied into a buffer of `table` from a source of origin `src`:
    /// after a write of the table (before the source's read or now) it is not measured after a failed Insert
    /// (`check_failed_insert_stale`).
    pub(crate) fn transferred_table_written(
        &self,
        src: Option<RowOrigin>,
        table: u32,
        company: &Option<String>,
    ) -> TableWritten {
        let before = src.is_some_and(|o| o.table_written != TableWritten::No);
        match before || self.table_written(table, company) != TableWritten::No {
            true => TableWritten::Otherwise,
            false => TableWritten::No,
        }
    }

    /// What the open transaction has written to `table` in `company` so far, for a row version reaching a buffer now
    /// (`RowOrigin::table_written`). Only a Record variable's Insert or Modify of the table is `ByRecord` (corpus PR
    /// 1523, aldevtools/BusinessCentral.AL.Language.Tests#1523, record/crud/TestRecordFailedInsertMovedKey.al: :393 Q1a
    /// and :414 Q1b an Insert of the row after a DeleteAll that deleted nothing, :432 Q2 an Insert of another row, :503
    /// Q5 a Modify of the row; a write of another table does not count, :454 Q3). A DeleteAll that deleted nothing
    /// counts only beside one. Any other write, a write through a RecordRef or in another company, and a LockTable are
    /// `Otherwise`.
    pub(crate) fn table_written(&self, table: u32, company: &Option<String>) -> TableWritten {
        use crate::state::TxWriteKind;
        // Most loads run with nothing written or locked since the last Commit: no scan of the write log then.
        if self.tx.open.writes.is_empty() && self.tx.open.locked_tables.is_empty() {
            return TableWritten::No;
        }
        let mut by_record = false;
        let mut other = self.tx.open.locked_tables.contains(&table);
        for w in self.tx.open.writes.iter().filter(|w| w.table == table) {
            let here = w.company == *company && !w.through_recordref;
            let plain = here && matches!(w.kind, TxWriteKind::Insert | TxWriteKind::Modify);
            let deleted_nothing = here
                && w.kind == TxWriteKind::DeleteAll
                && w.bulk.is_some_and(|b| b.entries == 0 && b.counter == w.before);
            by_record |= plain;
            other |= !plain && !deleted_nothing;
        }
        match (by_record, other) {
            (true, false) => TableWritten::ByRecord,
            (false, false) if !self.tx.open.writes.iter().any(|w| w.table == table) => TableWritten::No,
            _ => TableWritten::Otherwise,
        }
    }

    /// A buffer that only got TransferFields from a record read by Get or Find (Y76StaleBuffers.al
    /// Y76_Part1_Copy_TransferFromRecord, `transfer_from_record/*`): written after a LockTable and Get of the row and nothing else the
    /// new transaction did to it; every write to the row (changed or not, a Delete after it) is refused. Writes beside, other Commit
    /// counts and a rollback are not pinned.
    pub(crate) fn stale_lock_only_verdict(
        &self,
        t: &TableDef,
        company: &Option<String>,
        pk: &Pk,
        buffer_commits: u64,
        touched: bool,
        contact: bool,
    ) -> Result<StaleVerdict, Signal> {
        let writes_row = self
            .tx
            .open
            .writes
            .iter()
            .any(|w| w.table == t.object && w.company == *company && same_key(&w.pk, pk));
        if buffer_commits + 1 != self.commits || !self.writes_all_noted() || self.tx.rolled_back {
            return Err(Signal::unsupported(format!(
                "a stale {} buffer from TransferFields, several Commits later or after a rollback (not pinned)",
                t.name
            )));
        }
        if writes_row {
            return Ok(StaleVerdict::Refused);
        }
        if touched && self.tx.open.writes.is_empty() {
            return Ok(StaleVerdict::Written);
        }
        if !contact {
            return Ok(StaleVerdict::Refused);
        }
        Err(Signal::unsupported(format!(
            "a stale {} buffer from TransferFields after the new transaction locked or wrote the table in a way no test pins",
            t.name
        )))
    }
}
