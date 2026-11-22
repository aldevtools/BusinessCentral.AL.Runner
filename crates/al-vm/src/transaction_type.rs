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
    /// - once a write call has run (also a DeleteAll that finds no row) the type stays as it was: a set to Report or
    ///   Snapshot, or to the type it has, goes through without effect, and a set to Update raises
    ///   DB:CannotChangeTransactionType;
    /// - once a read call has run, a set of Report reads back as Update.
    ///
    /// Every other set after the transaction has started is not pinned and refused.
    pub(crate) fn current_transaction_type(&mut self, args: &[Value]) -> Result<Value, Signal> {
        use al_builtins::describe::Describe;
        let members: std::sync::Arc<[String]> = al_builtins::sysenum::option_members("TransactionType")
            .expect("a system option")
            .into();
        let new = match args.first() {
            None | Some(Value::Null) => return Ok(Value::Option(self.transaction_type_seen()?, members)),
            Some(Value::Option(t, _)) => *t,
            Some(other) => return unsupported(format!("CurrentTransactionType set to {}", other.describe())),
        };
        let (update, snapshot, report) = (1, 2, 4);
        // After a lock (codeunits/TestCodeunitRunWriteTransaction.al:211-278) the set is not pinned.
        if self.tx.phase != TxPhase::Write && self.tx.open.locks.any() {
            return unsupported(
                "Database.CurrentTransactionType set after a LockTable or a locking ReadIsolation (not pinned)",
            );
        }
        // At the start of a codeunit's first test method the test runner's transaction has started, and BC ignores a set to
        // another type there (corpus pull request 1488, session-system/TxTypeNextTestMethod.al TxN01's comment): not pinned.
        if self.tx.open.runner_tx && self.tx.phase == TxPhase::Fresh && new != self.tx.transaction_type {
            return unsupported(format!(
                "Database.CurrentTransactionType set to {} at the start of a test codeunit's first test method, where the test \
                 runner's transaction has already started (not pinned)",
                members.get(new as usize).cloned().unwrap_or_default()
            ));
        }
        // The set is a step of the moment an Update report is measured at (`OpenTx::moment`).
        self.moment_type_set(new, self.tx.phase == TxPhase::Fresh);
        if self.tx.phase == TxPhase::Fresh {
            (self.tx.transaction_type, self.tx.type_origin) = (new, TypeOrigin::default());
            return Ok(Value::Null);
        }
        // After a database call the rules below are measured from UpdateNoLocks (ProbeSystemEnumMembers.al:93-141), and
        // from Update after a Commit (TestClearAllTransactionTypeContracts.al:274-286); from another type they are not.
        let from = self.transaction_type_seen()?;
        let name = |t: i32| members.get(t as usize).cloned().unwrap_or_default();
        if !matches!(from, TRANSACTION_TYPE_UPDATE_NO_LOCKS | 1) {
            return unsupported(format!(
                "Database.CurrentTransactionType set to {} after a database call while the type is {} (not pinned)",
                name(new),
                name(from)
            ));
        }
        match self.tx.phase {
            TxPhase::Write if new == from => {}
            TxPhase::Write if new == snapshot || new == report => {}
            // The text is pinned for a change from UpdateNoLocks only (ProbeSystemEnumMembers.al:114-141).
            TxPhase::Write if new == update && from == TRANSACTION_TYPE_UPDATE_NO_LOCKS => {
                return Err(al_error_code(
                    &format!(
                        "The transaction type cannot be changed after a transaction has started.\\\\A transaction starts when the first database operation, such as FIND, NEXT, INSERT, DELETE, MODIFY, SUM, or COUNT, is run.\\\\The error occurred because the transaction type was changed from {} to {}.",
                        name(from),
                        name(new)
                    ),
                    "DB:CannotChangeTransactionType",
                ));
            }
            // From UpdateNoLocks, and from the Update the test before left (ProbeSystemEnumMembers.al:93-111, after :77-91).
            TxPhase::Read if new == report => {
                (self.tx.transaction_type, self.tx.type_origin) = (update, TypeOrigin::default())
            }
            _ => {
                return unsupported(format!(
                    "Database.CurrentTransactionType set to {} after a database call (not pinned)",
                    name(new)
                ));
            }
        }
        Ok(Value::Null)
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
        if self.tx.transaction_type == 1 {
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
