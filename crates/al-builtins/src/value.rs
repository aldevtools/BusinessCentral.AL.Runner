//! Runtime values (ADR-010).

use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::Rc;

/// The members of the system option `Action`, in docs order (none of the corpus reads an
/// ordinal: docs/probes-needed.md).
pub const ACTION_MEMBERS: [&str; 9] = [
    "None",
    "OK",
    "Cancel",
    "LookupOK",
    "LookupCancel",
    "Yes",
    "No",
    "RunObject",
    "RunSystem",
];

/// The members of the system option `PromptMode` of a PromptDialog page, in docs order.
pub const PROMPT_MODE_MEMBERS: [&str; 3] = ["Prompt", "Content", "Generate"];

/// Runtime type of a slot: what a variable, field or parameter holds.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum RtType {
    Boolean,
    Integer,
    BigInteger,
    Decimal,
    Byte,
    Char,
    Text(Option<u32>),
    Code(Option<u32>),
    Date,
    Time,
    DateTime,
    Duration,
    Guid,
    DateFormula,
    Option(Vec<String>),
    Enum(u32),
    Variant,
    List(Box<RtType>),
    Dictionary(Box<RtType>, Box<RtType>),
    Array(Vec<u32>, Box<RtType>),
    /// Record of the table with this object index; temporary flag.
    Record(u32, bool),
    /// Codeunit with this object index; also an xmlport variable, which is an instance with
    /// its own globals in the same way.
    Codeunit(u32),
    /// `TestPage` of the page with this object index (ADR-012).
    TestPage(u32),
    /// A `Page` variable of the page with this object index: a page instance that exists before
    /// it is run (ADR-012).
    Page(u32),
    /// A `Query` variable of the query with this object index.
    Query(u32),
    /// An `Interface` variable of the interface with this object index. It holds the
    /// codeunit instance it was assigned (or `Null` while unassigned).
    Interface(u32),
    /// Library types whose values are opaque handles (`JsonObject`, `TextBuilder`, ...).
    Library(crate::library_kind::LibraryKind),
    /// A `DotNet` variable declared through the alias at this index of `Program::dotnet_types`
    /// (ADR-014). Unassigned, it holds `Value::Null`.
    DotNet(u32),
}

/// The items of a List: a `Vec<Value>` that counts the times it was borrowed mutably, so that a `foreach` over it notices
/// a change (collections/RefSemantics.al:Foreach_ListAddWhileIterating, ...).
#[derive(Debug, Clone, Default)]
pub struct ListVec {
    items: Vec<Value>,
    pub version: u64,
    /// Reads by List methods whose effect on a `foreach` over the list is not pinned (`collections.rs`, `read_list_unpinned`):
    /// a foreach that sees this change refuses (`exec/run.rs`).
    pub unpinned_reads: u64,
    /// The list `Enum.Ordinals()` and `Enum.Names()` answer: every change raises NotSupportedOperation
    /// (collections/lt-cs-odd-leads/CsCollections.al:EnumOrdinals_Mutations, EnumNames_Mutations).
    pub read_only: bool,
}

impl From<Vec<Value>> for ListVec {
    fn from(items: Vec<Value>) -> Self {
        ListVec {
            items,
            version: 0,
            unpinned_reads: 0,
            read_only: false,
        }
    }
}

impl FromIterator<Value> for ListVec {
    fn from_iter<I: IntoIterator<Item = Value>>(iter: I) -> Self {
        ListVec::from(iter.into_iter().collect::<Vec<_>>())
    }
}

impl std::ops::Deref for ListVec {
    type Target = Vec<Value>;
    fn deref(&self) -> &Vec<Value> {
        &self.items
    }
}

impl std::ops::DerefMut for ListVec {
    fn deref_mut(&mut self) -> &mut Vec<Value> {
        self.version += 1;
        &mut self.items
    }
}

#[derive(Debug, Clone)]
pub enum Value {
    /// An unassigned Variant.
    Null,
    Boolean(bool),
    Integer(i32),
    BigInteger(i64),
    Decimal(Decimal),
    Byte(u8),
    Char(u16),
    Text(crate::text::Txt),
    Code(crate::text::Txt),
    /// Day number (0 = undefined `0D`), closing-date flag.
    Date(i32, bool),
    /// Milliseconds since midnight plus one (0 = undefined `0T`).
    Time(i64),
    /// Milliseconds since 0001-01-01 UTC plus one (0 = undefined `0DT`).
    DateTime(i64),
    /// Milliseconds.
    Duration(i64),
    Guid([u8; 16]),
    DateFormula(String),
    /// Ordinal and the option's member names (needed by Format).
    Option(i32, std::sync::Arc<[String]>),
    Enum(u32, i32),
    List(Rc<RefCell<ListVec>>),
    Dictionary(Rc<RefCell<Vec<(Value, Value)>>>),
    Array(Rc<RefCell<Vec<Value>>>),
    Codeunit(Rc<RefCell<Instance>>),
    Library(crate::library_kind::LibraryKind, Rc<RefCell<LibraryValue>>),
    /// A record variable (ADR-010: a handle; `:=` copies fields, see the VM).
    Record(Rc<RefCell<RecordState>>),
    /// A Blob field's content: a shared buffer that streams write to (see `stream`).
    Blob(crate::stream::BlobRef),
    /// A `TestPage` (or `TestPart`): a handle on a page and the test client
    /// driving it (ADR-012). Shared like a codeunit instance.
    TestPage(Rc<RefCell<PageState>>),
    /// A control of a test page (`TestField`, `TestAction`, `TestFilter`).
    TestControl(Rc<ControlRef>),
    /// A query variable: its runtime filters and, once opened, its result set.
    Query(Rc<RefCell<QueryState>>),
    /// A `DotNet` object (ADR-014). `:=` shares the reference (assumed BC rule, probe pending).
    DotNet(Rc<DotNetObject>),
    /// A column of a platform table row whose value nothing pins (ADR-013 decision 7, hard rule 3): the
    /// provider leaves it unknown instead of giving it a default. It only ever sits in a record buffer or a
    /// listed row; every read of it is refused with the reason it holds, and writing the field replaces it.
    /// An event argument the platform leaves unknown (RequestPageXml of SubstituteReport for an Html SaveAs) is
    /// the one exception: it sits in a by-value parameter, can be passed on by value, and any other use is refused.
    Unknown(std::sync::Arc<str>),
}

/// What a `Value::DotNet` refers to.
pub enum DotNetKind {
    /// An object held by the host (the sidecar), by handle, with its CLR type name.
    Host { handle: u64, clr_type: String },
    /// A .NET enum value, kept by value with its CLR type so it can go back as an argument.
    Enum {
        clr_type: String,
        name: String,
        ordinal: i64,
    },
    /// An object the runtime provides itself (the registry, ADR-014 section 1).
    Internal {
        type_name: String,
        state: RefCell<Vec<Value>>,
    },
}

/// A `DotNet` object. When the last `Rc` drops, a host handle goes onto the session's release
/// queue; `Drop` never calls the host (ADR-014 section 2).
pub struct DotNetObject {
    pub kind: DotNetKind,
    release: Option<Rc<RefCell<Vec<u64>>>>,
}

impl DotNetObject {
    /// A host object; its handle is queued on `release` when the object drops.
    pub fn host(handle: u64, clr_type: String, release: Rc<RefCell<Vec<u64>>>) -> DotNetObject {
        DotNetObject {
            kind: DotNetKind::Host { handle, clr_type },
            release: Some(release),
        }
    }

    pub fn new(kind: DotNetKind) -> DotNetObject {
        DotNetObject { kind, release: None }
    }
}

impl Drop for DotNetObject {
    fn drop(&mut self) {
        if let (DotNetKind::Host { handle, .. }, Some(queue)) = (&self.kind, &self.release) {
            queue.borrow_mut().push(*handle);
        }
    }
}

impl std::fmt::Debug for DotNetObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.kind {
            DotNetKind::Host { handle, clr_type } => write!(f, "DotNet({clr_type} #{handle})"),
            DotNetKind::Enum { clr_type, name, .. } => write!(f, "DotNet({clr_type}.{name})"),
            DotNetKind::Internal { type_name, .. } => write!(f, "DotNet(internal {type_name})"),
        }
    }
}

/// What a query variable carries (corpus query/). The definition is in the IR; the VM
/// (`al-vm/src/queries.rs`) executes it.
#[derive(Debug, Clone)]
pub struct QueryState {
    /// Object index of the query.
    pub object: u32,
    /// `SetRange` / `SetFilter` per column (position in `ir::Query::columns`); a later call on the
    /// same column replaces the earlier one.
    pub filters: Vec<(usize, crate::filter::FilterExpr)>,
    /// A `SetRange` or `SetFilter` replaced or removed a filter set before, so `filters` no longer
    /// holds the order in which each filter was first set.
    pub filter_replaced: bool,
    /// `TopNumberOfRows(n)`; `None` leaves the property in force.
    pub top: Option<i32>,
    /// `SecurityFiltering(x)` as a member index of the platform option; `None` before any call.
    pub security_filtering: Option<i32>,
    /// The result set between `Open` and `Close`.
    pub run: Option<QueryRun>,
    /// The instance of a query with code: its variables live as long as the query variable
    /// (query/TestQueryOnBeforeOpenVariablesContracts.al:104-140). Made on first use.
    pub instance: Option<Rc<RefCell<Instance>>>,
    /// The query's OnBeforeOpen trigger is running: `CurrQuery.Open` and `Close` raise
    /// (query/lt-ch-query-leads/ChQueryLeads.al:259-290).
    pub in_before_open: bool,
}

#[derive(Debug, Clone)]
pub struct QueryRun {
    /// Whether the query has run. `Open` does not run it: the first `Read` does, so a row written
    /// in between is read (query/TestQueryOuterJoinDescendingSaveAsContracts.al).
    pub started: bool,
    /// A SetRange or SetFilter came after `Open` and before the first `Read`: `Read` raises until the query is opened again
    /// (misc/lt-probes-needed-1/PnAQueryOpenRead.al:SetRangeBetweenOpenAndFirstRead, SetRangeBeforeOpen_ThenChangedBeforeFirstRead).
    pub filters_changed: bool,
    /// The rows still to read, in the query's order.
    pub rows: Vec<QueryRow>,
    pub next: usize,
    /// The column values of the row `Read` returned last (`None` before the first and after the last).
    pub current: Option<Vec<Value>>,
    /// Sort key of the last row returned: after a write the result is read again from there.
    pub last_key: Option<Vec<Value>>,
    /// Rows returned since `Open`, against `TopNumberOfRows`.
    pub returned: usize,
    /// Version of each table the query reads (table object, stamp) when `rows` were read.
    pub stamps: Vec<(u32, u64)>,
}

#[derive(Debug, Clone)]
pub struct QueryRow {
    /// One value per query column (`Null` for filter elements).
    pub columns: Vec<Value>,
    /// The sort key: the `OrderBy` columns, then the primary key of each dataitem.
    pub key: Vec<Value>,
}

/// How a page was opened by the test client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageMode {
    View,
    Edit,
    New,
}

/// A row a repeater has populated: its primary key, the fields as stored when it was read and as
/// the page showed them after OnAfterGetRecord (the page keeps that buffer for the row while it
/// stays in the window, corpus handlers/TestPageActionRunObjectPageStoredRow_Tests.al:
/// OpenHostOnB reads `CALC`, which only the host's OnAfterGetRecord puts into Rec.Grp).
#[derive(Debug, Clone)]
pub struct WindowRow {
    pub key: Vec<Value>,
    pub stored: Vec<Value>,
    pub shown: Vec<Value>,
}

/// Where the test client's cursor stands on a page (ADR-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cursor {
    /// No row: an empty view.
    None,
    /// On a stored row (or, for a new record, the row being typed).
    Row,
    /// On the blank line an editable, insert-allowed repeater carries past its data
    /// (corpus handlers/TestPageNewRowLine_Tests.al:1-30).
    NewLine,
}

/// Why a part reads its rows last to first (see `PageState::read_backwards`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReadBackwards {
    /// The host opens and the part's SubPageView names its linked field with `const`: 2 or 3 rows are pinned.
    AtOpen,
    /// The host moved back with Previous or First: 1, 2 and 3 rows are pinned.
    AfterMove,
}

/// A row key and what the repeater's page-variable controls show for it: (control index, value) (`PageState::row_vars`).
pub type RowVars = (Vec<Value>, Vec<(usize, Value)>);

/// The test client's state for one page (ADR-012). Fields grow with the
/// behaviours the corpus pins.
pub struct PageState {
    /// Object index of the page.
    pub object: u32,
    /// Open pages answer; closed ones, and ones torn down by an error in a row load,
    /// raise "The TestPage is not open." (corpus testpageteardown/TestPageTeardown.al:3-24).
    /// The page has been opened at least once (a Page variable that never ran has not).
    pub ran: bool,
    pub open: bool,
    pub mode: PageMode,
    /// The running page: its globals hold `Rec`, `xRec` and `CurrPage`.
    pub instance: Option<Rc<RefCell<Instance>>>,
    pub cursor: Cursor,
    /// The page offers the trailing new-row line.
    pub new_line: bool,
    /// The cursor stands on a row that is not stored yet (OpenNew, New).
    pub draft: bool,
    /// Fields written by the client since the row was loaded.
    pub dirty: bool,
    /// `Visible` of each control as it was when the page opened (the property is frozen then,
    /// corpus handlers/TestPageControlPropertyLiveness_Tests.al:56).
    pub frozen_visible: Vec<bool>,
    /// The session language when the page opened: OptionCaptionML shows the captions of that language, whatever
    /// `GlobalLanguage` says later (pages/lt-page-controls-aa/AaFieldProps.al:179-219, pageext/lt-z-modify-controls/
    /// ZModifyControls.al:322-360). `None` before the first open.
    pub open_language: Option<i32>,
    /// The caption each control's CaptionClass resolved to when the page filled: (control index, class text, caption).
    /// `Caption()` reads it without raising OnResolveCaptionClass again (corpus misc/W2CaptionClass.al:113-116).
    pub control_captions: Vec<(usize, String, String)>,
    /// The caption of each control over a table field whose CaptionClass is an expression, as the page filled it: (control
    /// index, caption). `Caption()` reads it and runs nothing (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:422-481).
    pub field_captions: Vec<(usize, String)>,
    /// The open ran the CaptionClass expressions of the source table's fields with an empty record (`field_captions`).
    pub field_captions_opened: bool,
    /// The open has filled its first rows (`field_captions` no longer changes at the fills of the open itself).
    pub field_captions_settled: bool,
    /// A `Next()` of the page is filling its row.
    pub moving_next: bool,
    /// A fill after the open (a part refilled after its host moved) runs the CaptionClass expressions with its first row
    /// only, as the open does (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:1022-1066 QO_Doc_OpenEdit_HostSteps).
    pub field_captions_fill_first: bool,
    /// The refill comes from a SetValue of the host's link field: when it finds no row it runs nothing
    /// (QdCaptionClassAreas.al:1068-1100 QO_Doc_OpenNew `no_setvalue`).
    pub field_captions_link_refill: bool,
    /// Where an action of a Document page stands that saved a SetValue of the host before its trigger: 0 none, 1 that save's
    /// refresh is running, 2 the save is done and a CurrPage.Update in the trigger has not refreshed yet, 3 it has
    /// (record/lt-qd-captionclass-areas/QdCaptionClassAreas.al:1015-1049 QO_Doc_OpenEdit_HostSteps `update`).
    pub field_captions_action_save: u8,
    /// Primary keys of the rows a repeater page has populated (its window).
    pub window: Vec<WindowRow>,
    /// The window holds the rows from the first row of the view on, with none missing: the page
    /// opened on its first row, or read on from there. A GoToKey reads on in blocks only then.
    pub window_from_start: bool,
    /// A read has run to the end of the view and read the last two rows once more.
    pub window_eof: bool,
    /// The window holds every row of the view, read by `Last` past a full window (see `last_past_the_window`).
    pub window_whole: bool,
    /// A read of a list that serves its rows from OnFindRecord and OnNextRecord ran to the end of the view (an open whose view
    /// ends inside the window, a GoToKey that read on to the end): Next on the last row then answers false without asking the
    /// page (see `move_in_client_rows`).
    pub view_read_to_end: bool,
    /// The open read of a descending find list went up from the row it opened on, the last row of the view: the window holds
    /// the rows above it in client order and the cursor stands on its last row (see `read_rows_up_from_the_last`).
    pub window_read_up: bool,
    /// The page is landing on its first row as it opens (the reads of a view that ends inside the window differ there).
    pub landing_first: bool,
    /// The page opens on the row its OnOpenPage left Rec on, not the first of the view: a list's open read also reads the
    /// rows above it (see `read_rows_above_the_moved_rec`).
    pub open_reads_above: bool,
    /// The list opened on the row its OnOpenPage left Rec on (see `open_on_rec_moved_by_onopenpage`).
    pub opened_on_moved_rec: bool,
    /// The list opened in a key other than the primary key (key index, ascending) and read its row once more for it; it reads
    /// each row once more at Next and Previous while Rec stays in that key and order (see `reads_once_more_in_its_key`).
    pub reads_once_more_in: Option<(usize, bool)>,
    /// The page is reading the first row it shows as it opens (`load_first_row` from the open, not a later refill).
    pub open_read: bool,
    /// The page opened for edit over an empty view and ran its new-record triggers on a blank record; it shows no row and its
    /// field controls are not editable.
    pub blank_on_open: bool,
    /// The part reads its rows last to first, once each, at its next fill: as it opens with its host (see
    /// `view_names_link_field`), or after its host moved back (see `sync_parts_after_move`).
    pub read_backwards: Option<ReadBackwards>,
    /// `CurrPage.SetSelectionFilter(Rec)` ran on this page while it stood on no row: the part's next refresh with its host
    /// drops the filter (pages/lt-hidden-part-selection-filter/HiddenPartSelectionFilter.al).
    pub selection_on_rec: bool,
    /// The control that has the client's focus, and whether it is on the current row. Moving
    /// focus to a non-key control of a new row is what inserts it (corpus
    /// handlers/TestPageBlankKeyInsert.al:1-25).
    pub focus: Option<usize>,
    pub focus_in_row: bool,
    /// Validation errors the client holds: (control index, stored text).
    pub errors: Vec<(usize, String)>,
    /// The controls among `errors` whose refusal was an error of an AL trigger (the table's or the control's OnValidate),
    /// not one of the client's own checks, with the trigger's message: Close validates those again
    /// (pages/lt-ah-pages/AhPageTests.al:155-159).
    pub trigger_errors: Vec<(usize, String)>,
    /// The started line's insert was refused and the client moved on without raising
    /// (corpus handlers/TestPageFailedInsert_Tests.al:1-30): `refused` keeps the line, `pending`
    /// is the line left behind by Previous()/First(), whose insert fails again at Close().
    pub refused: bool,
    /// The last close was refused because OnQueryClosePage raised an error (shown as a message).
    pub close_error: bool,
    /// The row the page was left on when it closed with LookupOK (`Page.RunModal(Id, Rec)` hands it back to `Rec`).
    pub selection: Option<Vec<Value>>,
    pub pending: Option<Vec<Value>>,
    /// Opened by AL code (`Page.RunModal`, `Page.Run`) and answered by a handler, not by the test.
    pub modal: bool,
    /// Whether the last run of this variable was modal (`None` before the first run): a second non-modal Run runs OnInit again.
    pub last_run_modal: Option<bool>,
    /// The rows of a tree (a repeater with ShowAsTree) expanded with `Expand(true)`, by primary key.
    pub expanded: Vec<Vec<Value>>,
    pub lookup: bool,
    /// The `Action` the page closed with, once it has.
    pub result: Option<String>,
    /// `TestPage.Trap()` was called on this variable and no page has been caught yet.
    pub trapping: bool,
    /// `CurrPage.Update` was called: the client re-reads the row once the call that made it returns.
    pub refresh: bool,
    /// `CurrPage.SaveRecord` ran in the action being invoked.
    pub save_record_called: bool,
    /// A SetValue on a control of this page is running its validation (CurrPage.SaveRecord inside it keeps xRec).
    pub validating_control: bool,
    /// The next refresh reads the row (OnAfterGetRecord) without OnAfterGetCurrRecord: a card action that only saved.
    pub refresh_without_curr: bool,
    /// The next refresh does not make the parts read their rows again (they just did).
    pub refresh_without_parts: bool,
    /// The row's read keeps xRec as it was while its triggers run (a host re-read after a part's Update).
    pub read_keeps_xrec: bool,
    /// The row CurrPage.SaveRecord stored inside a control's validation: it becomes xRec once the SetValue returns.
    pub xrec_after_validate: Option<Vec<Value>>,
    /// CurrPage.Update ran while the page's row was stored but outside the view the action had just set (list refresh).
    pub view_left: bool,
    /// A TestPage GoToKey moved the cursor of a list without OnAfterGetCurrRecord: the client reads the row once more before
    /// every later action (pages/lt-ai-list-order/AiListOrderTests.al:39 Keep_Nav `update` = `R:R1002;R:R1002;A:filter...`;
    /// QgGoToKeyNextCall.al:297 Plain_GoToKey_ThenActionWithoutUpdate_Twice `second_action_log`), until a move clears it.
    pub read_after_goto: bool,
    /// A call that is not pinned (a property read, an `As*` read) came between that GoToKey and the next call: whether the row
    /// is still read again before an action is not pinned (the action is refused). A Value read and SetValue are pinned
    /// (QgGoToKeyNextCall.al:236, :317) and do not set it.
    pub read_after_goto_doubt: bool,
    /// That GoToKey ran no page trigger (the key is among the rows the client holds): Close then reads the row once
    /// (pages/lt-ai-list-order/QgGoToKeyNextCall.al:223 Plain_GoToKey_ThenValueRead, :277 Plain_GoToKey_ThenClose `close_log`
    /// `R:R1003`). A GoToKey that read rows leaves Close reading nothing (handlers/TestPageSystemPartListWindowContracts.al:219).
    pub read_after_goto_quiet: bool,
    /// A SetValue came between that GoToKey and the next call.
    pub read_after_goto_set: bool,
    /// A Previous came after that GoToKey and no action since: what Close then reads is not pinned (it is refused).
    pub read_after_goto_prev: bool,
    /// A list opened for edit over an empty view went back to its new-row line after an action stored one row through another
    /// variable: First then reads that row as the list reads it on open (misc/lt-probes-needed-7/PnGShowRow.al:223).
    pub empty_list_row_behind: bool,
    /// An action put a list that showed stored rows on a row the client did not hold before it (an insert through Rec): what a
    /// move reads next is not pinned (misc/lt-probes-needed-7/PnGShowRow.al:144 NonEmptyListInsertViaRec `first-after` reads
    /// nothing, the Next after it reads the row it lands on twice); a move is refused.
    pub action_landed_on_new_row: bool,
    /// The page was run by the static `Page.RunModal` with its return value used (lookup mode set by the call, not by
    /// `LookupMode(true)` on a page variable).
    pub static_lookup: bool,
    /// Page triggers (the page's or an extension's) run on this page so far.
    pub triggers_run: u64,
    /// The reads after an action on a list without OnAfterGetCurrRecord ended the refresh: `CurrPage.Update` reads nothing more
    /// (pages/lt-ai-list-order/AiListOrderTests.al:233 Keep_Plain).
    pub refresh_read_done: bool,
    /// The fields of the row a list showed when the running action started (None outside an action): an action that put another
    /// row into Rec and called CurrPage.Close() gets this row back (misc/lt-probes-needed-7/PnGRefresh.al:608).
    pub action_shown_row: Option<Vec<Value>>,
    /// The action left another row (another primary key) in Rec: the list reads that row again from the table, unsaved values
    /// dropped (misc/lt-probes-needed-7/PnGRefresh.al:387 QL_RecAssign_OtherChanged_ThenClose `P.AGR(B,changed B);P.AGR(B,orig B)`).
    pub rec_moved_other: bool,
    /// CurrPage.Close() in the running action put the shown row back into Rec (PnGRefresh.al:608): one more read follows the action.
    pub close_restored_row: bool,
    /// The blank line of a top-level list took key filter values (a journal filtered to one template and batch, open on no row or
    /// after its only row went): its first line is numbered 10000 by AutoSplitKey, not 20000 (misc/lt-probes-needed-7/
    /// PnGRefresh.al:672, :792 `P.INSR(T1/B1/10000,typed)`).
    pub split_starts_low: bool,
    /// The action changed the row a list shows: it is saved after the first read of the refresh (misc/lt-probes-needed-7/
    /// PnGRefresh.al:503).
    pub save_after_first_read: bool,
    /// The parts read their rows for a host row a GoToKey walked past (pages/lt-qj-part-propagation/QjPartPropagation.al:1183):
    /// on the target row they read again even when the host stayed on it.
    pub walk_synced_parts: bool,
    /// xRec of a part that shows no row as it landed on its blank line: OnNewRecord sees it again on the part's Last() and New()
    /// (pages/lt-qj-part-propagation/QjPartPropagation.al:1346 QL_PartLastNew_NoLines `x=3 x.ln=30000`, the row the part stood on
    /// for the host row before).
    pub empty_part_xrec: Option<Vec<Value>>,
    /// The part stands on the blank line its Last() ran on a view without rows: a part without DelayedInsert and AutoSplitKey
    /// inserts that line at the first write to a field outside the key (pages/lt-qj-part-propagation/QjPartPropagation.al:3696).
    pub blank_after_empty_last: bool,
    /// The client's rows were read again from the row a list settled on after an action's filter left its row, with no row before
    /// it: First stays on that row and reads nothing (pages/lt-ai-list-order/AiListOrderTests.al:339, :373 `after_first` empty).
    /// Also set by the refresh of CurrPage.Update on a list with OnFindRecord and OnNextRecord that ends on the first row (:174,
    /// :203). Any TestPage call but First and a field read clears it.
    pub settled_first_row: bool,
    /// The caption was assigned while the page was not open: it stays for every later run of the variable.
    pub caption_assigned_closed: bool,
    /// A row with the new record's starting key was already stored when the draft started.
    pub draft_stored: bool,
    /// The new row of a card that its OnNewRecord inserted itself: the page still holds it as a new row
    /// (corpus pages/lt-cf-page-leads/CfPageLeads.al:241-289 Card_OnNewRecordInserts_OpenNewSetClose).
    pub new_row_inserted_by_trigger: bool,
    /// TestPage.OpenNew is reading the page's first row (cleared once it is read).
    pub test_opened_new: bool,
    /// On a new row whose key fields were still blank, a field outside the key was typed.
    pub typed_before_key: bool,
    /// The page reached a state whose behavior is not pinned: every later call on it or its parts except Close is refused
    /// with this reason.
    pub unpinned_state: Option<String>,
    /// With `unpinned_state`: Close is refused too.
    pub unpinned_close: bool,
    /// The row a new line was started below (`New()` on a stored row of a list): after the new row
    /// is saved, `Next()` goes to the row below that one, as the client's own row order has it.
    pub after_new: Option<Vec<Value>>,
    /// OnInit/OnOpenPage are running, and `CurrPage.Editable(false)` has been called in them.
    pub opening: bool,
    pub editable_off: bool,
    /// `editable_off` came from `CurrPage.Editable(false)` in OnInit/OnOpenPage (not from a preset): its effect depends on
    /// the open mode (see `PageOps::page_editable`).
    pub editable_off_in_open: bool,
    /// Controls given a value by `SetValue` since the page opened (a Decimal FlowFilter control at zero shows '' only
    /// while it is unset).
    pub set_controls: Vec<usize>,
    /// The key of the row the page's OnAfterGetRecord ran with last: what the page's variables hold was computed for that
    /// row (see `Session::control_value`).
    pub agr_row: Option<Vec<Value>>,
    /// The keys of every row OnAfterGetRecord ran with since the page opened.
    pub agr_rows: Vec<Vec<Value>>,
    /// Per row of a repeater (its key): what the page variables its controls show held after the row's OnAfterGetRecord, and
    /// for the current row after OnAfterGetCurrRecord: (control index, value) (pages/lt-qi-row-page-var/QiRowPageVar.al).
    pub row_vars: Vec<RowVars>,
    /// The row OnAfterGetRecord ran with last (its fields after the trigger): the next OnAfterGetRecord sees it as xRec;
    /// `None` before the first fetch, when xRec is a blank row (testpart/QiHostSaveParts.al:361 `L.AGR(D1/10000|d1 l1 x=/0)`).
    pub agr_xrec: Option<Vec<Value>>,
    /// The row OnAfterGetCurrRecord ran with last: the next OnAfterGetCurrRecord sees it as xRec; `None` before the first,
    /// when xRec is a blank row with the single-value key filters (the part link) on it (QiHostSaveParts.al:361
    /// `L.AGCR(D1/10000|d1 l1 x=D1/0)`, `U.AGCR(D1/10000 x=/0)`).
    pub agcr_xrec: Option<Vec<Value>>,
    /// OnAfterGetCurrRecord ran last on a row that is not stored (a new row): the next one on a stored row sees the xRec the
    /// page holds (misc/lt-probes-needed-7/PnGShowRow.al EmptyListInsertViaRec `List.AGCR(,);List.AGR(NEW1,Created);...;
    /// List.AGCR(NEW1,NEW1)`).
    pub agcr_after_new: bool,
    /// A sibling part saved a row before a host SetValue and this unlinked part over the same table read it and shows it
    /// (testpart/QiHostSaveParts.al:500 `U.AGR(D1/30000 ...);U.AGCR(D1/30000 x=/0)`): the fields of the row it stands on.
    pub shows_sibling_save: Option<Vec<Value>>,
    /// A part: a TestPart call or a call on one of its fields reached it since its host opened.
    pub part_accessed: bool,
    /// A host that is a new, unsaved record while a part linked by a field shows rows of the blank key: only Close, OK, New()
    /// and a read of a host field are pinned for it, and a part field read or First that inserts it (pages/
    /// lt-part-line-before-host-key/PartLineBeforeHostKey.al QH_BlankRows_*).
    pub new_host_part_rows: bool,
    /// `P.Editable(false)` or `P.Editable := false` on a Page variable before it runs: the page
    /// opens non-editable (handlers/TestPageVariableEditableContracts.al).
    pub editable_preset: Option<bool>,
    /// `CurrPage.Caption := x` (handlers/TestPageCaption_Page.al:44); reset when the page opens.
    pub caption: Option<String>,
    /// The pages shown by the part controls of this page: (control index, child page). Only parts
    /// that are in the control tree at open have one.
    pub parts: Vec<(usize, Rc<RefCell<PageState>>)>,
    /// Parts dropped at open because their `Visible` came out false: the page's own code still reaches them
    /// (`CurrPage.<Part>.Page`), the test client does not.
    pub hidden_parts: Vec<(usize, Rc<RefCell<PageState>>)>,
    /// Primary key of the stored row the cursor stood on when the current new row was started:
    /// the lower neighbour AutoSplitKey numbers it from (`None`: no such row).
    pub lower_key: Option<Vec<Value>>,
    /// The blank line the cursor stands on is the list's placeholder line (reached when the page opened or by moving onto it),
    /// not a line `New()` started: a `New()` from it burns one AutoSplitKey interval on it.
    pub placeholder_line: bool,
    /// The current new row was started by `New()` from the placeholder line: AutoSplitKey counts one interval more.
    pub split_burned: bool,
    /// `OnOpenPage` left filters in `FilterGroup(-1)`: the page dropped them from the rows it shows.
    pub or_group_dropped: bool,
    /// Fields whose page filter reads back as a text nothing pins (a SourceTableView `const(0D)`): a TestPage Filter read refuses.
    pub unpinned_filter_fields: Vec<usize>,
    /// The record `Page.SetRecord` handed a page variable, shown when the page runs.
    pub set_record: Option<Rc<RefCell<RecordState>>>,
    /// `Page.SetTableView` gave `set_record` its filters and order: a later `SetRecord` moves the position only
    /// (pages/lt-page-setrecord-settableview/PageVarSetRecordTableView.al:P157_List_SetTableViewThenSetRecord_*).
    pub set_view: bool,
    /// The line the page stands on was started by a `New()` call, not reached by moving onto the blank line.
    pub new_by_call: bool,
    /// A `Page`-typed modal handler is running on this page: the page never opened, `GetRecord` and `SetRecord`
    /// read and write its Rec (handlers/PSModalPageHandlerTyped.al).
    pub typed_handler: bool,
    /// The page opened on a source table with no row to show (a card over an empty table): its Rec was never a stored row.
    pub never_stored: bool,
    /// For a part: the page that hosts it and the index of the part control on that page.
    pub host: Option<(std::rc::Weak<RefCell<PageState>>, usize)>,
    /// The instances of the page's extensions (extension object, instance): they share the page's
    /// `Rec`, `xRec` and `CurrPage`, and hold the extension's own variables.
    pub ext_instances: Vec<(usize, Rc<RefCell<Instance>>)>,
    /// A request page that `Report.RunRequestPage` opened: its OK confirms the parameters, whatever
    /// the report renders.
    pub parameters_wanted: bool,
    /// The parameters string a `Report.RunRequestPage(Id, Parameters)` hands the request page: loaded after its OnInit
    /// and before its OnOpenPage (report/lt-aq/AqTests.al:362-381).
    pub request_seed: Option<String>,
    /// The session's application areas when the page opened: controls whose area they do not
    /// enable are not on the page.
    pub areas: String,
    /// OnInit has run on the current instance. BC constructs a page, and so runs OnInit, when a
    /// `Page` variable is first used, which may be a procedure call before it runs (corpus
    /// handlers/TestPageOnInit_Tests.al:188 POI_SetterBeforeRunModal_RunsAfterOnInitAndSurvives);
    /// the open that follows must not run it again. The open consumes the flag.
    pub inited: bool,
    /// A page whose rowset OnFindRecord/OnNextRecord serve was refreshed: which row the client
    /// settles on is not modelled, so Rec-bound controls are not read until the cursor moves.
    pub stale: bool,
    /// The page is a lookup without the new-row line: `LookupMode(true)` was called on this variable, or
    /// `Page.RunModal(Id, Rec)` / `Page.RunModal(0, Rec)` had its return value used.
    pub lookup_set: bool,
    /// `Page.RunModal(Id, Rec)` called as a statement: not a lookup (pages/lt-be-runmodal-onlookup/BeRunModalOnLookup.al:916),
    /// and whether `Rec` takes the row the page closed on is not pinned: a later use of `Rec` refuses when that row differs.
    pub statement_run: bool,
    /// The refresh about to run follows an action on a card with OnFindRecord, without CurrPage.Update: pinned
    /// (pages/lt-be-card-onfindrecord/BeCardOnFindRecord.al), so it marks no assumption.
    pub refresh_pinned: bool,
    /// The refresh about to run was asked for by `CurrPage.Update` (not by an action that saved an edit).
    pub refresh_by_update: bool,
    /// The last `CurrPage.Update` call was `Update(false)` (it saves nothing).
    pub refresh_update_false: bool,
    /// Page background tasks the page enqueued and the client has not run yet: (task id, codeunit id,
    /// parameters), and the next task id.
    pub bg_tasks: std::collections::VecDeque<(i32, i32, Value)>,
    pub bg_next_id: i32,
}

// The page instance holds `CurrPage`, which points back here: print only the facts.
impl std::fmt::Debug for PageState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "PageState(page {}, open {}, mode {:?})",
            self.object, self.open, self.mode
        )
    }
}

impl PageState {
    pub fn closed(object: u32) -> PageState {
        PageState {
            object,
            open: false,
            ran: false,
            mode: PageMode::View,
            instance: None,
            cursor: Cursor::None,
            new_line: false,
            draft: false,
            dirty: false,
            frozen_visible: Vec::new(),
            open_language: None,
            control_captions: Vec::new(),
            field_captions: Vec::new(),
            field_captions_opened: false,
            field_captions_settled: false,
            moving_next: false,
            field_captions_fill_first: false,
            field_captions_link_refill: false,
            field_captions_action_save: 0,
            window: Vec::new(),
            window_from_start: false,
            window_eof: false,
            window_whole: false,
            view_read_to_end: false,
            window_read_up: false,
            landing_first: false,
            open_reads_above: false,
            opened_on_moved_rec: false,
            reads_once_more_in: None,
            open_read: false,
            blank_on_open: false,
            read_backwards: None,
            selection_on_rec: false,
            focus: None,
            focus_in_row: false,
            errors: Vec::new(),
            trigger_errors: Vec::new(),
            refused: false,
            close_error: false,
            selection: None,
            pending: None,
            modal: false,
            last_run_modal: None,
            expanded: Vec::new(),
            lookup: false,
            result: None,
            trapping: false,
            refresh: false,
            save_record_called: false,
            validating_control: false,
            refresh_without_curr: false,
            refresh_without_parts: false,
            read_keeps_xrec: false,
            xrec_after_validate: None,
            view_left: false,
            read_after_goto: false,
            read_after_goto_doubt: false,
            read_after_goto_quiet: false,
            read_after_goto_set: false,
            read_after_goto_prev: false,
            empty_list_row_behind: false,
            action_landed_on_new_row: false,
            static_lookup: false,
            triggers_run: 0,
            refresh_read_done: false,
            action_shown_row: None,
            rec_moved_other: false,
            close_restored_row: false,
            split_starts_low: false,
            save_after_first_read: false,
            walk_synced_parts: false,
            empty_part_xrec: None,
            blank_after_empty_last: false,
            settled_first_row: false,
            caption_assigned_closed: false,
            draft_stored: false,
            new_row_inserted_by_trigger: false,
            test_opened_new: false,
            typed_before_key: false,
            unpinned_state: None,
            unpinned_close: false,
            after_new: None,
            opening: false,
            editable_off: false,
            editable_off_in_open: false,
            set_controls: Vec::new(),
            agr_row: None,
            agr_rows: Vec::new(),
            row_vars: Vec::new(),
            agr_xrec: None,
            agcr_xrec: None,
            agcr_after_new: false,
            shows_sibling_save: None,
            part_accessed: false,
            new_host_part_rows: false,
            editable_preset: None,
            caption: None,
            parts: Vec::new(),
            hidden_parts: Vec::new(),
            lower_key: None,
            placeholder_line: false,
            split_burned: false,
            or_group_dropped: false,
            unpinned_filter_fields: Vec::new(),
            set_record: None,
            set_view: false,
            new_by_call: false,
            typed_handler: false,
            never_stored: false,
            host: None,
            ext_instances: Vec::new(),
            parameters_wanted: false,
            request_seed: None,
            areas: String::new(),
            inited: false,
            stale: false,
            lookup_set: false,
            statement_run: false,
            refresh_pinned: false,
            refresh_by_update: false,
            refresh_update_false: false,
            bg_tasks: Default::default(),
            bg_next_id: 1,
        }
    }
}

/// What a `TestControl` stands for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlTarget {
    /// A control of the page by index into its control list.
    Control(usize),
    /// A built-in action: OK, Cancel, Yes, No, Edit, View.
    BuiltIn(String),
    /// `TestPage.Filter`.
    Filter,
}

#[derive(Debug)]
pub struct ControlRef {
    pub page: Rc<RefCell<PageState>>,
    pub target: ControlTarget,
}

/// The identity of the private table of a temporary record. Records that share the table (a record
/// state clone, `Copy(x, true)`) hold clones of one `TempId`; when the last is dropped the id goes onto
/// the session's release queue and the session frees the table, as `DotNetObject` hands its handle back
/// (`Drop` never reaches into the session).
#[derive(Clone)]
pub struct TempId(Rc<TempToken>);

struct TempToken {
    id: u64,
    release: Rc<RefCell<Vec<u64>>>,
}

impl Drop for TempToken {
    fn drop(&mut self) {
        self.release.borrow_mut().push(self.id);
    }
}

impl TempId {
    pub fn new(id: u64, release: Rc<RefCell<Vec<u64>>>) -> TempId {
        TempId(Rc::new(TempToken { id, release }))
    }

    pub fn id(&self) -> u64 {
        self.0.id
    }
}

impl std::fmt::Debug for TempId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TempId({})", self.0.id)
    }
}

/// Everything a record variable carries.
#[derive(Debug, Clone)]
pub struct RecordState {
    /// Object index of the table.
    pub table: u32,
    pub temporary: bool,
    /// Identity of the private table behind a temporary record (shared by `Copy(x, true)`).
    pub temp_id: Option<TempId>,
    /// The company set by `ChangeCompany` or `RecordRef.Open(.., Company)`, as the Company table
    /// names it; `None` is the session company.
    pub company: Option<String>,
    /// The table is `TableType = CRM`: any use needs a registered table connection
    /// (session/TestCrmTableNoConnectionContracts.al).
    pub crm: bool,
    /// Field values in the table's field order.
    pub fields: Vec<Value>,
    pub filters: Vec<crate::filter::FieldFilter>,
    /// The filter group `SetRange` and `SetFilter` write to and `GetFilter` reads.
    pub filter_group: i32,
    /// Primary keys marked with `Mark(true)`, and whether `MarkedOnly` restricts the set to them.
    pub marks: Vec<Vec<Value>>,
    pub marked_only: bool,
    /// `ReadIsolation` and `SecurityFiltering` as member indexes of their platform option types.
    pub read_isolation: i32,
    pub security_filtering: i32,
    /// Index of the current key in the table's keys.
    pub key: usize,
    /// `SetCurrentKey` on fields no key of the table starts with: the key index it stands for (one
    /// past the table's keys) and the field positions. `Session::table` appends it to the table's
    /// keys while `key` is that index, so a scan orders by these fields, then by the primary key.
    pub adhoc: Option<(usize, Vec<usize>)>,
    pub ascending: bool,
    /// `SetAscending(Field, false)`: bit i is set when the i-th field of the current key's position (the key's fields,
    /// then the primary key's) runs descending. Unlike `ascending` it does not survive a page's view round trip
    /// (handlers/TestPageViewRoundTripRoutes.al).
    pub field_descending: u32,
    /// The primary key of the row the record last read: `Find` still finds that row after the
    /// record moved it out of its own filter.
    pub position: Option<Vec<Value>>,
    /// Field positions of `SetAutoCalcFields`: calculated whenever a row is read.
    pub auto_calc: Vec<usize>,
    /// Partial records: `SetLoadFields` and which fields the buffer holds.
    pub load: LoadState,
    /// AL code has written a field of the buffer. BC builds the buffer of a record variable on
    /// the first write, and evaluates the `InitValue`s only then (record/TestDateTimeInitValue.Codeunit.al).
    pub touched: bool,
    /// AL code assigned a field since the last `Init` or `Clear`, even to its own default value; `AreFieldsLoaded` on a
    /// record nothing was fetched into reads it (record/lt-arefieldsloaded-default).
    pub assigned: bool,
    /// This variable inserted its row since the last `Init` or `Clear`: a `CalcFields` of a Blob that finds the row gone
    /// (rolled back) empties the Blob (misc/lt-probes-needed-5/PnE08RecordMisc.al:TestField_Blob).
    pub inserted: bool,
    /// What a RecordRef last read or wrote into the buffer: `RecordRef.IsDirty` compares with it, not with the stored row
    /// (recordref/lt-recordref-fieldref-members/RecordRefIsDirtyEdges.al:IsDirty_TwoRefs_OtherModifies). `None` when the
    /// buffer came from elsewhere (GetTable, Init).
    pub dirty_base: Option<Vec<Value>>,
    /// What `Record.IsDirty` compares the buffer with (lt-ac/ACRecordIsDirty.al). A clone of the state starts unknown.
    pub is_dirty_base: DirtyBase,
    /// The row this variable's last Modify wrote. A later Modify of the unchanged buffer, after a rollback undid that write,
    /// writes nothing (record/lt-record-conversion-assumptions/StaleBuffer.al:76-86).
    pub last_modify: Option<Vec<Value>>,
    /// The table's and its extensions' global variables of this record variable (ADR-021).
    pub globals: TableGlobals,
    /// Every use of the record refuses with this reason: what its buffer holds is not pinned (the row a page closed on after
    /// `Page.RunModal(Id, Rec)` called as a statement, see `hand_back_selection`).
    pub unpinned_buffer: Option<&'static str>,
}

/// What `Record.IsDirty` compares a record variable's buffer with: a copy of the fields the variable last read or wrote
/// (lt-ac/ACRecordIsDirty.al). BC compares values, not writes: a field set back to what was read is not dirty
/// (ACRecordIsDirty.al:54-56, `after_value_restored`).
///
/// `Clone` gives `Unknown`: whether a copy of a record state (a record passed by value, xRec, a row image,
/// `RecordRef.GetTable`) keeps the snapshot is not pinned. `:=` copies it with [`DirtyBase::copy`]
/// (ACRecordIsDirty.al:63-64, `copy_dirty`).
#[derive(Debug)]
pub enum DirtyBase {
    /// Nothing pins what IsDirty answers; the text says why.
    Unknown(&'static str),
    /// A declared variable nothing was read into: compared with a fresh buffer (ACRecordIsDirty.al:38-43).
    Fresh,
    /// The variable read or wrote its buffer and no field was written since: the snapshot is the buffer itself. A write
    /// copies it first ([`RecordState::before_field_write`]), so a read costs no copy.
    Synced,
    /// The fields the variable last read or wrote. `alt` is a second candidate where the corpus does not tell which one BC
    /// keeps (after TransferFields and Delete): IsDirty answers only where both give the same answer.
    Row { base: Vec<Value>, alt: Option<Vec<Value>> },
}

impl Default for DirtyBase {
    fn default() -> Self {
        DirtyBase::Unknown("a record state that is not a declared variable")
    }
}

impl Clone for DirtyBase {
    fn clone(&self) -> Self {
        DirtyBase::Unknown("a copy of a record variable (passed by value, xRec, a row image, a RecordRef)")
    }
}

impl DirtyBase {
    /// A real copy, for `:=` between record variables.
    pub fn copy(&self) -> DirtyBase {
        match self {
            DirtyBase::Unknown(why) => DirtyBase::Unknown(why),
            DirtyBase::Fresh => DirtyBase::Fresh,
            DirtyBase::Synced => DirtyBase::Synced,
            DirtyBase::Row { base, alt } => DirtyBase::Row {
                base: base.clone(),
                alt: alt.clone(),
            },
        }
    }

    /// `Synced` becomes a copy of `fields`, before something writes them.
    pub fn materialize(&mut self, fields: &[Value]) {
        if matches!(self, DirtyBase::Synced) {
            *self = DirtyBase::Row {
                base: fields.to_vec(),
                alt: None,
            };
        }
    }
}

impl RecordState {
    /// A field of the buffer is about to be written: a snapshot that is the buffer itself becomes a copy.
    #[inline]
    pub fn before_field_write(&mut self) {
        if matches!(self.is_dirty_base, DirtyBase::Synced) {
            let RecordState {
                is_dirty_base, fields, ..
            } = self;
            is_dirty_base.materialize(fields);
        }
    }
}

/// The global variables of a table and of its table extensions, as one record variable holds
/// them (ADR-021). They live as long as the variable: every procedure and trigger call on it
/// runs on the same `Instance`, created on the first call. `Clone` gives an EMPTY set, because
/// a copy of a record state (a record passed by value, a row image, `RecordRef.GetTable`) starts
/// with fresh globals; use [`TableGlobals::share`] to keep two states on one set.
///
/// The instances hold `Rec` and `xRec` only while a call runs (the VM puts back what the slots
/// held before), so a state owns its globals and nothing points back: no `Rc` cycle.
#[derive(Default)]
pub struct TableGlobals {
    // Interior mutability: a handle is made through a shared borrow of the record state, which a
    // caller may hold (`xrec_handle(rec, rec.borrow().fields.clone())`).
    data: RefCell<Option<Rc<RefCell<GlobalsData>>>>,
}

/// Debug prints the number of instances only: while a call runs, an instance points at its record.
impl std::fmt::Debug for TableGlobals {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let n = self.data.borrow().as_ref().map_or(0, |d| d.borrow().instances.len());
        write!(f, "TableGlobals({n})")
    }
}

#[derive(Default)]
struct GlobalsData {
    /// One instance per object (the table, each table extension) that has been called.
    instances: Vec<(u32, Rc<RefCell<Instance>>)>,
    /// `xRec`'s own set: it persists with the variable and survives `Clear` and `Reset` of `Rec`.
    xrec: TableGlobals,
}

impl Clone for TableGlobals {
    fn clone(&self) -> Self {
        TableGlobals::default()
    }
}

impl TableGlobals {
    /// An empty set that other states can [`share`](TableGlobals::share).
    pub fn new_shared() -> TableGlobals {
        TableGlobals {
            data: RefCell::new(Some(Rc::default())),
        }
    }

    /// Whether nothing has been called on this set yet.
    pub fn is_unset(&self) -> bool {
        self.data.borrow().is_none()
    }

    fn data(&self) -> Rc<RefCell<GlobalsData>> {
        self.data.borrow_mut().get_or_insert_with(Default::default).clone()
    }

    /// A second handle onto this very set (not a copy). Creates the set if there is none yet.
    pub fn share(&self) -> TableGlobals {
        TableGlobals {
            data: RefCell::new(Some(self.data())),
        }
    }

    /// The instance of `object`, if one was created.
    pub fn instance(&self, object: u32) -> Option<Rc<RefCell<Instance>>> {
        let data = self.data.borrow();
        let d = data.as_ref()?.borrow();
        d.instances.iter().find(|(o, _)| *o == object).map(|(_, i)| i.clone())
    }

    pub fn set_instance(&self, object: u32, inst: Rc<RefCell<Instance>>) {
        self.data().borrow_mut().instances.push((object, inst));
    }

    /// Every instance created so far.
    pub fn instances(&self) -> Vec<(u32, Rc<RefCell<Instance>>)> {
        match &*self.data.borrow() {
            Some(d) => d.borrow().instances.clone(),
            None => Vec::new(),
        }
    }

    /// A handle onto `xRec`'s set (created on first use).
    pub fn xrec(&self) -> TableGlobals {
        let data = self.data();
        let d = data.borrow();
        d.xrec.share()
    }
}

/// A set of field positions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldSet {
    All,
    Of(Vec<usize>),
}

impl FieldSet {
    pub fn contains(&self, pos: usize) -> bool {
        match self {
            FieldSet::All => true,
            FieldSet::Of(v) => v.contains(&pos),
        }
    }

    pub fn add(&mut self, pos: usize) {
        if let FieldSet::Of(v) = self {
            if !v.contains(&pos) {
                v.push(pos);
            }
        }
    }

    pub fn remove(&mut self, pos: usize) {
        if let FieldSet::Of(v) = self {
            v.retain(|&p| p != pos);
        }
    }
}

/// What a record's buffer holds after a fetch (partial records, `Record.AreFieldsLoaded`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    /// The fields known to be loaded.
    pub have: FieldSet,
    /// Fields outside `have` whose state the corpus does not pin: asking about one is refused.
    pub unsure: FieldSet,
    /// The primary key of the row fetched, when the buffer is partial (`None` for a full one).
    pub row: Option<String>,
    /// `Clear` kept this buffer: it carries over to a fetch of the same row only; a fetch of another row starts afresh
    /// (record/TestRecordPartialLoadNarrowAfterFetch.al:121-185).
    pub cleared: bool,
}

/// Partial-record state of a record variable. A default one is a full load that fetched nothing.
#[derive(Debug, Clone, Default)]
pub struct LoadState {
    /// `SetLoadFields`: the field positions requested; `None` is the table's default (all).
    pub request: Option<Vec<usize>>,
    /// What the buffer holds; `None` until a row was fetched into it (and after `Clear`).
    pub buffer: Option<Buffer>,
    /// The buffer is partial: a read of an omitted field loads it, so reads and writes of
    /// fields tell `touch` and `written`. Kept as its own flag so the full-load path pays one
    /// branch.
    pub partial: bool,
}

impl LoadState {
    /// A field of the buffer was read: an omitted one is loaded on demand
    /// (record/TestRecordPartialLoad.al:144-190).
    pub fn touch(&mut self, pos: usize) {
        if let Some(b) = &mut self.buffer {
            if !b.have.contains(pos) {
                b.have.add(pos);
                b.unsure.remove(pos);
            }
        }
    }

    /// A field of the buffer was assigned. Whether that counts as loading it is not pinned.
    pub fn written(&mut self, pos: usize) {
        if let Some(b) = &mut self.buffer {
            if !b.have.contains(pos) {
                b.unsure.add(pos);
            }
        }
    }
}

/// What a `WebServiceActionContext` holds: the object an API action points the OData layer at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WebServiceActionState {
    pub object_id: i32,
    /// Ordinal in the platform's object type option; 0 is the unnamed default.
    pub object_type: i32,
    /// Index into the `WebServiceActionResultCode` members; 0 is `None`.
    pub result_code: i32,
    /// Field ids that have an entity key.
    pub keys: Vec<i32>,
}

/// One control of a `FilterPageBuilder`: a name and the record buffer whose filters it holds.
#[derive(Debug, Clone)]
pub struct FilterControl {
    pub name: String,
    /// Object index of the table.
    pub table: u32,
    /// Shared between copies of the builder (corpus filterpage/TestFilterPageBuilder.al:
    /// FilterPageBuilder_Assignment_ViewOnCopyDoesReachOriginal).
    pub record: Rc<RefCell<RecordState>>,
}

/// What a `FilterPageBuilder` holds. An untouched one is `LibraryValue::Empty`.
#[derive(Debug, Clone, Default)]
pub struct FilterPageState {
    pub controls: Vec<FilterControl>,
    /// `None` until assigned: the getter then answers a default.
    pub caption: Option<String>,
}

/// What a `SessionSettings` holds. A property nobody wrote is `None`: the corpus does not say what
/// a pristine language id, locale id, time zone or profile id reads as, so reading one is refused.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionSettingsState {
    pub language_id: Option<i32>,
    pub locale_id: Option<i32>,
    pub time_zone: Option<String>,
    pub profile_id: Option<String>,
    pub profile_app_id: Option<[u8; 16]>,
    pub company: String,
}

/// Which collection of headers a bag is: they differ in the headers they accept (a request, response or default
/// header collection refuses the content headers; the content's own accepts them).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum HeaderKind {
    #[default]
    Request,
    Content,
    Response,
    /// `HttpClient.DefaultRequestHeaders`.
    Default,
}

/// The headers of a request, response, content or client: names with the casing they were first added with,
/// each with its values in order. Secret values (added as `SecretText`) are kept apart.
#[derive(Debug, Default)]
pub struct HeaderBag {
    pub kind: HeaderKind,
    pub items: Vec<(String, Vec<String>)>,
    pub secret: Vec<(String, Vec<String>)>,
}

/// The headers of an `HttpContent`, request, response or client, shared with every `HttpHeaders` handed out.
pub type HttpHeaderList = Rc<RefCell<HeaderBag>>;

/// A header bag of `kind`.
pub fn new_header_list(kind: HeaderKind) -> HttpHeaderList {
    Rc::new(RefCell::new(HeaderBag {
        kind,
        ..HeaderBag::default()
    }))
}

/// What an `HttpRequestMessage` holds (corpus network/TestHttpRequestMessage.al,
/// network/lt-http/HttpRequest.al). Nothing is sent but through a test handler.
#[derive(Debug, Clone)]
pub struct HttpRequestState {
    pub method: Option<String>,
    pub uri: Option<String>,
    /// The `Content` set on the request: the `HttpContent` handle itself, so the request and the
    /// variable it was set from share it (nothing pins a copy; no corpus test tells them apart).
    pub content: Option<Value>,
    pub headers: HttpHeaderList,
}

impl Default for HttpRequestState {
    fn default() -> Self {
        HttpRequestState {
            method: None,
            uri: None,
            content: None,
            headers: new_header_list(HeaderKind::Request),
        }
    }
}

/// What an `HttpContent` holds (corpus network/TestHttpRequestMessage.al:34-43, network/lt-http/HttpContent.al).
#[derive(Debug, Clone)]
pub struct HttpContentState {
    /// The body as bytes (text is written as UTF-8).
    pub data: Vec<u8>,
    pub headers: HttpHeaderList,
    /// The content a request has when nobody set one: `ReadAs` answers false, `GetHeaders` raises.
    pub uninitialized: bool,
    /// The bytes were handed out as an `InStream`: a second `ReadAs(InStream)` gets an empty stream.
    pub stream_taken: bool,
    /// A `WriteFrom` has put a body here (a second one may add a Content-Length, see `write_body`).
    pub written: bool,
    /// The value is a copy an assignment or a by-value parameter made (`deep_copy`), not the content a request handed out.
    /// `ReadAs` on an uninitialized content runs only the shapes the corpus pins, and the two pins differ in this.
    pub copied: bool,
    /// The last `WriteFrom` took an `InStream`.
    pub from_stream: bool,
}

impl Default for HttpContentState {
    fn default() -> Self {
        let headers = new_header_list(HeaderKind::Content);
        headers
            .borrow_mut()
            .items
            .push(("Content-Type".into(), vec!["text/plain; charset=utf-8".into()]));
        HttpContentState {
            data: Vec::new(),
            headers,
            uninitialized: false,
            stream_taken: false,
            written: false,
            copied: false,
            from_stream: false,
        }
    }
}

/// What a `Cookie` holds (platform/lt-xmlport-session-navapp-cookie/P94Cookie.al).
#[derive(Debug, Clone, Default)]
pub struct CookieState {
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub secure: bool,
    pub http_only: bool,
    /// A `DateTime` value, 0 for none.
    pub expires: i64,
}

/// What an `HttpResponseMessage` holds (network/lt-http/HttpResponse.al). A response nobody filled is
/// 200 OK with an empty body.
#[derive(Debug, Clone)]
pub struct HttpResponseState {
    pub status: i32,
    pub reason: String,
    /// A handler set a status whose reason phrase BC gives is not measured: reading `ReasonPhrase` is refused.
    pub reason_unpinned: bool,
    /// The `HttpContent` handle, shared with `Content`.
    pub content: Value,
    pub headers: HttpHeaderList,
    pub blocked: bool,
}

/// What an `HttpClient` holds (network/lt-http/HttpClient.al).
#[derive(Debug, Clone)]
pub struct HttpClientState {
    pub base_address: String,
    pub timeout_ms: i64,
    pub default_headers: HttpHeaderList,
}

impl Default for HttpClientState {
    fn default() -> Self {
        HttpClientState {
            base_address: String::new(),
            timeout_ms: 100_000,
            default_headers: new_header_list(HeaderKind::Default),
        }
    }
}

/// What a `TestHttpRequestMessage` holds: what a `[HttpClientHandler]` sees of the request.
#[derive(Debug, Clone)]
pub struct TestHttpRequestState {
    /// Scheme, host and path without the query string.
    pub path: String,
    /// The `Dictionary of [Text, Text]` of the decoded query, shared with `QueryParameters`.
    pub query: Value,
    pub has_secret_uri: bool,
    /// The `HttpRequestType` enum value (`Null` for a request no client call made).
    pub request_type: Value,
}

/// What a `TestHttpResponseMessage` holds: what a `[HttpClientHandler]` answers.
#[derive(Debug, Clone)]
pub struct TestHttpResponseState {
    /// The request went out for real (no handler answered it) and failed before any connection
    /// (`http::failure_without_connection`): the response reads status 0 and keeps nothing of the handler's.
    pub request_failed: bool,
    /// The error a failed request leaves as the last error (text, code) when the call's result is used.
    pub failure: Option<(String, String)>,
    pub status: i32,
    pub reason: String,
    /// The handler set `ReasonPhrase`; when it did not, BC gives the reason phrase of the status.
    pub reason_set: bool,
    /// See [`HttpResponseState::reason_unpinned`].
    pub reason_unpinned: bool,
    pub success: bool,
    pub blocked: bool,
    pub content: Value,
    pub headers: HttpHeaderList,
}

/// What a `Notification` holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotificationState {
    pub message: Option<String>,
    /// `Id := guid` (handlers/TestNotificationHandler.al:55-56); the default is not pinned.
    pub id: Option<[u8; 16]>,
    /// Ordinal in the `NotificationScope` members.
    pub scope: Option<i32>,
    pub data: Vec<(String, String)>,
    /// (Caption, codeunit id, method name, description): kept, never run.
    pub actions: Vec<(String, i32, String, Option<String>)>,
}

/// What an `ErrorInfo` holds (corpus errorinfo/TestErrorInfoType.al).
#[derive(Debug, Clone)]
pub struct ErrorInfoState {
    pub message: String,
    pub title: String,
    pub detailed_message: String,
    pub control_name: String,
    /// Indexes into the `Verbosity` and `DataClassification` members.
    pub verbosity: i32,
    pub data_classification: i32,
    /// `None` until a test sets one: reads as Client.
    pub error_type: Option<i32>,
    pub collectible: bool,
    pub table_id: i32,
    /// The `RecordId` value of the record `Create` was given.
    pub record_id: Option<Value>,
    pub system_id: Option<Value>,
    pub field_no: i32,
    pub page_no: i32,
    /// The dictionary the test assigned, held by reference.
    pub custom_dimensions: Option<Value>,
}

/// What a `ModuleInfo` reports: an app's manifest (corpus session/TestNavAppExtended.al,
/// session/TestBCPlatformContracts.al, session/TestMiscTypes.al).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleInfoState {
    pub id: [u8; 16],
    pub name: String,
    pub publisher: String,
    /// `None` when the manifest gave none the runtime can read.
    pub version: Option<[i32; 4]>,
    pub dependencies: Vec<[u8; 16]>,
    /// The module BC makes up for the null app id: only its data version is pinned
    /// (session/TestEnvironmentUserTenantContracts.al:195-201); the other properties are refused.
    pub synthesized: bool,
}

/// The global variables of one codeunit instance.
#[derive(Debug, Default)]
pub struct Instance {
    pub object: u32,
    pub globals: Vec<Value>,
    pub initialized: bool,
    /// A report variable a `Run` just reset: its next method call other than `Run` runs `OnInitReport` again,
    /// a `Run` does not (report/TestProbeReportContracts.al:ReportVariable_RunTwice_GlobalsReset against
    /// handlers/TestReportRuntimeContracts.al:Report_Run_ResetsTheGlobals_NotOnInitReport_RunTwiceIsAllowed_ExecuteTwiceThrows).
    pub rerun: bool,
    /// For the instance of a page or report extension: the instance of the page or report it
    /// extends (ADR-023). `Weak`, so the link adds no reference cycle. `None` for every other
    /// instance.
    pub base: Option<std::rc::Weak<RefCell<Instance>>>,
    /// For the instance of a report variable: the instances of the reportextensions that change it, by object
    /// index, made at the first call of an extension procedure or run of the variable and kept with the variable's
    /// state (a `Run` and `Clear` drop them). Empty for every other instance.
    pub exts: Vec<(u32, Rc<RefCell<Instance>>)>,
    /// For the instance of an xmlport variable: the separators a program set (`FieldDelimiter(..)`, ...), as given, by
    /// kind (see `al-vm/src/xmlports.rs` `SEPARATORS`). Empty until one is set.
    pub xp_text: Vec<(u8, String)>,
}

/// State behind a library-type handle; grows with the builtins that need it.
#[derive(Debug, Default)]
pub enum LibraryValue {
    #[default]
    Empty,
    Text(String),
    /// A TextBuilder whose capacity was set or has to be kept: the text and the capacity in characters.
    TextCap(String, i32),
    /// An open `RecordRef`: it owns a record buffer exactly as a `Record`
    /// variable does (fields, filters, key), so record operations are shared.
    RecordRef(Rc<RefCell<RecordState>>),
    /// A `FieldRef`: the record buffer it points into and the field's position.
    FieldRef(Rc<RefCell<RecordState>>, usize),
    /// A `FieldRef` reached through `KeyRef.FieldIndex`: the same field, but the hidden key on SystemId names
    /// it `$systemId` (recordref/TestRecordRefKeyIndexSetRecFilterContracts.al:93).
    KeyFieldRef(Rc<RefCell<RecordState>>, usize),
    /// A `KeyRef`: the record buffer and the key's index in the table's keys.
    KeyRef(Rc<RefCell<RecordState>>, usize),
    /// A `RecordRef` opened on a system table the workspace has no schema for:
    /// only its number is known (corpus recordref/TestRecordRefOpenCompilationTarget.al).
    SystemTable(i64),
    /// A `RecordId`: table object index and number, table name (for `Format`), primary key values.
    RecordId {
        table: u32,
        table_no: u32,
        name: String,
        key: Vec<Value>,
    },
    /// A `WebServiceActionContext` that has been written to (an untouched one is `Empty`).
    WebServiceAction(WebServiceActionState),
    /// A `SessionSettings` that has been written to (an untouched one is `Empty`).
    SessionSettings(SessionSettingsState),
    /// A `FilterPageBuilder` that has been written to (an untouched one is `Empty`).
    FilterPageBuilder(FilterPageState),
    /// A `Notification` that has been written to (an untouched one is `Empty`).
    Notification(NotificationState),
    /// A `ModuleInfo`: what one app reports about itself.
    ModuleInfo(ModuleInfoState),
    /// A `DataTransfer` that `SetTables` staged: (source table, destination table).
    DataTransfer(Option<(i32, i32)>),
    /// A `Cookie`: an untouched one is `Empty`, which reads as no name, no value and no attributes.
    Cookie(CookieState),
    /// An `HttpRequestMessage` that was configured (an untouched one is `Empty`).
    HttpRequest(HttpRequestState),
    /// An `HttpContent` that was written to or asked for its headers (an untouched one is `Empty`).
    HttpContent(HttpContentState),
    /// An `HttpHeaders` handed out by `GetHeaders`, `Headers` or `DefaultRequestHeaders`: the owner's own list.
    HttpHeaders(HttpHeaderList),
    /// An `HttpResponseMessage` (an untouched one is `Empty`, which reads as 200 OK).
    HttpResponse(HttpResponseState),
    /// An `HttpClient` that was configured (an untouched one is `Empty`).
    HttpClient(HttpClientState),
    /// A `TestHttpRequestMessage` a handler was given (an untouched one is `Empty`).
    TestHttpRequest(TestHttpRequestState),
    /// A `TestHttpResponseMessage` (an untouched one is `Empty`).
    TestHttpResponse(TestHttpResponseState),
    /// A `Version`: major, minor, build, revision.
    Version([i32; 4]),
    /// An `ErrorInfo` (the zero-argument `Create` and `Create(Message, ...)` build one).
    ErrorInfo(ErrorInfoState),
    /// A JSON node behind a JsonObject / JsonArray / JsonToken / JsonValue handle.
    Json(crate::json::JsonCell),
    /// A node behind an Xml* handle.
    Xml(crate::xml::XmlCell),
    /// An XmlNodeList: a snapshot of nodes.
    XmlList(Vec<crate::json::Handle>),
    /// An XmlAttributeCollection: a view of one element's attributes.
    XmlAttrs(crate::json::Handle),
    /// An XmlNameTable (opaque).
    XmlNameTable,
    /// An `XmlWriteOptions` that has been set: `PreserveWhitespace`.
    /// An `XmlReadOptions` that has been set: `PreserveWhitespace` (what is read does not depend on it).
    XmlReadOptions {
        preserve_whitespace: bool,
    },
    XmlWriteOptions {
        preserve_whitespace: bool,
    },
    /// A `File` that was created or opened, or had a mode set (an untouched one is `Empty`).
    File(crate::file::FileState),
    /// An XmlNamespaceManager: the (prefix, namespace) pairs added to it.
    XmlNamespaces(Vec<(String, String)>),
    /// An InStream: a snapshot being read.
    InStream(crate::stream::InState),
    /// An OutStream: appends to a Blob buffer.
    OutStream(crate::stream::BlobRef, crate::stream::Encoding),
}

impl Value {
    /// Exact structural identity: the same variant with the same contents, down to a decimal's scale and sign, a
    /// list's mutation count and every flag of a Blob buffer. This is NOT AL equality (`=` coerces, ignores case
    /// and compares numbers by value); use it only to ask "is this exactly what it was". Handle-like variants
    /// (record, codeunit, library, test page, query, DotNet) are compared by what their `Debug` prints, as the
    /// page rows did before this function existed.
    pub fn identical(&self, other: &Value) -> bool {
        fn all(a: &[Value], b: &[Value]) -> bool {
            a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.identical(y))
        }
        use Value::*;
        match (self, other) {
            (Null, Null) => true,
            (Boolean(a), Boolean(b)) => a == b,
            (Integer(a), Integer(b)) => a == b,
            (BigInteger(a), BigInteger(b)) => a == b,
            (Decimal(a), Decimal(b)) => a.serialize() == b.serialize(),
            (Byte(a), Byte(b)) => a == b,
            (Char(a), Char(b)) => a == b,
            (Text(a), Text(b)) | (Code(a), Code(b)) => a.as_str() == b.as_str(),
            (Date(a, f), Date(b, g)) => a == b && f == g,
            (Time(a), Time(b)) | (DateTime(a), DateTime(b)) | (Duration(a), Duration(b)) => a == b,
            (Guid(a), Guid(b)) => a == b,
            (DateFormula(a), DateFormula(b)) => a == b,
            (Option(a, n), Option(b, m)) => a == b && n[..] == m[..],
            (Enum(a, x), Enum(b, y)) => a == b && x == y,
            (List(a), List(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.version == b.version && all(&a, &b)
            }
            (Array(a), Array(b)) => all(&a.borrow(), &b.borrow()),
            (Dictionary(a), Dictionary(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.len() == b.len()
                    && a.iter()
                        .zip(b.iter())
                        .all(|((k, v), (l, w))| k.identical(l) && v.identical(w))
            }
            (Blob(a), Blob(b)) => {
                let (a, b) = (a.borrow(), b.borrow());
                a.bytes == b.bytes
                    && a.write_pos == b.write_pos
                    && a.loaded == b.loaded
                    && a.fetched == b.fetched
                    && a.refetch == b.refetch
                    && a.persisted == b.persisted
                    && a.written == b.written
                    && a.render_pending == b.render_pending
            }
            (Codeunit(_), Codeunit(_))
            | (Library(..), Library(..))
            | (Record(_), Record(_))
            | (TestPage(_), TestPage(_))
            | (TestControl(_), TestControl(_))
            | (Query(_), Query(_))
            | (DotNet(_), DotNet(_)) => {
                struct Shown<'a>(&'a Value);
                impl std::fmt::Display for Shown<'_> {
                    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        std::fmt::Debug::fmt(self.0, f)
                    }
                }
                Shown(self).to_string() == Shown(other).to_string()
            }
            _ => false,
        }
    }

    /// `identical` over two rows of fields.
    pub fn all_identical(a: &[Value], b: &[Value]) -> bool {
        a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.identical(y))
    }

    /// Whether the value holds no `Rc` handle: plain data (`Option` member lists and blob buffers
    /// are `Arc`s), so
    /// that rows made of such values can be read from several threads.
    pub fn is_plain(&self) -> bool {
        matches!(
            self,
            Value::Null
                | Value::Boolean(_)
                | Value::Integer(_)
                | Value::BigInteger(_)
                | Value::Decimal(_)
                | Value::Byte(_)
                | Value::Char(_)
                | Value::Text(_)
                | Value::Code(_)
                | Value::Date(..)
                | Value::Time(_)
                | Value::DateTime(_)
                | Value::Duration(_)
                | Value::Guid(_)
                | Value::DateFormula(_)
                | Value::Option(..)
                | Value::Enum(..)
                | Value::Blob(_)
                | Value::Unknown(_)
        )
    }

    /// Why the value cannot be read, for a column a platform provider left unknown.
    pub fn unknown(&self) -> Option<&str> {
        match self {
            Value::Unknown(why) => Some(why),
            _ => None,
        }
    }

    pub fn default_for(t: &RtType) -> Value {
        match t {
            RtType::Boolean => Value::Boolean(false),
            RtType::Integer => Value::Integer(0),
            RtType::BigInteger => Value::BigInteger(0),
            RtType::Decimal => Value::Decimal(Decimal::ZERO),
            RtType::Byte => Value::Byte(0),
            RtType::Char => Value::Char(0),
            RtType::Text(_) => Value::Text(String::new().into()),
            RtType::Code(_) => Value::Code(String::new().into()),
            RtType::Date => Value::Date(0, false),
            RtType::Time => Value::Time(0),
            RtType::DateTime => Value::DateTime(0),
            RtType::Duration => Value::Duration(0),
            RtType::Guid => Value::Guid([0; 16]),
            RtType::DateFormula => Value::DateFormula(String::new()),
            RtType::Option(m) => Value::Option(0, std::sync::Arc::from(m.as_slice())),
            RtType::Enum(id) => Value::Enum(*id, 0),
            RtType::Variant => Value::Null,
            RtType::List(_) => Value::List(Rc::default()),
            RtType::Dictionary(..) => Value::Dictionary(Rc::default()),
            RtType::Array(dims, el) => {
                let n = dims.iter().map(|d| *d as usize).product();
                Value::Array(Rc::new(RefCell::new((0..n).map(|_| Value::default_for(el)).collect())))
            }
            RtType::Codeunit(id) => Value::Codeunit(Rc::new(RefCell::new(Instance {
                object: *id,
                ..Default::default()
            }))),
            // An unassigned interface holds nothing.
            RtType::Interface(_) => Value::Null,
            RtType::TestPage(id) | RtType::Page(id) => Value::TestPage(Rc::new(RefCell::new(PageState::closed(*id)))),
            RtType::Query(id) => Value::Query(Rc::new(RefCell::new(QueryState {
                object: *id,
                filters: Vec::new(),
                filter_replaced: false,
                top: None,
                security_filtering: None,
                run: None,
                instance: None,
                in_before_open: false,
            }))),
            // Records need the table's field types: the VM builds them.
            RtType::Record(..) => Value::Null,
            RtType::Library(crate::library_kind::LibraryKind::Blob) => Value::Blob(crate::stream::new_blob()),
            // A Media or MediaSet holds the id of its media (or set): a Guid, null while empty.
            RtType::Library(crate::library_kind::LibraryKind::Media | crate::library_kind::LibraryKind::MediaSet) => {
                Value::Guid([0; 16])
            }
            RtType::Library(name) => Value::Library(name.clone(), Rc::default()),
            // An unassigned DotNet variable is null (ADR-014 section 4).
            RtType::DotNet(_) => Value::Null,
        }
    }

    /// Length in UTF-16 code units, as AL's StrLen and MaxStrLen count it.
    pub fn text_len(s: &str) -> usize {
        s.encode_utf16().count()
    }

    /// AL's dynamic type tests (`Variant.IsInteger`, ...).
    pub fn is(&self, kind: &str) -> bool {
        let k = kind.to_ascii_lowercase();
        match self {
            Value::Null => false,
            Value::Boolean(_) => k == "boolean",
            Value::Integer(_) => k == "integer",
            Value::BigInteger(_) => k == "biginteger",
            Value::Decimal(_) => k == "decimal",
            Value::Byte(_) => k == "byte",
            Value::Char(_) => k == "char",
            Value::Text(_) => k == "text",
            Value::Code(_) => k == "code",
            Value::Date(..) => k == "date",
            Value::Time(_) => k == "time",
            Value::DateTime(_) => k == "datetime",
            Value::Duration(_) => k == "duration",
            Value::Guid(_) => k == "guid",
            Value::DateFormula(_) => k == "dateformula",
            // An enum in a Variant reports IsOption: corpus types/TestVariantExtended.al:261.
            // A platform enum is its own type there, not an option (types/lt-variant-is-text-streams/LTP89IsEnumsA.al).
            Value::Option(_, members) => match crate::sysenum::kind_of(members) {
                Some(kind) => k == kind,
                None => k == "option",
            },
            Value::Enum(..) => k == "option",
            Value::List(_) => k == "list",
            Value::Dictionary(_) => k == "dictionary",
            Value::Array(_) => false,
            Value::Codeunit(_) => k == "codeunit",
            Value::Library(n, _) => n.eq_ignore_ascii_case(&k),
            Value::Record(_) => k == "record",
            Value::TestPage(_) | Value::TestControl(_) | Value::Query(_) => false,
            Value::Blob(_) => false,
            Value::DotNet(_) => k == "dotnet",
            // Never reached: a read of an unknown column is refused before its value reaches a Variant.
            Value::Unknown(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults() {
        assert!(matches!(Value::default_for(&RtType::Integer), Value::Integer(0)));
        assert!(matches!(Value::default_for(&RtType::Boolean), Value::Boolean(false)));
        assert!(matches!(Value::default_for(&RtType::Text(Some(10))), Value::Text(ref s) if s.is_empty()));
        assert!(matches!(Value::default_for(&RtType::Code(None)), Value::Code(ref s) if s.is_empty()));
        assert!(matches!(Value::default_for(&RtType::Date), Value::Date(0, false)));
        assert!(matches!(Value::default_for(&RtType::Decimal), Value::Decimal(d) if d.is_zero()));
        assert!(matches!(Value::default_for(&RtType::Variant), Value::Null));
        assert!(matches!(
            Value::default_for(&RtType::Option(vec!["A".into()])),
            Value::Option(0, _)
        ));
        assert!(matches!(Value::default_for(&RtType::Guid), Value::Guid(g) if g == [0; 16]));
    }

    #[test]
    fn arrays_default_to_their_element_default_per_cell() {
        let Value::Array(a) = Value::default_for(&RtType::Array(vec![2, 3], Box::new(RtType::Integer))) else {
            panic!()
        };
        assert_eq!(a.borrow().len(), 6);
        assert!(matches!(a.borrow()[5], Value::Integer(0)));
    }

    #[test]
    fn lists_share_on_clone_text_copies() {
        let l = Value::default_for(&RtType::List(Box::new(RtType::Integer)));
        let l2 = l.clone();
        if let Value::List(v) = &l {
            v.borrow_mut().push(Value::Integer(1));
        }
        let Value::List(v2) = &l2 else { panic!() };
        assert_eq!(v2.borrow().len(), 1, "a List is a reference type");
        let t = Value::Text("a".into());
        let mut t2 = t.clone();
        if let Value::Text(s) = &mut t2 {
            s.push('b');
        }
        assert!(matches!(t, Value::Text(ref s) if s == "a"), "Text is a value type");
    }

    #[test]
    fn each_default_list_is_a_fresh_handle() {
        let a = Value::default_for(&RtType::List(Box::new(RtType::Integer)));
        let b = Value::default_for(&RtType::List(Box::new(RtType::Integer)));
        let (Value::List(a), Value::List(b)) = (a, b) else {
            panic!()
        };
        assert!(!Rc::ptr_eq(&a, &b));
    }

    #[test]
    fn a_dropped_host_object_queues_its_handle_and_clones_share_it() {
        let queue = Rc::new(RefCell::new(Vec::new()));
        let a = Value::DotNet(Rc::new(DotNetObject::host(
            7,
            "System.IO.MemoryStream".into(),
            queue.clone(),
        )));
        let b = a.clone();
        drop(a);
        assert!(queue.borrow().is_empty(), "a second reference is alive");
        drop(b);
        assert_eq!(*queue.borrow(), vec![7]);
        // Enum and internal objects have nothing to release.
        drop(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Enum {
            clr_type: "E".into(),
            name: "A".into(),
            ordinal: 0,
        }))));
        assert_eq!(queue.borrow().len(), 1);
        assert!(matches!(Value::default_for(&RtType::DotNet(0)), Value::Null));
    }

    #[test]
    fn utf16_length() {
        assert_eq!(Value::text_len("abc"), 3);
        assert_eq!(Value::text_len("ä"), 1);
        assert_eq!(Value::text_len("😀"), 2, "a surrogate pair counts as two");
    }

    #[test]
    fn dynamic_type_tests() {
        assert!(Value::Integer(1).is("Integer"));
        assert!(!Value::Integer(1).is("Decimal"));
        assert!(Value::Code("A".into()).is("code"));
        assert!(!Value::Code("A".into()).is("Text"));
        assert!(Value::Option(1, std::sync::Arc::from(vec![])).is("Option"));
        assert!(Value::Enum(1, 0).is("Option"));
        assert!(!Value::Null.is("Integer"));
    }

    #[test]
    fn scalars_and_options_are_plain_and_handles_are_not() {
        let plain = [
            Value::Null,
            Value::Integer(1),
            Value::Text("a".into()),
            Value::Guid([1; 16]),
            Value::Enum(1, 2),
            Value::Option(1, std::sync::Arc::from(vec!["a".to_string()])),
        ];
        assert!(plain.iter().all(Value::is_plain));
        assert!(!Value::List(Default::default()).is_plain());
        assert!(!Value::Library("X".into(), Default::default()).is_plain());
    }
}

#[cfg(test)]
mod identical_tests {
    use super::*;
    use rust_decimal::Decimal;

    fn t(s: &str) -> Text {
        Text::new(s.to_string())
    }
    type Text = crate::text::Txt;

    #[test]
    fn same_contents_are_identical_and_different_are_not() {
        assert!(Value::Integer(1).identical(&Value::Integer(1)));
        assert!(!Value::Integer(1).identical(&Value::Integer(2)));
        assert!(!Value::Integer(1).identical(&Value::BigInteger(1)));
        assert!(Value::Null.identical(&Value::Null));
    }

    #[test]
    fn text_and_code_are_distinct() {
        assert!(Value::Text(t("A")).identical(&Value::Text(t("A"))));
        assert!(!Value::Text(t("A")).identical(&Value::Code(t("A"))));
        assert!(!Value::Text(t("A")).identical(&Value::Text(t("a"))));
    }

    #[test]
    fn enum_needs_id_and_ordinal() {
        assert!(Value::Enum(5, 1).identical(&Value::Enum(5, 1)));
        assert!(!Value::Enum(5, 1).identical(&Value::Enum(6, 1)));
        assert!(!Value::Enum(5, 1).identical(&Value::Enum(5, 2)));
    }

    #[test]
    fn decimals_by_exact_representation() {
        let z = Decimal::ZERO;
        let mut nz = Decimal::ZERO;
        nz.set_sign_negative(true);
        assert!(!Value::Decimal(z).identical(&Value::Decimal(nz)));
        assert!(!Value::Decimal(Decimal::new(10, 1)).identical(&Value::Decimal(Decimal::new(100, 2))));
        assert!(Value::Decimal(Decimal::new(10, 1)).identical(&Value::Decimal(Decimal::new(10, 1))));
    }

    #[test]
    fn blob_bytes_and_flags_are_compared() {
        let a = crate::stream::new_blob();
        let b = crate::stream::new_blob();
        assert!(Value::Blob(a.clone()).identical(&Value::Blob(b.clone())));
        crate::stream::write_bytes(&a, b"xy");
        assert!(!Value::Blob(a.clone()).identical(&Value::Blob(b.clone())));
        crate::stream::write_bytes(&b, b"xy");
        assert!(Value::Blob(a.clone()).identical(&Value::Blob(b.clone())));
        b.borrow_mut().loaded = false;
        assert!(!Value::Blob(a).identical(&Value::Blob(b)));
    }

    #[test]
    fn collections_compare_element_wise() {
        let l = |v: Vec<Value>| Value::List(Rc::new(RefCell::new(ListVec::from(v))));
        assert!(l(vec![Value::Integer(1)]).identical(&l(vec![Value::Integer(1)])));
        assert!(!l(vec![Value::Integer(1)]).identical(&l(vec![Value::Integer(2)])));
        assert!(!l(vec![Value::Integer(1)]).identical(&l(vec![])));
        let d = |v: Vec<(Value, Value)>| Value::Dictionary(Rc::new(RefCell::new(v)));
        let kv = |a: i32, b: i32| (Value::Integer(a), Value::Integer(b));
        assert!(d(vec![kv(1, 2)]).identical(&d(vec![kv(1, 2)])));
        assert!(!d(vec![kv(1, 2)]).identical(&d(vec![kv(1, 3)])));
        // A list nested in an array.
        let a = |v: Vec<Value>| Value::Array(Rc::new(RefCell::new(v)));
        assert!(a(vec![l(vec![Value::Byte(1)])]).identical(&a(vec![l(vec![Value::Byte(1)])])));
        assert!(!a(vec![l(vec![Value::Byte(1)])]).identical(&a(vec![l(vec![Value::Byte(2)])])));
    }

    #[test]
    fn rows_of_fields() {
        let x = vec![Value::Integer(1), Value::Text(t("a"))];
        assert!(Value::all_identical(&x, &x.clone()));
        assert!(!Value::all_identical(&x, &x[..1]));
    }
}
