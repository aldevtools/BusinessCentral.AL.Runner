//! Sub-states of a [`Session`] (ADR-037 item 3): fields that belong together, so `Session` stays readable.

use super::*;

/// Transactions: the depth of transaction-beginning constructs, and where the open transaction stands.
pub(crate) struct TxState {
    /// Nesting depth of constructs that begin a transaction of their own (`Codeunit.Run`,
    /// a TestPage operation); a `TransactionModel::None` test may write inside them.
    pub(crate) tx_world: usize,
    /// The `[CommitBehavior]` of the innermost procedure that declares one.
    pub(crate) commit_behavior: transactions::CommitBehavior,
    /// How far the transaction has started since the test method began or the last `Commit()`:
    /// `TX_FRESH` (nothing yet), `TX_READ` (a read call) or `TX_WRITE` (a write call, also a DeleteAll that
    /// finds no row). `CurrentTransactionType` depends on it (`sessionops.rs`).
    pub(crate) tx_phase: u8,
    /// `Database.CurrentTransactionType`: a member index of the TransactionType option; a test
    /// method starts at 0, UpdateNoLocks (`start_test_method`).
    pub(crate) transaction_type: i32,
    /// `Database.LockTimeout`: whether a lock wait ends in a timeout error.
    pub(crate) lock_timeout: bool,
    /// `Database.LockTimeoutDuration` in milliseconds.
    pub(crate) lock_timeout_duration: i64,
    /// The highest rowversion handed out when the last `Commit` ran: rows at or below it are durable.
    pub(crate) committed_row_version: i64,
    /// `committed_row_version` before the last `Commit`: a row whose committed version is above it was written in the
    /// transaction the last Commit closed (`Session::stale_verdict`, al-runner issue 6522).
    pub(crate) prior_committed_row_version: i64,
    /// The rows (table object, company, primary key) the Record writes of the transaction the last Commit closed wrote, or
    /// `None` when not every write of it was noted.
    pub(crate) closed_tx_rows: Option<Vec<RowKey>>,
    /// `closed_tx_rows` before the last Commit: the rows written by the transaction the Commit before it closed
    /// (`Session::stale_verdict`, al-runner issue 6522, three Commits).
    pub(crate) closed_tx_rows_before: Option<Vec<RowKey>>,
    /// The rows (table object, company, primary key) a Record Modify wrote, or a Get read while its table was locked by
    /// `LockTable`, since the last Commit or rollback: a stale buffer from the Insert of the transaction before may then be
    /// written (record/TestRecordStaleBufferWrite.al:495-584, `Session::stale_verdict`).
    pub(crate) rows_touched: Vec<(u32, Option<String>, Vec<Value>)>,
    /// The tables (table object, company) a Record Insert wrote a row of since the last Commit or rollback (`Session::stale_verdict`).
    pub(crate) tables_inserted: Vec<(u32, Option<String>)>,
    /// The tables a `Record.LockTable` locked since the last Commit or rollback.
    pub(crate) locked_tables: Vec<u32>,
    /// The database store's change counter when the last Commit or rollback ran: with `writes` it tells whether every write
    /// since was a Record Insert, Modify or Delete (`Session::stale_verdict`, al-runner issue 6028).
    pub(crate) commit_counter: u64,
    /// The Record writes to the database store since the last Commit or rollback, in order (`Session::stale_verdict`).
    pub(crate) writes: Vec<TxWrite>,
    /// The tables (table object, company) a Record FindSet read since the last Commit or rollback (`Session::stale_verdict`).
    pub(crate) find_set_tables: Vec<(u32, Option<String>)>,
    /// A locking ReadIsolation (a FindFirst under RepeatableRead or UpdLock) set `lock_tx` in the open transaction.
    pub(crate) isolation_lock: bool,
    /// A rollback ran since the last Commit.
    pub(crate) rolled_back: bool,
    /// The last `Commit` made writes durable (the transaction it ended had written): `MinimumActiveRowVersion` is then not pinned.
    pub(crate) last_commit_wrote: bool,
    /// A duplicate-key Insert into a table without a Blob field does not fail where it is
    /// called: the table (object index) and the error come out at the next access to the table
    /// or at Commit (corpus record/TestRecordRuntimeTextsAndViews.al:
    /// Record_Insert_DuplicateKey_WithoutBlobField_IsDeferred_AndSurfacesAtTheNextAccess).
    pub(crate) deferred_insert: Option<(usize, AlError)>,
    /// How many deferred duplicate-key Inserts have come out so far (a TestPage action wraps the code of one its own
    /// Insert left, `action_invoke`).
    pub(crate) deferred_raised: u32,
    /// Set while the page client or an XmlPort import writes a record: their duplicate-key
    /// errors are raised at once (their own tests pin it), not deferred like an AL `Insert`.
    pub(crate) immediate_errors: bool,
    /// Try functions on the call stack (Truncate is refused inside them).
    pub(crate) try_depth: usize,
    /// The TryFunction calls on the call stack whose result is used, innermost last: a write inside one fails with
    /// TryFunctionWrite (`transactions::try_write_error`).
    pub(crate) used_tries: Vec<transactions::UsedTry>,
    /// What `Database.LastUsedRowVersion` reports inside the open transaction (`Session::last_used_row_version`): the
    /// database tables an Insert wrote rows of that nothing has sent to the database yet (table, company, highest row
    /// version), the highest row version a sent write holds, how many database row versions were handed out and how
    /// many of them an Insert or Modify accounts for, and why the value is not pinned.
    pub(crate) row_versions: RowVersionTx,
    /// The transaction world of an XmlPort export started inside a TryFunction whose result is used: its triggers' Insert and
    /// Modify are not writes of the try (xmlport/lt-y-xmlport-props/YXmlPortProps.al:861); other writes there are refused.
    pub(crate) xp_export_try_world: Option<usize>,
    /// The tables (object indexes) `Consistent(false)` marked in the open transaction: `Commit` fails while one is marked.
    pub(crate) inconsistent: Vec<u32>,
    /// A lock started the open write transaction: `LockTable` on a database table, `ReadIsolation := UpdLock`, or a FindFirst
    /// under RepeatableRead or UpdLock (codeunit/TestCodeunitRunWriteTransaction.al:211-278). A Commit ends it.
    pub(crate) lock_tx: bool,
    /// A lock whose effect on the write transaction is not pinned ran since the last Commit (a RecordRef.LockTable, a
    /// RepeatableRead set without a read, another read than FindFirst under a locking ReadIsolation): while no pinned write
    /// or lock opened the transaction, what depends on it is refused (`Session::in_write_transaction`).
    pub(crate) lock_unpinned: Option<&'static str>,
    /// A `RecordRef.LockTable` on a database table ran since the last Commit: the write transaction is open
    /// (recordref/lt-wc/WcRecordRefLockTable.al:28-51), while what else it changes stays refused through `lock_unpinned`.
    pub(crate) recref_lock_tx: bool,
    /// The tables (object index, company) a locking ReadIsolation locked since the last Commit: UpdLock set on a database
    /// table, or a read under RepeatableRead or UpdLock.
    pub(crate) isolation_lock_tables: Vec<(u32, Option<String>)>,
    /// A read under ReadIsolation ReadUncommitted ran since the last Commit: it opens no write transaction under the
    /// assumption `readuncommitted-read-opens-no-write-transaction`.
    pub(crate) read_uncommitted_read: bool,
    /// A read under UpdLock or RepeatableRead whose effect on the write transaction is not pinned ran since the last Commit
    /// (a FindFirst, FindSet, FindLast or another Find under an isolation set in an earlier transaction): `lock_unpinned`
    /// then refuses IsInWriteTransaction. Without it, the pinned reads that set `lock_unpinned` open no write transaction.
    pub(crate) locking_read_tx_unpinned: bool,
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
    /// A ModifyAll, DeleteAll, Truncate or another Rename: its own writes are not told apart.
    Other,
}

/// The row versions of the open transaction as `Database.LastUsedRowVersion` sees them (`TxState::row_versions`).
#[derive(Clone, Debug, Default)]
pub(crate) struct RowVersionTx {
    pub(crate) pending_inserts: Vec<(u32, Option<String>, i64)>,
    pub(crate) sent: i64,
    pub(crate) handed_out: u64,
    pub(crate) accounted: u64,
    pub(crate) last: i64,
    pub(crate) unpinned: Option<&'static str>,
}

/// A row by table object, company and primary key.
pub(crate) type RowKey = (u32, Option<String>, Vec<Value>);

/// One Record write to the database store: the store's change counter before and after it.
#[derive(Clone, Debug)]
pub(crate) struct TxWrite {
    pub(crate) table: u32,
    pub(crate) company: Option<String>,
    pub(crate) kind: TxWriteKind,
    pub(crate) pk: Vec<Value>,
    pub(crate) before: u64,
    pub(crate) after: u64,
}

impl TxState {
    /// The transaction state of a session that has just started: no transaction, nothing locked.
    pub(crate) fn fresh() -> TxState {
        TxState {
            tx_world: 0,
            commit_behavior: Default::default(),
            tx_phase: crate::transactions::TX_FRESH,
            transaction_type: 0,
            lock_timeout: true,
            lock_timeout_duration: 10,
            committed_row_version: 0,
            prior_committed_row_version: 0,
            closed_tx_rows: None,
            closed_tx_rows_before: None,
            rows_touched: Vec::new(),
            tables_inserted: Vec::new(),
            locked_tables: Vec::new(),
            commit_counter: 0,
            writes: Vec::new(),
            find_set_tables: Vec::new(),
            isolation_lock: false,
            rolled_back: false,
            last_commit_wrote: false,
            deferred_insert: None,
            deferred_raised: 0,
            immediate_errors: false,
            try_depth: 0,
            used_tries: Vec::new(),
            row_versions: RowVersionTx::default(),
            xp_export_try_world: None,
            inconsistent: Vec::new(),
            lock_tx: false,
            lock_unpinned: None,
            recref_lock_tx: false,
            isolation_lock_tables: Vec::new(),
            read_uncommitted_read: false,
            locking_read_tx_unpinned: false,
        }
    }
}

/// Handler functions and notifications of the running test method.
pub(crate) struct UiState {
    /// Handler functions bound to the running test method (None outside one).
    pub(crate) scope: Option<ui::UiScope>,
    /// Page handler functions running right now (`[PageHandler]`, `[ModalPageHandler]`).
    pub(crate) page_handler_depth: usize,
    /// Ids of the notifications a `[SendNotificationHandler]` took in this session.
    pub(crate) sent_notifications: Vec<[u8; 16]>,
}

/// What the TestPage client keeps between calls.
pub(crate) struct PageClients {
    /// The TestPage variables whose `Trap()` waits for the next plain `Page.Run` of their page (ADR-012). Several wait at
    /// once, one per page: a test traps the preview page and, inside it, the entries page its action opens.
    /// The variable-bound control values each SaveValues page stored when it closed: (owner, global slot, value), the owner
    /// being the page extension whose variable it is, `None` for the page's own.
    pub(crate) saved_values: std::collections::HashMap<u32, Vec<(Option<usize>, usize, Value)>>,
    /// `saved_values` at the last commit: a rollback goes back to it (`Session::rollback_transaction`).
    pub(crate) saved_values_committed: std::collections::HashMap<u32, Vec<(Option<usize>, usize, Value)>>,
    pub(crate) traps: Vec<Rc<RefCell<al_builtins::value::PageState>>>,
    /// Open TestPages in a state whose end of scope is not pinned (what BC saves when the variable goes away): a routine
    /// whose local holds the last reference to one is refused when it returns.
    pub(crate) scope_unpinned: Vec<Rc<RefCell<al_builtins::value::PageState>>>,
    /// A page background task worker is running: its parameters and the result it set.
    pub(crate) bg_worker: Option<pages::BgWorker>,
    /// The page whose enqueued background tasks run once the `EnqueueBackgroundTask` call that queued them has
    /// written its by-reference task id back (see `Session::bg_enqueue`).
    pub(crate) bg_drain_after_call: Option<Rc<RefCell<al_builtins::value::PageState>>>,
    /// The xRec fields the next `Modify` hands the table's triggers instead of the stored row: a
    /// page saving a row passes its own xRec (corpus
    /// handlers/TestPageActionRunObjectPageRec_Tests.al:HostSaveAfterTheTargetEditedTheRowCarriesXRec).
    pub(crate) page_xrec: Option<Vec<Value>>,
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
    /// The open that starts next is a TestPage's own OpenEdit, OpenView or OpenNew (taken by `open_page_as`).
    pub(crate) test_open: bool,
    /// Set by a FindFirst or FindLast that found a row on Rec in the OnOpenPage of a page an action opened through
    /// RunPageLink (`records.rs`); taken by `open_page_as` into `PageState::run_page_link_open_moved`.
    pub(crate) run_page_link_open_moved: bool,
    /// The close that starts next is a TestPage's own Close() (taken by `close_with`).
    pub(crate) test_close: bool,
    /// The TestPage calls running now, innermost last: the root page of the call's page tree and what the pages' own code
    /// did to that tree meanwhile (`pages/shown.rs`).
    pub(crate) shown_frames: Vec<(
        *const RefCell<al_builtins::value::PageState>,
        Vec<crate::pages::ShownNote>,
    )>,
    /// Per page object, the expression controls whose value the page keeps between refreshes (`pages/shown.rs`).
    pub(crate) shown_controls: std::collections::HashMap<u32, Rc<Vec<usize>>>,
}

/// Event subscriptions and what the Event Subscription table reports.
pub(crate) struct EventState {
    /// Manual event subscriptions made by `BindSubscription`.
    pub(crate) bindings: Vec<dispatch::Binding>,
    /// What the Event Subscription table reports (calls handled, bound instances).
    pub(crate) event_stats: Rc<platform::EventStats>,
    /// `publish` is about to raise its event: the next `raise` has an event frame on the stack.
    pub(crate) publish_frame: bool,
    /// Set by `publish` for an IncludeSender event whose sender is not pinned: a subscriber that asks for it is refused with this text.
    pub(crate) sender_refused: Option<String>,
    /// Set by `finish_call` for a publisher: per argument, the global of the caller's instance a `var` argument names
    /// (instance, global slot). `publish` hands it to `raise` and the subscribers' frames alias those globals.
    pub(crate) var_origins: Vec<Option<(Rc<RefCell<crate::Instance>>, usize)>>,
    /// Set just before a subscriber is called, taken by its frame: (parameter slot, instance, global slot).
    pub(crate) next_aliases: Vec<(usize, Rc<RefCell<crate::Instance>>, usize)>,
    /// Set just before a subscriber is called, taken by its frame: the call is an event subscriber dispatch.
    pub(crate) next_is_subscriber: bool,
    /// The event subscriber calls on the stack, innermost last: (publisher object, subscriber object, subscriber routine) for
    /// the shapes whose event recursion error text is pinned, `None` for another (the call-depth error names event
    /// recursion when any is on the stack).
    pub(crate) subscriber_stack: Vec<Option<(usize, usize, usize)>>,
    /// Set by `finish_call` just before the call, taken by the callee's frame (`Frame::var_lens`).
    pub(crate) next_var_lens: Vec<Option<i32>>,
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
    /// What the subscribers of `Global Triggers` answered, per table object (globaltriggers.rs).
    pub(crate) global_setups: std::collections::HashMap<usize, globaltriggers::GlobalSetup>,
}

/// A filter of a SetTableView as (field, filter group, filter text).
pub(crate) type ViewFilterKey = (usize, i32, String);

/// Code coverage recording.
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
}

/// `Language` and `FormatRegion` set on a report variable outside a run.
pub(crate) struct VariableCulture {
    pub(crate) inst: std::rc::Weak<std::cell::RefCell<al_builtins::value::Instance>>,
    pub(crate) language: Option<i32>,
    pub(crate) region: Option<String>,
    pub(crate) spent: bool,
}

/// The report being run: language, trigger flow, dataset.
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
    /// (report/TestProbeReportContracts.al:RequestPage_TableWriteInsideHandler).
    pub(crate) request_only_page: bool,
    /// The run failed on a DataItemTableView `const(0D)` (`report_prepare`): a SaveAs whose answer is used keeps that error
    /// as the last error (report/lt-cj-report-leads/CjDataset.al:190-211).
    pub(crate) zero_date_view_failed: bool,
    pub(crate) report_executed: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// The parameters string of the `Execute` about to run (taken by `report_body`).
    pub(crate) execute_parameters: Option<String>,
    /// Report variables whose `RunModal` ran to its end: a second `RunModal` and a `RunRequestPage` on one raise
    /// ObjectHasAlreadyBeenRun (see `reports.rs`, `REPORT_RUN_1 | REPORT_RUNMODAL_1`).
    pub(crate) report_run_modal_done: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// `CurrReport.PrintOnlyIfDetail` of the running processing-only report: set by its setter, cleared when a run starts.
    pub(crate) print_only_if_detail: bool,
    /// The Blob buffer the last `Query.SaveAsCsv` wrote, and its length then (see `queries.rs`, `query_save_csv`).
    pub(crate) csv_written: Option<(al_builtins::stream::BlobRef, usize)>,
    /// Report variables a `SaveAs` was called on: a second `SaveAs` on one whose first ended before it was marked
    /// executed is refused (what BC does then is not pinned).
    pub(crate) report_save_tried: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// Report variables a procedure of the caller initialized before their run (`SetParameters` and the like).
    pub(crate) prepared: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// Report variables whose request page OnInit ran at a procedure's first use (`Session::first_use`): the run does not
    /// run it again.
    pub(crate) rp_init_done: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// The report instances whose request page OnInit ran before their `[ReportHandler]` was called: a run the handler starts
    /// does not run it again (report/lt-ae-reporthandler/AeReportHandlerCalls.al:83 Handler_RunsReport).
    pub(crate) rp_init_by_handler: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// For each report body running, innermost last: how it was started, which says what `CurrReport.Preview()` answers.
    pub(crate) report_preview_pinned: Vec<ReportStart>,
    /// The report running now was started by `RunModal` (its request page's Preview and Print are not pinned).
    pub(crate) run_modal: bool,
    /// The filters each data item record got from a `SetTableView`: the request page shows those and not the filters a procedure
    /// of the report set on the item. Held by a `Weak` of the record and not by its address: a `Weak` keeps the allocation, so no
    /// later record can get the address of one that is gone and read its view (al-runner CI flake of report/lt-report-requestpage/
    /// RptFilters.al:510 RequestFilterFields_Handler_SetsFilters). The entries of dropped records go on each new SetTableView.
    pub(crate) set_view_filters: Vec<(std::rc::Weak<RefCell<RecordState>>, Vec<ViewFilterKey>)>,
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

impl TxState {
    /// Notes a table a locking ReadIsolation locked (`isolation_lock_tables`).
    pub(crate) fn note_isolation_lock(&mut self, table: u32, company: Option<String>) {
        let key = (table, company);
        if !self.isolation_lock_tables.contains(&key) {
            self.isolation_lock_tables.push(key);
        }
    }
}
