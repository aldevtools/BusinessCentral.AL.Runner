//! The objects of the cells that measured a part's calls after the test's own filter read its refilled rows (al-runner issue
//! 8537, `part_update_left_view.rs`), line for line as `pinned_body.rs` renders them: the probe's (pages/parts/
//! PartRefillTestFilter.al, corpus PRs 1474 and 1475, merged, not pinned yet, and the styled part of corpus PR 1534, open) and
//! the reporters' repros in issues 8537 and 8704. Name slots: `«T»` the host's table, `«P»` the part page, `«L»` the part's table, `«C»` the
//! codeunit that calculates the rows, `«B»` the table it reads, `«K»` the id of the background task's codeunit.
//!
//! A cell is one host with one part, as measured: another combination of a host and a part from different cells is no cell.

use super::pinned_body::{Callees, Names, callees_are, page_is, routine_text, table_is};
use super::*;
use al_builtins::page_op::PageMethod;

/// One measured combination of objects, the position of the part's key field the test's filter is on, and the moves and reads
/// its tests made after that filter (`part_update_left_view.rs` `measured_step_after_test_filter`).
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Cell {
    host: &'static [&'static str],
    host_table: &'static [&'static str],
    part: &'static [&'static str],
    part_table: &'static [&'static str],
    booking_table: &'static [&'static str],
    callees: &'static [&'static str],
    /// The `OnRun` of the background task's codeunit, for the host that fills the part from a page background task.
    task: Option<&'static str>,
    pub(super) filter_field: usize,
    /// `Last()` right after the filter.
    pub(super) last: bool,
    /// `Next()` after `First()`, then `Previous()`.
    pub(super) next_previous: bool,
    /// The controls whose `Value` the tests read after `First()`.
    pub(super) value_reads: &'static [&'static str],
    /// The AsDate and AsDecimal reads the tests made after each move: (control, method).
    pub(super) fact_reads: &'static [(&'static str, PageMethod)],
    /// Those reads right after the filter, before any move.
    pub(super) read_without_move: bool,
}

/// The reads of every probe cell after each move (the `Fact` of PartRefillTestFilter.al).
const PROBE_READS: &[(&str, PageMethod)] = &[
    ("Period Start", PageMethod::AsDate),
    ("Allocated Qty.", PageMethod::AsDecimal),
];

/// The host's code that fills the part in its own OnAfterGetCurrRecord (:152 "LT PR Load Host", :200, :403, PR 1475's
/// "LT PR Typed Trig Host", and the repro's host).
const FILL_IN_TRIGGER: &str = "trigger onaftergetcurrrecord: var LoadCalc: Codeunit \"«C»\"; begin if \
     CurrPage.Load.IsLoadedFor(Rec.\"Resource No.\") then begin exit; end; CurrPage.Load.ClearLines(Rec.\"Resource No.\"); \
     CurrPage.Load.SetLoadLines(LoadCalc.Calculate(Rec.\"Resource No.\")); end";
const REFRESH_ACTION: &str = "  trigger onaction: begin CurrPage.Load.Invalidate(); CurrPage.Update(false); end";

/// :152 "LT PR Load Host", :403 "LT PR Load Trig Host" and PR 1475's "LT PR Typed Trig Host".
const WORKSHEET_HOST: &[&str] = &[
    "PageType = Worksheet",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Ops in - {}",
    "field No. in Ops = No. {applicationarea=All}",
    "field Resource No. in Ops = Resource No. {applicationarea=All}",
    "part Load in - page «P» {applicationarea=All}",
    "action Refresh in - {applicationarea=All}",
    REFRESH_ACTION,
    "area Processing: Refresh",
    FILL_IN_TRIGGER,
];

/// :200 "LT PR Load List Host".
const LIST_HOST: &[&str] = &[
    "PageType = List",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Ops in - {}",
    "field No. in Ops = No. {applicationarea=All}",
    "field Resource No. in Ops = Resource No. {applicationarea=All}",
    "part Load in - page «P» {applicationarea=All}",
    "action Refresh in - {applicationarea=All}",
    REFRESH_ACTION,
    "area Processing: Refresh",
    FILL_IN_TRIGGER,
];

/// :248 "LT PR Load Bg Host": the part is filled from a page background task.
const BACKGROUND_HOST: &[&str] = &[
    "PageType = Worksheet",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Ops in - {}",
    "field No. in Ops = No. {applicationarea=All}",
    "field Resource No. in Ops = Resource No. {applicationarea=All}",
    "part Load in - page «P» {applicationarea=All}",
    "action Refresh in - {applicationarea=All}",
    REFRESH_ACTION,
    "area Processing: Refresh",
    "trigger onaftergetcurrrecord: var Params: Dictionary of [Text, Text]; begin if \
     CurrPage.Load.IsLoadedFor(Rec.\"Resource No.\") then begin exit; end; if (LoadTaskId <> 0) then begin \
     CurrPage.CancelBackgroundTask(LoadTaskId); end; LoadTaskId := 0; CurrPage.Load.ClearLines(Rec.\"Resource No.\"); \
     Params.Add('Resource', Rec.\"Resource No.\"); CurrPage.EnqueueBackgroundTask(LoadTaskId, «K», Params, 120000, \
     option#Warning); end",
    "trigger onpagebackgroundtaskcompleted: begin if (TaskId <> LoadTaskId) then begin exit; end; LoadTaskId := 0; \
     CurrPage.Load.SetLoadLines(Results); end",
];

/// :311 "LT PR Load Task".
const BACKGROUND_TASK: &str = "var LoadCalc: Codeunit \"«C»\"; Params: Dictionary of [Text, Text]; begin Params := \
     GetBackgroundParameters(); SetBackgroundTaskResult(LoadCalc.Calculate(CopyStr(Params.Get('Resource'), 1, 20))); end";

/// The repro's host `Repro Load Host` (issue 8537): no ApplicationArea on its controls.
const REPRO_HOST: &[&str] = &[
    "PageType = Worksheet",
    "SourceTable = \"«T»\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Ops in - {}",
    "field No. in Ops = No. {}",
    "field Resource No. in Ops = Resource No. {}",
    "part Load in - page «P» {}",
    "action Refresh in - {}",
    REFRESH_ACTION,
    "area Processing: Refresh",
    FILL_IN_TRIGGER,
];

/// :90 "LT PR Load Part": no trigger.
const PLAIN_PART: &[&str] = &[
    "PageType = ListPart",
    "SourceTable = \"«L»\"",
    "SourceTableTemporary = true",
    "InsertAllowed = false",
    "DeleteAllowed = false",
    "Editable = false",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Period Start in Lines = Period Start {applicationarea=All}",
    "field Allocated Qty. in Lines = Allocated Qty. {applicationarea=All}",
];

/// Corpus PR 1534's "LT PR Styled Part" (al-runner issue 8704): the plain part with StyleExpr on Allocated Qty. set in its
/// OnAfterGetRecord.
const STYLED_PART: &[&str] = &[
    "PageType = ListPart",
    "SourceTable = \"«L»\"",
    "SourceTableTemporary = true",
    "InsertAllowed = false",
    "DeleteAllowed = false",
    "Editable = false",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Period Start in Lines = Period Start {applicationarea=All}",
    "field Allocated Qty. in Lines = Allocated Qty. {applicationarea=All; styleexpr=LoadStyle}",
    "trigger onaftergetrecord: begin LoadStyle := 'Standard'; if (Rec.\"Allocated Qty.\" > 20) then begin LoadStyle := \
     'Unfavorable'; end; end",
];

/// :323 "LT PR Load Trig Part" and PR 1475's "LT PR Typed Trig Part": OnOpenPage and OnAfterGetRecord.
const TRIGGER_PART: &[&str] = &[
    "PageType = ListPart",
    "SourceTable = \"«L»\"",
    "SourceTableTemporary = true",
    "InsertAllowed = false",
    "DeleteAllowed = false",
    "Editable = false",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Period Start in Lines = Period Start {applicationarea=All}",
    "field Allocated Qty. in Lines = Allocated Qty. {applicationarea=All; styleexpr=QtyStyle}",
    "field QtyText in Lines = var QtyText: Text {applicationarea=All; styleexpr=QtyStyle}",
    "field Scope in Lines = var ScopeText: Text {applicationarea=All}",
    "trigger onopenpage: begin ScopeText := 'open'; end",
    "trigger onaftergetrecord: begin QtyText := ('agr' + Format(Rec.\"Allocated Qty.\", 0, 9)); QtyStyle := 'Standard'; \
     if (Rec.\"Allocated Qty.\" > 20) then begin QtyStyle := 'Unfavorable'; end; end",
];

/// The repro's part `Repro Load Part`.
const REPRO_PART: &[&str] = &[
    "PageType = ListPart",
    "SourceTable = \"«L»\"",
    "SourceTableTemporary = true",
    "InsertAllowed = false",
    "DeleteAllowed = false",
    "Editable = false",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Period Start in Lines = Period Start {}",
    "field Allocated Qty. in Lines = Allocated Qty. {}",
];

/// The part of the repro of al-runner issue 8704: the repro's part of issue 8537 with StyleExpr on Allocated Qty. set in its
/// OnAfterGetRecord.
const REPRO_STYLED_PART: &[&str] = &[
    "PageType = ListPart",
    "SourceTable = \"«L»\"",
    "SourceTableTemporary = true",
    "InsertAllowed = false",
    "DeleteAllowed = false",
    "Editable = false",
    "ApplicationArea = All",
    "UsageCategory = None",
    "repeater Lines in - {}",
    "field Period Start in Lines = Period Start {}",
    "field Allocated Qty. in Lines = Allocated Qty. {styleexpr=LoadStyle}",
    "trigger onaftergetrecord: begin LoadStyle := 'Standard'; if (Rec.\"Allocated Qty.\" > 20) then begin LoadStyle := \
     'Unfavorable'; end; end",
];

/// :18 "LT PR Op", :34 "LT PR Booking", :52 "LT PR Load Line", PR 1475's "LT PR Load Line Typed".
const OP_TABLE: &[&str] = &[
    "DataClassification = SystemMetadata",
    "field 1 \"No.\": Code[20]",
    "field 2 \"Resource No.\": Code[20]",
    "key \"No.\" {Clustered=true}",
];
const BOOKING_TABLE: &[&str] = &[
    "DataClassification = SystemMetadata",
    "field 1 \"Entry No.\": Integer",
    "field 2 \"Resource No.\": Code[20]",
    "field 3 \"Day\": Date",
    "field 4 \"Qty\": Decimal",
    "key \"Entry No.\" {Clustered=true}",
];
const LINE_TABLE: &[&str] = &[
    "DataClassification = SystemMetadata",
    "TableType = Temporary",
    "field 1 \"Period Start\": Date",
    "field 2 \"Allocated Qty.\": Decimal",
    "key \"Period Start\" {Clustered=true}",
];
const TYPED_LINE_TABLE: &[&str] = &[
    "DataClassification = SystemMetadata",
    "TableType = Temporary",
    "field 1 \"Period Type\": Option Day,Week",
    "field 2 \"Period Start\": Date",
    "field 3 \"Allocated Qty.\": Decimal",
    "key \"Period Type\", \"Period Start\" {Clustered=true}",
];

/// The repro's tables `Repro Op`, `Repro Booking` and `Repro Load Line`.
const REPRO_OP_TABLE: &[&str] = &[
    "DataClassification = CustomerContent",
    "field 1 \"No.\": Code[20]",
    "field 2 \"Resource No.\": Code[20]",
    "key \"No.\" {Clustered=true}",
];
const REPRO_BOOKING_TABLE: &[&str] = &[
    "DataClassification = CustomerContent",
    "field 1 \"Entry No.\": Integer",
    "field 2 \"Resource No.\": Code[20]",
    "field 3 \"Day\": Date",
    "field 4 \"Qty\": Decimal",
    "key \"Entry No.\" {Clustered=true}",
];
const REPRO_LINE_TABLE: &[&str] = &[
    "DataClassification = CustomerContent",
    "TableType = Temporary",
    "field 1 \"Period Start\": Date",
    "field 2 \"Allocated Qty.\": Decimal",
    "key \"Period Start\" {Clustered=true}",
];

/// The procedures the host's code calls: the part's four and the calculation (:69 "LT PR Load Calc", the repro's
/// `Repro Load Calc`).
const CALCULATE: &str = "\"«C»\".Calculate: var Booking: Record \"«B»\"; DayKey: Text; Qty: Decimal; begin \
     Booking.SetRange(field#1, ResourceNo); if Booking.FindSet() then begin repeat DayKey := Format(Booking.\"Day\", 0, 9); \
     Qty := 0; if Results.ContainsKey(DayKey) then begin Evaluate(Qty, Results.Get(DayKey), 9); end; Qty := (Qty + \
     Booking.\"Qty\"); Results.Set(DayKey, Format(Qty, 0, 9)); until (Booking.Next() = 0); end; end";
const IS_LOADED_FOR: &str = "\"«P»\".IsLoadedFor: begin exit((LinesLoaded and (NewResourceNo = ResourceNo))); end";
const INVALIDATE: &str = "\"«P»\".Invalidate: begin LinesLoaded := false; end";
const CLEAR_LINES: &str = "\"«P»\".ClearLines: begin ResourceNo := NewResourceNo; LinesLoaded := false; Rec.Reset(); \
     Rec.DeleteAll(false); CurrPage.Update(false); end";
const PART_CALLEES: &[&str] = &[
    CALCULATE,
    IS_LOADED_FOR,
    INVALIDATE,
    CLEAR_LINES,
    "\"«P»\".SetLoadLines: var DayKey: Text; begin foreach DayKey in Results.Keys() do begin Rec.Init(); \
     Evaluate(Rec.\"Period Start\", DayKey, 9); Evaluate(Rec.\"Allocated Qty.\", Results.Get(DayKey), 9); Rec.Insert(); \
     end; LinesLoaded := true; if Rec.FindFirst() then begin end; CurrPage.Update(false); end",
];
/// PR 1475's "LT PR Typed Trig Part": SetLoadLines also sets `Period Type`.
const TYPED_PART_CALLEES: &[&str] = &[
    CALCULATE,
    IS_LOADED_FOR,
    INVALIDATE,
    CLEAR_LINES,
    "\"«P»\".SetLoadLines: var DayKey: Text; begin foreach DayKey in Results.Keys() do begin Rec.Init(); \
     Rec.\"Period Type\" := option#Day; Evaluate(Rec.\"Period Start\", DayKey, 9); Evaluate(Rec.\"Allocated Qty.\", \
     Results.Get(DayKey), 9); Rec.Insert(); end; LinesLoaded := true; if Rec.FindFirst() then begin end; \
     CurrPage.Update(false); end",
];

/// The measured cells, with the corpus lines of the tests that ran them.
const CELLS: &[Cell] = &[
    // :512, :537 (a read right after the filter), :561 (First, Next, Previous), :590 (Last) (and :615, :643, which never
    // reach the steps).
    Cell {
        host: WORKSHEET_HOST,
        host_table: OP_TABLE,
        part: PLAIN_PART,
        part_table: LINE_TABLE,
        booking_table: BOOKING_TABLE,
        callees: PART_CALLEES,
        task: None,
        filter_field: 0,
        last: true,
        next_previous: true,
        value_reads: &[],
        fact_reads: PROBE_READS,
        read_without_move: true,
    },
    // :671 PartRefill_ListActionThenFilterFirst_FindsRefilledRow.
    Cell {
        host: LIST_HOST,
        host_table: OP_TABLE,
        part: PLAIN_PART,
        part_table: LINE_TABLE,
        booking_table: BOOKING_TABLE,
        callees: PART_CALLEES,
        task: None,
        filter_field: 0,
        last: false,
        next_previous: false,
        value_reads: &[],
        fact_reads: PROBE_READS,
        read_without_move: false,
    },
    // :696 PartRefill_BackgroundTaskActionThenFilterFirst_FindsRefilledRow.
    Cell {
        host: BACKGROUND_HOST,
        host_table: OP_TABLE,
        part: PLAIN_PART,
        part_table: LINE_TABLE,
        booking_table: BOOKING_TABLE,
        callees: PART_CALLEES,
        task: Some(BACKGROUND_TASK),
        filter_field: 0,
        last: false,
        next_previous: false,
        value_reads: &[],
        fact_reads: PROBE_READS,
        read_without_move: false,
    },
    // :723 PartRefill_TriggerPartActionThenFilterFirst_FindsRefilledRow: Value of QtyText and Scope after First.
    Cell {
        host: WORKSHEET_HOST,
        host_table: OP_TABLE,
        part: TRIGGER_PART,
        part_table: LINE_TABLE,
        booking_table: BOOKING_TABLE,
        callees: PART_CALLEES,
        task: None,
        filter_field: 0,
        last: false,
        next_previous: false,
        value_reads: &["QtyText", "Scope"],
        fact_reads: PROBE_READS,
        read_without_move: false,
    },
    // PR 1475 PartRefill_TypedKeyTriggerPartActionThenFilterFirst_FindsRefilledRow: the filter on the key's second field; Value of
    // QtyText and Scope after First.
    Cell {
        host: WORKSHEET_HOST,
        host_table: OP_TABLE,
        part: TRIGGER_PART,
        part_table: TYPED_LINE_TABLE,
        booking_table: BOOKING_TABLE,
        callees: TYPED_PART_CALLEES,
        task: None,
        filter_field: 1,
        last: false,
        next_previous: false,
        value_reads: &["QtyText", "Scope"],
        fact_reads: PROBE_READS,
        read_without_move: false,
    },
    // The reporter's repro (issue 8537, green on Business Central 29.1): only the AsDecimal of Allocated Qty.
    Cell {
        host: REPRO_HOST,
        host_table: REPRO_OP_TABLE,
        part: REPRO_PART,
        part_table: REPRO_LINE_TABLE,
        booking_table: REPRO_BOOKING_TABLE,
        callees: PART_CALLEES,
        task: None,
        filter_field: 0,
        last: false,
        next_previous: false,
        value_reads: &[],
        fact_reads: &[("Allocated Qty.", PageMethod::AsDecimal)],
        read_without_move: false,
    },
    // Corpus PR 1534 PartRefill_StyledPartActionThenFilterFirst_FindsRefilledRow (al-runner issue 8704).
    Cell {
        host: WORKSHEET_HOST,
        host_table: OP_TABLE,
        part: STYLED_PART,
        part_table: LINE_TABLE,
        booking_table: BOOKING_TABLE,
        callees: PART_CALLEES,
        task: None,
        filter_field: 0,
        last: false,
        next_previous: false,
        value_reads: &[],
        fact_reads: PROBE_READS,
        read_without_move: false,
    },
    // The reporter's repro of issue 8704 (green on Business Central 29.1): only the AsDecimal of Allocated Qty.
    Cell {
        host: REPRO_HOST,
        host_table: REPRO_OP_TABLE,
        part: REPRO_STYLED_PART,
        part_table: REPRO_LINE_TABLE,
        booking_table: REPRO_BOOKING_TABLE,
        callees: PART_CALLEES,
        task: None,
        filter_field: 0,
        last: false,
        next_previous: false,
        value_reads: &[],
        fact_reads: &[("Allocated Qty.", PageMethod::AsDecimal)],
        read_without_move: false,
    },
];

impl Session {
    /// The cell of `CELLS` host `host`, part `part`, their tables, the table the calculation reads and every procedure their code
    /// calls read as; `None` otherwise.
    pub(super) fn refill_cell(&self, host: &St, part: &St) -> Option<&'static Cell> {
        let engine = self.engine();
        let host_object = host.borrow().object as usize;
        let part_object = part.borrow().object as usize;
        let host_table = self.page_rec(host).borrow().table as usize;
        let part_table = self.page_rec(part).borrow().table as usize;
        // The host shows this part as its one part control.
        let page = page_of(&engine, host);
        let mut parts = page.controls.iter().filter(|c| c.kind == ir::ControlKind::Part);
        let one_part = matches!((parts.next(), parts.next()), (Some(c), None) if c.part_page == Some(part_object));
        if !one_part {
            return None;
        }
        CELLS.iter().find(|cell| {
            let mut callees = Callees::new();
            let names = &mut Names::default();
            let read = page_is(&engine, host_object, cell.host, &mut callees, names)
                && page_is(&engine, part_object, cell.part, &mut callees, names)
                && names.bound('P') == Some(engine.program.objects[part_object].name.as_str())
                && table_is(&engine, host_table, cell.host_table, &mut callees, names)
                && table_is(&engine, part_table, cell.part_table, &mut callees, names)
                && cell.task.is_none_or(|task| task_is(&engine, task, &mut callees, names))
                && callees_are(&engine, &callees, cell.callees, names);
            // The table the calculation reads, as `CALCULATE` bound it.
            let booking = read
                .then(|| object_named(&engine.program, names.bound('B')?, ir::ObjKind::Table))
                .flatten();
            booking.is_some_and(|b| table_is(&engine, b, cell.booking_table, &mut Callees::new(), names))
        })
    }
}

/// Whether the codeunit whose id `«K»` holds has one routine, its `OnRun`, reading as `want`, no global and no subscriber; the
/// procedures it calls join `callees`.
fn task_is(engine: &crate::Engine, want: &str, callees: &mut Callees, names: &mut Names) -> bool {
    let Some(id) = names.bound('K').and_then(|k| k.parse::<u32>().ok()) else {
        return false;
    };
    let Some(task) = engine
        .program
        .objects
        .iter()
        .position(|o| o.kind == ir::ObjKind::Codeunit && o.id == Some(id))
    else {
        return false;
    };
    let o = &engine.program.objects[task];
    let [run] = o.routines.as_slice() else {
        return false;
    };
    run.is_trigger
        && run.name.eq_ignore_ascii_case("onrun")
        && o.globals.is_empty()
        && o.properties.is_empty()
        && !engine.events.any_subscriber_of(task)
        && routine_text(&engine.program, task, 0, callees).is_some_and(|text| names.line_is(&text, want))
}

/// The object of kind `kind` named `name`.
fn object_named(program: &ir::Program, name: &str, kind: ir::ObjKind) -> Option<usize> {
    program.objects.iter().position(|o| o.kind == kind && o.name == name)
}
