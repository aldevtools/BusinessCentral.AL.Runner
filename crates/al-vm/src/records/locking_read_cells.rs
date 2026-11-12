//! The measured cells of a write through a stale own-write buffer after a locking ReadIsolation read in the open transaction
//! (al-runner issue 7795), and the note of each such read.
//!
//! The rule of `stale_rule.rs` does not decide these: only the exact cells record/crud/CodeunitRunTableNoWrites.al measures
//! (BC 27.5, 28.5, 29.0) run, each with the result measured for it. A cell is: the write through the buffer, the Commits since
//! the buffer's own write, the one database read of the open transaction (its isolation level and what it read), and the
//! writes of the open transaction before that read. Anything else is refused by the caller.

use super::stale_rule::Contact;
use super::*;
use crate::recmeta::{ISOLATION_REPEATABLE_READ, ISOLATION_UPDLOCK};
use crate::state::TxWriteKind;
use al_builtins::filter::FilterExpr;
use al_builtins::value::RowOrigin;

/// The write through a stale buffer that asks for a verdict (`Session::stale_verdict`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaleWrite {
    /// `Modify(RunTrigger)`.
    Modify {
        run_trigger: bool,
    },
    Delete,
}

/// The form of a database read under a locking ReadIsolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LockingRead {
    Get,
    FindSet,
    FindFirst,
    /// Any other read: Find, FindLast, Next, Count, IsEmpty, CalcFields, CalcSums, GetBySystemId.
    Other,
}

/// One database read under RepeatableRead or UpdLock since the last Commit or rollback (`TxState::locking_reads`).
#[derive(Debug, Clone)]
pub(crate) struct LockingReadNote {
    table: u32,
    company: Option<String>,
    /// `ISOLATION_REPEATABLE_READ` or `ISOLATION_UPDLOCK`.
    level: i32,
    form: LockingRead,
    /// The one row the read names: the key a Get was given, or the value of a FindSet or FindFirst whose only filter is a
    /// `SetRange(Field, Value)` on the table's one primary key field.
    key: Option<Vec<Value>>,
    /// The record had no filter and no MarkedOnly.
    unfiltered: bool,
    /// The read found a row.
    found: bool,
    /// The store's change counter after the read: no write of the open transaction followed it while it still holds.
    counter: u64,
}

/// What the one locking read of a cell read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadOf {
    /// `Get` of the buffer's row.
    GetOwnRow,
    /// `Get` of another row of the buffer's table that the open transaction did not write.
    GetOtherRow,
    /// `FindSet` with `SetRange` on the primary key to such another row.
    FindSetOtherRow,
    /// `FindFirst` of another table, without a filter.
    FindFirstOtherTable,
}

/// One measured cell and its result.
struct Cell {
    write: StaleWrite,
    /// Commits since the buffer's own write.
    commits: u64,
    level: i32,
    read: ReadOf,
    /// Before the read the open transaction inserted and modified one other row of the buffer's table (`true`), or wrote nothing.
    other_row_written: bool,
    result: Contact,
}

const fn cell(run_trigger: bool, commits: u64, level: i32, read: ReadOf, other_row_written: bool) -> Cell {
    Cell {
        write: StaleWrite::Modify { run_trigger },
        commits,
        level,
        read,
        other_row_written,
        // With the other row written the buffer's Modify is written, without it DB:RecordChanged, whatever the read.
        result: if other_row_written {
            Contact::Table
        } else {
            Contact::Nothing
        },
    }
}

/// record/crud/CodeunitRunTableNoWrites.al. The buffer wrote its row (Insert, or Get and Modify), another variable modified
/// the row, two Commits; one read; `Modify()`: DB:RecordChanged. The buffer did Insert(true) and Modify(true), a partial record
/// modified the row, one Commit; another row inserted and modified, or not; one read; `Modify(true)`.
const LOCKING_READ_CELLS: [Cell; 10] = [
    // :446 OwnModify_UpdLock_GetOtherRow, :512 OwnInsert_UpdLock_GetOtherRow
    cell(false, 2, ISOLATION_UPDLOCK, ReadOf::GetOtherRow, false),
    // :457 OwnModify_UpdLock_FindSetOtherRow, :523 OwnInsert_UpdLock_FindSetOtherRow
    cell(false, 2, ISOLATION_UPDLOCK, ReadOf::FindSetOtherRow, false),
    // :468 OwnModify_RepeatableRead_GetOtherRow, :534 OwnInsert_RepeatableRead_GetOtherRow
    cell(false, 2, ISOLATION_REPEATABLE_READ, ReadOf::GetOtherRow, false),
    // :629 PartialWrite_OtherRow_UpdLockOtherTable
    cell(true, 1, ISOLATION_UPDLOCK, ReadOf::FindFirstOtherTable, true),
    // :642 PartialWrite_NoOtherRow_UpdLockOtherTable
    cell(true, 1, ISOLATION_UPDLOCK, ReadOf::FindFirstOtherTable, false),
    // :654 PartialWrite_OtherRow_UpdLockOwnRow
    cell(true, 1, ISOLATION_UPDLOCK, ReadOf::GetOwnRow, true),
    // :680 PartialWrite_NoOtherRow_UpdLockOwnRow
    cell(true, 1, ISOLATION_UPDLOCK, ReadOf::GetOwnRow, false),
    // :692 PartialWrite_OtherRow_UpdLockOtherRow
    cell(true, 1, ISOLATION_UPDLOCK, ReadOf::GetOtherRow, true),
    // :705 PartialWrite_NoOtherRow_UpdLockOtherRow
    cell(true, 1, ISOLATION_UPDLOCK, ReadOf::GetOtherRow, false),
    // :730 PartialWrite_OtherRow_RepeatableReadOwnRow
    cell(true, 1, ISOLATION_REPEATABLE_READ, ReadOf::GetOwnRow, true),
];

impl Session {
    /// Notes the database read `method` of `rec` that just ended with `result`, when the record reads under RepeatableRead or
    /// UpdLock (`record_op_unprofiled` calls this after every record method).
    #[inline]
    pub(crate) fn note_locking_read(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &[Value],
        result: &Result<Value, Signal>,
    ) {
        if matches!(
            rec.borrow().read_isolation,
            ISOLATION_REPEATABLE_READ | ISOLATION_UPDLOCK
        ) {
            self.note_locking_read_cold(method, rec, args, result);
        }
    }

    #[cold]
    #[inline(never)]
    fn note_locking_read_cold(&mut self, method: RecordOp, rec: &Rec, args: &[Value], result: &Result<Value, Signal>) {
        let form = match method {
            RecordOp::Get => LockingRead::Get,
            RecordOp::FindSet => LockingRead::FindSet,
            RecordOp::FindFirst => LockingRead::FindFirst,
            RecordOp::GetBySystemId
            | RecordOp::Find
            | RecordOp::FindLast
            | RecordOp::Next
            | RecordOp::Count
            | RecordOp::IsEmpty
            | RecordOp::CalcFields
            | RecordOp::CalcSums => LockingRead::Other,
            _ => return,
        };
        let r = rec.borrow();
        if r.temporary {
            return;
        }
        let pk_field = self.engine.tables[r.table as usize]
            .as_ref()
            .and_then(|t| match t.keys[0].as_slice() {
                [one] => Some(*one),
                _ => None,
            });
        let key = match (form, r.filters.as_slice()) {
            (LockingRead::Get, _) => Some(args.to_vec()),
            (LockingRead::FindSet | LockingRead::FindFirst, [f]) if Some(f.field) == pk_field && !r.marked_only => {
                match &f.expr {
                    FilterExpr::Range(Some(a), Some(b))
                        if same_key(std::slice::from_ref(a), std::slice::from_ref(b)) =>
                    {
                        Some(vec![a.clone()])
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        let note = LockingReadNote {
            table: r.table,
            company: r.company.clone(),
            level: r.read_isolation,
            form,
            key,
            unfiltered: r.filters.is_empty() && !r.marked_only,
            found: !matches!(result, Err(_) | Ok(Value::Boolean(false))),
            counter: self.store.change_counter(),
        };
        drop(r);
        self.tx.locking_reads.push(note);
    }

    /// The result of the measured cell the open transaction is, for a `write` through the stale own-write buffer of origin `o`
    /// on the row `pk` of `t`; `None` when it is no cell of `LOCKING_READ_CELLS`.
    pub(super) fn locking_read_cell(
        &self,
        t: &TableDef,
        company: &Option<String>,
        pk: &[Value],
        o: RowOrigin,
        write: StaleWrite,
    ) -> Option<Contact> {
        let tx = &self.tx;
        // One database read in the open transaction, the locking one, after every write of it; its table the only one a
        // ReadIsolation locked; no LockTable.
        let [read] = tx.locking_reads.as_slice() else {
            return None;
        };
        let lock_of_the_read =
            matches!(tx.isolation_lock_tables.as_slice(), [(ob, c)] if *ob == read.table && *c == read.company);
        if tx.db_reads != 1
            || !read.found
            || read.counter != self.store.change_counter()
            || !lock_of_the_read
            || !tx.locked_tables.is_empty()
        {
            return None;
        }
        let mine = |table: u32, c: &Option<String>| table == t.object && c == company;
        // The writes before the read: none, or the Insert and the Modify of one other row of the buffer's table.
        let other_row = match tx.writes.as_slice() {
            [] => None,
            [i, m]
                if i.kind == TxWriteKind::Insert
                    && m.kind == TxWriteKind::Modify
                    && mine(i.table, &i.company)
                    && mine(m.table, &m.company)
                    && same_key(&i.pk, &m.pk)
                    && !same_key(&i.pk, pk) =>
            {
                Some(&i.pk)
            }
            _ => return None,
        };
        let another_row = |k: &[Value]| !same_key(k, pk) && other_row.is_none_or(|w| !same_key(k, w));
        let own_table = mine(read.table, &read.company);
        let read_of = match (read.form, &read.key) {
            (LockingRead::Get, Some(k)) if own_table && same_key(k, pk) => ReadOf::GetOwnRow,
            (LockingRead::Get, Some(k)) if own_table && another_row(k) => ReadOf::GetOtherRow,
            (LockingRead::FindSet, Some(k)) if own_table && another_row(k) => ReadOf::FindSetOtherRow,
            (LockingRead::FindFirst, None) if !own_table && read.unfiltered => ReadOf::FindFirstOtherTable,
            _ => return None,
        };
        let commits = self.commits.checked_sub(o.commits)?;
        LOCKING_READ_CELLS
            .iter()
            .find(|c| {
                c.write == write
                    && c.commits == commits
                    && c.level == read.level
                    && c.read == read_of
                    && c.other_row_written == other_row.is_some()
            })
            .map(|c| c.result)
    }
}
