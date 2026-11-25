//! Sub-states of a [`Session`] (ADR-037 item 3): fields that belong together, so `Session` stays readable.
//!
//! Sub-states of `Session`: `TxState` (what ends with the open transaction is `OpenTx`, which a
//! Commit and a rollback reset in one step, `Session::end_open_transaction`; its locks are `TxLocks`;
//! how far the transaction has started is `transactions::TxPhase`), `UiState`, `PageClients`,
//! `EventState`, `CoverageState`, `ReportState` (the .NET host state is `dotnet::DotNetState`).

use super::*;

/// Transactions: the depth of transaction-beginning constructs, and where the open transaction stands.
pub(crate) struct TxState {
    /// Nesting depth of constructs that begin a transaction of their own (`Codeunit.Run`,
    /// a TestPage operation); a `TransactionModel::None` test may write inside them.
    pub(crate) tx_world: usize,
    /// The `[CommitBehavior]` of the innermost procedure that declares one.
    pub(crate) commit_behavior: transactions::CommitBehavior,
    /// How far the transaction has started since the test method began or the last `Commit()`: nothing yet, a read call or a
    /// write call (also a DeleteAll that finds no row). `CurrentTransactionType` depends on it (`transactions.rs`). A Commit or
    /// a rollback ends a write phase (`Session::end_open_transaction`); a read phase stays.
    pub(crate) phase: transactions::TxPhase,
    /// `Database.CurrentTransactionType`: a member index of the TransactionType option. A session starts at 0, UpdateNoLocks,
    /// and so does the first test method of a codeunit; a later test method starts with the type the one before it left
    /// (`start_test_method`), and a Commit keeps it.
    pub(crate) transaction_type: i32,
    /// What `transaction_type` rests on when no corpus case pins it (`Session::transaction_type_seen`): the assumptions it
    /// went through (a Commit kept it, al-runner issue 8605; a test method before left it, 8606), or that it is not known (a
    /// test method before it did not run to its end and could set it). A set that takes effect clears it.
    pub(crate) type_origin: crate::transaction_type::TypeOrigin,
    /// A test method of this codeunit has ended (`Session::test_method_ended`): the next one starts with the type it left.
    pub(crate) test_ended: bool,
    /// Reads of the type while it was not known whose value only sets it back (`transaction_type_saved.rs`), innermost last.
    pub(crate) saved_types: Vec<crate::transaction_type_saved::SavedType>,
    /// The rollback of an asserterror in the code of the test ended the test runner's transaction in a codeunit's first test
    /// method (`OpenTx::runner_tx`) and no Commit came after it: a set of the type then rests on
    /// `first-test-rollback-ends-runner-transaction`.
    pub(crate) runner_tx_rolled_back: bool,
    /// `Database.LockTimeout`: whether a lock wait ends in a timeout error.
    pub(crate) lock_timeout: bool,
    /// `Database.LockTimeoutDuration` in milliseconds.
    pub(crate) lock_timeout_duration: i64,
    /// The highest rowversion handed out when the last `Commit` ran: rows at or below it are durable.
    pub(crate) committed_row_version: i64,
    /// The database store's change counter when the last Commit or rollback ran: with `writes` it tells whether every write
    /// since was a Record Insert, Modify or Delete (`Session::stale_verdict`, al-runner issue 6028).
    pub(crate) commit_counter: u64,
    /// A rollback ran since the last Commit.
    pub(crate) rolled_back: bool,
    /// Why a rollback since the last Commit is not one the stale-buffer rule knows (`Session::note_rollback_for_stale_rule`);
    /// `None` without a rollback or after rollbacks the rule knows only.
    pub(crate) rollback_unpinned: Option<&'static str>,
    /// What trapped the rollbacks since the last Commit, while they are all of one kind the stale-buffer rule knows
    /// (`Session::trapped_error_rollback`).
    pub(crate) rollback_kind: Option<crate::records::TrappedRollback>,
    /// The last `Commit` made writes durable (the transaction it ended had written): `MinimumActiveRowVersion` is then not pinned.
    pub(crate) last_commit_wrote: bool,
    /// What ends with the open transaction: a Commit and a rollback put it back to its default (`Session::end_open_transaction`).
    pub(crate) open: OpenTx,
    /// The record of the last Rename that raised the SQL range error of its new key (`records/sql_range.rs`): what it holds
    /// afterwards is not measured, so catching that error while the record still lives is refused (`catch_error`).
    pub(crate) failed_sql_rename: Option<std::rc::Weak<std::cell::RefCell<al_builtins::value::RecordState>>>,
    /// Set by an Insert that AL code calls on a Record or a RecordRef, taken by the record method it starts
    /// (`Session::record_op_unprofiled`): only such an Insert goes into the insert queue. The platform's own inserts (an XmlPort
    /// import, a page's new row, a FlowField's row, a record link) are not measured and stay out of it.
    pub(crate) insert_from_al: bool,
    /// Set by a Rename that AL code calls on a Record or a RecordRef, taken by the Rename arm of `record_op_inner`: only such
    /// a Rename is in a measured cell of `rename_blank_key.rs`. The page client's rename at close clears it.
    pub(crate) rename_from_al: bool,
    /// Set by a Get that AL code calls on a Record variable (not a RecordRef), taken by the load it starts
    /// (`Session::rolled_read_of_load`) and cleared when the call returns.
    pub(crate) get_from_al: bool,
    /// Which write statement of AL code on a Record variable (not a RecordRef) is starting: set by `record_op_of_al`, and by
    /// ModifyAll(.., true) of such a statement for each row it modifies; taken by the arm it starts (`Session::take_write_of_al`)
    /// and cleared when the call returns. Only such a write is in the measured cells of `records/sql_range.rs`. A page's save
    /// and the platform's own writes do not set it.
    pub(crate) write_of_al: Option<WriteOfAl>,
    /// The record (its address) whose Insert was the last to leave a duplicate key pending: the insert queue asks whether the
    /// Insert that sends it was itself a duplicate (`Session::note_insert_buffer`).
    pub(crate) deferred_by: Option<usize>,
    /// The record (its address) whose queued Insert runs now, until it writes the store (`Session::queue_stored_row`).
    pub(crate) queue_pending: Option<usize>,
    /// The records (their addresses) whose queued Insert put its row into the queue at the store write, before its After
    /// event ran; taken by `Session::note_insert_buffer`.
    pub(crate) queue_counted: Vec<usize>,
    /// How many deferred duplicate-key Inserts have come out so far (a TestPage action wraps the code of one its own
    /// Insert left, `action_invoke`).
    pub(crate) deferred_raised: u32,
    /// Set while the page client or an XmlPort import writes a record: their duplicate-key
    /// errors are raised at once (their own tests pin it), not deferred like an AL `Insert`.
    pub(crate) immediate_errors: bool,
    /// Try functions on the call stack (Truncate is refused inside them).
    pub(crate) try_depth: usize,
    /// `asserterror` statements whose body is running.
    pub(crate) assert_depth: usize,
    /// The TryFunction calls on the call stack whose result is used, innermost last: a write inside one fails with
    /// TryFunctionWrite (`transactions::try_write_error`).
    pub(crate) used_tries: Vec<transactions::UsedTry>,
    /// For each guarded `Codeunit.Run` running: the call stack depth of its OnRun frame.
    pub(crate) guarded_runs: Vec<usize>,
    /// For each guarded `Codeunit.Run` running: how far its body got along the pinned steps (`records/guarded_run_body.rs`).
    pub(crate) guarded_bodies: Vec<crate::records::GuardedBody>,
    /// The transaction world of an XmlPort export started inside a TryFunction whose result is used: its triggers' Insert and
    /// Modify are not writes of the try (xmlports/shapes-properties/YXmlPortProps.al:861); other writes there are refused.
    pub(crate) xp_export_try_world: Option<usize>,
}

/// What ends with the open transaction: every field goes back to its default at a Commit and at a rollback
/// (`Session::end_open_transaction`). A field that must outlive one of them does not belong here.
#[derive(Default)]
pub(crate) struct OpenTx {
    /// The rows (table object, company, primary key) a Record Modify wrote, or a Get read while its table was locked by
    /// `LockTable`, since the last Commit or rollback: a stale buffer from the Insert of the transaction before may then be
    /// written (record/TestRecordStaleBufferWrite.al:495-584, `Session::stale_verdict`).
    pub(crate) rows_touched: Vec<(u32, Option<String>, Vec<Value>)>,
    /// The tables (table object, company) a Record Insert wrote a row of since the last Commit or rollback (`Session::stale_verdict`).
    pub(crate) tables_inserted: Vec<(u32, Option<String>)>,
    /// The tables a `Record.LockTable` locked since the last Commit or rollback.
    pub(crate) locked_tables: Vec<u32>,
    /// The Record writes to the database store since the last Commit or rollback, in order (`Session::stale_verdict`).
    pub(crate) writes: Vec<TxWrite>,
    /// The database reads (table object, company, the key a Get named; `None` for any other read) since the last rollback, while
    /// the transaction then open began at it (`Session::note_read_after_rollback`).
    pub(crate) reads_after_rollback: Vec<(u32, Option<String>, Option<Vec<Value>>)>,
    /// A duplicate-key Insert into a table without a Blob field does not fail where it is
    /// called: the table (object index) and the error come out at the next access to the table
    /// or at Commit (corpus record/TestRecordRuntimeTextsAndViews.al:
    /// Record_Insert_DuplicateKey_WithoutBlobField_IsDeferred_AndSurfacesAtTheNextAccess). An Insert of a Date or DateTime its
    /// SQL column cannot hold is deferred the same way (`records/sql_range.rs`).
    pub(crate) deferred_insert: Option<crate::records::DeferredInsert>,
    /// The rows each table's insert queue holds since it was last sent (`Session::note_insert_buffer`).
    pub(crate) insert_buffer: Vec<crate::records::InsertQueue>,
    /// What `Database.LastUsedRowVersion` reports inside the open transaction (`Session::last_used_row_version`): the
    /// database tables an Insert wrote rows of that nothing has sent to the database yet (table, company, highest row
    /// version), the highest row version a sent write holds, how many database row versions were handed out and how
    /// many of them an Insert or Modify accounts for, and why the value is not pinned.
    pub(crate) row_versions: RowVersionTx,
    /// The tables (object indexes) `Consistent(false)` marked in the open transaction: `Commit` fails while one is marked.
    pub(crate) inconsistent: Vec<crate::consistent_marks::ConsistentMark>,
    /// The locks the open transaction holds and the locking reads it ran; a test method also starts without them
    /// (`Session::start_test_method`).
    pub(crate) locks: TxLocks,
    /// Where the open transaction began (the test method's start, an AL Commit) and which measured steps ran in it since: the
    /// moments the TransactionType term of an Update report is measured at (`reports/transaction_world.rs`). Set by
    /// `Session::start_test_method` and the AL Commit of test code; a database call (`records/op_inner.rs`) and a
    /// CurrentTransactionType set (`transactions.rs`) are noted in it or end it, a Codeunit.Run (`transactions.rs`) and a
    /// report run end it, and so does the end of the transaction.
    pub(crate) moment: Option<crate::reports::transaction_world::Moment>,
    /// The open transaction may be one the test runner started before the first test method of the codeunit: whether a
    /// CurrentTransactionType set to another type takes effect there depends on the runner and on what ran before (the
    /// description of corpus pull request 1520), so such a set is refused. A Commit and the rollback of an asserterror in the
    /// code of the test end it (`transaction_type.rs`); every other rollback keeps it.
    pub(crate) runner_tx: bool,
}

/// The locks of the open transaction (`OpenTx::locks`).
#[derive(Default)]
pub(crate) struct TxLocks {
    /// A lock started the open write transaction: `LockTable` on a database table, `ReadIsolation := UpdLock`, or a FindFirst
    /// under RepeatableRead or UpdLock (codeunits/TestCodeunitRunWriteTransaction.al:211-278). A Commit ends it.
    pub(crate) lock_tx: bool,
    /// A lock whose effect on the write transaction is not pinned ran since the last Commit (a RecordRef.LockTable, a
    /// RepeatableRead set without a read, another read than FindFirst under a locking ReadIsolation): while no pinned write
    /// or lock opened the transaction, what depends on it is refused (`Session::in_write_transaction`).
    pub(crate) lock_unpinned: Option<UnpinnedLock>,
    /// A `RecordRef.LockTable` on a database table ran since the last Commit: the write transaction is open
    /// (recordref-fieldref/WcRecordRefLockTable.al:28-51), while what else it changes stays refused through `lock_unpinned`.
    pub(crate) recref_lock_tx: bool,
    /// A locking ReadIsolation (a FindFirst under RepeatableRead or UpdLock) set `lock_tx` in the open transaction.
    pub(crate) isolation_lock: bool,
    /// The tables (object index, company) a locking ReadIsolation locked since the last Commit: UpdLock set on a database
    /// table, or a read under RepeatableRead or UpdLock.
    pub(crate) isolation_lock_tables: Vec<(u32, Option<String>)>,
    /// The database reads under a locking ReadIsolation since the last Commit or rollback (`Session::locking_read_cell`).
    pub(crate) locking_reads: Vec<crate::records::LockingReadNote>,
    /// How many database reads (record methods that read a table that is not temporary) ran since the last Commit or rollback.
    pub(crate) db_reads: u32,
    /// A read under UpdLock or RepeatableRead whose effect on the write transaction is not pinned ran since the last Commit
    /// (a FindFirst, FindSet, FindLast or another Find under an isolation set in an earlier transaction): `lock_unpinned`
    /// then refuses IsInWriteTransaction. Without it, the pinned reads that set `lock_unpinned` open no write transaction.
    pub(crate) locking_read_tx_unpinned: bool,
}

/// Why a lock's effect on the write transaction is not pinned (`TxLocks::lock_unpinned`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum UnpinnedLock {
    /// A FindFirst, FindSet, FindLast or another Find under a ReadIsolation RepeatableRead or UpdLock set in an earlier transaction.
    LockingRead,
    /// Any other cause, by its refusal text.
    Other(&'static str),
}

impl UnpinnedLock {
    /// The cause, as refusal texts name it.
    pub(crate) fn text(self) -> &'static str {
        match self {
            UnpinnedLock::LockingRead => crate::records::UNPINNED_LOCKING_READ,
            UnpinnedLock::Other(what) => what,
        }
    }
}

/// A write statement of AL code on a Record variable (`TxState::write_of_al`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum WriteOfAl {
    Insert,
    Modify,
    ModifyAll,
    /// The Modify of one row by ModifyAll(.., true) of such a statement.
    ModifyAllRow,
}

/// What kind of Record write a [`TxWrite`] was.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TxWriteKind {
    Insert,
    Modify,
    Delete,
    /// A Rename of a row to another key, with no trigger write before it and no field that relates to the table (`pk` is the
    /// old key).
    Rename,
    /// A ModifyAll: the Modifies of its rows follow it in `OpenTx::writes` (`TxWrite::bulk`).
    ModifyAll,
    /// A DeleteAll: the Deletes of its rows follow it, or the store deleted them in one step (`TxWrite::bulk`).
    DeleteAll,
    /// A Truncate.
    Truncate,
    /// A Rename that also rewrote related rows, or one with a trigger write before it: its own writes are not told apart.
    Other,
}

/// How a ModifyAll or DeleteAll ended (`TxWrite::bulk`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct BulkEnd {
    /// How many entries of `OpenTx::writes` its rows (and the triggers they ran) added after it.
    pub(crate) entries: usize,
    /// The store's change counter when it ended.
    pub(crate) counter: u64,
}

/// The row versions of the open transaction as `Database.LastUsedRowVersion` sees them (`OpenTx::row_versions`).
#[derive(Clone, Debug, Default)]
pub(crate) struct RowVersionTx {
    /// (table, company, first, last): the row versions the Inserts of rows of that table that nothing sent yet handed out.
    pub(crate) pending_inserts: Vec<(u32, Option<String>, i64, i64)>,
    pub(crate) sent: i64,
    pub(crate) handed_out: u64,
    pub(crate) accounted: u64,
    pub(crate) last: i64,
    pub(crate) unpinned: Option<&'static str>,
    /// A Rename (`TxWriteKind::Other`) whose row version counts when it ends as a plain one (`Session::end_row_version_write`), or
    /// a ModifyAll whose first modified row ends it (`Session::note_row_versions`). Set: `last_used_row_version` refuses.
    pub(crate) write_sends: Option<(u32, Option<String>, TxWriteKind)>,
    /// The value moved by a write after an Insert through the assumption `row-version-write-after-insert`
    /// (`Session::note_row_versions`); `Session::last_used_row_version` marks the answer.
    pub(crate) assumed_write: bool,
}

/// The rows of `table` whose SystemRowVersion lies in (`from`, `to`], in any company: the open transaction inserted them, and a row
/// version sent before them was at or above theirs (`Session::send_pending_insert`). BC gives them their row version when it
/// sends them (RowVersionWrites.al:392 of corpus PR 1509, merged, not pinned: InsertTwoTables `row/other` greater); this
/// runtime keeps the Insert's, so what shows it is refused (`records/restamped.rs`). Kept until the session's data is replaced.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RestampedRows {
    pub(crate) table: u32,
    pub(crate) from: i64,
    pub(crate) to: i64,
}

/// One Record write to the database store: the store's change counter before and after it.
#[derive(Clone, Debug)]
pub(crate) struct TxWrite {
    pub(crate) table: u32,
    pub(crate) company: Option<String>,
    pub(crate) kind: TxWriteKind,
    pub(crate) pk: Vec<Value>,
    pub(crate) before: u64,
    pub(crate) after: u64,
    /// A ModifyAll or DeleteAll that ended: what it wrote (`Session::note_bulk_end`). `None` for other writes and for one that
    /// stopped with an error.
    pub(crate) bulk: Option<BulkEnd>,
    /// The write ran through a RecordRef (`Session::through_recordref`).
    pub(crate) through_recordref: bool,
    /// The record facts of a bulk Record write (`Session::note_bulk_write`); all false for other writes.
    pub(crate) bulk_shape: BulkShape,
}

/// What a bulk Record write's record carried when it wrote (`Session::note_bulk_write`).
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct BulkShape {
    /// The record had a record filter.
    pub(crate) filtered: bool,
    /// `MarkedOnly` was set on the record.
    pub(crate) marked_only: bool,
    /// A Truncate's ResetAutoIncrement argument was true (`Session::note_truncate_write`).
    pub(crate) reset: bool,
}

impl TxState {
    /// The transaction state of a session that has just started: no transaction, nothing locked.
    pub(crate) fn fresh() -> TxState {
        TxState {
            tx_world: 0,
            commit_behavior: Default::default(),
            phase: crate::transactions::TxPhase::Fresh,
            transaction_type: 0,
            type_origin: Default::default(),
            test_ended: false,
            saved_types: Vec::new(),
            runner_tx_rolled_back: false,
            lock_timeout: true,
            lock_timeout_duration: 10,
            committed_row_version: 0,
            commit_counter: 0,
            rolled_back: false,
            rollback_unpinned: None,
            rollback_kind: None,
            last_commit_wrote: false,
            open: OpenTx::default(),
            failed_sql_rename: None,
            insert_from_al: false,
            rename_from_al: false,
            get_from_al: false,
            write_of_al: None,
            deferred_by: None,
            queue_pending: None,
            queue_counted: Vec::new(),
            deferred_raised: 0,
            immediate_errors: false,
            try_depth: 0,
            assert_depth: 0,
            used_tries: Vec::new(),
            guarded_runs: Vec::new(),
            guarded_bodies: Vec::new(),
            xp_export_try_world: None,
        }
    }
}

/// Handler functions and notifications of the running test method.
#[derive(Default)]
pub(crate) struct UiState {
    /// Handler functions bound to the running test method (None outside one).
    pub(crate) scope: Option<ui::UiScope>,
    /// Page handler functions running right now (`[PageHandler]`, `[ModalPageHandler]`).
    pub(crate) page_handler_depth: usize,
    /// Ids of the notifications a `[SendNotificationHandler]` took in this session.
    pub(crate) sent_notifications: Vec<[u8; 16]>,
    /// `--dev-endpoint-ui` (al-runner issue 3697).
    pub(crate) dev_endpoint: crate::dev_endpoint_ui::DevEndpointUi,
    /// The whole test session is a session without a client (`--no-client`, `Session::set_clientless_session`).
    pub(crate) clientless: bool,
    /// The login code that sets WorkDate did not run because of `clientless` (`clientless.rs`).
    pub(crate) clientless_login_skipped: bool,
}

/// What the TestPage client keeps between calls.
#[derive(Default)]
pub(crate) struct PageClients {
    /// What the action running now did (`pages/action_tally.rs`); armed only around an action of an empty List on its new-row line.
    pub(crate) action_tally: Option<crate::pages::action_tally::ActionTally>,
    /// The TestPage variables whose `Trap()` waits for the next plain `Page.Run` of their page (ADR-012). Several wait at
    /// once, one per page: a test traps the preview page and, inside it, the entries page its action opens.
    /// The variable-bound control values each SaveValues page stored when it closed: (owner, global slot, value), the owner
    /// being the page extension whose variable it is, `None` for the page's own.
    pub(crate) saved_values: std::collections::HashMap<u32, Vec<(Option<usize>, usize, Value)>>,
    /// `saved_values` at the last commit: a rollback goes back to it (`Session::rollback_transaction`).
    pub(crate) saved_values_committed: std::collections::HashMap<u32, Vec<(Option<usize>, usize, Value)>>,
    /// For each report whose request page stored values while the program has the Object Options table: whether those
    /// values stand in a row this runtime wrote (`pages/open/saved_row.rs`). A rollback keeps it: a row is written only just
    /// before its run commits.
    pub(crate) saved_rows: std::collections::HashMap<u32, crate::pages::SavedRow>,
    /// The rows OKs of request pages ask for, until their runs return, innermost last: (report object, the transaction
    /// world of the run, the store's change counter at the save).
    pub(crate) saved_row_pending: Vec<(u32, usize, u64)>,
    pub(crate) traps: Vec<Rc<RefCell<crate::pages::PageState>>>,
    /// Open TestPages in a state whose end of scope is not pinned (what BC saves when the variable goes away): a routine
    /// whose local holds the last reference to one is refused when it returns.
    pub(crate) scope_unpinned: Vec<Rc<RefCell<crate::pages::PageState>>>,
    /// A page background task worker is running: its parameters and the result it set.
    pub(crate) bg_worker: Option<pages::BgWorker>,
    /// The test session's last error as a worker's end gave it back, when the worker had set one of its own: until the first
    /// read of another last error, a read of it differs from the answer before issue 8697, which gave the worker's
    /// (`Session::bg_note_last_error_read`, which ends it).
    pub(crate) bg_kept_last_error: std::cell::RefCell<Option<al_builtins::native::AlError>>,
    /// A failing worker's error left `TestPage.RunPageBackgroundTask` and has not left a routine of the test session yet: an
    /// asserterror around the call itself keeps the last error (`Session::bg_catch_keeps_last_error`).
    pub(crate) bg_task_error_unrecorded: bool,
    /// The query cache page background tasks and the test session read through (`query_cache.rs`).
    pub(crate) query_cache: crate::query_cache::QueryCache,
    /// Which dialogs and errors the CaptionClass runs in progress may raise (`Session::caption_dialog_refusal`).
    pub(crate) caption_dialogs: crate::pages::CaptionDialogs,
    /// The CaptionClass expression whose own call runs now, on a page whose open ran its empty-record runs without a pinned
    /// number (`Session::caption_write_refusal`).
    pub(crate) caption_write_guard: Option<crate::pages::CaptionWriteGuard>,
    /// The page whose enqueued background tasks run once the `EnqueueBackgroundTask` call that queued them has
    /// written its by-reference task id back (see `Session::bg_enqueue`).
    pub(crate) bg_drain_after_call: Option<Rc<RefCell<crate::pages::PageState>>>,
    /// The xRec fields the next `Modify` hands the table's triggers instead of the stored row: a
    /// page saving a row passes its own xRec (corpus
    /// testpage/TestPageActionRunObjectPageRec_Tests.al:HostSaveAfterTheTargetEditedTheRowCarriesXRec).
    pub(crate) page_xrec: Option<Vec<Value>>,
    /// The page whose OnModifyRecord runs now, innermost last: a `Rec.Validate` of its Rec in its own code takes its xRec
    /// (`pages/modify_record_xrec.rs`).
    pub(crate) modify_record_xrec: Vec<crate::pages::modify_record_xrec::ModifyRecordXRec>,
    /// Records whose `CurrFieldNo` is set (a TestPage write validating a field, or an assignment in table
    /// code), with the field's number: what `CurrFieldNo` reads. Held weakly: the entry keeps the record's
    /// allocation, so a later record cannot get the same address and inherit the number.
    pub(crate) curr_field_no: Vec<(std::rc::Weak<RefCell<RecordState>>, i32)>,
    /// The report instances that `Execute` has run: a second `Execute` on one is refused.
    /// A page opened for a `RunObject` action looks its handler up without failing when there is
    /// none (al-vm pages/runobject.rs).
    pub(crate) lenient_bind: bool,
    /// The page's own trigger (lower case) whose AL error ended the last page trigger call that failed, with the page's
    /// object; `None` after an extension's trigger failed. Read where the client turns a trigger's error into its own.
    pub(crate) failed_trigger: Option<(u32, String)>,
    /// The answers the page extensions' triggers of the last page trigger call stated (`exit(Value)`), in the order they ran.
    pub(crate) ext_answers: Vec<bool>,
    /// The last `page_trigger_answer` was a veto by a page extension's trigger.
    pub(crate) ext_vetoed: bool,
    /// The page's own trigger of the last `page_trigger` call ended by running `exit(value)`.
    pub(crate) own_exited: bool,
    /// The last `page_trigger_answer` was the page's own OnInsertRecord vetoing by its named return value
    /// (pages/record_trigger_answer.rs); taken by `insert_draft`.
    pub(crate) named_false_insert: bool,
    /// The open that starts next is a TestPage's own OpenEdit, OpenView or OpenNew (taken by `open_page_as`).
    pub(crate) test_open: bool,
    /// How many test methods the runner started in this session (`Session::start_test_method`): a page records it at each open
    /// (`PageState::opened_in_test_method`).
    pub(crate) test_methods_started: u32,
    /// Set by a FindFirst or FindLast that found a row on Rec in the OnOpenPage of a page an action opened through
    /// RunPageLink (`records.rs`); taken by `open_page_as` into `PageState::run_page_link_open_moved`.
    pub(crate) run_page_link_open_moved: bool,
    /// How many OnOpenPage triggers of pages an action opened through RunPageLink are running now: a by-value copy of Rec taken
    /// in one is `records::RUN_PAGE_LINK_OPEN_COPY` (see `Session::record_by_value`).
    pub(crate) run_page_link_opening: u32,
    /// The close that starts next is a TestPage's own Close() (taken by `close_with`).
    pub(crate) test_close: bool,
    /// The AL Error the save of the close running now failed with (`pages/close_save_failed.rs`,
    /// `pages/modal_close_save_error.rs`).
    pub(crate) close_save_error: Option<al_builtins::native::AlError>,
    /// The object whose routine raised `close_save_error` (`Session::error_raised_in` when the save failed).
    pub(crate) close_save_raised_in: Option<usize>,
    /// The TestPage calls running now, innermost last: the root page of the call's page tree and what the pages' own code
    /// did to that tree meanwhile (`pages/shown.rs`).
    pub(crate) shown_frames: Vec<(*const RefCell<crate::pages::PageState>, Vec<crate::pages::ShownNote>)>,
    /// Per page object, the expression controls whose value the page keeps between refreshes (`pages/shown.rs`).
    pub(crate) shown_controls: std::collections::HashMap<u32, Rc<Vec<usize>>>,
    /// The page triggers running now, innermost last, with their page and, for the OnOpenPage of a page the test opened with
    /// `TestPage.OpenView`, its page object (`pages/code_close_veto.rs`, read by
    /// `permissions/permission_probed/extension_code.rs`).
    pub(crate) trigger_kinds: Vec<(
        *const RefCell<crate::pages::PageState>,
        crate::pages::TriggerKind,
        Option<usize>,
    )>,
    /// How many TestPage calls run now, nested (`pages/code_close_veto.rs`).
    pub(crate) client_depth: u32,
    /// Vetoed `CurrPage.Close()` calls waiting for the end of their TestPage call (`pages/code_close_veto.rs`).
    pub(crate) code_close_vetoes: Vec<crate::pages::CodeCloseVeto>,
    /// Pages whose close, requested by OnAfterGetCurrRecord at OpenView, OnQueryClosePage refused twice (`pages/code_close_veto.rs`).
    pub(crate) open_vetoed: Vec<std::rc::Weak<RefCell<crate::pages::PageState>>>,
    /// The pages whose control OnValidate runs now, innermost last (`Session::run_control_onvalidate`): page code's record calls
    /// on their Rec are recorded in that OnValidate (`Session::note_page_rec_call`).
    pub(crate) onvalidate_pages: Vec<Rc<RefCell<crate::pages::PageState>>>,
}

/// Event subscriptions and what the Event Subscription table reports.
#[derive(Default)]
pub(crate) struct EventState {
    /// Manual event subscriptions made by `BindSubscription`.
    pub(crate) bindings: Vec<dispatch::Binding>,
    /// Successful binding and language changes invalidate literal field caption reuse.
    pub(crate) caption_context_revision: u64,
    /// Manual bindings whose bound instance the globals of a page reached when they were made (`page_held_bindings.rs`,
    /// al-runner issue 6924).
    pub(crate) page_held: Vec<crate::page_held_bindings::PageHeld>,
    /// The pages that have an instance and may still run code (`page_held_bindings.rs`).
    pub(crate) pages_with_instance: Vec<std::rc::Weak<RefCell<crate::pages::PageState>>>,
    /// The pages a Page variable's non-modal `Run` showed last (`page_held_bindings.rs`).
    pub(crate) page_var_runs: Vec<std::rc::Weak<RefCell<crate::pages::PageState>>>,
    /// What the Event Subscription table reports (calls handled, bound instances).
    pub(crate) event_stats: Rc<platform::EventStats>,
    /// `publish` is about to raise its event: the next `raise` has an event frame on the stack.
    pub(crate) publish_frame: bool,
    /// The subscribers (object, routine) the next raise reaches, when only some of them (`Session::raise_to`); the events
    /// raised inside them reach every subscriber.
    pub(crate) only_targets: Option<Vec<(usize, usize)>>,
    /// Set by `publish` for an IncludeSender event whose sender is not pinned: a subscriber that asks for it is refused with this text.
    pub(crate) sender_refused: Option<String>,
    /// Set by `finish_call` for a publisher: per argument, the global of the caller's instance a `var` argument names
    /// (instance, global slot). `publish` hands it to `raise` and the subscribers' frames alias those globals.
    pub(crate) var_origins: Vec<Option<(Rc<RefCell<crate::Instance>>, usize)>>,
    /// Set just before a subscriber is called, taken by its frame: (parameter slot, instance, global slot).
    pub(crate) next_aliases: Vec<(usize, Rc<RefCell<crate::Instance>>, usize)>,
    /// Set just before a subscriber is called, taken by its frame: the call is an event subscriber dispatch.
    pub(crate) next_is_subscriber: bool,
    /// Set by `publish` just before the raise: the publisher routine (object, routine) and the answer of
    /// `Session::raise_arg_var_param` it took from `raise_arg_var_param`; taken by `raise_to_subscribers` for
    /// `Session::event_publisher`.
    pub(crate) by_value_publisher: Option<(usize, usize, bool)>,
    /// Set by `finish_call` for a publisher call, taken by `publish`, cleared after the call: `Session::raise_arg_var_param`.
    pub(crate) raise_arg_var_param: bool,
    /// Set just before a subscriber is called, taken by its frame (`records::ByValueCall`), cleared after the call: per
    /// subscriber parameter, what `Session::event_args` says of it.
    pub(crate) next_event_args: Vec<crate::records::EventArg>,
    /// Set by `publish` for a publisher `blob_transfer::measured_publisher` names and by `table_event` for an event
    /// `blob_transfer::measured_table_event` names, just before the raise; taken by `raise_to_subscribers`.
    pub(crate) blob_publisher: Option<crate::blob_transfer::BlobEvent>,
    /// Set just before a subscriber is called, taken by its frame, cleared after the call: the raise of a `blob_publisher`
    /// event to a subscriber `blob_transfer::measured_subscriber` names (`blob_transfer::Argument::measured_call`).
    pub(crate) next_blob_event: bool,
    /// The event subscriber calls on the stack, innermost last: (publisher object, subscriber object, subscriber routine) for
    /// the shapes whose event recursion error text is pinned, `None` for another (the call-depth error names event
    /// recursion when any is on the stack).
    pub(crate) subscriber_stack: Vec<Option<(usize, usize, usize)>>,
    /// Set by `finish_call` just before the call, taken by the callee's frame (`Frame::var_lens`).
    pub(crate) next_var_lens: Vec<Option<i32>>,
    /// Set by `finish_call` just before the call, taken by the callee's frame (`Frame::var_codeunits`).
    pub(crate) next_var_codeunits: Vec<Option<crate::clear_shared_codeunit::VarCodeunit>>,
    /// Set by `finish_call` just before the call, taken by the callee's frame (`Frame::option_hops`).
    pub(crate) next_option_hops: Vec<bool>,
    /// Set by `finish_call` just before the call, taken by the callee's frame (`Frame::hand_on_origins`).
    pub(crate) next_hand_on_origins: Vec<bool>,
    /// Set by `finish_call` (by `raise_to_subscribers` for a subscriber) just before the call, taken by the callee's frame
    /// (`Frame::option_carried`).
    pub(crate) next_option_carried: Vec<crate::var_option::Carried>,
    /// A publisher's marks of `var_option::option_carried_of`, set by `finish_call` and taken by `publish` and
    /// `raise_to_subscribers`, under the measured event contract only.
    pub(crate) publisher_option_carried: Vec<crate::var_option::Carried>,
    /// Per parameter: `var_option::carried::carried_args` let a value that carries another list or none through to this `var`
    /// Option parameter with members, which binds it as it is. Set by `finish_call` just before the call, taken by the
    /// callee's binding (`lib.rs`), cleared after the call.
    pub(crate) next_carried_kept: Vec<bool>,
    /// The declared lengths of a publisher's `var` Text and Code arguments (`var_lens_of`), set by `finish_call` just before
    /// the publisher runs and taken by `publish`, which hands them to the subscribers that read them.
    pub(crate) publisher_var_lens: Vec<Option<i32>>,
    /// Events being raised that have no publisher routine (table, page and Global Triggers
    /// events): BC puts frames between the subscriber and its caller that are not pinned, so the
    /// live call stack is not answered while one runs.
    pub(crate) unpinned_event_depth: usize,
    /// Set while `MakeDateFilter` is raised: subscribers of dependency apps (Microsoft's Filter Tokens) are left out (filterevents.rs).
    pub(crate) skip_dependency_subscribers: bool,
    /// Apps whose subscribers do not run, because they are not installed yet (`Session::set_apps_not_installed`).
    pub(crate) not_installed_apps: Option<std::collections::BTreeSet<u32>>,
    /// Apps with no dependency path to the app whose install code runs (`Session::set_apps_install_order_unknown`): BC's
    /// order between them is not known, so a subscriber of one of them that the install code reaches is refused.
    pub(crate) order_unknown_apps: Option<std::collections::BTreeSet<u32>>,
    /// The pairs (installing app, app of `order_unknown_apps`) whose refusal the install code met
    /// (`Session::take_order_unknown_hits`, ADR-046 amendment 7).
    pub(crate) order_unknown_hits: std::cell::RefCell<std::collections::BTreeSet<(u32, u32)>>,
    /// The tables the install code of the apps under test may write (`Session::set_install_written_tables`); `None`: not
    /// known, so no table counts as empty in either install order (empty_reads.rs).
    pub(crate) install_written: Option<std::collections::BTreeSet<u32>>,
    /// Tables of `install_written` that only their own app's install code writes, with the rows it may insert
    /// (`Session::set_install_rows`, empty_reads.rs).
    pub(crate) install_rows: Option<std::sync::Arc<crate::empty_reads::InstallRows>>,
    /// While the database trigger setup of one table is asked, or a database trigger event of one table is raised: the
    /// subscribers of `order_unknown_apps` that leave every flag unchanged for it (setup_no_op.rs), or do nothing for it
    /// (db_event_no_op.rs), which the raise leaves out instead of refusing.
    pub(crate) setup_no_ops: std::collections::BTreeSet<(usize, usize)>,
    /// While the subscribers of `GetDatabaseTableTriggerSetup` answer for one table: that table (object index), for the read
    /// they make under a restricted set (permission_trigger_setup.rs).
    pub(crate) db_setup_table: Option<usize>,
    /// The trial runs of subscribers whose install order is not known (order_trial.rs).
    pub(crate) trial: crate::order_trial::TrialState,
    /// What the subscribers of `Global Triggers` answered, per table object (globaltriggers.rs).
    pub(crate) global_setups: std::collections::HashMap<usize, globaltriggers::GlobalSetup>,
}

/// A filter of a SetTableView as (field, filter group, filter text).
pub(crate) type ViewFilterKey = (usize, i32, String);

/// Code coverage recording.
#[derive(Default)]
pub(crate) struct CoverageState {
    /// `CodeCoverageLog` is recording.
    pub(crate) code_coverage: bool,
    /// `al-runner test --coverage`: runs per statement position (`Program::positions`) since the last
    /// [`Session::take_coverage`]. `None` when coverage is off, which costs one branch per statement.
    pub(crate) coverage_counts: Option<(Vec<u32>, Vec<u32>)>,
    /// What recording noted (`platform/coverage.rs`).
    pub(crate) recorder: platform::coverage::Recorder,
    /// `al-runner impact-index build`: what the code raised and touched (`impact.rs`). `None` when off. A cell, so
    /// that the readers that take `&self` (a report's schema) can note too.
    pub(crate) impact: std::cell::RefCell<Option<crate::impact::Touches>>,
    /// `impact` is on: the one branch a note costs when it is off.
    pub(crate) impact_on: bool,
    /// The cell tap of the observer fixture (`observer_tap.rs`). `None` when off: one branch per routine call.
    pub(crate) cells: Option<Box<crate::observer_tap::Tap>>,
    /// `al-runner test --profile`: the session's call tree (`profile.rs`). `None` when off: one branch per statement,
    /// call, builtin return and record method.
    pub(crate) profiler: Option<Box<crate::profile::Profiler>>,
    /// The footprint of a partner app's install session while one is recorded (`install_footprint.rs`). `None` when off.
    pub(crate) install_footprint: std::cell::RefCell<Option<Box<crate::install_footprint::Recording>>>,
    /// `install_footprint` is `Some`: the one branch a builtin call costs when no footprint is recorded.
    pub(crate) footprint_on: bool,
}

/// `Language` and `FormatRegion` set on a report variable outside a run.
pub(crate) struct VariableCulture {
    pub(crate) inst: std::rc::Weak<std::cell::RefCell<al_builtins::value::Instance>>,
    pub(crate) language: Option<i32>,
    pub(crate) region: Option<String>,
    pub(crate) spent: bool,
}

/// The report being run: language, trigger flow, dataset.
#[derive(Default)]
pub(crate) struct ReportState {
    /// `Report.Run` on a variable: the variable gets a new instance when the run ends and the old one keeps its state for whoever holds it.
    pub(crate) rebind_after_run: bool,
    /// `CurrReport.Language` of the running report (`reports.rs`); `None` until a trigger sets it.
    pub(crate) curr_report_language: Option<i32>,
    /// `CurrReport.FormatRegion` of the report started by `Run` or `RunModal` (empty text until a trigger sets it).
    pub(crate) format_region: Option<String>,
    /// `Language` and `FormatRegion` set on a report variable outside a run, by variable (`reports.rs` `variable_culture`):
    /// the language, the region, and whether a SaveAs already used them.
    pub(crate) variable_culture: Vec<VariableCulture>,
    /// The language the running SaveAs got from its variable: `CurrReport.Language` reads it and `Format` follows it until a
    /// trigger sets another.
    pub(crate) run_language: Option<i32>,
    /// The running SaveAs got its `format_region` from its variable: `CurrReport.FormatRegion` reads it.
    pub(crate) variable_region: bool,
    /// The running SaveAs writes a report whose `FormatRegion` property is set: `CurrReport.FormatRegion` is not pinned there.
    pub(crate) region_property: bool,
    /// While `OnInitReport` of a report with the `FormatRegion` property runs: that region. Which culture `Format` uses there is
    /// not pinned, so a culture-dependent `Format` is refused (`FormatCtx::region_unpinned`).
    pub(crate) region_in_init: Option<String>,
    /// The call depth of the `OnInitReport` trigger being run, and whether it called
    /// `CurrReport.Quit` (al-vm reports.rs).
    pub(crate) report_init_depth: Option<usize>,
    pub(crate) report_quit: bool,
    /// The call depth of the report trigger being run and what it may ask of the report
    /// (`CurrReport.Skip`, `Break`, `Quit`), and the request it made (al-vm reports.rs).
    pub(crate) report_trigger: Option<(usize, crate::reports::TrigKind)>,
    pub(crate) report_flow: Option<crate::reports::Flow>,
    /// A `CurrReport.Skip`, `Break` or `Quit` is leaving the procedures down to the report trigger at this depth.
    pub(crate) flow_unwind: Option<usize>,
    /// The request page of a `RunRequestPage` is open: a table write in its handler finds no transaction
    /// (reports/TestProbeReportContracts.al:RequestPage_TableWriteInsideHandler).
    pub(crate) request_only_page: bool,
    /// The run failed on a DataItemTableView `const(0D)` (`report_prepare`): a SaveAs whose answer is used keeps that error
    /// as the last error (streams/CjDataset.al:190-211).
    pub(crate) zero_date_view_failed: bool,
    pub(crate) report_executed: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// The parameters string of the `Execute` about to run (taken by `report_body`).
    pub(crate) execute_parameters: Option<String>,
    /// Report variables whose `RunModal` ran to its end: a second `RunModal` and a `RunRequestPage` on one raise
    /// ObjectHasAlreadyBeenRun (see `reports.rs`, `REPORT_RUN_1 | REPORT_RUNMODAL_1`).
    pub(crate) report_run_modal_done: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// Report variables whose `UseRequestPage(false)`, as their first use, ran an OnInitReport that quit: their next Run or
    /// RunModal runs nothing (`reports/quit_in_init.rs`).
    pub(crate) report_quit_deferred: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// The `ExcelLayoutMultipleDataSheets = true` admission of the running report call (`reports/excel_sheets.rs`: set and
    /// restored around each report call that runs a report or reads its layout).
    pub(crate) excel_sheets: crate::reports::ExcelSheetsAdmission,
    /// Report instances whose Run, RunModal or Execute is running now, innermost last (`RunningReport`).
    pub(crate) report_running: Vec<RunningReport>,
    /// `RequestOptionsPage.Caption` set by the code of a report outside its request page's OnOpenPage, by report instance: the
    /// getter reads it back (see `reports.rs`, `request_page_caption`).
    pub(crate) request_captions: Vec<(std::rc::Weak<RefCell<Instance>>, String, Option<String>)>,
    /// `CurrReport.PrintOnlyIfDetail` of the running processing-only report: set by its setter, cleared when a run starts.
    pub(crate) print_only_if_detail: bool,
    /// The Blob buffer the last `Query.SaveAsCsv` wrote, and its length then (see `queries.rs`, `query_save_csv`).
    pub(crate) csv_written: Option<(al_builtins::stream::BlobRef, usize)>,
    /// Report variables a `SaveAs` was called on: a second `SaveAs` on one whose first ended before it was marked
    /// executed is refused (what BC does then is not pinned).
    pub(crate) report_save_tried: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// Report variables a procedure of the caller initialized before their run (`SetParameters` and the like).
    pub(crate) prepared: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// For each of those: the global slots the procedures the caller ran on it can write (`Session::note_caller_writes`).
    pub(crate) caller_writes: Vec<(std::rc::Weak<RefCell<Instance>>, std::collections::HashSet<usize>)>,
    /// Report variables whose request page OnInit ran at a procedure's first use (`Session::first_use`): the run does not
    /// run it again.
    pub(crate) rp_init_done: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// The report instances whose request page OnInit ran before their `[ReportHandler]` was called: a run the handler starts
    /// does not run it again (reports/reporthandler/AeReportHandlerCalls.al:83 Handler_RunsReport).
    pub(crate) rp_init_by_handler: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// For each report body running, innermost last: how it was started, which says what `CurrReport.Preview()` answers.
    pub(crate) report_preview_pinned: Vec<ReportStart>,
    /// The report running now was started by `RunModal` (its request page's Preview and Print are not pinned).
    pub(crate) run_modal: bool,
    /// The report running now was started by the static `Report.Run` or `Report.RunModal` with the report id as its only
    /// argument and not swapped by SubstituteReport (`run_started_as`): not on a report variable, not another overload.
    pub(crate) run_bare: bool,
    /// The report running now was started with SystemPrinter `true` (`Report.Run(Id, .., true)`): a run that renders would
    /// print (`report_placeholder::run`).
    pub(crate) system_printer: bool,
    /// The filters each data item record got from a `SetTableView`: the request page shows those and not the filters a procedure
    /// of the report set on the item. Held by a `Weak` of the record and not by its address: a `Weak` keeps the allocation, so no
    /// later record can get the address of one that is gone and read its view (al-runner CI flake of reports/request-page/
    /// RptFilters.al:510 RequestFilterFields_Handler_SetsFilters). The entries of dropped records go on each new SetTableView.
    pub(crate) set_view_filters: Vec<(std::rc::Weak<RefCell<RecordState>>, Vec<ViewFilterKey>)>,
    /// The filters each `DataItemTableView` gave the record of a data item of the report's own, by record
    /// (`reports/item_view.rs`).
    pub(crate) item_views: Vec<crate::reports::item_view::ItemView>,
    /// The dataset a `Report.SaveAs(.., Xml, ..)` run is writing (al-vm reports.rs).
    pub(crate) dataset: Option<crate::dataset::Dataset>,
}

impl ReportState {
    /// The `SetTableView` filters of `rec`, if it got any.
    pub(crate) fn view_filters_of(&self, rec: &Rc<RefCell<RecordState>>) -> Option<&Vec<ViewFilterKey>> {
        self.set_view_filters
            .iter()
            .find(|(w, _)| std::ptr::eq(w.as_ptr(), Rc::as_ptr(rec)))
            .map(|(_, v)| v)
    }

    /// Records that `rec` got `origin` from a `SetTableView`, replacing its earlier entry and dropping those of records gone.
    pub(crate) fn set_view_filters_of(&mut self, rec: &Rc<RefCell<RecordState>>, origin: Vec<ViewFilterKey>) {
        self.set_view_filters
            .retain(|(w, _)| w.strong_count() > 0 && !std::ptr::eq(w.as_ptr(), Rc::as_ptr(rec)));
        self.set_view_filters.push((Rc::downgrade(rec), origin));
    }
}

/// One report run in progress (`ReportState::report_running`).
pub(crate) struct RunningReport {
    pub(crate) inst: std::rc::Weak<RefCell<Instance>>,
    /// Started by Run or RunModal (not Execute): a RunModal of the instance raises ObjectHasAlreadyBeenRun (see `reports.rs`,
    /// `REPORT_RUN_1 | REPORT_RUNMODAL_1`).
    pub(crate) by_run: bool,
    /// Whether this run showed its request page: what `UseRequestPage()` answers in the run's triggers
    /// (`reports/running.rs`, `use_request_page_getter`).
    pub(crate) shows_page: bool,
    /// Whether the run's triggers answer `UseRequestPage()` with `shows_page`: not for a Print run as an Execute (not pinned).
    pub(crate) answers_getter: bool,
    /// The call depth and the number of page triggers running when the run started: a report trigger entered deeper and the
    /// one page trigger above them belong to this run (`running_page_answer`).
    pub(crate) depth: usize,
    pub(crate) page_triggers: usize,
    /// The run entered a transaction world of a TransactionType = Update report: its writes run in that world's
    /// transaction, whatever the session's type (record/TxTypeUpdateReport.al TxU13 to TxU15 of corpus pull request 1488).
    pub(crate) update_world: bool,
}

/// How a running report body was started (`report_preview_pinned`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReportStart {
    /// `Run` or `RunModal`.
    Run,
    /// `Execute`.
    Execute,
    /// `SaveAs` (to a stream or, OnPrem, to a file).
    SaveAs,
    /// `OnInitReport` of a report started by one of the methods above or by `RunRequestPage`.
    Init,
    /// `OnInitReport` run by the first call of a procedure of the report.
    FirstUse,
    /// The request page of a `RunRequestPage`, while it is open.
    RequestOnly,
}

impl OpenTx {
    /// Holds what a new session holds (`install_left.rs`). Every field is named: a new one does not compile here until it is
    /// decided.
    pub(crate) fn is_clear(&self) -> bool {
        let OpenTx {
            rows_touched,
            tables_inserted,
            locked_tables,
            writes,
            reads_after_rollback,
            deferred_insert,
            insert_buffer,
            row_versions,
            inconsistent,
            locks:
                TxLocks {
                    lock_tx,
                    lock_unpinned,
                    recref_lock_tx,
                    isolation_lock,
                    isolation_lock_tables,
                    locking_reads,
                    db_reads,
                    locking_read_tx_unpinned,
                },
            // `Session::start_test_method` sets them before any test code runs.
            moment: _,
            runner_tx: _,
        } = self;
        rows_touched.is_empty()
            && tables_inserted.is_empty()
            && locked_tables.is_empty()
            && writes.is_empty()
            && reads_after_rollback.is_empty()
            && deferred_insert.is_none()
            && insert_buffer.is_empty()
            && row_versions.pending_inserts.is_empty()
            && row_versions.sent == 0
            && row_versions.handed_out == 0
            && row_versions.accounted == 0
            && row_versions.last == 0
            && row_versions.unpinned.is_none()
            && inconsistent.is_empty()
            && !lock_tx
            && lock_unpinned.is_none()
            && !recref_lock_tx
            && !isolation_lock
            && isolation_lock_tables.is_empty()
            && locking_reads.is_empty()
            && *db_reads == 0
            && !locking_read_tx_unpinned
    }
}

impl TxLocks {
    /// A lock opened the write transaction, or a lock whose effect on it is not pinned ran.
    pub(crate) fn any(&self) -> bool {
        self.lock_tx || self.lock_unpinned.is_some()
    }

    /// Notes a table a locking ReadIsolation locked (`isolation_lock_tables`).
    pub(crate) fn note_isolation_lock(&mut self, table: u32, company: Option<String>) {
        let key = (table, company);
        if !self.isolation_lock_tables.contains(&key) {
            self.isolation_lock_tables.push(key);
        }
    }
}

#[cfg(test)]
mod open_tx_tests {
    use super::{TxState, UnpinnedLock};

    #[test]
    fn a_new_session_holds_no_open_transaction_and_a_reset_clears_one() {
        let mut tx = TxState::fresh();
        assert!(tx.open.is_clear());
        tx.open.locked_tables.push(7);
        tx.open.locks.lock_unpinned = Some(UnpinnedLock::LockingRead);
        assert!(!tx.open.is_clear());
        tx.open = Default::default();
        assert!(tx.open.is_clear());
    }

    #[test]
    fn an_unpinned_lock_names_its_cause() {
        assert_eq!(UnpinnedLock::LockingRead.text(), crate::records::UNPINNED_LOCKING_READ);
        assert_eq!(UnpinnedLock::Other("a lock").text(), "a lock");
    }
}
