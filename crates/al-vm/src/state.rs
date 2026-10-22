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
    /// The tables (object indexes) `Consistent(false)` marked in the open transaction: `Commit` fails while one is marked.
    pub(crate) inconsistent: Vec<u32>,
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
    /// The variable-bound control values each SaveValues page stored when it closed.
    pub(crate) saved_values: std::collections::HashMap<u32, Vec<(usize, Value)>>,
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
    /// The open that starts next is a TestPage's own OpenEdit, OpenView or OpenNew (taken by `open_page_as`).
    pub(crate) test_open: bool,
    /// The close that starts next is a TestPage's own Close() (taken by `close_with`).
    pub(crate) test_close: bool,
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

/// The report being run: language, trigger flow, dataset.
pub(crate) struct ReportState {
    /// `Report.Run` on a variable: the variable gets a new instance when the run ends and the old one keeps its state for whoever holds it.
    pub(crate) rebind_after_run: bool,
    /// `CurrReport.Language` of the running report (`reports.rs`); `None` until a trigger sets it.
    pub(crate) curr_report_language: Option<i32>,
    /// `CurrReport.FormatRegion` of the report started by `Run` or `RunModal` (empty text until a trigger sets it).
    pub(crate) format_region: Option<String>,
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
    /// Report variables a `SaveAs` was called on: a second `SaveAs` on one whose first ended before it was marked
    /// executed is refused (what BC does then is not pinned).
    pub(crate) report_save_tried: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// Report variables a procedure of the caller initialized before their run (`SetParameters` and the like).
    pub(crate) prepared: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// Report variables whose request page OnInit ran at a procedure's first use (`Session::first_use`): the run does not
    /// run it again.
    pub(crate) rp_init_done: Vec<std::rc::Weak<RefCell<Instance>>>,
    /// For each report body running, innermost last: whether `CurrReport.Preview()` is pinned to be
    /// false for it (it was started by `Run` or `RunModal`).
    pub(crate) report_preview_pinned: Vec<bool>,
    /// The filters each data item record got from a `SetTableView` (keyed by the record's address, as text): the request page shows
    /// those and not the filters a procedure of the report set on the item.
    pub(crate) set_view_filters: Vec<(usize, Vec<ViewFilterKey>)>,
    /// The dataset a `Report.SaveAs(.., Xml, ..)` run is writing (al-vm reports.rs).
    pub(crate) dataset: Option<crate::dataset::Dataset>,
}
