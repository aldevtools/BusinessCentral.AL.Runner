//! `Database.CurrentTransactionType`: the session's transaction type, what a set does at each point of a transaction, what a
//! Commit and the start of a test method do to it, and what the type rests on when no corpus case pins it
//! (`TxState::type_origin`). The Update report decision that reads it lives in `reports/transaction_world.rs`.

use crate::transactions::{TxPhase, al_error_code};
use crate::{Session, Signal};
use al_builtins::assumptions::Assumption;
use al_builtins::native::unsupported;
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
        // At the start of a codeunit's first test method the test runner's transaction has started
        // (session-system/TxTypeFirstTestMethod.al of the open probe): a set is a set after the start until a Commit or a
        // rollback ends it.
        let fresh = self.tx.phase == TxPhase::Fresh && !self.tx.open.runner_tx;
        // The set is a step of the moment an Update report is measured at (`OpenTx::moment`).
        self.moment_type_set(
            new,
            fresh || (self.tx.phase == TxPhase::Fresh && new == self.tx.transaction_type),
        );
        if fresh {
            // After a rollback in a codeunit's first test method the set takes effect as after a Commit
            // (TxTypeFirstTestMethod.al, the A tests after an asserterror).
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

    /// A set to `new` once the transaction has started: after a database call, or at the start of a codeunit's first test
    /// method (`OpenTx::runner_tx`). Measured (record/TxTypeSetAfterCall.al TxS01 to TxS15, TxTypeFirstTestMethod.al of the
    /// open probe, `transaction-type-set-after-start`; the pins named at each arm):
    /// - a set to the type the transaction has has no effect;
    /// - after a write, or the test runner's start: at UpdateNoLocks a set to Update or Snapshot raises
    ///   DB:CannotChangeTransactionType and a set to Browse or Report has no effect; at Update no set has an effect;
    /// - after a read call of the test's own, whether the read started the transaction depends on the server's data cache, so
    ///   every other set is refused, but the pinned Report after a read at Update.
    fn set_after_start(&mut self, new: i32) -> Result<(), Signal> {
        let from = self.transaction_type_seen()?;
        if new == from {
            return Ok(());
        }
        let members = type_members();
        let name = |t: i32| members.get(t as usize).cloned().unwrap_or_default();
        let decided = self.tx.phase == TxPhase::Write || self.tx.open.runner_tx;
        // The test runner's transaction is measured under AutoCommit (TxTypeFirstTestMethod.al).
        if self.tx.open.runner_tx && self.tx_model != crate::transactions::TxModel::AutoCommit {
            return unsupported(format!(
                "Database.CurrentTransactionType set to {} at the start of a test codeunit's first test method under \
                 TransactionModel {:?} (measured under AutoCommit)",
                name(new),
                self.tx_model
            ));
        }
        let pinned = match (decided, from, new) {
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
            // After a write: a set to Update raises and a set to Report has no effect (ProbeSystemEnumMembers.al:114-141);
            // at Update a set to Snapshot has no effect (TestClearAllTransactionTypeContracts.al:274-286).
            (_, TRANSACTION_TYPE_UPDATE_NO_LOCKS, UPDATE | REPORT) | (_, UPDATE, SNAPSHOT) => {
                self.tx.phase == TxPhase::Write && !self.tx.open.runner_tx
            }
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
        if from == TRANSACTION_TYPE_UPDATE_NO_LOCKS && matches!(new, UPDATE | SNAPSHOT) {
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

    /// A write of a record that is not temporary (`records/op_inner.rs`), at a type that does not write: Snapshot, Browse and
    /// Report raise DB:TransActionIsReadOnly in the code of a test for Insert, Modify, Delete, DeleteAll and ModifyAll
    /// (record/TxTypeSetAfterCall.al TxS16 to TxS34 of the open probe, `transaction-type-read-only-writes`; the text and the
    /// code are pinned for a report's own TransactionType, reports/W3ReportTransactions.al:207-249). Every other write
    /// there is refused.
    pub(crate) fn gate_read_only_type(
        &mut self,
        method: al_builtins::record_op::RecordOp,
        table_id: Option<u32>,
        crm: bool,
    ) -> Result<(), Signal> {
        use al_builtins::record_op::RecordOp::{Delete, DeleteAll, Insert, Modify, ModifyAll};
        // A report that entered the world of its TransactionType = Update writes there.
        if self.report.report_running.last().is_some_and(|r| r.update_world) {
            return Ok(());
        }
        // A type a test method that did not run to its end may have changed is taken as it is, as the other session state
        // such a test leaves (al-testrun `written.rs` counts a write as a read of the type, so a later failure is not
        // trusted).
        if !matches!(self.tx.transaction_type, SNAPSHOT | BROWSE | REPORT) {
            return Ok(());
        }
        let ty = self.transaction_type_seen()?;
        let plain = self.report.report_running.is_empty()
            && self.tx.tx_world == 0
            && self.tx.try_depth == 0
            && !self.no_transaction()
            && !crm
            && table_id.is_none_or(|id| id < 2_000_000_000)
            && matches!(method, Insert | Modify | Delete | DeleteAll | ModifyAll);
        if !plain {
            return unsupported(format!(
                "{} while Database.CurrentTransactionType is {} (measured for Insert, Modify, Delete, DeleteAll and ModifyAll \
                 of an application table in the code of a test)",
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

    /// The type a test method starts with. The first test method of a codeunit starts at UpdateNoLocks, whatever install
    /// code left (session-system/TestClearAllTransactionTypeContracts.al:12-13, read at :261). A later one starts with
    /// the type the one before left: UpdateNoLocks is read at a test's start after every kind of test before it in the corpus;
    /// any other type, Update too, rests on `transaction-type-carries-to-next-test`.
    pub(crate) fn start_test_transaction_type(&mut self) {
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
