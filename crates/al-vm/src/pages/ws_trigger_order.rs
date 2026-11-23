//! The row triggers and the DataCaptionExpression runs of a Worksheet the test opens itself with `TestPage.OpenEdit` or
//! `OpenView` (al-runner issue 8109). Pinned by tests/al-language/pages/properties/WorksheetTriggersOpen.al (codeunit 78924
//! "TRG Tests"): Cap_Three_Edit :292, Cap_Three_View :322, Cap_Three_EditNew :352, Cap_Empty_Edit :438, Cap_Empty_View :468,
//! Cap_Empty_EditNew :498, NoCap_Empty_View :739.
//! - The expression runs right after the OnAfterGetRecord of each row the window reads, once after the two reads of the current
//!   row, and right after OnNewRecord and OnAfterGetCurrRecord (Cap_Three_Edit `AGR A | DC=Cap RowA | AGR B | DC=Cap RowB |
//!   AGR C | DC=Cap RowC | AGR A | AGR A | DC=Cap RowA | AGC A | DC=Cap RowA`, Next `AGR B | AGR B | DC=Cap RowB | AGC B |
//!   DC=Cap RowB`; Cap_Empty_EditNew `NEW | DC=Cap  | AGC  | DC=Cap `). A read of the caption runs nothing and shows what the
//!   last run returned.
//! - OpenView of an empty view runs OnNewRecord and OnAfterGetCurrRecord twice, as OpenEdit does (Cap_Empty_View,
//!   NoCap_Empty_View `INIT | OPEN | NEW | AGC  | NEW | AGC `).
//!
//! Only the probe's two pages take the cells (`ws_open_cell`), and after the open only the probe's calls (`STEPS`). A page of the
//! same shape outside the cells refuses at the open where main's order is known to differ (`ws_open_neighbor`); every other page
//! keeps main's path.
//!
//! The row triggers and the DataCaptionExpression runs of the probe's Worksheet the test opens with
//! OpenEdit or OpenView (al-runner issue 8109, pages/properties/WorksheetTriggersOpen.al): the
//! expression after the window's row reads, after the current-row reads, OnNewRecord and
//! OnAfterGetCurrRecord; OpenView of an empty view lands on the new line twice; the caption shows the
//! last value; only the probe's calls follow (`STEPS`). The same shape outside the cells refuses at
//! the open where main's order differs (`ws_open_neighbor`). Entry points: `offers_new_line_at_open`
//! (by `open_page_as_inner`), `ws_caption_before_trigger` and `ws_caption_after_trigger` (by
//! `page_trigger`), `populate_window_row` (the window loop of `rows.rs`), `ws_trigger_caption` (by
//! `caption_expr_list.rs`), `refuse_ws_trigger_order_step` (by `dirty_step.rs`).

use super::pinned_body::{Callees, Names, callees_are, page_is, routine_text, table_is};
use super::pinned_shape::TestCall;
use super::*;
use al_builtins::page_op::PageMethod;

/// What a page in a measured cell carries from its open (`PageState::ws_trigger_order`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WsTriggerOrder {
    /// The page runs its DataCaptionExpression after its row triggers (TRG WS); `false` for TRG WS NoCap.
    caption: bool,
    /// The test opened the page with OpenView over an empty view: it runs OnNewRecord and OnAfterGetCurrRecord twice.
    view_empty: bool,
    /// The view held the probe's three rows at the open (else none).
    three_rows: bool,
    /// The test opened the page with OpenEdit.
    edit: bool,
    /// An OnAfterGetRecord or OnNewRecord ran since the expression last did: it runs before the next OnAfterGetCurrRecord.
    due: bool,
    /// What the expression returned at its last run.
    value: Option<String>,
}

/// The probe's page with the expression (WorksheetTriggersOpen.al:32-100, TRG WS), line for line.
const CAP_PAGE: &[&str] = &[
    "PageType = Worksheet",
    "SourceTable = \"TRG Row\"",
    "Caption = 'TRG WS'",
    "DataCaptionExpression = Cap()",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field CodeCtl in Lines = Code {applicationarea=All}",
    "field NameCtl in Lines = Name {applicationarea=All}",
    "trigger oninit: begin Log.Add('INIT'); end",
    "trigger onopenpage: begin Log.Add('OPEN'); end",
    "trigger onaftergetrecord: begin Log.Add(('AGR ' + Rec.\"Code\")); end",
    "trigger onaftergetcurrrecord: begin Log.Add(('AGC ' + Rec.\"Code\")); end",
    "trigger onnewrecord: begin Log.Add('NEW'); end",
    "trigger onqueryclosepage: begin Log.Add(('QCP ' + Format(CloseAction))); exit(true); exit(true); end",
    "trigger onclosepage: begin Log.Add('CLOSE'); end",
];

/// The probe's page without it (WorksheetTriggersOpen.al:102-160, TRG WS NoCap), line for line.
const NOCAP_PAGE: &[&str] = &[
    "PageType = Worksheet",
    "SourceTable = \"TRG Row\"",
    "Caption = 'TRG WS NoCap'",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field CodeCtl in Lines = Code {applicationarea=All}",
    "field NameCtl in Lines = Name {applicationarea=All}",
    "trigger oninit: begin Log.Add('INIT'); end",
    "trigger onopenpage: begin Log.Add('OPEN'); end",
    "trigger onaftergetrecord: begin Log.Add(('AGR ' + Rec.\"Code\")); end",
    "trigger onaftergetcurrrecord: begin Log.Add(('AGC ' + Rec.\"Code\")); end",
    "trigger onnewrecord: begin Log.Add('NEW'); end",
    "trigger onqueryclosepage: begin Log.Add(('QCP ' + Format(CloseAction))); exit(true); exit(true); end",
    "trigger onclosepage: begin Log.Add('CLOSE'); end",
];

/// The routine the compiler makes of `DataCaptionExpression = Cap()` (WorksheetTriggersOpen.al:37) and the procedure it calls
/// (:56-63).
const DATA_CAPTION_TEXT: &str = "begin exit(Cap()); end";
const CAP_TEXT: &str = "var V: Text; begin V := ('Cap ' + Rec.\"Name\"); Log.Add(('DC=' + V)); exit(V); end";

/// The probe's table (WorksheetTriggersOpen.al:16-30, TRG Row).
const ROW_TABLE: &[&str] = &[
    "DataClassification = CustomerContent",
    "field 1 \"Code\": Code[20]",
    "field 2 \"Name\": Text[50]",
    "key \"Code\" {Clustered=true}",
];

/// The log the pages call (WorksheetTriggersOpen.al:162-191, TRG Log).
const LOG_ADD: &[&str] = &["\"TRG Log\".Add: begin Lines.Add(Line); end"];

/// The page triggers the probe's pages declare (WorksheetTriggersOpen.al:65-99).
const PROBE_TRIGGERS: [&str; 7] = [
    "oninit",
    "onopenpage",
    "onaftergetrecord",
    "onaftergetcurrrecord",
    "onnewrecord",
    "onqueryclosepage",
    "onclosepage",
];

/// The calls the probe's tests make after the open, with whether each answered No: the reads of WorksheetTriggersOpen.al:302-312
/// on three rows (`First` and `Next` move), :478-488 on an empty view (neither moves), and OpenEdit then `New` (:361-363).
const STEPS: [(&[(TestCall, bool)], Rows); 3] = [
    (
        &[
            (TestCall::Page(PageMethod::Caption), false),
            (TestCall::Read, false),
            (TestCall::Page(PageMethod::First), false),
            (TestCall::Read, false),
            (TestCall::Page(PageMethod::Next), false),
            (TestCall::Read, false),
            (TestCall::Page(PageMethod::Close), false),
        ],
        Rows::Three,
    ),
    (
        &[
            (TestCall::Page(PageMethod::Caption), false),
            (TestCall::Read, false),
            (TestCall::Page(PageMethod::First), true),
            (TestCall::Page(PageMethod::Next), true),
            (TestCall::Page(PageMethod::Close), false),
        ],
        Rows::Empty,
    ),
    (
        &[
            (TestCall::Page(PageMethod::New), false),
            (TestCall::Read, false),
            (TestCall::Page(PageMethod::Close), false),
        ],
        Rows::EditAny,
    ),
];

/// Which open a list of `STEPS` follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rows {
    Three,
    Empty,
    /// OpenEdit with either row count (Cap_Three_EditNew, Cap_Empty_EditNew).
    EditAny,
}

const STEP_REFUSAL: &str = "a call of the test on a Worksheet that ran its row triggers and DataCaptionExpression in the order \
                            WorksheetTriggersOpen.al measured, other than the probe's calls (not pinned; \
                            al-runner issue 8109)";

impl Session {
    /// Whether page `st` offers the new-row line at the open (`offers_new_line`), and its measured cell noted on it
    /// (`ws_open_cell`): OpenView of the probe's empty view lands on the line twice, as OpenEdit does.
    pub(super) fn offers_new_line_at_open(&mut self, st: &St) -> Result<bool, Signal> {
        let offered = self.offers_new_line(st)?;
        let cell = self.ws_open_cell(st)?;
        let view_empty = cell.as_ref().is_some_and(|c| c.view_empty);
        st.borrow_mut().ws_trigger_order = cell;
        Ok(offered || view_empty)
    }

    /// The measured cell page `st` opens in, after its OnOpenPage, every fact required as WorksheetTriggersOpen.al has it:
    /// - the open: the test's own OpenEdit (:300, :446, :359, :505) or OpenView (:330, :476, :747) of a top-level page;
    /// - the page, its table and the log it calls: the probe's, line for line (TRG WS :32-100 or TRG WS NoCap :102-160, TRG Row
    ///   :16-30, TRG Log :174-177), nothing subscribes to their events;
    /// - Rec: stored, no filter, the primary key ascending, no marks (the test hands nothing);
    /// - the table holds the three rows A, B, C or none (`Seed`, :201-215).
    ///
    /// TRG WS takes the cell in each open (Cap_Three_Edit, Cap_Three_View, Cap_Empty_Edit, Cap_Empty_View); TRG WS NoCap only at
    /// OpenView of the empty view (NoCap_Empty_View): with rows or opened by OpenEdit main's path gives BC's order (NoCap_Three_Edit,
    /// NoCap_Three_View, NoCap_Empty_Edit). A page outside the cells refuses where `ws_open_neighbor` says so.
    fn ws_open_cell(&mut self, st: &St) -> Result<Option<WsTriggerOrder>, Signal> {
        let (edit, view) = {
            let s = st.borrow();
            (s.test_open_edit(), s.test_open_view())
        };
        if !(edit || view) || !self.is_worksheet_list(st) {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        let plain_rec = {
            let r = rec.borrow();
            !r.temporary
                && r.filters.is_empty()
                && r.key == 0
                && r.ascending
                && r.field_descending == 0
                && r.adhoc.is_none()
                && r.marks.is_empty()
                && !r.marked_only
                && r.filter_group == 0
        };
        let caption = self.has_own_data_caption_expression(st);
        let probe_page = {
            let engine = self.engine();
            let object = st.borrow().object as usize;
            let table = rec.borrow().table as usize;
            let mut callees = Callees::new();
            let mut names = Names::default();
            let pin = if caption { CAP_PAGE } else { NOCAP_PAGE };
            let expression = !caption || probe_expression(&engine.program, object, &mut callees);
            page_is(&engine, object, pin, &mut callees, &mut names)
                && expression
                && table_is(&engine, table, ROW_TABLE, &mut callees, &mut names)
                && callees_are(&engine, &callees, LOG_ADD, &mut names)
        };
        if !probe_page && !self.neighbor_shape(st) {
            return Ok(None);
        }
        // The rows of Rec's own view (its filters in every group), as OnOpenPage left them.
        let own_filters = rec.borrow().filters.clone();
        let rows = self.stored_rows_passing(&rec, own_filters)?;
        let three = rows == 3 && plain_rec && self.probe_rows_are_three(&rec)?;
        let view_empty = view && rows == 0;
        if probe_page && plain_rec && (three || rows == 0) && (caption || view_empty) {
            return Ok(Some(WsTriggerOrder {
                caption,
                view_empty,
                three_rows: three,
                edit,
                due: false,
                value: None,
            }));
        }
        self.ws_open_neighbor(st, caption, view_empty)?;
        Ok(None)
    }

    /// Page `st` is a Worksheet with a repeater, not a tree.
    fn is_worksheet_list(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        self.engine.object_facts[object].page_type == Some(al_ir::PageType::Worksheet)
            && self.is_list(st)
            && self.tree_indentation(st).is_none()
    }

    /// The rows of the table of `rec` are the probe's, each by both fields: A/RowA, B/RowB, C/RowC (`Seed`, :208-214).
    fn probe_rows_are_three(&mut self, rec: &Rec) -> Result<bool, Signal> {
        let all = Rc::new(RefCell::new(rec.borrow().clone()));
        {
            let mut a = all.borrow_mut();
            a.filters.clear();
            a.marks.clear();
            a.marked_only = false;
        }
        let rows = self.view_rows(&all, 4)?;
        let want = [("A", "RowA"), ("B", "RowB"), ("C", "RowC")];
        Ok(rows.len() == want.len()
            && rows.iter().zip(want).all(|(r, (code, name))| {
                matches!(r.as_slice(), [Value::Code(c), Value::Text(n), ..] if c.as_str() == code && n.as_str() == name)
            }))
    }

    /// Page `st` has the shape `ws_open_neighbor` describes.
    fn neighbor_shape(&self, st: &St) -> bool {
        self.has_source(st)
            && !self.page_rec(st).borrow().temporary
            && self.page_declares_within(st, &PROBE_TRIGGERS, &["datacaptionexpression"])
            && self.controls_are_rec_fields(st, false)
            && self.table_code_is(self.page_rec(st).borrow().table as usize, &[])
    }

    /// A page outside the cells with the probe pages' shape, where main's order is known to differ from BC's: a top-level
    /// Worksheet the test opened with OpenEdit or OpenView, over a stored table without code, with no trigger but the probe's
    /// (`PROBE_TRIGGERS`), no property but the plain ones and DataCaptionExpression, no extension or subscriber, and controls
    /// bound to fields of Rec without triggers. It refuses when its expression can raise an error or leave something behind
    /// (`Engine::caption_open_risk`: BC runs it after every row trigger, main only at a caption read), or when OpenView of an empty
    /// view (no row passes Rec's own filters, also after OnOpenPage) meets OnNewRecord or OnAfterGetCurrRecord (BC runs both twice, main neither). Every other page keeps main's path.
    fn ws_open_neighbor(&self, st: &St, caption: bool, view_empty: bool) -> Result<(), Signal> {
        if !self.neighbor_shape(st) {
            return Ok(());
        }
        if caption && let Some(risk) = self.data_caption_expression_risk(st) {
            return unsupported(format!(
                "a Worksheet the test opened, with a DataCaptionExpression that can raise an error or leave something behind \
                 ({risk}), outside the pages and rows WorksheetTriggersOpen.al of corpus PR 1401 measured: BC runs it after each \
                 row trigger, main only at a caption read (al-runner issue 8109)"
            ));
        }
        if view_empty && (self.declares(st, "onnewrecord") || self.declares(st, "onaftergetcurrrecord")) {
            return unsupported(
                "OnNewRecord or OnAfterGetCurrRecord of a Worksheet the test opened with OpenView over an empty view, outside the \
                 pages WorksheetTriggersOpen.al of corpus PR 1401 measured: BC runs both twice at the open, main neither \
                 (al-runner issue 8109)",
            );
        }
        Ok(())
    }

    /// Before page trigger `name` of `st`: in a measured cell a due expression runs before OnAfterGetCurrRecord.
    pub(super) fn ws_caption_before_trigger(&mut self, st: &St, name: &str) -> Result<(), Signal> {
        let due = st
            .borrow()
            .ws_trigger_order
            .as_ref()
            .is_some_and(|o| o.caption && o.due);
        if due && name == "onaftergetcurrrecord" {
            self.run_ws_caption(st)?;
        }
        Ok(())
    }

    /// After page trigger `name` of `st` ran: in a measured cell OnAfterGetRecord and OnNewRecord leave the expression due, and
    /// it runs right after OnAfterGetCurrRecord.
    pub(super) fn ws_caption_after_trigger(&mut self, st: &St, name: &str) -> Result<(), Signal> {
        {
            let mut s = st.borrow_mut();
            let Some(order) = s.ws_trigger_order.as_mut().filter(|o| o.caption) else {
                return Ok(());
            };
            if matches!(name, "onaftergetrecord" | "onnewrecord") {
                order.due = true;
                return Ok(());
            }
        }
        if name == "onaftergetcurrrecord" {
            self.run_ws_caption(st)?;
        }
        Ok(())
    }

    /// Reads the row Rec holds into the window (`populate_current`); in a measured cell the expression runs right after the
    /// row's OnAfterGetRecord.
    pub(super) fn populate_window_row(&mut self, st: &St) -> Result<(), Signal> {
        self.populate_current(st)?;
        let due = st
            .borrow()
            .ws_trigger_order
            .as_ref()
            .is_some_and(|o| o.caption && o.due);
        if due {
            self.run_ws_caption(st)?;
        }
        Ok(())
    }

    /// Runs the expression of a page in a measured cell and keeps what it returned.
    fn run_ws_caption(&mut self, st: &St) -> Result<(), Signal> {
        if let Some(o) = st.borrow_mut().ws_trigger_order.as_mut() {
            o.due = false;
        }
        let value = self.data_caption_expression_value(st)?;
        if let Some(o) = st.borrow_mut().ws_trigger_order.as_mut() {
            o.value = Some(value);
        }
        Ok(())
    }

    /// The caption of a page in a measured cell: the page caption, ` - ` and what the expression returned last, without its
    /// one trailing blank (:315 `cap=TRG WS - Cap RowA`, :491 `cap=TRG WS - Cap` from `Cap `). `None` outside the cells. Only
    /// `Cap RowA`, `Cap RowB`, `Cap RowC` (the probe's rows) and `Cap ` (the new-row line) are measured.
    pub(super) fn ws_trigger_caption(&self, st: &St, base: &str) -> Option<Result<String, Signal>> {
        let s = st.borrow();
        let order = s.ws_trigger_order.as_ref().filter(|o| o.caption)?;
        let value = order.value.as_deref().unwrap_or("");
        Some(if matches!(value, "Cap RowA" | "Cap RowB" | "Cap RowC" | "Cap ") {
            Ok(format!("{base} - {}", value.trim_end_matches(' ')))
        } else {
            unsupported(
                "the caption of a Worksheet in a cell of WorksheetTriggersOpen.al whose DataCaptionExpression returned a value \
                 other than the probe's rows or the new-row line (not pinned)",
            )
        })
    }

    /// A test call `call` on page `st`: after an open in a measured cell only the probe's calls follow (`STEPS`), in its order.
    pub(super) fn refuse_ws_trigger_order_step(st: &St, call: Option<TestCall>) -> Result<(), Signal> {
        let s = st.borrow();
        let Some(order) = s.ws_trigger_order.as_ref() else {
            return Ok(());
        };
        if call.is_none() {
            return Ok(());
        }
        let open = if order.three_rows { Rows::Three } else { Rows::Empty };
        let (earlier, running) = match s.tree_calls.split_last() {
            Some((last, earlier)) if !s.tree_calls_unknown && s.tree_calls.iter().all(|c| c.part.is_none()) => {
                (earlier, last.call)
            }
            _ => return unsupported(STEP_REFUSAL),
        };
        let measured = STEPS.iter().any(|(steps, rows)| {
            let fits = *rows == open || (*rows == Rows::EditAny && order.edit && order.caption);
            fits && steps.len() > earlier.len()
                && earlier
                    .iter()
                    .zip(steps.iter())
                    .all(|(c, (want, no))| c.call == *want && c.no == *no && !c.failed)
                && steps[earlier.len()].0 == running
        });
        if measured { Ok(()) } else { unsupported(STEP_REFUSAL) }
    }
}

/// Whether the DataCaptionExpression of page `object` is the probe's: the compiled expression calls the page's procedure `Cap`
/// alone, and that procedure reads as WorksheetTriggersOpen.al:56-63 (its calls go to `callees`).
fn probe_expression(program: &ir::Program, object: usize, callees: &mut Callees) -> bool {
    let routines = &program.objects[object].routines;
    let Some(expression) = routines.iter().position(|r| r.name == ir::DATA_CAPTION_ROUTINE) else {
        return false;
    };
    let mut called = Callees::new();
    if routine_text(program, object, expression, &mut called).as_deref() != Some(DATA_CAPTION_TEXT) {
        return false;
    }
    let [(o, cap)] = called.as_slice() else {
        return false;
    };
    *o == object
        && routines[*cap].name == "Cap"
        && routine_text(program, object, *cap, callees).as_deref() == Some(CAP_TEXT)
}
