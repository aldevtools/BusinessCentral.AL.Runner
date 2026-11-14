//! Transactions as AL code sees them: `Codeunit.Run` (guarded and unguarded), the test
//! method's `TransactionModel`, `Commit` under `AutoRollback` and `[CommitBehavior]`, and
//! `Database.IsInWriteTransaction`.
//!
//! What counts as "a write transaction is open" is "the store holds a change since the last
//! commit" (`MemStore::has_uncommitted`). Every rule below is pinned by a corpus test, cited
//! where it is applied; what the corpus does not pin is refused (`Signal::Unsupported`) and
//! listed in docs/probes-needed.md.

use crate::{AlError, Session, Signal};
use al_builtins::builtin_id::ids;
use al_builtins::native::unsupported;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;

/// The transaction model of the running test method (`[TransactionModel(...)]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TxModel {
    /// The default: the test's work is committed when it passes.
    #[default]
    AutoCommit,
    /// The test's work is rolled back when it finishes; `Commit` is refused.
    AutoRollback,
    /// The test body runs with no transaction at all.
    None,
}

/// The error code of a write inside a TryFunction whose result is used (errors/CuTryTests.al:83-150).
pub(crate) const TRY_FUNCTION_WRITE: &str = "TryFunctionWrite";

/// A TryFunction call on the call stack whose result is used (`Session::enter_used_try`).
pub(crate) struct UsedTry {
    /// The scope name the TryFunctionWrite error names; `None` where it is not pinned.
    scope: Option<String>,
    /// `tx_world` when the call began.
    world: usize,
    /// `Codeunit.Run` calls begun inside the call that have not ended.
    runs: usize,
}

/// The `[CommitBehavior(...)]` in force for the dynamic extent of a procedure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CommitBehavior {
    #[default]
    Allow,
    Ignore,
    Error,
}

impl TxModel {
    /// The model named by a `[TransactionModel(TransactionModel::X)]` attribute's source text.
    pub fn from_attribute(text: &str) -> Option<TxModel> {
        let t = text.to_ascii_lowercase();
        let (_, value) = t.split_once("::")?;
        match value.trim_end_matches([')', ']', ' ']) {
            "autocommit" => Some(TxModel::AutoCommit),
            "autorollback" => Some(TxModel::AutoRollback),
            "none" => Some(TxModel::None),
            _ => None,
        }
    }
}

impl CommitBehavior {
    /// The behavior inside a procedure that declares `inner`, called where `self` is in force: an outer `Error` stays,
    /// an inner `Error` wins over an outer `Ignore` (errors/CuCommitTests.al:217-245,
    /// Nested_ErrorOuter_IgnoreInner and Nested_IgnoreOuter_ErrorInner).
    pub(crate) fn nested(self, inner: CommitBehavior) -> CommitBehavior {
        if self == CommitBehavior::Error {
            CommitBehavior::Error
        } else {
            inner
        }
    }

    /// The behaviour named by a `[CommitBehavior(CommitBehavior::X)]` attribute's source text.
    pub(crate) fn from_attribute(text: &str) -> Option<CommitBehavior> {
        let t = text.to_ascii_lowercase();
        let (_, value) = t.split_once("::")?;
        match value.trim_end_matches([')', ']', ' ']) {
            "ignore" => Some(CommitBehavior::Ignore),
            "error" => Some(CommitBehavior::Error),
            _ => None,
        }
    }
}

/// Refusal of a guarded `Codeunit.Run` while the caller holds an uncommitted write: the full text
/// and the code `DB:Ok` are pinned by codeunits/TestCodeunitRunErrorTextsAndModels.al:75-90.
/// The same text refuses a guarded XmlPort
/// import and a report run with a write pending.
pub(crate) const TRANSACTION_STOPPED: &str =
    "An error occurred and the transaction is stopped. Contact your administrator or partner for further assistance.";
const TRANSACTION_STOPPED_CODE: &str = "DB:Ok";

/// The refusal of a run that needs a transaction of its own while the caller holds a write.
pub(crate) fn transaction_stopped() -> Signal {
    al_error_code(TRANSACTION_STOPPED, TRANSACTION_STOPPED_CODE)
}

/// record/TestTransactionModelAutoRollback.al:ExplicitCommitNotAllowedErr.
const COMMIT_UNDER_AUTO_ROLLBACK: &str =
    "Tests cannot call the Commit function if TransactionModel property is set to AutoRollback.";

/// errors/TestCommitBehaviorErrorText.al:34-35: the full text, with code `DB:Ok`.
const COMMIT_PROHIBITED: &str =
    "Commit is prohibited in the current scope. The operation cannot continue. Contact your system administrator.";

/// codeunits/TestWriteTxTestBoundary.al:WriteTxBoundary_Test09_NoneTestCannotWriteDirectly (Insert);
/// codeunits/TestWriteTxNoneModelContracts.al:59-143 pins the same text and the code `DB:BWTMissing`
/// for Modify, Delete, DeleteAll, ModifyAll, Rename and Truncate.
pub(crate) const NO_TRANSACTION: &str = "A transaction must be started before changes can be made to the database.";
const NO_TRANSACTION_CODE: &str = "DB:BWTMissing";

/// codeunits/TestCodeunitRunErrorTextsAndModels.al:48-52 and :64-91: the two sentences joined by
/// one space, with the code `TestCodeUnitNestedInvocation`.
fn nested_test_codeunit(id: u32, name: &str) -> Signal {
    al_error_code(
        &format!(
            "You cannot nest the execution of test codeunits. Test codeunit {id} {name} was called from another test codeunit."
        ),
        "TestCodeUnitNestedInvocation",
    )
}

pub(crate) fn al_error(message: &str) -> Signal {
    al_error_code(message, "")
}

/// `Session::tx_phase`: nothing of the transaction has run yet.
pub(crate) const TX_FRESH: u8 = 0;
/// `Session::tx_phase`: a read call has started the transaction.
pub(crate) const TX_READ: u8 = 1;
/// `Session::tx_phase`: a write call has started the transaction.
pub(crate) const TX_WRITE: u8 = 2;

pub(crate) fn al_error_code(message: &str, code: &str) -> Signal {
    Signal::Error(Box::new(AlError {
        message: message.to_string(),
        code: code.to_string(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// The emit version BC reports in a missing-object error. Only the measured builds are known
/// (BC 27.5 is 27024 and BC 28.1 is 28014, read off the service tier by AL Runner v2:
/// tests/runner-extras/object-metadata-system-table/OmstTests.Codeunit.al); any other build gets
/// `<major>000`, which keeps the shape the corpus asserts (a number).
fn emit_version(major: u16) -> u32 {
    match major {
        27 => 27024,
        28 => 28014,
        m => u32::from(m) * 1000,
    }
}

impl Session {
    /// The error BC raises when code names an object that no app declares
    /// (codeunits/TestCodeunitRunErrorTextsAndModels.al:162-196 for `CodeUnit`,
    /// recordref-fieldref/recordref/TestRecordRefRuntimeContracts.al:30 for `Table`). The text names the outermost running object (the test codeunit when a
    /// helper issues the call).
    pub(crate) fn missing_object(&self, kind: &str, id: i64) -> Signal {
        // The outermost running object is named, not the helper that issued the call: a test that calls a helper
        // codeunit that runs a missing codeunit reads "from the object <the test codeunit>" (errors/W5MissingObject.al:
        // MissingCodeunit_FromHelperCodeunit, BC 27.5 and 29.0).
        // Under a test runner codeunit (ADR-066) its frames are below the test codeunit's and do not count.
        let caller = self
            .frames
            .get(self.test_frame_base())
            .map(|&o| self.engine.program.objects[o].name.as_str())
            .unwrap_or_default();
        // A page is opened by the client side of the test runner, which is what the error names
        // (errors/last-error/P70_06RunModalField.al:RunModal_UnknownPageId: Page.RunModal and Page.Run of
        // an unknown id, and of id 0).
        // Inside a try function the page error names the outermost object as the other kinds do (platform/
        // lt-xmlport-session-navapp-cookie/P94PageRun.al:211 Run_UnknownPage, errors/W5MissingObject.al:52 `run_missing_page`).
        // A codeunit the test method runs itself, outside a try function, names the test runner too
        // (errors/error-texts/AzErrorTexts.al:110-134 CodeunitRun_MissingId_RawText). Deeper calls from the test codeunit
        // keep the outermost object: only a helper codeunit is pinned (MissingCodeunit_FromHelperCodeunit above).
        // A report the test method runs itself names the test runner as well (reports/request-page/
        // P91Run.al:117-165, Report.Run, RunModal and Execute).
        let caller = if (kind == "Page" && self.tx.try_depth == 0)
            || (matches!(kind, "CodeUnit" | "Report") && self.in_test_method_itself())
        {
            "Test Runner - Mgt"
        } else {
            caller
        };
        self.missing_object_from(kind, id, caller)
    }

    /// Whether the running code is the test method itself, outside a try function: the call stack holds only a
    /// test codeunit.
    pub(crate) fn in_test_method_itself(&self) -> bool {
        self.tx.try_depth == 0
            && self.frames.len() == self.test_frame_base() + 1
            && self.frames.last().is_some_and(|&o| {
                self.engine.program.objects[o]
                    .properties
                    .iter()
                    .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Test"))
            })
    }

    /// The missing-object error with the object it names as the caller.
    pub(crate) fn missing_object_from(&self, kind: &str, id: i64, caller: &str) -> Signal {
        if let Some(why) = self.missing_may_be_skipped(kind, id) {
            return Signal::unsupported(why);
        }
        let major = self.engine.target.as_ref().map_or(29, |t| t.bc.major);
        al_error_code(
            &format!(
                "You tried to invoke the {kind} object with the ID {id} from the object {caller}. \
                 An object with that ID does not exist in the current application compiled with emit version {}.",
                emit_version(major)
            ),
            "AL:MetadataNotFound",
        )
    }

    /// Why an object of `kind` and `id` that no loaded app declares may still exist on BC: the run skipped a dependency
    /// app, and the object may be one of its own (al-runner issue 5749). `None`: no app was skipped, or the id is none an
    /// app can declare (0, or the platform's range from 2 000 000 000).
    pub(crate) fn missing_may_be_skipped(&self, kind: &str, id: i64) -> Option<String> {
        let skipped = &self.engine.skipped_apps;
        ((1..2_000_000_000).contains(&id) && !skipped.is_empty()).then(|| {
            format!(
                "{kind} object {id} is in no app that was loaded, and the run skipped the dependency {} (no package with its AL source was found): the object may be one of its own, which BC would find",
                skipped.join(", ")
            )
        })
    }

    /// Whether the session holds an open write transaction
    /// (codeunits/TestWriteTxTestBoundary.al:Test02/Test03, `Database.IsInWriteTransaction`).
    ///
    /// A page background worker runs in a session of its own, outside the caller's write
    /// transaction, and cannot write to a table (`bg_refuse_write`): its transaction never holds
    /// a change, whatever the caller has pending (testpage/TestPageBgTaskTx_Tests.al:
    /// RunPageBackgroundTask_WorkerIsOutsideCallersWriteTransaction).
    ///
    /// A lock starts it too (`TxState::lock_tx`, codeunits/TestCodeunitRunWriteTransaction.al:211-278); after a lock whose
    /// effect is not pinned (`TxState::lock_unpinned`) the answer is refused unless a write or a pinned lock opened it.
    pub(crate) fn in_write_transaction(&self) -> Result<bool, Signal> {
        // A write call starts the transaction even when it changes nothing (record/TestProbeRecordRefContracts.al:
        // WriteTransaction_State: a DeleteAll or ModifyAll of no row).
        if self.pages.bg_worker.is_some() {
            return Ok(false);
        }
        if self.store.has_uncommitted() || self.tx.tx_phase == TX_WRITE || self.tx.lock_tx || self.tx.recref_lock_tx {
            return Ok(true);
        }
        // Only reads under a locking isolation set in an earlier transaction whose effect is pinned: no write transaction
        // (format-evaluate/Np4Misc.al Y9_*_SetBeforeCommit_FirstRead, corpus PR 896).
        let unpinned = match self.tx.lock_unpinned {
            Some(crate::records::UNPINNED_LOCKING_READ) if !self.tx.locking_read_tx_unpinned => None,
            other => other,
        };
        match unpinned {
            Some(what) => Err(Signal::unsupported(format!(
                "whether {what} starts a write transaction (not pinned)"
            ))),
            // A ReadUncommitted read opens none either (record/Y70Probes.al:839-866).
            None => Ok(false),
        }
    }

    /// Whether the running test method has no transaction: `TransactionModel::None` outside
    /// anything that begins a transaction of its own (a `Codeunit.Run`, a page).
    pub(crate) fn no_transaction(&self) -> bool {
        self.tx_model == TxModel::None && self.tx.tx_world == 0
    }

    /// `Commit()`. The next transaction's type is Update until a set changes it
    /// (session-system/ProbeSystemEnumMembers.al:94-112, TestClearAllTransactionTypeContracts.al:245-253). A `[CommitBehavior(Ignore)]` scope turns it into a no-op, and exempts it from
    /// the `AutoRollback` refusal (record/TestTransactionModelAutoRollback.al:Test07); the
    /// `AutoRollback` refusal outranks `[CommitBehavior(Error)]` (:Test08).
    pub(crate) fn commit_builtin(&mut self) -> Result<(), Signal> {
        if let Some(tally) = self.pages.action_tally.as_mut() {
            tally.note_commit();
        }
        self.bg_refuse_transaction_call("Commit")?;
        if self.tx.commit_behavior == CommitBehavior::Ignore {
            return Ok(());
        }
        if self.tx_model == TxModel::AutoRollback {
            return Err(al_error(COMMIT_UNDER_AUTO_ROLLBACK));
        }
        if self.tx.commit_behavior == CommitBehavior::Error {
            return Err(al_error_code(COMMIT_PROHIBITED, "DB:Ok"));
        }
        // Inside a TryFunction whose result is used, after the `[CommitBehavior]` checks (misc/lt-cu-odd-leads/
        // CuCommitTests.al:203-264 TryFunction_WithCommitBehavior, :361-381 Commit_InTryFunction).
        self.gate_used_try_write("COMMIT")?;
        // A table `Consistent(false)` marked fails the Commit (errors/CuCommitTests.al:405-419
        // Consistent_False_ThenCommit); `Consistent(true)` lifts the mark (:422-433).
        if let Some(&table) = self.tx.inconsistent.first() {
            return Err(self.inconsistent_error(table as usize));
        }
        // A deferred duplicate-key Insert comes out here (corpus record/TestRecordRuntimeTextsAndViews.al:
        // Record_Insert_DuplicateKey_WithoutBlobField_IsDeferred_...: "raised by Commit").
        if let Some((_, e)) = self.tx.deferred_insert.take() {
            return Err(Signal::Error(Box::new(e)));
        }
        self.commit_transaction();
        // A Commit puts LockTimeout back on (session-system/PnBRecordMisc.al:LastUsedRowVersion_AndLockTimeout
        // `lockTimeoutAfterCommit`). The timeout duration is not pinned.
        self.tx.lock_timeout = true;
        // The next transaction has not started and its type is Update (see `tx_phase`).
        self.tx.tx_phase = TX_FRESH;
        self.tx.transaction_type = 1;
        Ok(())
    }

    /// The error of a Commit while `Consistent(false)` marks `table` (errors/CuCommitTests.al:405-419). The
    /// table is named in the text; only a table whose caption is its name is pinned.
    fn inconsistent_error(&self, table: usize) -> Signal {
        let t = &self.engine.program.objects[table];
        let caption = t
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("Caption"))
            .map(|(_, v)| v.trim().trim_matches('\'').to_string());
        if caption.is_some_and(|c| c != t.name) {
            return Signal::unsupported(format!(
                "Commit after Consistent(false) on table {}, which has a caption other than its name (which one the error names is not pinned)",
                t.name
            ));
        }
        al_error_code(
            &format!(
                "The transaction cannot be completed because it will cause inconsistencies in the {} table. Check where and how \
                 the CONSISTENT function is used in the transaction to find the reason for the error.\\\\Contact your system \
                 administrator.\\\\Tables can be marked as inconsistent during comprehensive tasks, such as posting. This \
                 prevents data from being updated incorrectly.",
                t.name
            ),
            "DB:TableInConsistent",
        )
    }

    /// The name of a table `Consistent(false)` marks in the open transaction, for the test runner's commit at the end of a
    /// test (not pinned: refused there).
    /// The table of a duplicate-key `Insert` whose error is still deferred: the next access to the table or a `Commit`
    /// raises it (record/TestRecordRuntimeTextsAndViews.al:180-209). `None` when there is none.
    pub fn deferred_insert_table(&self) -> Option<String> {
        self.tx
            .deferred_insert
            .as_ref()
            .map(|(t, _)| self.engine.program.objects[*t].name.clone())
    }

    /// Takes the error of a duplicate-key `Insert` that is still deferred, for the test runner at the end of a test.
    pub fn take_deferred_insert_error(&mut self) -> Option<AlError> {
        self.tx.deferred_insert.take().map(|(_, e)| e)
    }

    pub fn inconsistent_table(&self) -> Option<String> {
        self.tx
            .inconsistent
            .first()
            .map(|&t| self.engine.program.objects[t as usize].name.clone())
    }

    /// A transaction that ends other than by `Commit()` while `Consistent(false)` marks a table: what BC does is not
    /// pinned (only `Commit()` is, errors/CuCommitTests.al:405-419), so it is refused.
    pub(crate) fn refuse_inconsistent_end(&self, how: &str) -> Result<(), Signal> {
        match self.tx.inconsistent.first() {
            Some(&t) => Err(Signal::unsupported(format!(
                "{how} ends a transaction while Consistent(false) marks table {} (only Commit() is pinned)",
                self.engine.program.objects[t as usize].name
            ))),
            None => Ok(()),
        }
    }

    /// `Record.Consistent(Value)` on a database table marks or unmarks it for the open transaction
    /// (errors/CuCommitTests.al:382-433). A temporary record is not pinned.
    pub(crate) fn set_consistent(&mut self, table: u32, temporary: bool, consistent: bool) -> Result<(), Signal> {
        if temporary {
            return Err(Signal::Unsupported(
                "Consistent on a temporary record (not pinned)".into(),
            ));
        }
        self.tx.inconsistent.retain(|&t| t != table);
        if !consistent {
            self.tx.inconsistent.push(table);
        }
        Ok(())
    }

    /// A TryFunction call whose result is used begins: `consumer` is the frame count of the caller that uses the
    /// result. What the TryFunctionWrite error names is the caller of that procedure: the test method when a helper
    /// of it uses the result (errors/CuTryTests.al:105-121 TryInsertThenError_IfNotForm), the test
    /// runner's root scope when the test method uses it itself (:236-238 and corpus commit ff42e3d3: 'RunTests' under the
    /// classic runner on BC 27.5, 'RootMethodScope' under the TestRunnerHub runner on 28.5/29.0). Any other caller is not pinned:
    /// the error is raised with an unpinned text.
    pub(crate) fn enter_used_try(&mut self) {
        let base = self.test_frame_base();
        let scope = match self.call_stack.len() - base {
            // The runner's name per major (al_version::test_runner_root_scope, corpus commit ff42e3d3).
            0 | 1 => al_version::test_runner_root_scope(self.engine.target.as_ref()).map(str::to_string),
            // The OnRun of a guarded `Codeunit.Run` uses the result: what is named is the routine that called the run, when a
            // helper of the test method did (errors/Y68Small.al:191-201, 221-225 'RunMode'). The test method
            // calling the run itself is not pinned.
            n if self.tx.guarded_runs.last() == Some(&(base + n - 1)) => {
                let (o, r) = (self.frames[base], self.call_stack[base].routine);
                (n == 3 && self.is_test_method(o, r)).then(|| {
                    let (o1, r1) = (self.frames[base + 1], self.call_stack[base + 1].routine);
                    self.engine.program.objects[o1].routines[r1].name.clone()
                })
            }
            2 => {
                let (o, r) = (self.frames[base], self.call_stack[base].routine);
                self.is_test_method(o, r)
                    .then(|| self.engine.program.objects[o].routines[r].name.clone())
            }
            _ => None,
        };
        self.tx.used_tries.push(UsedTry {
            scope,
            world: self.tx.tx_world,
            runs: 0,
        });
    }

    /// Whether routine `r` of object `o` is a `[Test]` method of a test codeunit.
    fn is_test_method(&self, o: usize, r: usize) -> bool {
        let obj = &self.engine.program.objects[o];
        obj.properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Test"))
            && obj.routines[r]
                .attributes
                .iter()
                .any(|a| a.name.eq_ignore_ascii_case("Test"))
    }

    pub(crate) fn leave_used_try(&mut self) {
        self.tx.used_tries.pop();
    }

    /// A write `function` (as BC names it, `INSERT`) inside a TryFunction whose result is used fails with
    /// TryFunctionWrite (errors/CuTryTests.al:83-150, CuCommitTests.al:361-381); the try does not trap
    /// it (`finish_call`). Inside a `Codeunit.Run` that began in the try it is not pinned and refused.
    pub(crate) fn gate_used_try_write(&self, function: &str) -> Result<(), Signal> {
        let Some(t) = self.tx.used_tries.last() else {
            return Ok(());
        };
        if let Some(r) = self.allowed_try_write() {
            return r;
        }
        // Install code has no test runner around it: a Commit in a TryFunction there commits and the code goes on
        // (metadata/CmInstallLeads.al:58-59, CmInstallObjects.al:69-70). Other writes there are not pinned.
        if self.install_without_client {
            if function == "COMMIT" {
                return Ok(());
            }
            return Err(Signal::unsupported(format!(
                "{function} inside a TryFunction whose result is used, in install code (not pinned)"
            )));
        }
        if self.tx.xp_export_try_world == Some(self.tx.tx_world) && function != "INSERT" && function != "MODIFY" {
            return Err(Signal::unsupported(format!(
                "{function} in an XmlPort export started inside a TryFunction whose result is used (only Insert and Modify are pinned)"
            )));
        }
        if let Some(r) = self.used_try_nested_world()? {
            return r;
        }
        if self.tx_model == TxModel::AutoRollback {
            return Err(Signal::unsupported(format!(
                "{function} inside a TryFunction whose result is used, under TransactionModel::AutoRollback (not pinned)"
            )));
        }
        Err(match &t.scope {
            Some(scope) => Signal::Error(Box::new(AlError {
                message: format!(
                    "Call to the function '{function}' is not allowed inside the call to '{scope}' when it is used as a TryFunction."
                ),
                code: TRY_FUNCTION_WRITE.into(),
                ..Default::default()
            })),
            None => Signal::Error(Box::new(AlError {
                code: TRY_FUNCTION_WRITE.into(),
                ..al_builtins::native::unpinned_error_for("TryFunctionWrite scope name")
            })),
        })
    }

    /// An Insert or a Modify of a database table inside a TryFunction whose result is used. Whether the check comes before the
    /// table's own code is not pinned: subscribers and table-extension triggers that would see the order are refused. The
    /// table's own trigger is kept: the pinned table has one (errors/CuFixtures.al:20-24), and refusing it would
    /// lose 85310 TryInsertThenError_ExpressionForm, TryInsertThenError_IfNotForm and TryInsertOnly_ExpressionForm. Insert is
    /// named INSERT (CuTryTests.al:83-150), Modify MODIFY, also for a row that does not exist
    /// (errors/Y68Small.al:263-267, TryModifyOnly).
    pub(crate) fn gate_used_try_row_write(
        &mut self,
        t: &al_data::TableDef,
        method: al_builtins::record_op::RecordOp,
    ) -> Result<(), Signal> {
        // NavApp.LoadPackageData inside a TryFunction whose result is used loads its rows and the try returns true (corpus
        // install-upgrade/install-triggers/Y88PackageData.al: every load of codeunit 94682 runs in TryLoad, and the log says
        // `ok` with the rows there, Y88_Events_LoadRaisesNoTableEvent, Y88_Option_UndefinedOrdinal).
        if self.package_load {
            return Ok(());
        }
        let (function, what, events, triggers) = match method {
            al_builtins::record_op::RecordOp::Modify => (
                "MODIFY",
                "Modify",
                ["onbeforemodifyevent", "onaftermodifyevent"],
                ["OnBeforeModify", "OnModify", "OnAfterModify"],
            ),
            _ => (
                "INSERT",
                "Insert",
                ["onbeforeinsertevent", "onafterinsertevent"],
                ["OnBeforeInsert", "OnInsert", "OnAfterInsert"],
            ),
        };
        if self.used_try_write_applies()
            && !self.engine.write_in_try_allowed
            && (self.listened(t, events[0], None)?
                || self.listened(t, events[1], None)?
                || triggers.iter().any(|n| !self.ext_triggers(t, n).is_empty()))
        {
            return Err(Signal::unsupported(format!(
                "{what} inside a TryFunction whose result is used, on a table with {} event subscribers or table-extension {} triggers (whether they run before the TryFunctionWrite error is not pinned)",
                what.to_lowercase(),
                what.to_lowercase()
            )));
        }
        self.gate_used_try_write(function)
    }

    /// With DisableWriteInsideTryFunctions off (`Engine::write_in_try_allowed`, al-runner issue 7143, Business Central online)
    /// a write of a TryFunction whose result is used runs like any write: Insert, Modify, Delete, DeleteAll, ModifyAll, Rename
    /// and Commit change the rows, the TryFunction returns true, and an error after the write makes it return false without
    /// undoing the write (corpus tests/al-language-trywrite-allowed/TryWriteAllowed.al:44-144, measured on 27.5, 28.5 and 29.0
    /// in the corpus CI job "BC TryWrites"). The table's triggers run as for any write (the table there, LT CU Row, has OnInsert
    /// and OnModify: errors/CuFixtures.al:20-30). The same holds in install code: OnInstallAppPerDatabase writing DataPerCompany =
    /// false tables, also after NavApp.GetResource and Media.ImportStream (trywrite-allowed/TryWriteInstall.al:52-372, al-runner issue
    /// 7348). `None` in the default mode. Still refused: a write inside a `Codeunit.Run` the try started (`used_try_nested_world`).
    fn allowed_try_write(&self) -> Option<Result<(), Signal>> {
        if !self.engine.write_in_try_allowed {
            return None;
        }
        if let Err(e) = self.used_try_nested_world() {
            return Some(Err(e));
        }
        Some(Ok(()))
    }

    /// Delete, DeleteAll, ModifyAll and Rename of a database table inside a TryFunction whose result is used raise
    /// TryFunctionWrite named DELETE, DELETEALL, MODIFYALL and RENAME, before any row changes; DeleteAll also on an empty
    /// table (errors/lt-r7143-try-write/R7143TryWriteDeny.al:94-176, corpus PR 1156, measured on 27.5, 28.5 and 29.0;
    /// al-runner issue 7143). Whether the check comes before
    /// the table's own triggers and the subscribers is not pinned: a table that has any is refused. Other writes keep
    /// `refuse_used_try_write`.
    pub(crate) fn gate_used_try_other_write(&mut self, t: &al_data::TableDef, method: RecordOp) -> Result<(), Signal> {
        let (function, events, triggers): (&str, &[&str], &[&str]) = match method {
            RecordOp::Delete | RecordOp::DeleteAll => (
                if method == RecordOp::Delete {
                    "DELETE"
                } else {
                    "DELETEALL"
                },
                &["onbeforedeleteevent", "onafterdeleteevent"],
                &["OnBeforeDelete", "OnDelete", "OnAfterDelete"],
            ),
            RecordOp::ModifyAll => (
                "MODIFYALL",
                &["onbeforemodifyevent", "onaftermodifyevent"],
                &["OnBeforeModify", "OnModify", "OnAfterModify"],
            ),
            RecordOp::Rename => (
                "RENAME",
                &["onbeforerenameevent", "onafterrenameevent"],
                &["OnBeforeRename", "OnRename", "OnAfterRename"],
            ),
            _ => return self.refuse_used_try_write(method),
        };
        if !self.used_try_write_applies()
            || self.tx.xp_export_try_world == Some(self.tx.tx_world)
            || self.engine.write_in_try_allowed
        {
            return self.refuse_used_try_write(method);
        }
        let mut code = false;
        for e in events {
            code |= self.listened(t, e, None)?;
        }
        code |= triggers
            .iter()
            .any(|n| self.table_trigger(t, n).is_some() || !self.ext_triggers(t, n).is_empty());
        if code {
            return Err(Signal::unsupported(format!(
                "{} inside a TryFunction whose result is used, on a table with triggers or event subscribers for it (whether they run before the TryFunctionWrite error is not pinned)",
                method.name()
            )));
        }
        self.gate_used_try_write(function)
    }

    /// Whether a write here is a write of a TryFunction whose result is used (`gate_used_try_write` would raise).
    pub(crate) fn used_try_write_applies(&self) -> bool {
        self.tx
            .used_tries
            .last()
            .is_some_and(|t| t.runs == 0 && t.world == self.tx.tx_world)
    }

    /// Writes other than Insert inside a TryFunction whose result is used are not pinned (docs/probes-needed.md).
    pub(crate) fn refuse_used_try_write(&self, method: RecordOp) -> Result<(), Signal> {
        // Truncate has a refusal of its own inside any try function (`autoinc.rs`).
        if self.tx.used_tries.is_empty() || method == RecordOp::Truncate {
            return Ok(());
        }
        if let Some(r) = self.allowed_try_write() {
            return r;
        }
        if self.tx.xp_export_try_world == Some(self.tx.tx_world) && method != RecordOp::Modify {
            return Err(Signal::unsupported(format!(
                "{} in an XmlPort export started inside a TryFunction whose result is used (only Insert and Modify are pinned)",
                method.name()
            )));
        }
        if let Some(r) = self.used_try_nested_world()? {
            return r;
        }
        Err(Signal::unsupported(format!(
            "{} of a database table inside a TryFunction whose result is used (only Insert, Modify and Commit are pinned)",
            method.name()
        )))
    }

    /// A write inside an XmlPort import or a report the try function started (their own transaction world) is not a
    /// write of the try: BC lets it happen (xmlports/TestXmlPortObject.al:234-251 TryImportUniversalXmlPort and the
    /// tests that use it, record/P136Report.al:165-180 TryRunOne and TryRunTwo). Inside a `Codeunit.Run` the try started it
    /// is not pinned and refused. `None` when the write is the try's own.
    fn used_try_nested_world(&self) -> Result<Option<Result<(), Signal>>, Signal> {
        let Some(t) = self.tx.used_tries.last() else {
            return Ok(Some(Ok(())));
        };
        if t.runs > 0 {
            return Err(Signal::Unsupported(
                "a write inside a Codeunit.Run inside a TryFunction whose result is used (not pinned)".into(),
            ));
        }
        Ok((self.tx.tx_world != t.world).then_some(Ok(())))
    }

    /// A TestPage operation inside a TryFunction whose result is used is a call of the page client, not of the try:
    /// the writes its triggers make are allowed (pages/extensions/ZPageExtProps.al:663-690
    /// ZDeleteAllowed_1, testpage/CfPageLeads.al:373-390 FlowFieldRelation_OpenNew, baseline cases).
    pub(crate) fn outside_used_tries<T>(
        &mut self,
        body: impl FnOnce(&mut Session) -> Result<T, Signal>,
    ) -> Result<T, Signal> {
        if self.tx.used_tries.is_empty() {
            return body(self);
        }
        let saved = std::mem::take(&mut self.tx.used_tries);
        let r = body(self);
        self.tx.used_tries = saved;
        r
    }

    /// `Codeunit.Run(id)` in its statement form, as the test runner's own AL code calls it (an error is not trapped).
    pub fn run_codeunit_by_id(&mut self, id: u32) -> Result<(), Signal> {
        let n = i32::try_from(id).map_err(|_| Signal::unsupported(format!("codeunit id {id} out of range")))?;
        self.codeunit_run(ids::CODEUNIT_RUN_3, None, &[Value::Integer(n)], false)
            .map(|_| ())
    }

    /// `Codeunit.Run`: `Codeunit.Run#1` is the instance form, `#3` the static form by id. The
    /// call is guarded when its result is used (`ret_used`).
    pub(crate) fn codeunit_run(
        &mut self,
        id: al_ir::BuiltinId,
        receiver: Option<Value>,
        args: &[Value],
        guarded: bool,
    ) -> Result<Value, Signal> {
        if let Some(tally) = self.pages.action_tally.as_mut() {
            tally.note_codeunit_run();
        }
        // Only the guarded form is pinned inside a page background worker
        // (testpage/TestPageBgTaskTx_Tests.al:RunPageBackgroundTask_WorkerIsOutsideCallersWriteTransaction).
        if !guarded {
            self.bg_refuse_transaction_call("Codeunit.Run")?;
        }
        let (inst, record_arg) = match (id, receiver) {
            (ids::CODEUNIT_RUN_1, Some(Value::Codeunit(rc))) => (self.shared_instance(rc), args.first()),
            (ids::CODEUNIT_RUN_3, None) => {
                let Some(Value::Integer(n)) = args.first() else {
                    return unsupported("Codeunit.Run with an id that is not an integer");
                };
                let id = u32::try_from(*n).ok();
                let Some(object) = self.engine.compiled_codeunit(id) else {
                    // A codeunit of a dependency app that nothing reached is not compiled (ADR-013);
                    // BC has it, so this is not the missing-object error. (With no compiled codeunit of the id, the
                    // first codeunit object of the id is a shell.)
                    if self.engine.codeunit_object(id).is_some() {
                        return unsupported(format!(
                            "Codeunit.Run of codeunit {n}, which belongs to a dependency app and was not compiled (no code the workspace reaches uses it)"
                        ));
                    }
                    // The object is resolved before the run begins, so the guarded form raises
                    // too (codeunits/TestCodeunitRunMissing.al). Text and code:
                    // codeunits/TestCodeunitRunErrorTextsAndModels.al:162-196.
                    return Err(self.missing_object("CodeUnit", i64::from(*n)));
                };
                let fresh = self.instance(object);
                (self.shared_instance(fresh), args.get(1))
            }
            (other, _) => return unsupported(format!("builtin {other} with this receiver")),
        };
        let object = inst.borrow().object as usize;
        // A codeunit with no OnRun still runs (and its properties count): `--affected` notes it as entered.
        self.note_entered(object);
        let engine = self.engine.clone();
        let obj = &engine.program.objects[object];
        let has_property = |name: &str| obj.properties.iter().any(|(k, _)| k.eq_ignore_ascii_case(name));

        // The nested-run refusal comes from the platform before the run begins, so the guarded
        // form does not trap it (codeunits/TestCodeunitRunNestedTestCodeunit.al:
        // CodeunitRun_TestCodeunitFromTest_Guarded_StillThrows, ...VariableFromTest_Throws).
        let is_test = obj
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Test"));
        if is_test && self.in_test_codeunit {
            return Err(nested_test_codeunit(obj.id.unwrap_or(0), &obj.name));
        }
        // ADR-066 decision 1: from a test runner codeunit's OnRun, the run of a test codeunit is the test runner's
        // (al-testrun), when one is installed.
        if is_test {
            if let Some(result) = self.run_for_runner(object, guarded) {
                return result;
            }
        }
        // Codeunit.Run of a codeunit with Subtype = Upgrade fails before it runs, with or without upgrade triggers
        // (events/platform-triggers/PUpgradeTriggers.al:PUpgrade_RunWithTriggers line 121, PUpgrade_RunWithoutTriggers line 134; both by id).
        if obj
            .properties
            .iter()
            .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("Upgrade"))
        {
            return Err(al_error_code(
                &format!(
                    "Upgrade codeunit '{}' cannot be invoked outside the schema synchronization process.",
                    obj.id.unwrap_or(0)
                ),
                "UpgradeCodeunitOnRunCall",
            ));
        }
        // A record passed to a codeunit without TableNo is not touched
        // (codeunits/TestCodeunitRunContracts.al:Codeunit_Run_WithRecord_PassesRecord).
        //
        // A codeunit with TableNo reads the record as its Rec
        // (testpage/TestPageActionRunObjectKinds_Tests.al:ProbeControlEachTargetRecordsItsOwnExecution,
        // :143-150, reads Rec.Descr in OnRun). The record is passed by reference: a change OnRun
        // makes to Rec reaches the caller's variable, in the guarded and the statement form
        // (codeunits/TestCodeunitRunErrorTextsAndModels.al:198-220). What Rec holds when no record is passed is not pinned and is refused
        // before the run (docs/probes-needed.md).
        let mut record_ref_row = None;
        let mut caller_rec = None;
        let mut body_rec = None;
        if has_property("TableNo") {
            let Some(slot) = obj.rec_slot else {
                return unsupported(format!(
                    "Codeunit.Run of {} (its TableNo table is outside the workspace)",
                    obj.name
                ));
            };
            // A Variant that holds an open RecordRef: Rec is the RecordRef's row with its filters, and a change OnRun makes reaches
            // the RecordRef (recordref-fieldref/recordref/RecordRefVariantToRecord.al, QP_CodeunitRun_* tests; it
            // compiles, a RecordRef itself does not: must-not-compile 1786/1787). Microsoft's Batch Processing Mgt. passes one.
            // Anything else a Variant holds (an unopened RecordRef included) is not pinned.
            if let Some(v) = record_arg {
                if !matches!(v, Value::Record(_)) {
                    let rr = match v {
                        Value::Library(_, l) => match &*l.borrow() {
                            al_builtins::value::LibraryValue::RecordRef(rs) => Some(rs.clone()),
                            _ => None,
                        },
                        _ => None,
                    };
                    match rr {
                        Some(rs) => record_ref_row = Some(Value::Record(rs)),
                        None => {
                            let what = match v {
                                Value::Library(n, _) if n.eq_ignore_ascii_case("RecordRef") => {
                                    "a RecordRef that is not open".to_string()
                                }
                                other => format!("a {}", crate::value_kind(other)),
                            };
                            return unsupported(format!(
                                "Codeunit.Run of {} (TableNo) with a Variant that holds {what} (what Rec holds is not pinned)",
                                obj.name
                            ));
                        }
                    }
                }
            }
            let record_arg = record_ref_row.as_ref().or(record_arg);
            // Without a record Rec is a blank record; a temporary one is handed over by reference like any other
            // (session-system/TestProbeSessionMiscContracts.al:CodeunitRun_TableNo_NoRecord, CodeunitRun_TableNo_TemporaryRecord).
            self.ensure_initialized(&inst);
            // A record of another table fails before the run, and the guarded form does not trap it
            // (session-system/isolated-storage/CuWriteTxTests.al:203-233, codeunits/TestCodeunitRunErrorTextsAndModels.al:273-297).
            if let (Some(Value::Record(rc)), Value::Record(own)) = (record_arg, &inst.borrow().globals[slot]) {
                let (given, wanted) = (rc.borrow().table as usize, own.borrow().table as usize);
                if given != wanted {
                    let id = |o: usize| engine.program.objects[o].id.unwrap_or(0);
                    return Err(al_error_code(
                        &format!(
                            "Record({}) is not compatible with Codeunit.Run(Record({})).",
                            id(given),
                            id(wanted)
                        ),
                        "InvalidType",
                    ));
                }
            }
            if let Some(Value::Record(rc)) = record_arg {
                inst.borrow_mut().globals[slot] = Value::Record(rc.clone());
            }
            if guarded {
                caller_rec = crate::run_restore::caller_record(record_arg);
                body_rec = record_ref_row.is_none().then(|| record_arg.cloned()).flatten();
            }
        }

        if guarded {
            // The guarded form needs a transaction world of its own, which BC refuses to open
            // while the caller holds an uncommitted write
            // (codeunits/TestCodeunitRunWriteTransaction.al:GuardedRun_WithUncommittedWrite_IsRefused_AndCodeunitNeverRuns).
            // (in_write_transaction is false in a page background worker.)
            if self.in_write_transaction()? {
                // The same refusal under AutoRollback, and inside a TryFunction, which traps it
                // and returns false (codeunits/TestCodeunitRunErrorTextsAndModels.al:93-160 and
                // :248-262).
                return Err(transaction_stopped());
            }
        }

        let on_run = obj
            .routines
            .iter()
            .position(|r| r.is_trigger && r.name.eq_ignore_ascii_case("OnRun"));
        // A codeunit without an OnRun runs as a no-op and succeeds
        // (types/TestLanguageEdgeCases.al:CodeunitRun_Success_ReturnsTrue,
        // codeunits/TestCodeunitRunContracts.al:Codeunit_Run_SetsErrorTextOnFailure).
        let Some(on_run) = on_run else {
            return Ok(if guarded { Value::Boolean(true) } else { Value::Null });
        };

        // Codeunit.Run begins a transaction of its own, so a write is allowed inside it even
        // under TransactionModel::None, and the transaction ends with the run
        // (codeunits/TestWriteTxTestBoundary.al:Test10_ARunCodeunitMayWriteUnderNone).
        let began_world = self.tx_model == TxModel::None && self.tx.tx_world == 0;
        self.tx.tx_world += 1;
        if let Some(t) = self.tx.used_tries.last_mut() {
            t.runs += 1;
        }
        if guarded {
            self.tx.guarded_runs.push(self.call_stack.len());
        }
        self.guarded_body_begin(guarded, id == ids::CODEUNIT_RUN_3, body_rec.as_ref());
        // A test runner codeunit's OnRun under an installed host: its test codeunit runs go to the host (ADR-066).
        let runner = self.has_test_codeunit_host()
            && obj
                .properties
                .iter()
                .any(|(k, v)| k.eq_ignore_ascii_case("Subtype") && v.trim().eq_ignore_ascii_case("TestRunner"));
        if runner {
            self.enter_test_runner(object, inst.clone());
        }
        let result = self.call(object, on_run, inst, Vec::new());
        if runner {
            self.leave_test_runner();
        }
        let body = if guarded {
            self.tx.guarded_runs.pop();
            self.tx.guarded_bodies.pop()
        } else {
            None
        };
        if let Some(t) = self.tx.used_tries.last_mut() {
            t.runs -= 1;
        }
        self.tx.tx_world -= 1;

        match result {
            Ok(_) if guarded => {
                // A successful guarded run commits its own transaction
                // (TestCodeunitRunWriteTransaction.al:GuardedRun_LeavingUncommittedWrite_CommitsItWithTheRunsOwnTransaction).
                // Under AutoRollback it commits too: the run's row survives the test, and only the
                // test's own writes roll back (codeunits/TestCodeunitRunErrorTextsAndModels.al:
                // 247-275, AutoRollback_Test03/Test04).
                // The refusal above guarantees nothing of the test's own is pending here.
                //
                // In a page background worker the run's transaction is the worker's own and holds
                // no change, so there is nothing to commit: the caller's pending rows stay pending
                // (testpage/TestPageBgTaskTx_Tests.al:RunPageBackgroundTask_WorkerCommitLeavesCallersRowsUncommitted).
                // A duplicate-key Insert that was deferred comes out of that commit: the run fails with its error
                // (xmlports/import/XpTImportBasic.al:Import_DuplicateKeyExisting, through a guarded run).
                if let Some((_, e)) = self.tx.deferred_insert.take() {
                    let mut e = Box::new(e);
                    self.add_top_frame(&mut e);
                    if self.pages.bg_worker.is_none() {
                        self.rollback_transaction();
                    }
                    self.error_finals = None;
                    self.catch_error(*e);
                    if let Some(c) = caller_rec.take() {
                    c.restore();
                }
                    return Ok(Value::Boolean(false));
                }
                if self.pages.bg_worker.is_none() {
                    self.refuse_inconsistent_end("A guarded Codeunit.Run")?;
                    self.commit_transaction();
                }
                Ok(Value::Boolean(true))
            }
            Ok(_) => {
                if began_world {
                    self.refuse_inconsistent_end("Codeunit.Run")?;
                    self.commit_transaction();
                }
                Ok(Value::Null)
            }
            // A guarded run traps a TryFunctionWrite error from its OnRun: false, the error text and code set, the run's rows
            // undone (errors/Y68Small.al:221-225, 249-267). Where the error text is not pinned (the scope
            // name), it is not trapped here.
            Err(Signal::Error(e)) if guarded && e.code == TRY_FUNCTION_WRITE && al_builtins::native::text_unpinned(&e) => {
                Err(Signal::Unsupported(
                    "a guarded Codeunit.Run whose run fails with TryFunctionWrite and an unpinned scope name (not pinned)"
                        .into(),
                ))
            }
            // The license error of a Device Insert is not trapped (al-language-onprem/metadata/PtOnPremTables.al:177-184).
            Err(Signal::Error(e)) if guarded && e.message == crate::records::DEVICE_LICENSE_ERROR => {
                Err(Signal::Error(e))
            }
            Err(Signal::Error(mut e)) if guarded => {
                self.add_top_frame(&mut e);
                // The run's writes since its last Commit are undone, the error becomes the last
                // error and the call returns false (TestCodeunitRunGuard.al:GuardedRun_TrapsInnerError_ReturnsFalse,
                // :GuardedRun_InstanceForm_CommitThenError_KeepsCommittedRowOnly).
                // (Nothing to undo in a page background worker: the caller's rows are not its own.)
                if self.pages.bg_worker.is_none() {
                    self.guarded_run_rollback(&e, body)?;
                }
                self.error_finals = None;
                self.catch_error(*e);
                if let Some(c) = caller_rec.take() {
                    c.restore();
                }
                Ok(Value::Boolean(false))
            }
            // An unguarded run lets the error through (:UnguardedRun_PropagatesInnerError).
            Err(other) => Err(other),
        }
    }

    /// Refuses a write to a persistent table while the test body has no transaction. Only
    /// `Insert` is pinned (codeunits/TestWriteTxTestBoundary.al:Test09_NoneTestCannotWriteDirectly);
    /// the other writes are refused as unsupported rather than given a guessed outcome.
    pub(crate) fn gate_write(&self, method: RecordOp) -> Result<(), Signal> {
        if !self.no_transaction() && !self.report.request_only_page {
            return Ok(());
        }
        match method {
            RecordOp::Insert
            | RecordOp::Modify
            | RecordOp::Delete
            | RecordOp::DeleteAll
            | RecordOp::ModifyAll
            | RecordOp::Rename
            | RecordOp::Truncate => Err(al_error_code(NO_TRANSACTION, NO_TRANSACTION_CODE)),
            m => unsupported(format!(
                "{} under TransactionModel::None (the refusal is not pinned)",
                m.name()
            )),
        }
    }

    /// Runs a TestPage operation. A page begins a transaction of its own, so under
    /// `TransactionModel::None` its writes are allowed and the transaction ends with the
    /// operation (codeunits/TestWriteTxTestBoundary.al:Test13, Test13c, Test13d). That each
    /// operation, not the whole open-to-close sequence, ends it is an assumption
    /// (docs/probes-needed.md).
    pub(crate) fn in_page_transaction<T>(
        &mut self,
        body: impl FnOnce(&mut Session) -> Result<T, Signal>,
    ) -> Result<T, Signal> {
        let began_world = self.no_transaction();
        self.tx.tx_world += 1;
        let r = body(self);
        self.tx.tx_world -= 1;
        if began_world && r.is_ok() {
            self.refuse_inconsistent_end("A TestPage operation")?;
            self.commit_transaction();
        }
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transaction_model_attribute_names_its_model() {
        // The three models the corpus uses (record/TestTransactionModelAutoRollback.al, codeunits/TestWriteTxTestBoundary.al).
        assert_eq!(
            TxModel::from_attribute("[TransactionModel(TransactionModel::AutoCommit)]"),
            Some(TxModel::AutoCommit)
        );
        assert_eq!(
            TxModel::from_attribute("[TransactionModel(TransactionModel::AutoRollback)]"),
            Some(TxModel::AutoRollback)
        );
        assert_eq!(
            TxModel::from_attribute("[TransactionModel(TransactionModel::None)]"),
            Some(TxModel::None)
        );
        assert_eq!(
            TxModel::from_attribute("[transactionmodel(transactionmodel::autorollback )]"),
            Some(TxModel::AutoRollback)
        );
        // Anything else, or no member at all, is not a model.
        assert_eq!(
            TxModel::from_attribute("[TransactionModel(TransactionModel::Other)]"),
            None
        );
        assert_eq!(TxModel::from_attribute("[TransactionModel]"), None);
    }

    #[test]
    fn a_commit_behavior_attribute_names_ignore_or_error_only() {
        assert_eq!(
            CommitBehavior::from_attribute("[CommitBehavior(CommitBehavior::Ignore)]"),
            Some(CommitBehavior::Ignore)
        );
        assert_eq!(
            CommitBehavior::from_attribute("[CommitBehavior(CommitBehavior::Error)]"),
            Some(CommitBehavior::Error)
        );
        assert_eq!(
            CommitBehavior::from_attribute("[CommitBehavior(CommitBehavior::Allow)]"),
            None
        );
        assert_eq!(CommitBehavior::from_attribute("[CommitBehavior]"), None);
    }
}
