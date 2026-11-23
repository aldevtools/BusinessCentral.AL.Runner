//! `Database.CurrentTransactionType`: the session's transaction type, what a set does at each point of a transaction, what a
//! Commit and the start of a test method do to it, and what the type rests on when no corpus case pins it
//! (`TxState::type_origin`). The Update report decision that reads it lives in `reports/transaction_world.rs`.

use crate::transactions::{TxPhase, al_error_code};
use crate::{Session, Signal};
use al_builtins::assumptions::Assumption;
use al_builtins::native::unsupported;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;

/// `TxState::transaction_type` of a session's start: the TransactionType member UpdateNoLocks
/// (session-system/ProbeSystemEnumMembers.al:60-161).
pub(crate) const TRANSACTION_TYPE_UPDATE_NO_LOCKS: i32 = 0;
/// The other members of TransactionType (ProbeSystemEnumMembers.al:60-75).
const UPDATE: i32 = 1;
const SNAPSHOT: i32 = 2;
const BROWSE: i32 = 3;
const REPORT: i32 = 4;

/// The members of the TransactionType option, by ordinal.
fn type_members() -> std::sync::Arc<[String]> {
    al_builtins::sysenum::option_members("TransactionType")
        .expect("a system option")
        .into()
}

/// What the session's transaction type rests on when no corpus case pins it (`TxState::type_origin`). The default: a pinned
/// value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub(crate) struct TypeOrigin {
    /// The assumptions the value went through, every one of them (a carried type a Commit then kept rests on both).
    marks: al_builtins::assumptions::HitBits,
    /// A test method before this one did not run to its end and could set the type: the value is not known.
    unknown: bool,
}
impl Session {
    /// `Database.CurrentTransactionType`, read with no argument and set with one (session-system/ProbeSystemEnumMembers.al:
    /// 60-161; TestClearAllTransactionTypeContracts.al:224-253):
    /// - a test starts as UpdateNoLocks, and a set before any database call takes effect;
    /// - a Commit keeps the type ([`Self::transaction_type_seen`]), and a set takes effect again;
    /// - once the transaction has started, [`Self::set_after_start`] says what a set does.
    pub(crate) fn current_transaction_type(&mut self, args: &[Value]) -> Result<Value, Signal> {
        use al_builtins::describe::Describe;
        let new = match args.first() {
            None | Some(Value::Null) => {
                return Ok(Value::Option(self.transaction_type_seen()?, type_members()));
            }
            Some(Value::Option(t, _)) => *t,
            Some(other) => return unsupported(format!("CurrentTransactionType set to {}", other.describe())),
        };
        // After a lock (codeunits/TestCodeunitRunWriteTransaction.al:211-278) the set is not pinned.
        if self.tx.phase != TxPhase::Write && self.tx.open.locks.any() {
            return unsupported(
                "Database.CurrentTransactionType set after a LockTable or a locking ReadIsolation (not pinned)",
            );
        }
        // At the start of a codeunit's first test method the test runner's transaction may have started (the description of
        // corpus pull request 1520): until a Commit or the rollback of an asserterror in the code of the test ends it, a set is
        // one after the start.
        let fresh = self.tx.phase == TxPhase::Fresh && !self.tx.open.runner_tx;
        // The set is a step of the moment an Update report is measured at (`OpenTx::moment`).
        self.moment_type_set(
            new,
            fresh || (self.tx.phase == TxPhase::Fresh && new == self.tx.transaction_type),
        );
        if fresh {
            // After a rollback in a codeunit's first test method the set takes effect as after a Commit
            // (session-system/TxTypeFirstTestMethod.al of corpus pull request 1520, the asserterror cells).
            let mut marks = al_builtins::assumptions::HitBits::default();
            if self.tx.runner_tx_rolled_back {
                al_builtins::assumptions::assume(Assumption::FirstTestRollbackEndsRunnerTransaction);
                marks = marks.with(Assumption::FirstTestRollbackEndsRunnerTransaction);
            }
            (self.tx.transaction_type, self.tx.type_origin) = (new, TypeOrigin { marks, unknown: false });
            return Ok(Value::Null);
        }
        self.set_after_start(new)?;
        Ok(Value::Null)
    }

    /// A set to `new` once the transaction may have started: after a database call, or at the start of a codeunit's first
    /// test method (`OpenTx::runner_tx`). Measured (record/TxTypeSetAfterCall.al TxS01 to TxS15 of the open probe,
    /// `transaction-type-set-after-start`; the pins named at each arm):
    /// - a set to the type the transaction has has no effect;
    /// - after a write: at UpdateNoLocks a set to Update or Snapshot raises DB:CannotChangeTransactionType and a set to
    ///   Browse or Report has no effect; at Update no set has an effect;
    /// - after a read call, whether the read started the transaction depends on the server's data cache, and at a first test
    ///   method's start whether the test runner's transaction has started depends on the runner and on what ran before it
    ///   (corpus pull request 1520: a set to Update raised after another codeunit on a local container, and took effect
    ///   when the codeunit ran alone and on the BC 27.5 leg), so every other set is refused, but the pinned Report after a
    ///   read at Update.
    fn set_after_start(&mut self, new: i32) -> Result<(), Signal> {
        let from = self.transaction_type_seen()?;
        let members = type_members();
        let name = |t: i32| members.get(t as usize).cloned().unwrap_or_default();
        let decided = self.tx.phase == TxPhase::Write;
        if self.tx.phase == TxPhase::Fresh {
            // A set to the type it has leaves the type as it is whether the test runner's transaction has started or not
            // (corpus pull request 1488, record/TxTypeUpdateReport.al TxU00 sets UpdateNoLocks there).
            if new == from {
                return Ok(());
            }
            return unsupported(format!(
                "Database.CurrentTransactionType set to {} at the start of a test codeunit's first test method (whether the \
                 test runner's transaction has started there depends on the runner and on what ran before it: not pinned)",
                name(new)
            ));
        }
        let pinned = match (decided, from, new) {
            // A set to the type it has after a write at UpdateNoLocks or Update: main's answer, kept (the passing tests
            // ProbeSystemEnumMembers.al:128 and :153 make it after a write at UpdateNoLocks; TxS01 and TxS07 measure it).
            // After a read it rests on the open probe (TxS11 to TxS15).
            (true, TRANSACTION_TYPE_UPDATE_NO_LOCKS | UPDATE, _) if new == from => true,
            (_, _, _) if new == from => false,
            // A set to Report after a read at the Update the test before left (ProbeSystemEnumMembers.al:93-111).
            (false, UPDATE, REPORT) => true,
            (false, ..) => {
                return unsupported(format!(
                    "Database.CurrentTransactionType set to {} after a read call while the type is {} (whether the read \
                     started the transaction depends on the server's data cache: not pinned)",
                    name(new),
                    name(from)
                ));
            }
            // After a write at UpdateNoLocks: a set to Update raises and a set to Report has no effect
            // (ProbeSystemEnumMembers.al:114-141), a set to Browse or Report has no effect and a set to Snapshot raises
            // (metadata/platform-tables/PlTDatabase.al:143-171); at Update a set to Snapshot has no effect
            // (TestClearAllTransactionTypeContracts.al:274-286).
            (_, TRANSACTION_TYPE_UPDATE_NO_LOCKS, UPDATE | SNAPSHOT | BROWSE | REPORT) | (_, UPDATE, SNAPSHOT) => true,
            (_, TRANSACTION_TYPE_UPDATE_NO_LOCKS | UPDATE, _) => false,
            // A write at another type raises (`gate_read_only_type`), so it cannot have started the transaction.
            _ => {
                return unsupported(format!(
                    "Database.CurrentTransactionType set to {} after a database call while the type is {} (not pinned)",
                    name(new),
                    name(from)
                ));
            }
        };
        if !pinned {
            al_builtins::assumptions::assume(Assumption::TransactionTypeSetAfterStart);
            let origin = &mut self.tx.type_origin;
            origin.marks = origin.marks.with(Assumption::TransactionTypeSetAfterStart);
        }
        if new != from && from == TRANSACTION_TYPE_UPDATE_NO_LOCKS && matches!(new, UPDATE | SNAPSHOT) {
            return Err(al_error_code(
                &format!(
                    "The transaction type cannot be changed after a transaction has started.\\\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\\\The error occurred because the transaction type was changed from {} to {}.",
                    name(from),
                    name(new)
                ),
                "DB:CannotChangeTransactionType",
            ));
        }
        Ok(())
    }

    /// Before the rollback of an asserterror: whether it ends the transaction the test runner may have started before a
    /// codeunit's first test method (`OpenTx::runner_tx`). Measured for an asserterror in the code of the test, with and
    /// without a write in it (session-system/TxTypeFirstTestMethod.al of corpus pull request 1520); the rollback of a guarded
    /// Codeunit.Run, of a report's world and of an XmlPort import keep it (`Session::rollback_transaction`).
    pub(crate) fn asserterror_ends_runner_tx(&self) -> bool {
        self.tx.open.runner_tx && self.tx.tx_world == 0 && self.tx.try_depth == 0 && self.in_test_code()
    }

    /// The rollback of an asserterror ended the test runner's transaction ([`Self::asserterror_ends_runner_tx`]).
    pub(crate) fn runner_tx_ended_by_asserterror(&mut self) {
        self.tx.open.runner_tx = false;
        self.tx.runner_tx_rolled_back = true;
    }

    /// A write of a record that is not temporary (`records/op_inner.rs`, before the events and triggers of the call), at a
    /// type that does not write: Snapshot, Browse and Report raise DB:TransActionIsReadOnly in the shapes of
    /// record/TxTypeSetAfterCall.al TxS16 to TxS34 of corpus pull request 1520 (`transaction-type-read-only-writes`; the text
    /// and the code are pinned for a report's own TransactionType, reports/W3ReportTransactions.al:207-249), checked by
    /// [`Self::read_only_write_measured`]. Every other write there is refused. A write while the type a test before left is
    /// not known is marked `transaction-type-carries-to-next-test`.
    pub(crate) fn gate_read_only_type(
        &mut self,
        method: RecordOp,
        t: &al_data::TableDef,
        rec: &crate::records::Rec,
        args: &[Value],
        cx: &al_data::AccessContext,
    ) -> Result<(), Signal> {
        // A report that entered the world of its TransactionType = Update writes there.
        if self.report.report_running.last().is_some_and(|r| r.update_world) {
            return Ok(());
        }
        if !self.tx.type_origin.unknown && !matches!(self.tx.transaction_type, SNAPSHOT | BROWSE | REPORT) {
            return Ok(());
        }
        // The test runner's own steps around a test method write its results whatever type the test left: the test after
        // one that ended at Browse, Snapshot or Report runs and reports as usual (corpus pull request 1488,
        // session-system/TxTypeNextTestMethod.al:22-38, :43-56, :61-74, TxN01 to TxN06). Only a write under a test method
        // is measured.
        if !self
            .frames
            .iter()
            .zip(&self.call_stack)
            .any(|(&o, f)| self.is_test_method(o, f.routine))
        {
            return Ok(());
        }
        // A write while the type a test before left is not known runs at the type the runner holds and is marked: it rests
        // on that test having left a type that writes (al-testrun `written.rs` keeps a later failure from being trusted).
        let ty = if self.tx.type_origin.unknown {
            al_builtins::assumptions::add_hit_bits(self.tx.type_origin.marks);
            al_builtins::assumptions::assume(Assumption::TransactionTypeCarriesToNextTest);
            self.tx.transaction_type
        } else {
            self.transaction_type_seen()?
        };
        if !matches!(ty, SNAPSHOT | BROWSE | REPORT) {
            return Ok(());
        }
        if !self.read_only_write_measured(method, t, rec, args, cx)? {
            return unsupported(format!(
                "{} while Database.CurrentTransactionType is {} (measured for Insert of a new key, Modify that changes a row, \
                 Delete of a row, DeleteAll and ModifyAll of a matching row, without RunTrigger, of an application table \
                 with no AutoIncrement field, no subscriber of its database events and no global database trigger, in the \
                 code of a test)",
                method.name(),
                type_members().get(ty as usize).cloned().unwrap_or_default()
            ));
        }
        al_builtins::assumptions::assume(Assumption::TransactionTypeReadOnlyWrites);
        Err(al_error_code(
            "Write operations in the database (INSERT, DELETE, and MODIFY) are not allowed for the selected transaction type.",
            "DB:TransActionIsReadOnly",
        ))
    }

    /// Whether a write at a type that does not write has the shape the cells measure (TxTypeSetAfterCall.al TxS16 to TxS34): a
    /// record variable of an application table with no AutoIncrement field, in the code of a test (not in a report, a
    /// Codeunit.Run, a TryFunction or a page, not under TransactionModel None), the statement form, no RunTrigger, no
    /// subscriber of the table's database events and no global database trigger, so no AL code runs for the call before
    /// the write. Insert of a key that is not stored, Modify that changes a field of a fully loaded row that is stored,
    /// Delete of a stored row, DeleteAll (also of no row) and ModifyAll of at least one row, with no FlowField filter.
    fn read_only_write_measured(
        &mut self,
        method: RecordOp,
        t: &al_data::TableDef,
        rec: &crate::records::Rec,
        args: &[Value],
        cx: &al_data::AccessContext,
    ) -> Result<bool, Signal> {
        let obj = &self.engine.program.objects[t.object as usize];
        let auto_increment = obj
            .table
            .as_ref()
            .is_some_and(|s| s.fields.iter().any(|f| f.auto_increment));
        let events: &[&str] = match method {
            RecordOp::Insert => &["onbeforeinsertevent", "onafterinsertevent"],
            RecordOp::Modify | RecordOp::ModifyAll => &["onbeforemodifyevent", "onaftermodifyevent"],
            RecordOp::Delete | RecordOp::DeleteAll => &["onbeforedeleteevent", "onafterdeleteevent"],
            _ => return Ok(false),
        };
        let run_at = if method == RecordOp::ModifyAll { 2 } else { 0 };
        let run = match args.get(run_at) {
            Some(v) => crate::ops::truthy(v)?,
            None => false,
        };
        if !(self.report.report_running.is_empty()
            && self.tx.tx_world == 0
            && self.tx.try_depth == 0
            && !self.no_transaction()
            && !rec.borrow().crm
            && !self.through_recordref
            && !self.ret_used
            && obj.id.is_none_or(|id| id < 2_000_000_000)
            && !auto_increment
            && !run
            && !self.engine.events.has_global_triggers)
        {
            return Ok(false);
        }
        for e in events {
            if self.listened(t, e, None)? {
                return Ok(false);
            }
        }
        let mut stored = || -> Result<Option<Vec<Value>>, Signal> {
            let pk: Vec<Value> = {
                let r = rec.borrow();
                t.keys[0].iter().map(|&i| r.fields[i].clone()).collect()
            };
            Ok(al_data::Store::get(&*self.store_for(rec)?, cx, t, &pk)
                .map_err(|e| Signal::unsupported(format!("store error {e}")))?
                .map(|row| row.to_vec()))
        };
        Ok(match method {
            RecordOp::Insert => stored()?.is_none(),
            RecordOp::Delete => stored()?.is_some(),
            RecordOp::Modify => {
                let Some(row) = stored()? else { return Ok(false) };
                let r = rec.borrow();
                if r.load.partial {
                    return Ok(false);
                }
                let mut changed = false;
                // Stored fields only: a FlowField CalcFields filled is no change of the row.
                for (i, _) in t
                    .fields
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.stored && f.id < 2_000_000_000)
                {
                    let (Some(old), Some(new)) = (row.get(i), r.fields.get(i)) else {
                        return Ok(false);
                    };
                    // A value the comparison cannot order (a Blob, a Media) is not measured.
                    match crate::ops::compare(old, new) {
                        Ok(o) => changed |= !o.is_eq(),
                        Err(_) => return Ok(false),
                    }
                }
                changed
            }
            _ => {
                let (filters, flow) = self.split_filters(rec)?;
                let rows = al_data::Store::count(&*self.store_for(rec)?, cx, t, &filters)
                    .map_err(|e| Signal::unsupported(format!("{e}")))?;
                flow.is_empty() && (method == RecordOp::DeleteAll || rows > 0)
            }
        })
    }

    /// `TxState::transaction_type` as AL code sees it, through the assumptions it rests on (`TxState::type_origin`): a type a
    /// Commit kept and a type the test method before left are what the open probe of corpus pull request 1488 measures
    /// (record/TxTypeUpdateReport.al TxU21 to TxU26 and TxU34 to TxU38, `commit-keeps-transaction-type`;
    /// session-system/TxTypeNextTestMethod.al TxN01 to TxN12, `transaction-type-carries-to-next-test`).
    pub(crate) fn transaction_type_seen(&self) -> Result<i32, Signal> {
        let origin = self.tx.type_origin;
        if origin.unknown {
            return unsupported(
                "Database.CurrentTransactionType after a test method of the codeunit that did not run to its end and could \
                 set it (the type it left is not known)",
            );
        }
        al_builtins::assumptions::add_hit_bits(origin.marks);
        Ok(self.tx.transaction_type)
    }

    /// A test method of the codeunit did not run to its end and its reach can set the type (al-testrun `tx_type_left.rs`):
    /// the type it would have left on BC, which the next test method starts with, is not known until a set that takes effect.
    pub fn transaction_type_unknown(&mut self) {
        self.tx.type_origin.unknown = true;
    }

    /// Whether the session's transaction type is known ([`Self::transaction_type_unknown`]).
    pub fn transaction_type_known(&self) -> bool {
        !self.tx.type_origin.unknown
    }

    /// A Commit keeps the type. At Update that is pinned (session-system/ProbeSystemEnumMembers.al:94-110, and Update the
    /// test before left, TestClearAllTransactionTypeContracts.al:274-286), so a carried Update reads clean after it; any
    /// other type a Commit kept rests on `commit-keeps-transaction-type` as well.
    pub(crate) fn keep_type_at_commit(&mut self) {
        let origin = &mut self.tx.type_origin;
        if origin.unknown {
            return;
        }
        if self.tx.transaction_type == UPDATE {
            origin.marks = Default::default();
        } else {
            origin.marks = origin.marks.with(Assumption::CommitKeepsTransactionType);
        }
    }

    /// The open transaction at a test method's start: the type ([`Self::start_test_transaction_type`]), nothing of it has
    /// run, no lock, and the moment an Update report is measured at is the test's start (`OpenTx::moment`).
    pub(crate) fn start_test_transaction(&mut self) {
        self.start_test_transaction_type();
        self.tx.phase = TxPhase::Fresh;
        self.tx.open.locks = Default::default();
        self.moment_at_test_start();
    }

    /// The type a test method starts with. The first test method of a codeunit starts at UpdateNoLocks, whatever install
    /// code left (session-system/TestClearAllTransactionTypeContracts.al:12-13, read at :261). A later one starts with
    /// the type the one before left: UpdateNoLocks is read at a test's start after every kind of test before it in the corpus;
    /// any other type, Update too, rests on `transaction-type-carries-to-next-test`.
    fn start_test_transaction_type(&mut self) {
        let origin = &mut self.tx.type_origin;
        if !self.tx.test_ended {
            self.tx.transaction_type = TRANSACTION_TYPE_UPDATE_NO_LOCKS;
            *origin = TypeOrigin::default();
            self.tx.runner_tx_rolled_back = false;
        } else if origin.unknown {
        } else if self.tx.transaction_type == TRANSACTION_TYPE_UPDATE_NO_LOCKS {
            *origin = TypeOrigin::default();
        } else {
            origin.marks = origin.marks.with(Assumption::TransactionTypeCarriesToNextTest);
        }
        // Before the first test method the runner's own transaction has started (`OpenTx::runner_tx`).
        self.tx.open.runner_tx = !self.tx.test_ended;
    }

    /// A test method of this codeunit has ended (the test runner, after its step after the method): the next one starts with
    /// the type it left.
    pub fn test_method_ended(&mut self) {
        self.tx.test_ended = true;
    }
}
