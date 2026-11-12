//! The test client's state for one page, what a `TestControl` stands for (ADR-012), and the handles `Value::TestPage`
//! and `Value::TestControl` hold on them (ADR-072 decision 3). The state lives here, in the crate that reads and writes
//! it; `al-builtins` only stores, clones, drops and prints the handle (`al_builtins::page_handle`).
//!
//! A handle owns the same `Rc` the value held before the state moved here, so the count of references to a page does not
//! change: `page_held_bindings.rs` and `rows.rs` count them. Borrow the page with `st_of` (no new reference), take it
//! out of a value this code owns with `into_st`, and build a value with `page_value` (one new reference, as
//! `Value::TestPage(st.clone())` was).

use al_builtins::page_handle::{ControlHandle, PageHandle};
use al_builtins::value::{Instance, RecordState, RunPageLinkFill, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// The page a `Value::TestPage` holds. Every handle in a run is built here (`page_value`, `closed_page`); one that
/// holds anything else is a bug (ADR-072 decision 3).
pub(crate) fn st_of(handle: &PageHandle) -> &Rc<RefCell<PageState>> {
    handle
        .get()
        .expect("a TestPage value whose handle holds no PageState (ADR-072 decision 3)")
}

/// The page of a `Value::TestPage` this code owns: the value's reference moves, no new one is made.
pub(crate) fn into_st(handle: PageHandle) -> Rc<RefCell<PageState>> {
    match handle.into_inner() {
        Ok(st) => st,
        Err(_) => panic!("a TestPage value whose handle holds no PageState (ADR-072 decision 3)"),
    }
}

/// A `Value::TestPage` on `st`: one more reference to the page.
pub(crate) fn page_value(st: &Rc<RefCell<PageState>>) -> Value {
    Value::TestPage(PageHandle::new(st.clone()))
}

/// What a `Value::TestControl` holds.
pub(crate) fn control_of(handle: &ControlHandle) -> &Rc<ControlRef> {
    handle
        .get()
        .expect("a TestControl value whose handle holds no ControlRef (ADR-072 decision 3)")
}

/// A `Value::TestControl` for `target` on `page`.
pub(crate) fn test_control(page: Rc<RefCell<PageState>>, target: ControlTarget) -> Value {
    Value::TestControl(ControlHandle::new(Rc::new(ControlRef { page, target })))
}

/// A page of `object` that was never opened: what a declared `TestPage` or `Page` variable holds, and what `Clear`
/// leaves in one.
pub(crate) fn closed_page(object: u32) -> PageHandle {
    PageHandle::new(Rc::new(RefCell::new(PageState::closed(object))))
}

/// Tells `Value::default_for` how a closed page is built (it has no `Ctx` to ask). Every engine does this before it
/// makes a value, so no handle of a run holds anything but a `PageState`.
pub(crate) fn register_closed_page() {
    al_builtins::page_handle::set_closed_page(closed_page);
}

/// How a page was opened by the test client.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PageMode {
    #[default]
    View,
    Edit,
    New,
}

/// What put a page into lookup mode. `PageState::lookup` says whether the mode is on (a page's code can switch it with
/// `LookupMode`); this says who asked for it first. The pins differ by kind, so a reader names the kinds it is pinned for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LookupKind {
    /// Nothing did: the page is not a lookup.
    #[default]
    None,
    /// `LookupMode(..)` was called on the page variable or on `CurrPage` (the mode it set is `PageState::lookup`).
    Variable,
    /// The static `Page.RunModal(Id, Rec)` or `Page.RunModal(0, Rec)` with its return value used: the call sets the mode, not
    /// `LookupMode(true)` on a page variable. Called as a statement the page is not a lookup
    /// (pages/triggers/BeRunModalOnLookup.al:916).
    StaticRunModal,
    /// `TestField.Lookup()` opened the page over a TableRelation.
    Relation,
}

/// A row a repeater has populated: its primary key, the fields as stored when it was read and as
/// the page showed them after OnAfterGetRecord (the page keeps that buffer for the row while it
/// stays in the window, corpus testpage/TestPageActionRunObjectPageStoredRow_Tests.al:
/// OpenHostOnB reads `CALC`, which only the host's OnAfterGetRecord puts into Rec.Grp).
#[derive(Debug, Clone)]
pub struct WindowRow {
    pub key: Vec<Value>,
    pub stored: Vec<Value>,
    pub shown: Vec<Value>,
}

/// Where the test client's cursor stands on a page (ADR-012).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Cursor {
    /// No row: an empty view.
    #[default]
    None,
    /// On a stored row (or, for a new record, the row being typed).
    Row,
    /// On the blank line an editable, insert-allowed repeater carries past its data
    /// (corpus testpage/TestPageNewRowLine_Tests.al:1-30).
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

/// A row key and what the repeater's expression controls showed when the list read the row: (control index, value; `None`
/// when the expression raised an error then) (`PageState::row_exprs`).
pub type RowExprs = (Vec<Value>, Vec<(usize, Option<Value>)>);

/// The view of a page's record an action started from: filters, key, the fields of an ad-hoc key, Ascending
/// (`PageState::view_before_action`).
pub type ActionView = (Vec<al_builtins::filter::FieldFilter>, usize, Option<Vec<usize>>, bool);

/// What closed a page (`PageState::closed_by`), set where the close happens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClosedBy {
    /// `TestPage.Close()` ran to its end (no veto, no error) on a page the test opened with OpenEdit, OpenView or OpenNew.
    TestClose,
    /// `CurrPage.Close()`, or `Close()` of a Page variable.
    PageCode,
    /// An action that closes the page (OK, Cancel, LookupOK, ...), invoked by the test or a handler.
    Action,
    /// The platform, after a page handler returned and left the page open.
    Platform,
    /// Any other close: a refused or failed `TestPage.Close()`, the close at the end of the test, a modal page closed after
    /// its handler raised an error, an error while the page opened, a failed save.
    Other,
}

/// The test client's state for one page (ADR-012). Fields grow with the
/// behaviours the corpus pins. A new field starts at its type's default (`PageState::closed`).
#[derive(Default)]
pub struct PageState {
    /// Object index of the page.
    pub object: u32,
    /// Open pages answer; closed ones, and ones torn down by an error in a row load,
    /// raise "The TestPage is not open." (corpus testpage/lifecycle/TestPageTeardown.al:3-24).
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
    /// The save of a changed row failed on a page whose ModifyAllowed is false (`SaveAbort`): the change stays shown, nothing is
    /// stored and a Next no longer moves (pages/modify-denied/Y80ModifyDenied.al Y80_MD_List_Set_Next_Next).
    pub save_aborted: bool,
    /// `Visible` of each control as it was when the page opened (the property is frozen then,
    /// corpus testpage/TestPageControlPropertyLiveness_Tests.al:56).
    pub frozen_visible: Vec<bool>,
    /// The session language when the page opened: OptionCaptionML shows the captions of that language, whatever
    /// `GlobalLanguage` says later (pages/controls/AaFieldProps.al:179-219, pages/extensions/
    /// ZModifyControls.al:322-360). `None` before the first open.
    pub open_language: Option<i32>,
    /// The caption each control's CaptionClass resolved to when the page filled: (control index, class text, caption).
    /// `Caption()` reads it without raising OnResolveCaptionClass again (corpus testpage/W2CaptionClass.al:113-116).
    pub control_captions: Vec<(usize, String, String)>,
    /// Why the caption in `control_captions` of a control is not known: (control index, refusal). `Caption()` refuses with it.
    /// Only read together with a `control_captions` entry of the same control, which an open clears.
    pub caption_refusals: Vec<(usize, String)>,
    /// The caption of each control over a table field whose CaptionClass is an expression, as the page filled it: (control
    /// index, caption). `Caption()` reads it and runs nothing (record/captions/QdCaptionClassAreas.al:422-481).
    pub field_captions: Vec<(usize, String)>,
    /// The open ran the CaptionClass expressions of the source table's fields with an empty record (`field_captions`).
    pub field_captions_opened: bool,
    /// The open has filled its first rows (`field_captions` no longer changes at the fills of the open itself).
    pub field_captions_settled: bool,
    /// The row (its field values) the CaptionClass expressions last ran with for the controls' captions.
    pub field_captions_row: Option<Vec<Value>>,
    /// A move whose row fills run no CaptionClass expression (GoToKey, GoToRecord and Previous on a Card or Document page).
    pub field_captions_quiet: bool,
    /// A move whose CaptionClass expression run with the new row is pinned (First and Last on a Card).
    pub field_captions_pinned_move: bool,
    /// A `Next()` of the page is filling its row.
    pub moving_next: bool,
    /// A fill after the open (a part refilled after its host moved) runs the CaptionClass expressions with its first row
    /// only, as the open does (record/captions/QdCaptionClassAreas.al:1022-1066 QO_Doc_OpenEdit_HostSteps).
    pub field_captions_fill_first: bool,
    /// The refill comes from a SetValue of the host's link field: when it finds no row it runs nothing
    /// (QdCaptionClassAreas.al:1068-1100 QO_Doc_OpenNew `no_setvalue`).
    pub field_captions_link_refill: bool,
    /// A FactBox part of a Provider chain at the refill after its host was inserted by the first call on a part: 1 while the refill
    /// skips it, 2 until the key typed into the host reads it (`provider_hidden.rs`).
    pub chain_refill: u8,
    /// The part does not read again when Close inserts its new host row (`draft.rs` `quiet_parts_at_new_host_close`).
    pub quiet_at_close: bool,
    /// The fields of the new row once it started (after the filters stamped it and OnNewRecord ran): what the test typed since
    /// is what differs from it (`dirty_step.rs` `refuse_action_on_new_host_with_parts`).
    pub draft_start: Option<Vec<Value>>,
    /// The new row is the one TestPage.OpenNew started (not a later New() or a filter's new row): its parts' blank reads are
    /// pinned (`card_part_new_host.rs`).
    pub draft_from_open: bool,
    /// A part a Visible expression hid at open has been visible since, at one of the times the runner looks: the refill as
    /// the host opens, a later refill, before or after an action's body (`provider_hidden.rs`).
    pub seen_visible: bool,
    /// Such a part was visible at its host's last refill (`provider_hidden.rs`).
    pub visible_at_sync: bool,
    /// Where an action of a Document page stands that saved a SetValue of the host before its trigger: 0 none, 1 that save's
    /// refresh is running, 2 the save is done and a CurrPage.Update in the trigger has not refreshed yet, 3 it has
    /// (record/captions/QdCaptionClassAreas.al:1015-1049 QO_Doc_OpenEdit_HostSteps `update`).
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
    /// The page opened as a temporary Worksheet whose rows a procedure set before the open (`open_temp_ws_preloaded`).
    pub temp_ws_preloaded: bool,
    /// The control validation that runs is an edit of that page the corpus pins, noted with the number of moves and writes
    /// the test had made (`temp_ws_edit_begin`).
    pub temp_ws_edit: Option<(usize, crate::pages::open::TempWsEdit)>,
    /// The page runs from `CurrPage.Run()` in its own procedure (`run_page_var`), not from a Page variable's `Run()`.
    pub run_by_currpage: bool,
    /// The page dropped a key the test typed into a stored row (`typed_key_dropped_on_currpage_run`).
    pub typed_key_dropped: bool,
    /// A read of a list that serves its rows from OnFindRecord and OnNextRecord ran to the end of the view (an open whose view
    /// ends inside the window, a GoToKey that read on to the end): Next on the last row then answers false without asking the
    /// page (see `move_in_client_rows`).
    pub view_read_to_end: bool,
    /// The open read of a descending find list went up from the row it opened on, the last row of the view: the window holds
    /// the rows above it in client order and the cursor stands on its last row (see `read_rows_up_from_the_last`).
    pub window_read_up: bool,
    /// The open read of a descending find list without row triggers stopped a full window of rows above the row it opened on,
    /// the last row of the view, with rows above still unread: `First` reads them (see `first_above_the_find_list_window`).
    pub window_top_unread: bool,
    /// The open read of a plain list on the row its OnOpenPage left Rec on held more rows than a window: (rows above the top row
    /// it holds were left unread, rows below its bottom row were left unread). `First` and `Last` read on from that edge (al-vm
    /// `read_rows_above_the_moved_rec`).
    pub moved_rec_open_partial: Option<(bool, bool)>,
    /// A modal list in a key other than the primary key opened on the last row of its descending view and read the rows above it
    /// (see `read_rows_up_from_the_landed_last_row` in al-vm): `Some(false)` after that open, `Some(true)` after the GoToKey that
    /// walked its rows from the first one. Other moves from there are not pinned.
    pub read_up_from_last: Option<bool>,
    /// The open read of a list with OnFindRecord and OnNextRecord stepped back from a full first window over the rows above it
    /// (assumption list-open-full-window-rows-before, al-runner issue 5887): the client holds those rows and the window, this
    /// many in all. Next and Previous inside them read nothing; other moves are refused. Stale once the window has another length.
    pub window_held_above: Option<usize>,
    /// The page is landing on its first row as it opens (the reads of a view that ends inside the window differ there).
    pub landing_first: bool,
    /// The landing read started with OnFindRecord(`-`) after a TestPart filter (`find_list_part`): the window starts
    /// at the first row of the view, so the open read's tail does not step back with OnNextRecord(-1).
    pub landed_from_view_start: bool,
    /// The window the page holds starts at the first row of the view: such a landing read, or the open read of the same part
    /// whose OnNextRecord(-1) answered 0 (`part_filter_find.rs`); cleared by the next window read.
    pub find_window_at_view_start: bool,
    /// The page opens on the row its OnOpenPage left Rec on, not the first of the view: a list's open read also reads the
    /// rows above it (see `read_rows_above_the_moved_rec`).
    pub open_reads_above: bool,
    /// The list opened on the row its OnOpenPage left Rec on (see `open_on_rec_moved_by_onopenpage`).
    pub opened_on_moved_rec: bool,
    /// The client read the rows around the cursor row again after a move onto a deleted row (`moved_onto_deleted`): Previous
    /// from the new-row line right after reads the last row as a move between rows does (Y80_Moved_All_FirstOntoDeleted
    /// `previous log`). Cleared by the next First, Last or Previous.
    pub refreshed_after_gone_move: bool,
    /// An action of the page ran since it opened: a row an action deleted is not the pinned `moved_onto_deleted` shape (that
    /// one is pinned for a row the test code deleted; PnGRefresh.al ListLosesOneOfTwoRowsWithoutUpdate pins another log).
    pub action_ran: bool,
    /// The page settled on the row after the one an action deleted (`arrive_after_gone_row`): the refresh that follows does not
    /// read that row first.
    pub gone_row_settled: bool,
    /// A list part opens on the row the host's code left its Rec on (`part_rec_row.rs`): its open read reads the rows above.
    pub part_on_host_row: bool,
    /// The page's source is temporary and its OnOpenPage left Rec as it stood before (key, order, filters and row): the row
    /// was set before the open, for example by a procedure of the page variable (see `rec_moved_by_onopenpage`).
    pub onopenpage_kept_rec: bool,
    /// A record was handed to the page (Page.Run(Id, Rec)) and its OnOpenPage left Rec on a primary key other than the handed one
    /// (`handed_moved.rs`).
    pub handed_rec_moved_by_onopenpage: bool,
    /// The value the page's DataCaptionExpression returned at its last evaluation, which happens when a row arrives
    /// (`data_caption_run.rs`); a TestPage's Caption() shows it without evaluating the expression again.
    pub data_caption_value: Option<String>,
    /// The page globals the DataCaptionExpression assigned, by global index: the page's code and controls see them only from
    /// the next row that arrives (`data_caption_run.rs`).
    pub data_caption_pending: Vec<(usize, Value)>,
    /// A row trigger ran since the DataCaptionExpression last did: it runs before the next trigger other than OnAfterGetRecord
    /// and before the test code touches the page.
    pub data_caption_due: bool,
    /// A lookup page handed the related row a field holds opens on it as a list opens on a row its OnOpenPage left Rec on
    /// (`lookup_start.rs`).
    pub lookup_held_row: bool,
    /// Set by the opener for the page's next open: while OnOpenPage runs, Rec's buffer is not pinned and a use that reads it
    /// refuses with this reason (`RecordState::unpinned_buffer`; a page an action opens through `RunPageLink`).
    pub open_rec_unpinned: Option<&'static str>,
    /// The page was opened by an action through `RunPageLink` (set as it opens, from `open_rec_unpinned`).
    pub run_page_link_opened: bool,
    /// Its OnOpenPage moved Rec to a linked row with a FindFirst or FindLast that found one.
    pub run_page_link_open_moved: bool,
    /// Set by the opener for the page's next open: the values an action's `RunPageLink` fills into Rec before OnOpenPage
    /// (field position, value), with the link's filters (`RecordState::run_page_link`). `None`: the link fills nothing pinned.
    pub open_rec_fill: Option<RunPageLinkFill>,
    /// A temporary Worksheet opened on the row in a key other than the primary key that was set before the open: it reads each
    /// row once more, as a list over a database table in such a key does (see `reads_once_more_in_its_key`).
    pub temp_reads_once_more: bool,
    /// The list opened in a key other than the primary key (key index, ascending) and read its row once more for it; it reads
    /// each row once more at Next and Previous while Rec stays in that key and order (see `reads_once_more_in_its_key`).
    pub reads_once_more_in: Option<(usize, bool)>,
    /// The page is reading the rows it shows as it opens: from the open, or from a refresh that reads them again as the open
    /// does (`replay_open_reads`, `lookup_next_refresh`).
    pub open_read: bool,
    /// The page is in the read of the open itself (not a later read that replays it) and no OnAfterGetRecord of that read has
    /// ended yet: the moment the extension columns of `pages/ext_column_visible.rs` take their Visible after.
    pub before_first_open_row: bool,
    /// The columns (control indices) that first OnAfterGetRecord turned from hidden to shown (`pages/ext_column_visible.rs`
    /// `ext_column_shown`); set anew by every open.
    pub ext_columns_shown: Vec<usize>,
    /// The page opened for edit over an empty view and ran its new-record triggers on a blank record; it shows no row and its
    /// field controls are not editable.
    pub blank_on_open: bool,
    /// The part reads its rows last to first, once each, at its next fill: as it opens with its host (see
    /// `view_names_link_field`), or after its host moved back (see `sync_parts_after_move`).
    pub read_backwards: Option<ReadBackwards>,
    /// `CurrPage.SetSelectionFilter(Rec)` ran on this page while it stood on no row: the part's next refresh with its host
    /// drops the filter (pages/parts/HiddenPartSelectionFilter.al).
    pub selection_on_rec: bool,
    /// The control that has the client's focus, and whether it is on the current row. Moving
    /// focus to a non-key control of a new row is what inserts it (corpus
    /// testpage/TestPageBlankKeyInsert.al:1-25).
    pub focus: Option<usize>,
    pub focus_in_row: bool,
    /// Validation errors the client holds: (control index, stored text).
    pub errors: Vec<(usize, String)>,
    /// A SetValue of a value that is not a Text, whose text is longer than the control's field: (control index, the text, the error
    /// the save raises). The control shows the text; the next SetValue of the control drops it; the Close raises the error
    /// (pages/properties/Y80SetValueTypes.al Y80_ST_Card_ShortCtl_*).
    pub deferred_overlong: Option<(usize, String, String)>,
    /// The control and the list row (its key) where a control's ValuesAllowed refused an entry: a Value read of that control on
    /// another row is not pinned (pages/values-allowed-set-value/Y37ValuesAllowedSetValue.al:313-316).
    pub values_allowed_refused_row: Option<(usize, Vec<Value>)>,
    /// The controls among `errors` whose refusal was an error of an AL trigger (the table's or the control's OnValidate),
    /// not one of the client's own checks, with the trigger's message: Close validates those again
    /// (testpage/AhPageTests.al:155-159).
    pub trigger_errors: Vec<(usize, String)>,
    /// A RunObject action saved the row's edit before its target ran (pages/actions/
    /// Y45SaveBeforeAction.al:431 List_SetName_RunObjectCodeunit): a list reads its rows again after the codeunit.
    pub saved_before_run_object: bool,
    /// A SetValue left the row dirty since the last save: an action's save runs the modify chain even when the value is the one
    /// stored (pages/actions/Y45SaveBeforeAction.al:371 Card_SetSameName_Action).
    pub set_value_held: bool,
    /// The row an OnModifyRecordEvent subscriber refused to save (AllowModify false): the page does not save it again; another
    /// edit saved after it is not pinned (al-vm pages/draft.rs `save_row`).
    pub modify_vetoed: Option<Vec<Value>>,
    /// A new line of this part was inserted before a host action: the part's refresh after the action stays on it
    /// (pages/actions/Y45SaveBeforeAction.al:1748 Doc_Existing_NewLine_Action `...;C H1;lC 30000`).
    pub inserted_before_host_action: bool,
    /// The call depth (`Session::frames`) at which the client's save of this page's row last failed, cleared before each step
    /// the client saves for: tells the save before a step from a save the step's own code made.
    pub save_failure_depth: Option<usize>,
    /// The client's save of this page's row before a step failed (pages/actions/Y45SaveBeforeAction.al:1808
    /// Card_ModifyError_Action_PageState): the page keeps the edit and one validation error; see `pages/dirty_step.rs`.
    pub save_failed: bool,
    /// The started line's insert was refused and the client moved on without raising
    /// (corpus testpage/TestPageFailedInsert_Tests.al:1-30): `refused` keeps the line, `pending`
    /// is the line left behind by Previous()/First(), whose insert fails again at Close().
    pub refused: bool,
    /// The last close was refused because OnQueryClosePage raised an error (shown as a message).
    pub close_error: bool,
    /// A `TestPage.Close()` of this Card failed on the save of its row (al-vm `pages/close_save_failed.rs`).
    pub close_save_failed: bool,
    /// The last close was refused because the page's own OnQueryClosePage answered false.
    pub close_vetoed: bool,
    /// A `TestPage.Close()` in the modal page's handler was refused by OnQueryClosePage and closed the TestPage all the same.
    pub test_close_vetoed: bool,
    /// A `TestPage.Close()` in the modal page's handler closed the page (no veto).
    pub test_closed_in_handler: bool,
    /// How the current modal run started: `Some(true)` the static `Page.RunModal(Id)`, `Some(false)` a Page variable's
    /// `RunModal()`, `None` any other run (an action, a lookup).
    pub run_static: Option<bool>,
    /// The row the page was left on when it closed with LookupOK (`Page.RunModal(Id, Rec)` hands it back to `Rec`).
    pub selection: Option<Vec<Value>>,
    pub pending: Option<Vec<Value>>,
    /// Opened by AL code (`Page.RunModal`, `Page.Run`) and answered by a handler, not by the test.
    pub modal: bool,
    /// The current run is AL code's non-modal `Page.Run` (`note_page_run`, `page_run_close.rs`); cleared when the page closes.
    pub page_run: Option<super::page_run_close::PageRun>,
    /// The first calls AL code made on this page through its Page variable or `CurrPage`, at most three
    /// (`note_page_code_call`, `page_run_close.rs`).
    pub code_calls: Vec<al_builtins::page_op::PageMethod>,
    /// Whether the last run of this variable was modal (`None` before the first run): a second non-modal Run runs OnInit again.
    pub last_run_modal: Option<bool>,
    /// The rows of a tree (a repeater with ShowAsTree) expanded with `Expand(true)`, by primary key.
    pub expanded: Vec<Vec<Value>>,
    /// The rows a tree shows (its cut), by primary key, as last computed (`apply_tree_cut`).
    pub tree_shown: Vec<Vec<Value>>,
    /// The rows a tree with row triggers read when it opened but does not show (below a collapsed row), in the view's order:
    /// they are not in `window`, and an Expand moves them there without reading them again.
    pub tree_hidden: Vec<WindowRow>,
    /// The level of each row of a tree whose IndentationColumn is a page variable, by primary key: the variable's value after
    /// OnAfterGetRecord last ran for the row (`pages/open/tree_var.rs`).
    pub tree_levels: Vec<(Vec<Value>, i32)>,
    /// The page is in lookup mode (what `LookupMode()` answers). What put it there is `lookup_kind`.
    pub lookup: bool,
    /// What put the page into lookup mode; it stays when the page's code switches the mode off with `LookupMode(false)`.
    pub lookup_kind: LookupKind,
    /// For a relation's lookup page: the pinned place of the control that opened it, and what closed it
    /// (`pages/relation_lookup_close.rs`).
    pub relation_origin: Option<crate::pages::relation_lookup_close::RelationOrigin>,
    pub relation_close: Option<crate::pages::relation_lookup_close::RelationClose>,
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
    /// The control validation step that runs now and what it asked of the page's xRec (`pages/validate_xrec.rs`).
    pub validate_step: Option<crate::pages::validate_xrec::ValidateStep>,
    /// Rec as it was when a page variable control started to validate; a change its OnValidate makes in code marks the row
    /// edited (`pages/dirty_step.rs`, al-runner issue 6669).
    pub rec_before_var_validate: Option<Vec<Value>>,
    /// CurrPage.Update ran while the page's row was stored but outside the view the action had just set (list refresh).
    pub view_left: bool,
    /// An OnDrillDown turned MarkedOnly on for Rec without `CurrPage.Update`: the client still shows the rows of the view
    /// without MarkedOnly (testpage/ScDrillDownMarkedOnly.al, DrilldownWithoutUpdateListAndGoTo).
    pub marked_only_unshown: bool,
    /// The same OnDrillDown on a list WITH OnAfterGetRecord or OnAfterGetCurrRecord: what the page shows then is not pinned
    /// (corpus testpage/ScDrillDownMarkedOnly.al:408 covers only a page without them); every TestPage call
    /// but Close is refused until Close.
    pub marked_only_unpinned: bool,
    /// A TestField.Lookup on this page, whose OnValidate called `CurrPage.Update(false)`, left that refresh for later, as BC
    /// does on a host with row triggers whose ListPart links two or three fields (`Session::defer_lookup_refresh`): reads and Close
    /// run no trigger; any other TestPage call on its tree is refused until Close.
    pub lookup_refresh_deferred: bool,
    /// A TestPage call reached this page as a part of its host (`lookup_refresh_deferred_call`).
    pub lookup_part_touched: bool,
    /// TestField.Lookup writes the text its OnLookup returned through the control's SetValue (`defer_lookup_refresh`).
    pub lookup_setting_value: bool,
    /// The TestField.Lookup running on this page wrote a value (its OnLookup returned true); false after a cancelled Lookup.
    pub lookup_value_set: bool,
    /// The first TestPart move after the Lookup ran the refresh the Lookup left (`Session::run_lookup_refresh_left`); the
    /// part's later moves and the host's Next are pinned until another call.
    pub lookup_refresh_used: bool,
    /// The host's Next is the first call after a Lookup that left its refresh for later: the move runs that refresh too
    /// (`Session::lookup_next_refresh`).
    pub lookup_next_refresh: bool,
    /// The host's move is the first call after a Lookup that left its refresh for later: the refresh runs after the move
    /// (`Session::lookup_move_refresh_after`).
    pub lookup_move_refresh: bool,
    /// That refresh runs on a host without OnAfterGetCurrRecord: its reads differ (`Session::lookup_next_refresh`).
    pub lookup_next_plain: bool,
    /// The host read the row of its unlinked ListPart after that part moved, and has not moved since (`Session::host_follows_part`).
    pub host_followed_part: bool,
    /// An action turned MarkedOnly on for a list with row triggers: the later moves are refused (not built).
    pub marked_only_by_action: bool,
    /// An action runs on a list whose Rec had MarkedOnly off: a refresh after it that finds MarkedOnly on holds the marked rows it
    /// read (`marked_rows_held`).
    pub marking_action: bool,
    /// The refresh after such an action took the marked rows it read as the rows the client holds.
    pub marked_rows_held: bool,
    /// An action left the list's view without rows (`empty_view_after_action`): while the page shows no row, its repeater
    /// controls read blank, page variables too.
    pub empty_after_action: bool,
    /// While an action runs: the view of the page's record (filters, key and its ad-hoc fields, Ascending) as it was before
    /// the action, so the refresh inside the action can tell the view changed (al-runner issue 6381).
    pub view_before_action: Option<ActionView>,
    /// An action runs on a list whose Rec had MarkedOnly on: a refresh after it that finds MarkedOnly off holds the rows it read.
    pub unmarking_action: bool,
    /// A marking OnDrillDown with CurrPage.Update(false) left Rec on another row of a list without row triggers, with a part, in a
    /// key other than the primary key: a field read before the next other call shows this row, the one shown before.
    pub shown_until_next_call: Option<Vec<Value>>,
    /// The primary key the last TestPage GoToKey or GoToRecord looked for (`marked_view_changed_by_move`).
    pub go_to_want: Option<Vec<Value>>,
    /// A marking OnDrillDown with CurrPage.Update(false) moved Rec off the shown row of a list in its primary key: a field read
    /// before the next move is refused (the page still shows the old row there; not built).
    pub marked_only_read_unbuilt: bool,
    /// An OnDrillDown turned MarkedOnly on and called `CurrPage.Update(false)` on a list in a key other than the primary key: the
    /// client still shows the row it stood on and refills its rows at the next GoToRecord (al-runner issue 6008). Holds the
    /// fields of Rec as the trigger left it; Rec shows the client's row until then.
    pub marked_refill_in_key: Option<Vec<Value>>,
    /// A TestPage GoToKey moved the cursor of a list without OnAfterGetCurrRecord: the client reads the row once more before
    /// every later action (pages/list-order/AiListOrderTests.al:39 Keep_Nav `update` = `R:R1002;R:R1002;A:filter...`;
    /// QgGoToKeyNextCall.al:297 Plain_GoToKey_ThenActionWithoutUpdate_Twice `second_action_log`), until a move clears it.
    pub read_after_goto: bool,
    /// A call that is not pinned (a property read, an `As*` read) came between that GoToKey and the next call: whether the row
    /// is still read again before an action is not pinned (the action is refused). A Value read and SetValue are pinned
    /// (QgGoToKeyNextCall.al:236, :317) and do not set it.
    pub read_after_goto_doubt: bool,
    /// That GoToKey ran no page trigger (the key is among the rows the client holds): Close then reads the row once
    /// (pages/list-order/QgGoToKeyNextCall.al:223 Plain_GoToKey_ThenValueRead, :277 Plain_GoToKey_ThenClose `close_log`
    /// `R:R1003`). A GoToKey that read rows leaves Close reading nothing (testpage/TestPageSystemPartListWindowContracts.al:219).
    pub read_after_goto_quiet: bool,
    /// A SetValue came between that GoToKey and the next call.
    pub read_after_goto_set: bool,
    /// A Previous came after that GoToKey and no action since: what Close then reads is not pinned (it is refused).
    pub read_after_goto_prev: bool,
    /// A list opened for edit over an empty view went back to its new-row line after an action stored one row through another
    /// variable: First then reads that row as the list reads it on open (testpage/PnGShowRow.al:223).
    pub empty_list_row_behind: bool,
    /// An action put a list that showed stored rows on a row the client did not hold before it (an insert through Rec): what a
    /// move reads next is not pinned (testpage/PnGShowRow.al:144 NonEmptyListInsertViaRec `first-after` reads
    /// nothing, the Next after it reads the row it lands on twice); a move is refused.
    pub action_landed_on_new_row: bool,
    /// The action of a tree part that showed no row left rows and the part read them (al-vm `pages/part_tree_typed.rs`
    /// `land_empty_tree_part_after_action`); read and cleared by `action_invoke`.
    pub empty_tree_part_landed: bool,
    /// A refresh after a control's OnValidate changed a list's filters read more rows than a window holds: what the client holds
    /// then is not pinned, a move is refused (assumption validate-filter-refresh-window, al-runner issue 5914).
    pub validate_refresh_unheld: bool,
    /// The refresh after an action that changed the view of a list with OnFindRecord read more rows than a window holds: a
    /// move is refused (al-runner issue 6469).
    pub action_refresh_unheld: bool,
    /// The fields the page's server-side Rec still has after the test client moved onto a row it holds without running a read
    /// trigger (`Session::note_server_row_lag`): a procedure called through a held `Page` variable reads this row.
    pub server_lag: Option<Vec<Value>>,
    /// The key of the row a `Next` that started `server_lag` just moved onto, on the page `held_row_page_pinned` names, and
    /// the store's write mark then (`next_held_row_read`). Any other call on the page or its controls clears it.
    pub next_held_row: Option<(Vec<Value>, (i64, u64))>,
    /// The calls the test client made on the page since it opened, the running one last (`pinned_shape.rs`).
    pub test_calls: Vec<crate::pages::pinned_shape::TestCall>,
    /// The test made more calls than `test_calls` keeps.
    pub test_calls_overflow: bool,
    /// The calls the test client made on the page and on its direct parts since it opened, in the order it made them, the
    /// running one last (`pinned_shape.rs`, `Session::part_steps`).
    pub tree_calls: Vec<crate::pages::pinned_shape::TreeCall>,
    /// `tree_calls` does not tell what happened to the page: more calls than it keeps, a call on a part of a part, or a write
    /// to the store between two calls (the test changed rows the page shows).
    pub tree_calls_unknown: bool,
    /// The store's write mark when the last call on the page or its parts ended; `None` before the first one ended.
    pub tree_calls_mark: Option<(i64, u64)>,
    /// A ListPart that shows no row on a top-level List host holds on Rec the link values of a landing BC was measured at
    /// (`empty_part_rec.rs`, `tie_empty_list_stamp`): the number of steps on the host and its parts (`Session::part_steps`)
    /// up to that landing, and the number of reads by an action of the part the landing's cell measured.
    pub empty_list_stamp: Option<(usize, usize)>,
    /// The test called `First()` on the page since it opened (`next_held_row_read`; set and cleared by `test_page_op_held`).
    pub first_since_open: bool,
    /// The static `Page.RunModal(0, Rec)` found the page from the table's LookupPageId.
    pub lookup_page_from_table: bool,
    /// Page triggers (the page's or an extension's) run on this page so far.
    pub triggers_run: u64,
    /// The reads after an action on a list without OnAfterGetCurrRecord ended the refresh: `CurrPage.Update` reads nothing more
    /// (pages/list-order/AiListOrderTests.al:233 Keep_Plain).
    pub refresh_read_done: bool,
    /// The fields of the row a list showed when the running action started (None outside an action): an action that put another
    /// row into Rec and called CurrPage.Close() gets this row back (testpage/PnGRefresh.al:608).
    pub action_shown_row: Option<Vec<Value>>,
    /// The action left another row (another primary key) in Rec: the list reads that row again from the table, unsaved values
    /// dropped (testpage/PnGRefresh.al:387 QL_RecAssign_OtherChanged_ThenClose `P.AGR(B,changed B);P.AGR(B,orig B)`).
    pub rec_moved_other: bool,
    /// CurrPage.Close() in the running action put the shown row back into Rec (PnGRefresh.al:608): one more read follows the action.
    pub close_restored_row: bool,
    /// A card closed by its deleted row moved to another row and read it before OnQueryClosePage: the read of the row that
    /// follows a close in the action is not made for it (corpus pages/deleted-rows-navigation/Y89GoneSingleMove.al:4372).
    pub gone_closed_on_row: bool,
    /// The blank line of a top-level list took key filter values (a journal filtered to one template and batch, open on no row or
    /// after its only row went): its first line is numbered 10000 by AutoSplitKey, not 20000 (misc/lt-probes-needed-7/
    /// PnGRefresh.al:672, :792 `P.INSR(T1/B1/10000,typed)`).
    pub split_starts_low: bool,
    /// The action changed the row a list shows: it is saved after the first read of the refresh (misc/lt-probes-needed-7/
    /// PnGRefresh.al:503).
    pub save_after_first_read: bool,
    /// The parts read their rows for a host row a GoToKey walked past (pages/parts/QjPartPropagation.al:1183):
    /// on the target row they read again even when the host stayed on it.
    pub walk_synced_parts: bool,
    /// xRec of a part that shows no row as it landed on its blank line: OnNewRecord sees it again on the part's Last() and New()
    /// (pages/parts/QjPartPropagation.al:1346 QL_PartLastNew_NoLines `x=3 x.ln=30000`, the row the part stood on
    /// for the host row before).
    pub empty_part_xrec: Option<Vec<Value>>,
    /// The part stands on the blank line its Last() ran on a view without rows: a part without DelayedInsert and AutoSplitKey
    /// inserts that line at the first write to a field outside the key (pages/parts/QjPartPropagation.al:3696).
    pub blank_after_empty_last: bool,
    /// A list without the new-row line lost its only row to an action and stands on a blank Rec: a later action runs
    /// OnNewRecord and OnAfterGetCurrRecord again (testpage/RmWorksheetGoneRow.al:336).
    pub gone_blank_no_new_line: bool,
    /// The built-in View switched this list without a card from edit to view (al-runner issue 6977,
    /// testpage/R6977ListViewNext.al:138, R6977ListAfterView.al:125): the built-in Edit switches it back.
    pub list_view_switched: bool,
    /// The built-in Edit switched such a list back to edit after View (testpage/R6977ListAfterView.al:187). Only Editable,
    /// the fields, the Edit and View Enabled reads, ValidationErrorCount and Close are pinned in this state.
    pub list_edit_back: bool,
    /// A list with the new-row line lost its only row to an action and stands on that line: its fields show the blank Rec's
    /// values, an Integer key 0 (testpage/RmWorksheetGoneRow.al:436 `shown after action = 0|`).
    pub gone_blank_shows_line: bool,
    /// The action's refresh just ran the new-row line of a list that lost its only row three times (`settle_after_action`); read
    /// by `post_left_other_key_settled` and cleared when the action ends (al-runner issue 7821).
    pub gone_new_line_settled: bool,
    /// A rule pinned for one scenario ran on this page or one of its parts, and the scenario does nothing after it but the
    /// calls `PinnedEnd` lets through: every other test call is refused (`Session::refuse_after_pinned_end`).
    pub pinned_end: Option<crate::pages::pinned_shape::PinnedEnd>,
    /// `SetSelectionFilter` on the empty modal Worksheet answered once since the page opened
    /// (`empty_modal_worksheet_selection`); its pin makes one call.
    pub empty_selection_given: bool,
    /// The fields of a part's Rec when the test's last SetValue on it began (`control_set_value`).
    pub row_before_write: Option<Vec<Value>>,
    /// The next first read of this part asks OnFindRecord `=>` instead of `=><` (`part_refresh_find_forward.rs`, al-runner
    /// issue 7820); taken by `page_first_row`.
    pub part_refind_forward: bool,
    /// A List with Editable = false opened for view lost its only row to an action and shows no row: a later action is not
    /// pinned and is refused (testpage/RmWorksheetGoneRow.al SH_ListViewNotEditable_, awaiting).
    pub gone_view_list: bool,
    /// The same for a List with Editable = false over a temporary source (`gone_read_only_temp_list`): a later action is built
    /// (al-runner issue 6664).
    pub gone_temp_list: bool,
    /// The part's link value failed its validation as the host opened: Last() answers No and reads nothing, a write raises
    /// TestRowNotFound (pages/parts/QjPartPropagation.al:3766); anything else on the part is refused.
    pub link_validate_failed: bool,
    /// The client's rows were read again from the row a list settled on after an action's filter left its row, with no row before
    /// it: First stays on that row and reads nothing (pages/list-order/AiListOrderTests.al:339, :373 `after_first` empty).
    /// Also set by the refresh of CurrPage.Update on a list with OnFindRecord and OnNextRecord that ends on the first row (:174,
    /// :203). Any TestPage call but First and a field read clears it.
    pub settled_first_row: bool,
    /// The caption was assigned while the page was not open: it stays for every later run of the variable.
    pub caption_assigned_closed: bool,
    /// A row with the new record's starting key was already stored when the draft started.
    pub draft_stored: bool,
    /// A page extension's OnInsertRecord answered false for the new row: the page goes on as if the row were stored, and the
    /// save of its next edit fails as the modify of a row that is not there (testpage/UeTriggerTests.al
    /// NewRow_FirstExtensionAnswersFalse).
    /// The primary key of that row.
    pub insert_vetoed_by_ext: Option<Vec<Value>>,
    /// The new row of a card that its OnNewRecord inserted itself: the page still holds it as a new row
    /// (corpus testpage/CfPageLeads.al:241-289 Card_OnNewRecordInserts_OpenNewSetClose).
    pub new_row_inserted_by_trigger: bool,
    /// TestPage.OpenNew is reading the page's first row (cleared once it is read).
    pub test_opened_new: bool,
    /// The last open was the test's own `TestPage.OpenEdit`, `OpenView` or `OpenNew` (not a run by AL code).
    pub opened_by_test: bool,
    /// What closed the page the last time (`None` before the first close, reset when the page opens). Decides what the
    /// page's code reaches through a part after the close (`pages/held_page_part.rs`).
    pub closed_by: Option<ClosedBy>,
    /// On a new row whose key fields were still blank, a field outside the key was typed.
    pub typed_before_key: bool,
    /// The page reached a state whose behavior is not pinned: every later call on it or its parts except Close is refused
    /// with this reason.
    pub unpinned_state: Option<String>,
    /// `RequestPage.Editable` and `RequestPage.LookupMode` of a request page, as its own code set them (al-vm reports.rs
    /// `request_page_member`).
    pub request_editable: bool,
    pub request_lookup_mode: bool,
    /// With `unpinned_state`: Close is refused too.
    pub unpinned_close: bool,
    /// An action failed on a control add-in call: the client reads the row again (OnAfterGetRecord, OnAfterGetCurrRecord) before
    /// the next call on the page (al-vm pages/actions.rs `read_row_after_add_in_error`).
    pub add_in_reread: bool,
    /// The row a new line was started below (`New()` on a stored row of a list): after the new row
    /// is saved, `Next()` goes to the row below that one, as the client's own row order has it.
    pub after_new: Option<Vec<Value>>,
    /// The client call that runs now (or the last one) saved the current row through `CurrPage.Update(true)` in page code;
    /// cleared at the start of every client call on the page except `New()` (al-vm pages/pageops.rs).
    pub saved_by_update: bool,
    /// `New()` started its line below a row `CurrPage.Update(true)` in page code saved in the call before (`saved_by_update`), the
    /// shape testpage/NewRowThenMove.al:407 TypeOneRowThenNewList pins (al-vm pages/leave_new_line.rs).
    pub after_new_by_update: bool,
    /// OnInit/OnOpenPage are running, and `CurrPage.Editable(false)` has been called in them.
    pub opening: bool,
    pub editable_off: bool,
    /// `editable_off` came from `CurrPage.Editable(false)` in OnInit/OnOpenPage (not from a preset): its effect depends on
    /// the open mode (see `PageOps::page_editable`).
    pub editable_off_in_open: bool,
    /// What `CurrPage.Editable(..)` set in the OnAction trigger that runs now: what page code reads back in the same trigger
    /// (pages/bare-page-methods/TcBarePageObjects.al:309-310, TcBarePageMethodsTests.al:359). A later action reads
    /// the page's own value again (pages/run-modal/P82CurrPage.al:196-219 Editable_ReadInTrigger).
    pub editable_set_later: Option<bool>,
    /// An action's OnAction (`TestAction.Invoke`) is running.
    pub in_action: bool,
    /// Controls given a value by `SetValue` since the page opened (a Decimal FlowFilter control at zero shows '' only
    /// while it is unset).
    pub set_controls: Vec<usize>,
    /// The key of the row the page's OnAfterGetRecord ran with last: what the page's variables hold was computed for that
    /// row (see `Session::control_value`).
    pub agr_row: Option<Vec<Value>>,
    /// The keys of every row OnAfterGetRecord ran with since the page opened.
    pub agr_rows: Vec<Vec<Value>>,
    /// Per row of a repeater (its key): what the page variables its controls show held after the row's OnAfterGetRecord, and
    /// for the current row after OnAfterGetCurrRecord: (control index, value) (pages/row-page-variable/QiRowPageVar.al).
    pub row_vars: Vec<RowVars>,
    /// Code of another page called a procedure of this page through a `Page` value since the page last noted `row_vars`
    /// (`row_var_outside_call.rs`).
    pub row_vars_behind_outside_call: bool,
    /// Per row of a repeater (its key): the page variables the Editable and Enabled expressions of its controls read, as the
    /// row's OnAfterGetRecord left them: (global slot, value) (writer RT's testpage/RtRowEditable.al).
    pub row_prop_vars: Vec<RowVars>,
    /// The row OnAfterGetRecord ran with last (its fields after the trigger): the next OnAfterGetRecord sees it as xRec;
    /// `None` before the first fetch, when xRec is a blank row (testpage/parts/QiHostSaveParts.al:361 `L.AGR(D1/10000|d1 l1 x=/0)`).
    pub agr_xrec: Option<Vec<Value>>,
    /// The row OnAfterGetCurrRecord ran with last: the next OnAfterGetCurrRecord sees it as xRec; `None` before the first,
    /// when xRec is a blank row with the single-value key filters (the part link) on it (QiHostSaveParts.al:361
    /// `L.AGCR(D1/10000|d1 l1 x=D1/0)`, `U.AGCR(D1/10000 x=/0)`).
    pub agcr_xrec: Option<Vec<Value>>,
    /// The row Rec stood on when OnAfterGetCurrRecord last started on a stored row (`marked_view_changed_by_move`).
    pub agcr_started: Option<Vec<Value>>,
    /// OnAfterGetCurrRecord ran last on a row that is not stored (a new row): the next one on a stored row sees the xRec the
    /// page holds (testpage/PnGShowRow.al EmptyListInsertViaRec `List.AGCR(,);List.AGR(NEW1,Created);...;
    /// List.AGCR(NEW1,NEW1)`).
    pub agcr_after_new: bool,
    /// A sibling part saved a row before a host SetValue and this unlinked part over the same table read it and shows it
    /// (testpage/parts/QiHostSaveParts.al:500 `U.AGR(D1/30000 ...);U.AGCR(D1/30000 x=/0)`): the fields of the row it stands on.
    pub shows_sibling_save: Option<Vec<Value>>,
    /// A part: a TestPart call or a call on one of its fields reached it since its host opened.
    pub part_accessed: bool,
    /// A host that is a new, unsaved record while a part linked by a field shows rows of the blank key: only Close, OK, New()
    /// and a read of a host field are pinned for it, and a part field read or First that inserts it (pages/
    /// lt-part-line-before-host-key/PartLineBeforeHostKey.al QH_BlankRows_*).
    pub new_host_part_rows: bool,
    /// New() on the request page of a report landed on a blank row that nothing stores (Y80_RX_New_*).
    pub request_new_blank: bool,
    /// A new host card or document that a first part access stored with its blank key (pages/new-host-key-entry/Y80NewHostKey.al
    /// Y80_NH_*): a key typed afterwards runs its OnValidate, then the page shows and stores the old key again.
    pub host_stored_blank_key: bool,
    /// A new host whose insert, tried by the first Visible() or Editable() of a part, failed and was swallowed (pages/parts/
    /// PartLineBeforeHostKey.al R7031_*): only Close and Visible() or Editable() of its parts are pinned afterwards.
    pub part_insert_swallowed: bool,
    /// A field outside the key was typed on that host row: its parts read their rows again (once is pinned).
    pub blank_key_edit_reread: bool,
    /// `P.Editable(false)` or `P.Editable := false` on a Page variable before it runs: the page
    /// opens non-editable (pages/run-modal/TestPageVariableEditableContracts.al).
    pub editable_preset: Option<bool>,
    /// `CurrPage.Caption := x` (testpage/TestPageCaption_Page.al:44); reset when the page opens.
    pub caption: Option<String>,
    /// The pages shown by the part controls of this page: (control index, child page). Only parts
    /// that are in the control tree at open have one.
    pub parts: Vec<(usize, Rc<RefCell<PageState>>)>,
    /// The shown parts of the last run, kept when the page closed: the page's own code reaches them after the close
    /// (`pages/held_page_part.rs`). Hidden parts stay in `hidden_parts`.
    pub closed_parts: Vec<(usize, Rc<RefCell<PageState>>)>,
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
    /// The row a part read in another order numbered last with AutoSplitKey, and the number of the row the grid shows after it
    /// (`pages/split_key_view.rs`).
    pub split_view_last: Option<(Vec<Value>, Option<Value>)>,
    /// The part reads its row once more when its host closes (`read_part_rows_unsettled`), if the test's calls up to the close
    /// are the pinned ones (`unsettled_close_pinned`).
    pub reread_at_close: bool,
    /// `OnOpenPage` left filters in `FilterGroup(-1)`: the page dropped them from the rows it shows.
    pub or_group_dropped: bool,
    /// The record handed to the page carried filters in `FilterGroup(-1)`, dropped before OnOpenPage.
    pub or_group_handed: bool,
    /// Fields whose page filter reads back as a text nothing pins (a SourceTableView `const(0D)`): a TestPage Filter read refuses.
    pub unpinned_filter_fields: Vec<usize>,
    /// Fields whose SourceTableView filter (one Enum member in `filter()`, an Option `const`) reads back as the member's number
    /// in a TestPage Filter read (`views::page_source_view`).
    pub view_number_fields: Vec<usize>,
    /// The record `Page.SetRecord` handed a page variable, shown when the page runs.
    pub set_record: Option<Rc<RefCell<RecordState>>>,
    /// `Page.SetTableView` gave `set_record` its filters and order: a later `SetRecord` moves the position only
    /// (pages/currpage/PageVarSetRecordTableView.al:P157_List_SetTableViewThenSetRecord_*).
    pub set_view: bool,
    /// The line the page stands on was started by a `New()` call, not reached by moving onto the blank line.
    pub new_by_call: bool,
    /// A `Page`-typed modal handler is running on this page: the page never opened, `GetRecord` and `SetRecord`
    /// read and write its Rec (ui-handlers/PSModalPageHandlerTyped.al).
    pub typed_handler: bool,
    /// The `Page`-typed handler that runs is a `[PageHandler]` of `Page.Run`, not a modal one.
    pub typed_run_handler: bool,
    /// The last `Page`-typed modal handler on this page called `SetRecord`: GetRecord after the run hands that record back.
    pub handler_set_record: bool,
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
    /// and before its OnOpenPage (reports/AqTests.al:362-381).
    pub request_seed: Option<String>,
    /// The session's application areas when the page opened: controls whose area they do not
    /// enable are not on the page.
    pub areas: String,
    /// The permission sets the session held when the page opened (AccessByPermission is decided against them, ADR-062 stage 3).
    pub permission_roles: Vec<String>,
    /// The positions of the FlowFields whose page controls the session's permission sets leave off the page (ADR-062 stage 2).
    pub hidden_flowfields: Vec<usize>,
    /// OnInit has run on the current instance. BC constructs a page, and so runs OnInit, when a
    /// `Page` variable is first used, which may be a procedure call before it runs (corpus
    /// testpage/TestPageOnInit_Tests.al:188 POI_SetterBeforeRunModal_RunsAfterOnInitAndSurvives);
    /// the open that follows must not run it again. The open consumes the flag.
    pub inited: bool,
    /// A page whose rowset OnFindRecord/OnNextRecord serve was refreshed: which row the client
    /// settles on is not modelled, so Rec-bound controls are not read until the cursor moves.
    pub stale: bool,
    /// `Page.RunModal(Id, Rec)` called as a statement: not a lookup (pages/triggers/BeRunModalOnLookup.al:916),
    /// and whether `Rec` takes the row the page closed on is not pinned: a later use of `Rec` refuses when that row differs.
    pub statement_run: bool,
    /// `Page.RunModal(Id, Rec, Field)` with its return value used: a close by the platform after the handler returned hands the
    /// row back as an action does (ui-handlers/F3TrappedAndModal.al:235 Card_HandlerEditsOnly, :341 List_HandlerDoesNothing).
    pub field_run: bool,
    /// The static `Page.RunModal` that runs the page got a record. Every pinned save error at the close of a static RunModal
    /// passes one (pages/close-errors/ModalCloseSaveErrorShapes.al:250, :271, :292, :314, :336, :356, :376;
    /// pages/close-errors/CemModalHandlerTests.al:900, :1743); without one such an error keeps the refusal.
    pub static_run_record: bool,
    /// The refresh about to run follows an action on a card with OnFindRecord, without CurrPage.Update: pinned
    /// (pages/triggers/BeCardOnFindRecord.al), so it marks no assumption.
    pub refresh_pinned: bool,
    /// A SetValue wrote a control bound to an expression on this page with no source table (an action after it is refused).
    pub expr_set_no_source: bool,
    /// The refresh about to run was asked for by `CurrPage.Update` (not by an action that saved an edit).
    pub refresh_by_update: bool,
    /// The last `CurrPage.Update` call was `Update(false)` (it saves nothing).
    pub refresh_update_false: bool,
    /// The filters of the page's record when a control's OnValidate began, for the refresh a `CurrPage.Update` in it asks for
    /// (`al-vm` `refresh_after_validate_filter`).
    pub filters_before_validate: Option<Vec<al_builtins::filter::FieldFilter>>,
    /// Page background tasks the page enqueued and the client has not run yet: (task id, codeunit id,
    /// parameters, the position of the `PageBackgroundTaskErrorLevel` member passed, if any), and the next task id.
    pub bg_tasks: std::collections::VecDeque<(i32, i32, Value, Option<i32>)>,
    pub bg_next_id: i32,
    /// What the page's expression controls showed when it last refreshed: (control index, value; `None` when the expression
    /// raised an error then). Only controls whose expression reads no record and runs no code that writes (al-vm
    /// `pages/shown.rs`, al-runner issue 5765).
    pub shown_exprs: Vec<(usize, Option<Value>)>,
    /// Per row of a list's repeater (its key): what each control of a function or an expression showed when the list read the
    /// row, after its OnAfterGetRecord: (control index, value; `None` when the expression raised an error then) (al-vm
    /// `pages/shown.rs` `note_row_exprs`).
    pub row_exprs: Vec<RowExprs>,
    /// The same for the function controls of a page in a tree with parts: what each showed when its page last ran them for the
    /// row (al-vm `pages/fn_controls.rs`).
    pub fn_rows: Vec<RowExprs>,
    /// The first repeated OnAfterGetRecord of the open read ran no function control (`fn_controls.rs`).
    pub fn_open_skipped: bool,
    /// The next OnAfterGetRecord runs no function control: the first of the two reads a list makes of the row it moves onto.
    pub fn_skip_agr: bool,
    /// The last TestField.Lookup on this page was cancelled and only reads and part moves followed (`note_lookup_cancelled`).
    pub lookup_cancelled: bool,
    /// `row_exprs` holds what the page shows: the list read its rows, and only pinned calls followed.
    pub row_exprs_live: bool,
    /// Since that refresh only events happened that are pinned to leave this page as it is.
    pub shown_current: bool,
    /// The kinds of unpinned events since that refresh (bits of al-vm `pages/shown.rs`).
    pub shown_doubts: u16,
    /// The first call since that refresh after which a read of an expression it does not keep is refused.
    pub shown_doubt_call: Option<&'static str>,
    /// The page was run by code and caught by `TestPage.Trap()`, not opened by the test (OpenView, OpenEdit, OpenNew).
    pub caught_by_trap: bool,
    /// The trap caught the static `Page.Run(Page::X, Rec)` (`PageRunCall::StaticWithRecord`), not a Page variable's `Run()` or
    /// another run. Set and cleared where `caught_by_trap` is (`PageRun::catch_by_trap`, RunObject, the test's own opens).
    pub trapped_static_run: bool,
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
    /// The page is one the test's own `TestPage.OpenEdit` opened, at the top level: not a page AL code ran that the test
    /// trapped or a handler got (`opened_by_test`), not a modal page, not a part. The entry condition of every rule whose pins
    /// all open with OpenEdit; another open keeps the earlier behavior or is refused there.
    pub(crate) fn test_open_edit(&self) -> bool {
        self.opened_by_test && self.mode == PageMode::Edit && !self.modal && self.host.is_none()
    }

    /// A static `Page.RunModal(Id [, Rec [, Field]])` runs the page: `used` says its return value is used, `record` that it
    /// got a record, `field` a field number, `from_table` that the id was 0 (the table's lookup page). Used, the page opens
    /// as a lookup (pages/triggers/BeRunModalOnLookup.al:916 Shape_ReturnNotUsed_WalkOnly, :949 Shape_ReturnInIf_WalkOnly).
    pub(crate) fn static_run_modal(&mut self, used: bool, record: bool, field: bool, from_table: bool) {
        self.run_static = Some(true);
        if used {
            self.open_as_lookup(LookupKind::StaticRunModal);
        }
        self.lookup_page_from_table = from_table;
        self.static_run_record = record;
        self.statement_run = !used && record;
        self.field_run = used && record && field;
    }

    /// The platform runs the page as a lookup of `kind`: the static `Page.RunModal` whose return value is used, or a
    /// TableRelation lookup.
    pub(crate) fn open_as_lookup(&mut self, kind: LookupKind) {
        self.lookup = true;
        self.lookup_kind = kind;
    }

    /// `LookupMode(on)` called from AL, on the page variable or on `CurrPage`. A page the platform opened as a lookup
    /// (`open_as_lookup`) keeps its kind, also when its code switches the mode off.
    pub(crate) fn set_lookup_mode(&mut self, on: bool) {
        self.lookup = on;
        if self.lookup_kind == LookupKind::None {
            self.lookup_kind = LookupKind::Variable;
        }
    }

    /// A page that was never opened. Every field starts at its type's default except these (`mode` and `cursor`
    /// name their starting variant with `#[default]`). A new field needs no line here.
    /// The page opens: what the rules pinned for one scenario noted on its last run is gone (`pinned_shape.rs`).
    pub(crate) fn forget_pinned_scenario(&mut self) {
        self.test_calls = Vec::new();
        self.test_calls_overflow = false;
        self.tree_calls = Vec::new();
        self.tree_calls_unknown = false;
        self.tree_calls_mark = None;
        self.empty_list_stamp = None;
        self.typed_key_dropped = false;
        self.pinned_end = None;
        self.empty_selection_given = false;
        self.temp_ws_edit = None;
        self.temp_ws_preloaded = false;
    }

    /// The page opens: the field captions of its last run are gone.
    pub(crate) fn forget_field_captions(&mut self) {
        self.field_captions = Vec::new();
        self.field_captions_opened = false;
        self.field_captions_settled = false;
        self.field_captions_row = None;
        self.field_captions_quiet = false;
        self.field_captions_pinned_move = false;
    }

    pub fn closed(object: u32) -> PageState {
        PageState {
            object,
            request_editable: true,
            bg_next_id: 1,
            ..Default::default()
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

#[cfg(test)]
mod tests {
    //! Written against the code before ADR-072 step A (the state then lived in `al-builtins/src/value.rs`); the
    //! assertions are unchanged.
    use super::*;
    use al_builtins::value::RtType;

    fn st(v: &Value) -> &Rc<RefCell<PageState>> {
        match v {
            Value::TestPage(handle) => st_of(handle),
            other => panic!("not a TestPage: {other:?}"),
        }
    }

    #[test]
    fn a_declared_page_variable_holds_a_closed_page_of_its_object() {
        register_closed_page();
        for ty in [RtType::TestPage(5), RtType::Page(5)] {
            let v = Value::default_for(&ty);
            let s = st(&v).borrow();
            assert_eq!((s.object, s.open, s.ran, s.trapping), (5, false, false, false));
            assert!(s.host.is_none() && s.instance.is_none());
        }
        // Each variable has a page of its own; an array element too.
        let (a, b) = (
            Value::default_for(&RtType::TestPage(5)),
            Value::default_for(&RtType::TestPage(5)),
        );
        assert!(!Rc::ptr_eq(st(&a), st(&b)));
        let Value::Array(cells) = Value::default_for(&RtType::Array(vec![2], Box::new(RtType::TestPage(7)))) else {
            panic!("not an array");
        };
        let cells = cells.borrow();
        assert_eq!(st(&cells[0]).borrow().object, 7);
        assert!(!Rc::ptr_eq(st(&cells[0]), st(&cells[1])));
    }

    #[test]
    fn a_page_value_prints_the_facts_of_its_page_and_nothing_else() {
        register_closed_page();
        // `Value::identical` compares two handles by this text.
        let v = Value::default_for(&RtType::TestPage(5));
        assert_eq!(
            format!("{v:?}"),
            "TestPage(RefCell { value: PageState(page 5, open false, mode View) })"
        );
        let page = st(&v).clone();
        page.borrow_mut().open = true;
        let c = test_control(page.clone(), ControlTarget::Control(3));
        assert_eq!(
            format!("{c:?}"),
            "TestControl(ControlRef { page: RefCell { value: PageState(page 5, open true, mode View) }, target: Control(3) })"
        );
        assert!(v.identical(&page_value(&page)));
        assert!(!v.identical(&Value::default_for(&RtType::TestPage(5))));
        // A page that is borrowed for writing while it is printed.
        let held = page.borrow_mut();
        assert_eq!(format!("{v:?}"), "TestPage(RefCell { value: <borrowed> })");
        drop(held);
    }

    #[test]
    fn a_page_value_holds_one_reference_to_its_page() {
        register_closed_page();
        // The page client counts the references to a page (`page_held_bindings.rs`, `pages/rows.rs`).
        let v = Value::default_for(&RtType::TestPage(5));
        assert_eq!(Rc::strong_count(st(&v)), 1);
        let page = st(&v).clone();
        assert_eq!(Rc::strong_count(&page), 2);
        let copy = v.clone();
        assert_eq!(Rc::strong_count(&page), 3);
        assert!(Rc::ptr_eq(st(&copy), &page));
        let again = page_value(&page);
        assert_eq!(Rc::strong_count(&page), 4);
        let c = test_control(page.clone(), ControlTarget::Filter);
        assert_eq!(Rc::strong_count(&page), 5);
        let c2 = c.clone();
        // A copy of a control handle shares the handle: no new reference to the page.
        assert_eq!(Rc::strong_count(&page), 5);
        drop((v, copy, again, c));
        assert_eq!(Rc::strong_count(&page), 2);
        drop(c2);
        assert_eq!(Rc::strong_count(&page), 1);
    }

    #[test]
    fn taking_the_page_out_of_a_value_moves_its_reference() {
        let page = Rc::new(RefCell::new(PageState::closed(9)));
        let Value::TestPage(handle) = page_value(&page) else {
            panic!("not a TestPage");
        };
        assert_eq!(Rc::strong_count(&page), 2);
        let taken = into_st(handle);
        assert_eq!(Rc::strong_count(&page), 2);
        assert!(Rc::ptr_eq(&taken, &page));
    }

    #[test]
    fn a_closed_page_starts_from_the_defaults_except_five_values() {
        // The five initial values that are not the type's default.
        let s = PageState::closed(9);
        assert_eq!(s.object, 9);
        assert!(s.request_editable);
        assert_eq!(s.bg_next_id, 1);
        assert_eq!(s.mode, PageMode::View);
        assert_eq!(s.cursor, Cursor::None);
        // Five of the others, one of each kind.
        assert!(!s.open);
        assert!(s.instance.is_none());
        assert!(s.window.is_empty());
        assert_eq!(s.triggers_run, 0);
        assert_eq!(s.areas, "");
    }
}
