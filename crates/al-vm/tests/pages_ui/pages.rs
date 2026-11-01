//! The page test client end to end: AL source → compile → run (ADR-012).
//! Each test names the corpus test it mirrors.

use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_testkit::text;

const OBJECTS: &str = r#"
table 50000 "PG Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Qty; Integer) { MinValue = 0; MaxValue = 10; trigger OnValidate() begin Trace.Add('t-qty;'); end; }
        field(3; Descr; Text[50]) { }
    }
    var
        Trace: Codeunit "PG Trace";
    keys { key(PK; "No.") { } }
}

page 50001 "PG Card"
{
    PageType = Card;
    SourceTable = "PG Row";
    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { }
            field(QtyCtl; Rec.Qty) { }
            field(DescrCtl; Rec.Descr) { }
        }
    }
}

page 50009 "PG CodeList"
{
    PageType = List;
    SourceTable = "PG Row";
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Code; Rec."No.") { }
            }
        }
    }
}

page 50003 "PG Format"
{
    PageType = Card;
    SourceTable = "PG Row";
    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { }
            field(Fmt; Rec.Qty) { AutoFormatType = 10; AutoFormatExpression = Pattern(); }
            field(Hide; Rec.Qty) { Visible = Shown; Editable = Shown; Enabled = Shown; }
            field(Named; Rec.Descr) { Caption = 'Shiny'; }
            field(Glob; GlobN) { }
        }
    }
    var
        Shown: Boolean;
        GlobN: Integer;
        Trace: Codeunit "PG Trace";
    trigger OnOpenPage() begin Shown := true; Trace.Add('open;'); end;
    trigger OnAfterGetRecord() begin Trace.Add('agr:' + Rec."No." + ';'); GlobN := Rec.Qty * 2; end;
    trigger OnAfterGetCurrRecord() begin Trace.Add('agcr;'); end;
    local procedure Pattern(): Text
    begin
        Trace.Add('fmt;');
        if Rec."No." = 'BOOM' then Error('boom for %1', Rec."No.");
        exit('<Precision,3:3><Standard Format,0>');
    end;
    actions
    {
        area(Processing)
        {
            action(Flip) { trigger OnAction() begin Shown := not Shown; end; }
        }
    }
}

page 50005 "PG Maker"
{
    PageType = List;
    SourceTable = "PG Row";
    layout { area(Content) { repeater(Lines) { field(NoCtl; Rec."No.") { } field(QtyCtl; Rec.Qty) { } } } }
    actions
    {
        area(Processing)
        {
            action(InsertFind) { trigger OnAction() begin Rec.Init(); Rec."No." := 'M'; Rec.Qty := 4; Rec.Insert(); Rec.FindFirst(); end; }
            action(FieldsOnly) { trigger OnAction() begin Rec.Qty := 9; end; }
        }
    }
}

page 50006 "PG Edit"
{
    PageType = Card;
    SourceTable = "PG Row";
    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { }
            field(QtyCtl; Rec.Qty) { trigger OnValidate() begin Trace.Add('p-qty;'); end; }
            field(DescrCtl; Rec.Descr) { }
            field(Guard; GuardVar) { trigger OnValidate() begin if GuardVar = 'REFUSE' then Error('guard refused'); end; }
        }
    }
    var
        GuardVar: Text;
        Trace: Codeunit "PG Trace";
    trigger OnNewRecord(BelowxRec: Boolean) begin Trace.Add('new:' + Format(BelowxRec) + ',x=' + xRec."No." + ';'); Rec.Descr := 'seeded'; end;
    trigger OnInsertRecord(BelowxRec: Boolean): Boolean begin Trace.Add('ins:' + Rec."No." + '/' + Rec.Descr + ';'); exit(true); end;
    trigger OnModifyRecord(): Boolean begin Trace.Add('mod:' + xRec.Descr + '>' + Rec.Descr + ';'); exit(true); end;
    trigger OnClosePage() begin Trace.Add('close;'); end;
    trigger OnQueryClosePage(CloseAction: Action): Boolean begin Trace.Add('qcp:' + Format(CloseAction) + ';'); exit(not Veto); end;
    var Veto: Boolean;
}

page 50007 "PG NoInsert"
{
    PageType = Card;
    SourceTable = "PG Row";
    InsertAllowed = false;
    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { }
            group(Hidden)
            {
                Visible = false;
                field(InHidden; Rec.Qty) { }
            }
            field(OwnHidden; Rec.Descr) { Visible = false; }
        }
    }
    actions { area(Processing) { action(Quiet) { Visible = false; trigger OnAction() begin Rec.Descr := 'ran'; Rec.Modify(); end; } } }
}

page 50008 "PG DelayedList"
{
    PageType = List;
    SourceTable = "PG Row";
    DelayedInsert = true;
    layout { area(Content) { repeater(Lines) { field(NoCtl; Rec."No.") { } field(DescrCtl; Rec.Descr) { } } } }
}

page 50004 "PG FormatList"
{
    PageType = List;
    SourceTable = "PG Row";
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(NoCtl; Rec."No.") { }
                field(Fmt; Rec.Qty) { AutoFormatType = 10; AutoFormatExpression = Pattern(); }
            }
        }
    }
    local procedure Pattern(): Text
    begin
        if Rec.Qty < 0 then Error('boom for %1', Rec."No.");
        exit('<Precision,3:3><Standard Format,0>');
    end;
}

codeunit 50102 "PG Evt Sub"
{
    SingleInstance = true;
    EventSubscriberInstance = Manual;
    [EventSubscriber(ObjectType::Page, Page::"PG Edit", 'OnOpenPageEvent', '', false, false)]
    local procedure OnOpen(var Rec: Record "PG Row") var T: Codeunit "PG Trace"; begin T.Add('evt-open;'); end;
    [EventSubscriber(ObjectType::Page, Page::"PG Edit", 'OnModifyRecordEvent', '', false, false)]
    local procedure OnMod(var Rec: Record "PG Row"; var xRec: Record "PG Row"; var AllowModify: Boolean) var T: Codeunit "PG Trace"; begin T.Add('evt-mod:' + xRec.Descr + '>' + Rec.Descr + ';'); AllowModify := false; end;
}

enum 50010 "PG Kind"
{
    value(0; Field) { Caption = 'Fields'; }
    value(1; Block) { Caption = 'Blocks'; }
    value(2; Plain) { }
}

page 50011 "PG Kind Page"
{
    PageType = Card;
    layout { area(Content) { field(KindCtl; Kind) { Caption = 'Kind'; } } }
    var Kind: Enum "PG Kind";
}

enum 50090 "PG Blank Kind"
{
    value(0; " ") { Caption = ' '; }
    value(1; Assets) { Caption = 'Assets'; }
}

page 50091 "PG Blank Kind Page"
{
    PageType = Card;
    layout { area(Content) { field(KindCtl; Kind) { Caption = 'Kind Filter'; } } }
    var Kind: Enum "PG Blank Kind";
}

table 50093 "PG Blank Kind Row"
{
    fields { field(1; Id; Integer) { } field(2; Kind; Enum "PG Blank Kind") { } field(3; Opt; Option) { OptionMembers = " ",Alpha,Beta; OptionCaption = ' ,Alpha,Beta'; } }
    keys { key(PK; Id) { Clustered = true; } }
}

page 50094 "PG Blank Kind Card"
{
    PageType = Card;
    SourceTable = "PG Blank Kind Row";
    layout { area(Content) { field(KindCtl; Rec.Kind) { } field(OptCtl; Rec.Opt) { } } }
}

page 50012 "PG Hidden Get"
{
    PageType = List;
    SourceTable = "PG Row";
    SourceTableView = where("No." = const('ZZZ'));
    layout { area(Content) { repeater(Lines) { field(NoCtl; Rec."No.") { } field(QtyCtl; Rec.Qty) { } } } }
    actions { area(Processing) { action(GetRow) { trigger OnAction() begin Rec.Get('A'); end; } } }
}

codeunit 50101 "PG Trace"
{
    SingleInstance = true;
    var Log: Text;
    procedure Add(T: Text) begin Log += T; end;
    procedure Take(): Text var R: Text; begin R := Log; Log := ''; exit(R); end;
}

page 50002 "PG List"
{
    PageType = List;
    SourceTable = "PG Row";
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(NoCtl; Rec."No.") { }
                field(QtyCtl; Rec.Qty) { }
            }
        }
    }
}

// corpus handlers/TestPageCurrFieldNo_Row.al and _Card.al
table 50020 "PG CFN"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(10; Amount; Decimal) { trigger OnValidate() begin Rec.ValidateFieldNo := CurrFieldNo; end; }
        field(11; ValidateFieldNo; Integer) { }
        field(12; ModifyFieldNo; Integer) { }
    }
    keys { key(PK; "No.") { } }
    trigger OnModify() begin Rec.ModifyFieldNo := CurrFieldNo; end;
}

page 50021 "PG CFN Card"
{
    PageType = Card;
    SourceTable = "PG CFN";
    layout { area(Content) { field("No."; Rec."No.") { } field(Amount; Rec.Amount) { } field(ValidateFieldNo; Rec.ValidateFieldNo) { } } }
}

page 50030 "PG Static Caption"
{
    PageType = Card;
    SourceTable = "PG Row";
    Caption = 'Static Caption';
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
}

page 50031 "PG Dynamic Caption"
{
    PageType = Card;
    SourceTable = "PG Row";
    Caption = 'Static Caption';
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
    trigger OnOpenPage() begin CurrPage.Caption := 'Dynamic Caption'; end;
}

table 50040 "PG RO Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Locked; Boolean) { Editable = false; }
        field(3; Free; Boolean) { }
    }
    keys { key(PK; "No.") { } }
}

page 50042 "PG ListPlus"
{
    PageType = ListPlus;
    SourceTable = "PG Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } } }
    var
        Trace: Codeunit "PG Trace";
        Flag: Boolean;
    trigger OnOpenPage() begin Trace.Add('open;'); end;
    trigger OnNewRecord(BelowxRec: Boolean) begin Trace.Add('new;'); end;
    trigger OnAfterGetCurrRecord() begin Flag := true; Trace.Add('agcr;'); end;
}

page 50041 "PG RO Card"
{
    PageType = Card;
    SourceTable = "PG RO Row";
    layout { area(Content) { field(NoCtl; Rec."No.") { } field(LockedCtl; Rec.Locked) { } field(FreeCtl; Rec.Free) { } } }
}
"#;

fn run(body: &str, routine: &str) -> Result<Value, Signal> {
    run_in(body, routine, false)
}

/// With `no_transaction`, the codeunit's `Seed` runs first under the default model and commits,
/// then `routine` runs under `TransactionModel::None`.
fn run_in(body: &str, routine: &str, no_transaction: bool) -> Result<Value, Signal> {
    let src = format!("{OBJECTS}\ncodeunit 50100 C {{ {body} }}");
    let l = al_testkit::compile(&src);
    let oi = l.program.objects.iter().position(|o| o.name == "C").unwrap();
    let ri = l.program.objects[oi]
        .routines
        .iter()
        .position(|r| r.name == routine)
        .unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut stack = vec![(oi, ri)];
    while let Some((o, r)) = stack.pop() {
        if !seen.insert((o, r)) {
            continue;
        }
        if let Some(why) = l.unsupported[o][r].first() {
            panic!(
                "{}.{} is blocked: {why}",
                l.program.objects[o].name, l.program.objects[o].routines[r].name
            );
        }
        stack.extend(l.calls[o][r].iter().copied());
    }
    let mut s = al_testkit::session(l);
    let inst = s.instance(oi);
    if no_transaction {
        let seed = s.engine.program.objects[oi]
            .routines
            .iter()
            .position(|r| r.name == "Seed")
            .unwrap();
        s.call(oi, seed, inst.clone(), vec![]).unwrap();
        s.tx_model = al_vm::TxModel::None;
    }
    s.call(oi, ri, inst, vec![]).map(|(v, _)| v)
}

fn error_text(r: Result<Value, Signal>) -> String {
    match r {
        Err(Signal::Error(e)) => e.message,
        other => panic!("expected an error, got {other:?}"),
    }
}

fn rows(n: &[(&str, i32)]) -> String {
    n.iter()
        .map(|(k, q)| format!("R.\"No.\" := '{k}'; R.Qty := {q}; R.Insert();"))
        .collect::<Vec<_>>()
        .join(" ")
}

// handlers/TestPageHandler.al:17 Page_TestPage_Open_FindsFirstRecord: a page opened on a table
// shows the first row, and a control reads as the text of its field.
#[test]
fn open_shows_the_first_row_and_controls_read_as_text() {
    let r = run(
        "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Card\"; begin
            R.\"No.\" := 'B'; R.Qty := 7; R.Insert();
            R.\"No.\" := 'A'; R.Qty := 5; R.Insert();
            TP.OpenView();
            exit(TP.NoCtl.Value() + ':' + TP.QtyCtl.Value());
        end;",
        "F",
    );
    assert_eq!(text(r), "A:5");
}

// handlers/TestPageHandler.al:28-70 Page_TestPage_Next_AdvancesToSecond / _Last_ReturnsLastRecord
#[test]
fn first_next_last_previous_walk_the_rows_in_key_order() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Card\"; T: Text; begin
                {}
                TP.OpenView();
                T := TP.NoCtl.Value();
                if TP.Next() then T += TP.NoCtl.Value();
                if TP.Next() then T += TP.NoCtl.Value();
                T += Format(TP.Next());
                if TP.Last() then T += TP.NoCtl.Value();
                if TP.Previous() then T += TP.NoCtl.Value();
                if TP.First() then T += TP.NoCtl.Value();
                exit(T);
            end;",
            rows(&[("C", 3), ("A", 1), ("B", 2)])
        ),
        "F",
    );
    assert_eq!(text(r), "ABCNoCBA");
}

// handlers/TestPageExtended.al:17,34 Page_GoToKey_ExistingRecord_ReturnsTrue / _MissingRecord_ReturnsFalse;
// handlers/TestPageGoToRecordNonKeyRefresh_Tests.al:1-15: a missing key leaves the page where it was.
#[test]
fn go_to_key_finds_a_row_and_a_missing_key_leaves_the_cursor() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Card\"; T: Text; begin
                {}
                TP.OpenView();
                T := Format(TP.GoToKey('B')) + TP.NoCtl.Value() + TP.QtyCtl.Value();
                T += Format(TP.GoToKey('Z')) + TP.NoCtl.Value() + TP.QtyCtl.Value();
                exit(T);
            end;",
            rows(&[("A", 1), ("B", 2)])
        ),
        "F",
    );
    assert_eq!(text(r), "YesB2NoB2");
}

// testpageclose/TestPageClosingActions.al:19-24 and testpageteardown/TestPageTeardown.al:227,339:
// a variable that is not open answers "The TestPage is not open."; an open one cannot be opened.
#[test]
fn closed_and_open_variables_refuse_with_the_corpus_texts() {
    let body = |stmt: &str| {
        format!(
            "procedure F(): Text var TP: TestPage \"PG Card\"; T: Text; begin
                {stmt}
            end;"
        )
    };
    let e = error_text(run(&body("T := TP.NoCtl.Value();"), "F"));
    assert_eq!(e, "The TestPage is not open.");
    let e = error_text(run(&body("TP.Close();"), "F"));
    assert_eq!(e, "The TestPage is not open.");
    let e = error_text(run(&body("TP.OpenView(); TP.OpenView();"), "F"));
    assert_eq!(e, "The TestPage is already open.");
    // Its code is TestPageAlreadyOpen (codeunit/lt-qu-commit-isolation/BlankKeyRowNextTest.al:274, F2_Read_CloseLeftOpen).
    match run(&body("TP.OpenView(); TP.OpenEdit();"), "F") {
        Err(Signal::Error(e)) => assert_eq!(e.code, "TestPageAlreadyOpen"),
        other => panic!("expected an error, got {other:?}"),
    }
    let e = error_text(run(&body("TP.OpenView(); TP.Close(); T := TP.NoCtl.Value();"), "F"));
    assert_eq!(e, "The TestPage is not open.");
}

// autoformat/TestAutoFormatExpressionTiming.al:5-12: the control expressions run when a row is
// populated, after OnOpenPage and before OnAfterGetRecord, even with no row at all; an error
// fails the open and leaves the variable closed.
#[test]
fn control_expressions_run_between_on_open_page_and_on_after_get_record() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Format\"; T: Codeunit \"PG Trace\"; begin
                {}
                TP.OpenView();
                exit(T.Take());
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    assert_eq!(text(r), "open;fmt;agr:A;agcr;");
    let r = run(
        "procedure F(): Text var TP: TestPage \"PG Format\"; T: Codeunit \"PG Trace\"; begin
            TP.OpenView();
            exit(T.Take());
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "open;fmt;",
        "an empty view populates the expressions too, but no row"
    );
}

// autoformat/TestAutoFormatExpressionTiming.al: a failing expression at open raises its own
// error and leaves the page not open; on a card, a move onto the failing row reads "not open".
#[test]
fn a_failing_expression_fails_the_open_and_tears_a_card_down_on_a_move() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Format\"; begin
                {}
                TP.OpenView();
                TP.GoToKey('BOOM');
            end;",
            rows(&[("A", 1), ("BOOM", 2)])
        ),
        "F",
    );
    assert_eq!(error_text(r), "The TestPage is not open.");
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Format\"; begin
                {}
                TP.OpenView();
            end;",
            rows(&[("BOOM", 2)])
        ),
        "F",
    );
    assert_eq!(error_text(r), "boom for BOOM");
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Format\"; T: Text; begin
                {}
                TP.OpenView();
                asserterror TP.GoToKey('BOOM');
                T := GetLastErrorText();
                asserterror T += TP.NoCtl.Value();
                exit(T + '|' + GetLastErrorText());
            end;",
            rows(&[("A", 1), ("BOOM", 2)])
        ),
        "F",
    );
    assert_eq!(text(r), "The TestPage is not open.|The TestPage is not open.");
}

// handlers/TestPageControlPropertyLiveness_Tests.al:40-73: Visible is the open-time value,
// Editable and Enabled follow the page state.
#[test]
fn visible_is_frozen_at_open_editable_and_enabled_are_live() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Format\"; T: Text; begin
                {}
                TP.OpenView();
                T := Format(TP.Hide.Visible()) + Format(TP.Hide.Enabled());
                TP.Flip.Invoke();
                T += '|' + Format(TP.Hide.Visible()) + Format(TP.Hide.Enabled());
                exit(T);
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    assert_eq!(text(r), "YesYes|YesNo");
}

// autoformat/TestAutoFormat.al: an AutoFormatExpression with a precision sets the places;
// handlers/TestPageFieldFormats.Page.al: page-variable controls read the variable.
#[test]
fn controls_read_through_their_format_and_binding() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Format\"; begin
                {}
                TP.OpenView();
                exit(TP.Fmt.Value() + '|' + TP.Glob.Value() + '|' + TP.Named.Caption() + '|' + TP.NoCtl.Caption());
            end;",
            rows(&[("A", 1500)])
        ),
        "F",
    );
    assert_eq!(text(r), "1500|3000|Shiny|No.");
}

// handlers/TestPageNewRowLine_Tests.al:1-30 (EditableInsertable_NextPastLastDataRow_LandsOnTheNewRowLine and
// the OpenView gating arm): an editable list carries a blank line past its data; OpenView has none.
#[test]
fn an_editable_list_has_a_blank_new_row_line_past_its_data() {
    let f = |open: &str| {
        run(
            &format!(
                "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG List\"; T: Text; begin
                    {}
                    TP.{open}();
                    T := Format(TP.First()) + TP.NoCtl.Value();
                    T += Format(TP.Next()) + '[' + TP.NoCtl.Value() + ']';
                    T += Format(TP.Next());
                    exit(T);
                end;",
                rows(&[("A", 1)])
            ),
            "F",
        )
    };
    assert_eq!(text(f("OpenEdit")), "YesAYes[]No");
    assert_eq!(text(f("OpenView")), "YesANo[A]No");
}

// handlers/TestPageNewRowLine_Tests.al: First() on an empty editable list answers false.
#[test]
fn first_on_an_empty_editable_list_is_false() {
    let r = run(
        "procedure F(): Text var TP: TestPage \"PG List\"; begin
            TP.OpenEdit();
            exit(Format(TP.First()) + Format(TP.Last()));
        end;",
        "F",
    );
    assert_eq!(text(r), "NoNo");
}

// autoformat/TestAutoFormatExpressionTiming.al, RowFormat_List_FailingRow20Of120_RaisesAtOpen /
// _FailingRow30Of120_OpensAndReadsTheFirstRow / _LastOntoAFailingRowBeyondTheWindow_RaisesItsOwnError:
// a list populates a window of rows (the corpus pins it between 20 and 24) and raises the expression's
// own error on a move that populates a new window.
#[test]
fn a_list_populates_a_window_of_rows() {
    // PG Format has the BOOM key hard-wired, so the rows are named so that BOOM falls at the wanted position.
    let body = |boom_at: i32, then: &str| {
        format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG FormatList\"; i: Integer; begin
                for i := 1 to 120 do begin
                    R.\"No.\" := 'R' + Format(1000 + i); R.Qty := i; if i = {boom_at} then R.Qty := -1; R.Insert();
                end;
                TP.OpenView();
                {then}
            end;"
        )
    };
    assert_eq!(error_text(run(&body(20, "exit('');"), "F")), "boom for R1020");
    assert_eq!(text(run(&body(30, "exit(TP.Fmt.Value());"), "F")), "1");
    assert_eq!(error_text(run(&body(120, "TP.Last();"), "F")), "boom for R1120");
}

// testpart/BnrBlankPart.al:1-30 (NoRow_Action_StoredListInsertAndFind_ShowsTheRow / _TempListFieldsOnly_StaysBlank):
// controls read blank on a page that shows no row, until page code inserts a row and stands on it.
#[test]
fn a_page_with_no_row_reads_blank_until_page_code_inserts_and_positions_a_row() {
    let r = run(
        "procedure F(): Text var TP: TestPage \"PG Maker\"; T: Text; begin
            TP.OpenView();
            T := '[' + TP.QtyCtl.Value() + ']' + Format(TP.QtyCtl.AsInteger());
            TP.FieldsOnly.Invoke();
            T += '[' + TP.QtyCtl.Value() + ']';
            TP.InsertFind.Invoke();
            T += '[' + TP.QtyCtl.Value() + ']' + TP.NoCtl.Value();
            exit(T);
        end;",
        "F",
    );
    assert_eq!(text(r), "[]0[][4]M");
}

// handlers/TestPageReadOnlySetValue.al:253-330, TestPageBlankKeyInsert.al, TestPageSaveRecordInOnValidate_Tests.al:
// SetValue validates (table trigger, then the page control's), shows at once, and reaches the
// table when the page leaves the row (Close); writing back the same value modifies nothing.
#[test]
fn set_value_validates_and_the_row_is_saved_when_the_page_closes() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; T: Codeunit \"PG Trace\"; X: Text; begin
                {}
                TP.OpenEdit();
                TP.QtyCtl.SetValue(5);
                X := TP.QtyCtl.Value();
                R.Get('A');
                X += '|' + Format(R.Qty);
                TP.Close();
                R.Get('A');
                exit(X + '|' + Format(R.Qty) + '|' + T.Take());
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    // table OnValidate before the page control's; OnModifyRecord sees xRec; OnClosePage after the save.
    assert_eq!(text(r), "5|1|5|t-qty;p-qty;mod:>;qcp:OK;close;");
}

// handlers/TestPageSameValueSetValue_Tests.al:1-30: writing back the value the field already holds
// does not modify the row.
#[test]
fn writing_the_same_value_back_does_not_modify() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; T: Codeunit \"PG Trace\"; begin
                {}
                TP.OpenEdit();
                TP.DescrCtl.SetValue('');
                TP.Close();
                exit(T.Take());
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    assert_eq!(text(r), "qcp:OK;close;");
}

// handlers/TestPageFieldMinMaxValue_Tests.al:20-110: MinValue and MaxValue are enforced on a control write
// and a refused write does not reach the row.
#[test]
fn min_and_max_value_refuse_a_control_write() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; X: Text; begin
                {}
                Commit();
                TP.OpenEdit();
                asserterror TP.QtyCtl.SetValue(11);
                X := GetLastErrorText();
                TP.Close();
                R.Get('A');
                exit(X + '|' + Format(R.Qty));
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    let t = text(r);
    assert!(t.contains("The value must be less than or equal to 10"), "{t}");
    assert!(t.ends_with("|1"), "{t}");
}

// handlers/TestPageValidationError_Tests.al:62-250: a refusing OnValidate records one validation error
// on the control and the page (with the refresh suffix for a Rec-bound control, none for a page
// variable) and raises the wrapper.
#[test]
fn a_refused_write_is_recorded_as_a_validation_error() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; X: Text; begin
                {}
                TP.OpenEdit();
                asserterror TP.Guard.SetValue('REFUSE');
                X := GetLastErrorText() + '|' + Format(TP.Guard.ValidationErrorCount()) + '|' + TP.Guard.GetValidationError(1);
                X += '|' + Format(TP.ValidationErrorCount()) + '|' + TP.GetValidationError(1);
                exit(X);
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    assert_eq!(
        text(r),
        "Validation error for Field: Guard,  Message = 'guard refused'|1|guard refused|1|guard refused"
    );
}

// GetValidationError() of a TestPage without an index (an assumption, page-getvalidationerror-no-index): the first
// error of the page's ledger, the empty text while there is none.
#[test]
fn page_get_validation_error_without_an_index_is_the_first_error_or_empty() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; X: Text; begin
                {}
                TP.OpenEdit();
                X := '[' + TP.GetValidationError() + ']';
                asserterror TP.Guard.SetValue('REFUSE');
                X += '[' + TP.GetValidationError() + ']';
                exit(X);
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    assert_eq!(text(r), "[][guard refused]");
}

// A control's GetValidationError() without an index while the control has a validation error ends the session on real
// BC with an uncatchable ArgumentOutOfRangeException (Windows check of the corpus, runs 39162782959 / 39162785239); the
// runtime cannot raise such an error, so it refuses. Without an error it is not pinned by the corpus, and is refused too.
#[test]
fn control_get_validation_error_without_an_index_is_refused() {
    for before in ["asserterror TP.Guard.SetValue('REFUSE');", ""] {
        let r = run(
            &format!(
                "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; begin
                    {}
                    TP.OpenEdit();
                    {before}
                    exit(TP.Guard.GetValidationError());
                end;",
                rows(&[("A", 1)])
            ),
            "F",
        );
        assert!(
            matches!(&r, Err(Signal::Unsupported(m)) if m.contains("without an index")),
            "{before}: {r:?}"
        );
    }
}

// handlers/TestPageBooleanSpelling_Tests.al: SetValue('False') is not an acceptable Boolean text.
// (TextBool: a Text value written to an Integer control that cannot parse it.) On a card the text says why
// (pages/lt-ah-pages/AhPageTests.al:112, pages/lt-page-runtime/P82TestField.al:296).
#[test]
fn a_text_that_does_not_parse_is_refused_with_the_acceptable_value_message() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; begin
                {}
                TP.OpenEdit();
                TP.QtyCtl.SetValue('abc');
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    assert_eq!(
        error_text(r),
        "Validation error for Field: QtyCtl,  Message = 'Your entry of 'abc' is not an acceptable value for 'Qty'. The value \"abc\" can't be evaluated into type Integer. (Select Refresh to discard errors)'"
    );
}

// handlers/TestPageRecordTriggers.al:32-130, TestPageCreateModeNewRecordArgs.al:243-270: OpenNew runs OnNewRecord
// (BelowxRec true, xRec the last row) before anything is typed; the key alone does not insert a
// Card row, a non-key control does (TestPageBlankKeyInsert.al:Card_TypedKey_ThenNonKeyWrite_InsertsBeforeClose).
#[test]
fn open_new_runs_on_new_record_and_a_non_key_write_inserts_the_row() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; T: Codeunit \"PG Trace\"; X: Text; begin
                {}
                TP.OpenNew();
                TP.NoCtl.SetValue('N');
                X := Format(R.Count());
                TP.DescrCtl.SetValue('typed');
                X += Format(R.Count());
                TP.Close();
                R.Get('N');
                exit(X + '|' + R.Descr + '|' + T.Take());
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    assert_eq!(
        text(r),
        "12|typed|new:Yes,x=A;ins:N/seeded;mod:seeded>typed;qcp:OK;close;"
    );
}

// handlers/TestPageInsertAllowed_Tests.al:70 New_OnInsertAllowedFalsePage_IsDenied
#[test]
fn new_is_denied_on_a_page_that_does_not_allow_inserts() {
    let r = run(
        "procedure F() var TP: TestPage \"PG NoInsert\"; begin TP.OpenEdit(); TP.New(); end;",
        "F",
    );
    assert_eq!(
        error_text(r),
        "New method failed because Insert is not allowed. Page = PG NoInsert, Id = 50007."
    );
}

// handlers/TestPageFieldVisibleGroup_Tests.al:1-30 and TestPageActionReachability_Tests.al:100-122: a control
// whose Visible is the literal false (or inside such a group) is not on the page; an action is, and still runs.
#[test]
fn literal_false_visible_removes_a_control_but_not_an_action() {
    let body = |stmt: &str| {
        format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG NoInsert\"; X: Boolean; begin
                {} TP.OpenEdit(); {stmt}
            end;",
            rows(&[("A", 1)])
        )
    };
    let e = error_text(run(&body("X := TP.InHidden.Visible();"), "F"));
    assert!(e.ends_with("is not found on the page."), "{e}");
    let e = error_text(run(&body("X := TP.OwnHidden.Visible();"), "F"));
    assert!(e.ends_with("is not found on the page."), "{e}");
    let r = run(
        &body("exit(Format(TP.Quiet.Visible()) + Format(TP.Quiet.Enabled()));"),
        "F",
    );
    assert_eq!(text(r), "NoYes");
}

// handlers/TestPageFailedInsert_Tests.al:1-30: a started line whose insert fails raises nothing on New(),
// stays, and counts one validation error on its key control; Previous() moves on and Close() raises.
#[test]
fn a_started_line_whose_insert_fails_is_not_raised_on_new_but_on_close_after_previous() {
    let f = |mv: &str, close: &str| {
        run(
            &format!(
                "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG DelayedList\"; X: Text; begin
                    {}
                    TP.OpenNew();
                    TP.NoCtl.SetValue('A');
                    TP.DescrCtl.SetValue('typed');
                    {mv}
                    X := TP.NoCtl.Value() + Format(TP.NoCtl.ValidationErrorCount()) + Format(TP.DescrCtl.ValidationErrorCount());
                    {close}
                    exit(X);
                end;",
                rows(&[("A", 1)])
            ),
            "F",
        )
    };
    assert_eq!(text(f("TP.New();", "TP.Close();")), "A10");
    let e = error_text(f("TP.First();", "TP.Close();"));
    assert!(e.starts_with("The record in table PG Row already exists."), "{e}");
}

// testpageclose/TestPageClosingActions.al:Card_OK_RaisesTheCloseTriggersOnce / _Cancel_IsNotOffered / _OK_RefusedByOnQueryClosePage:
// OK saves, asks OnQueryClosePage(OK), then OnClosePage; a card offers no Cancel; Close() reports Cancel.
#[test]
fn ok_closes_with_the_query_close_page_then_the_close_trigger() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; T: Codeunit \"PG Trace\"; X: Text; begin
                {}
                TP.OpenEdit();
                TP.DescrCtl.SetValue('saved');
                TP.OK().Invoke();
                R.Get('A');
                X := R.Descr + '|' + T.Take();
                asserterror TP.NoCtl.Value();
                X += GetLastErrorText();
                asserterror TP.Cancel();
                X += '|' + GetLastErrorText();
                TP.OpenEdit();
                asserterror TP.Cancel().Invoke();
                X += '|' + GetLastErrorText();
                TP.Close();
                exit(X + '|' + T.Take());
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    // The second OpenEdit sees an empty view (the asserterror before it rolled the seeded row back), so the page runs OnNewRecord on a
    // blank record (pages/lt-page-types-empty-table/PageTypesEmptyTable.al:Card_OpenEdit_EmptyTable).
    assert_eq!(
        text(r),
        "saved|mod:>saved;qcp:OK;close;The TestPage is not open.|The TestPage is not open.|The built-in action = Cancel is not found on the page.|new:Yes,x=;qcp:OK;close;"
    );
}

// handlers/TestPageModeSwitchAction_Tests.al:Claim 1 and 2, TestPageReadOnlySetValue.al:365: the built-in Edit
// and View switch an open card's mode in place; a page-variable control stays editable on a read-only page.
#[test]
fn edit_and_view_switch_a_card_in_place() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; X: Text; begin
                {}
                TP.OpenView();
                X := Format(TP.Editable()) + Format(TP.QtyCtl.Editable()) + Format(TP.Guard.Editable());
                X += Format(TP.Edit().Enabled()) + Format(TP.View().Enabled());
                TP.Edit().Invoke();
                X += '|' + Format(TP.Editable()) + Format(TP.QtyCtl.Editable()) + Format(TP.Edit().Enabled()) + Format(TP.View().Enabled());
                exit(X);
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    assert_eq!(text(r), "NoNoYesYesNo|YesYesNoYes");
}

// handlers/TestPageTriggerEvents.al:1-20, :230-300: the platform's page events run after the page's own trigger,
// and AllowModify := false vetoes the write.
#[test]
fn page_events_follow_the_triggers_and_can_veto_a_modify() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; T: Codeunit \"PG Trace\"; E: Codeunit \"PG Evt Sub\"; X: Text; begin
                BindSubscription(E);
                {}
                TP.OpenEdit();
                TP.DescrCtl.SetValue('x');
                TP.Close();
                R.Get('A');
                exit(R.Descr + '|' + T.Take());
            end;",
            rows(&[("A", 1)])
        ),
        "F",
    );
    assert_eq!(text(r), "|evt-open;mod:>x;evt-mod:>x;qcp:OK;close;");
}

// handlers/TestPageEnumVariableControl_Tests.al:50-56,86-90: an Enum control resolves a SetValue text
// by the member's caption; the bare member name of a captioned value is refused.
#[test]
fn an_enum_control_takes_the_caption_and_refuses_the_member_name() {
    let body = |set: &str| {
        format!(
            "procedure F(): Text var TP: TestPage \"PG Kind Page\"; begin
                TP.OpenEdit();
                TP.KindCtl.SetValue('{set}');
                exit(TP.KindCtl.Value());
            end;"
        )
    };
    assert_eq!(text(run(&body("Blocks"), "F")), "Blocks");
    assert_eq!(
        error_text(run(&body("Block"), "F")),
        "Validation error for Field: KindCtl,  Message = 'Your entry of 'Block' is not an acceptable value for 'Kind'.'"
    );
}

// A value that declares no caption shows, and is set by, its name (types/TestEnumCaption.al).
#[test]
fn an_enum_value_without_a_caption_is_set_by_its_name() {
    let r = run(
        "procedure F(): Text var TP: TestPage \"PG Kind Page\"; begin
            TP.OpenEdit();
            TP.KindCtl.SetValue('Plain');
            exit(TP.KindCtl.Value());
        end;",
        "F",
    );
    assert_eq!(text(r), "Plain");
}

// testpart/BnrBlankPart.al NoRow_Action_GetOfARowTheFilterHides_StaysBlank: an action that Gets a row the
// page's filter hides, on a key field of the filter, leaves the page showing nothing.
#[test]
fn a_get_of_a_row_the_view_hides_shows_nothing() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Hidden Get\"; begin
                {}
                TP.OpenView();
                TP.GetRow.Invoke();
                exit('[' + TP.QtyCtl.Value() + '][' + TP.NoCtl.Value() + ']');
            end;",
            rows(&[("A", 5)])
        ),
        "F",
    );
    assert_eq!(text(r), "[][]");
}

// codeunit/TestWriteTxTestBoundary.al:Test13, Test13c, Test13d: under TransactionModel::None a
// TestPage operation opens a transaction of its own, so the page may write, and the transaction
// ends with the operation. A direct write from the test is refused again afterwards.
#[test]
fn under_transaction_model_none_a_page_operation_may_write_and_ends_its_transaction() {
    let r = run_in(
        &format!(
            "procedure Seed() var R: Record \"PG Row\"; begin {} Commit(); end;
            procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG Edit\"; X: Text; begin
                TP.OpenEdit();
                TP.QtyCtl.SetValue(5);
                TP.Close();
                X := Format(Database.IsInWriteTransaction());
                R.Get('A');
                X += '|' + Format(R.Qty);
                asserterror begin R.\"No.\" := 'Z'; R.Insert(); end;
                exit(X + '|' + GetLastErrorText());
            end;",
            rows(&[("A", 1)])
        ),
        "F",
        true,
    );
    assert_eq!(
        text(r),
        "No|5|A transaction must be started before changes can be made to the database."
    );
}

// handlers/TestPageCurrFieldNo_Tests.al:36-60 (SetValue_OwnTableField_OnValidateSeesCurrFieldNo),
// :65-76 (Validate_FromCode_OnValidateSeesZero), :105-123 (SetValue_ThenClose_OnModifySeesZero).
#[test]
fn a_page_write_validates_with_the_field_number_and_code_with_zero() {
    let r = run(
        "procedure F(): Text var R: Record \"PG CFN\"; C: Record \"PG CFN\"; TP: TestPage \"PG CFN Card\"; a: Text; begin
            R.\"No.\" := 'A'; R.Insert();
            TP.OpenEdit(); TP.GoToRecord(R);
            TP.Amount.SetValue(50);
            a := TP.ValidateFieldNo.Value();
            TP.Close();
            R.Get('A');
            a += '|' + Format(R.ValidateFieldNo) + '|' + Format(R.ModifyFieldNo);
            C.\"No.\" := 'B'; C.Insert(); C.Validate(Amount, 50);
            exit(a + '|' + Format(C.ValidateFieldNo));
        end;",
        "F",
    );
    assert_eq!(text(r), "10|10|0|0");
}

// handlers/TestPageCaption_Tests.al:DynamicCaptionSetInOnOpenPage_IsReturned (and the static twin):
// `CurrPage.Caption := x` in OnOpenPage is what `TestPage.Caption()` answers.
#[test]
fn a_caption_assigned_in_on_open_page_is_the_test_pages_caption() {
    let r = run(
        "procedure F(): Text var D: TestPage \"PG Dynamic Caption\"; S: TestPage \"PG Static Caption\"; begin
            D.OpenView(); S.OpenView();
            exit(D.Caption() + '/' + S.Caption());
        end;",
        "F",
    );
    assert_eq!(text(r), "Dynamic Caption/Static Caption");
}

// handlers/TestPagePlatformPageSourceTable_Tests.al:66,125: a bare name in the field position of
// FindFirstField is a control of the page; a value no row carries raises the pinned error.
#[test]
fn find_first_field_takes_a_bare_control_name() {
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG FormatList\"; T: Text; begin
                {}
                TP.OpenView();
                T := Format(TP.FindFirstField(NoCtl, 'B')) + TP.NoCtl.Value();
                asserterror TP.FindFirstField(NoCtl, 'Q');
                exit(T + '|' + GetLastErrorText());
            end;",
            rows(&[("A", 1), ("B", 2)])
        ),
        "F",
    );
    assert_eq!(text(r), "YesB|The row does not exist on the TestPage.");
    // A control called Code (the data type's name), as page "Currencies" has.
    let r = run(
        &format!(
            "procedure F(): Text var R: Record \"PG Row\"; TP: TestPage \"PG CodeList\"; begin
                {}
                TP.OpenView();
                TP.FindFirstField(Code, 'B');
                exit(TP.Code.Value());
            end;",
            rows(&[("A", 1), ("B", 2)])
        ),
        "F",
    );
    assert_eq!(text(r), "B");
}

// pages/lt-page-field-editable-source/PageFieldEditableSource.al:Card_OpenEdit_FieldEditable `lockedNoProp`: a page field over a
// stored source field with `Editable = false` and no Editable of its own reads No. A FlowField is the assumption
// page-field-follows-source-field-editable-false (Microsoft's Payment Export Mgt Unit Test.FilterByHasPaymentExportError).
#[test]
fn a_page_field_over_a_source_field_with_editable_false_is_not_editable() {
    let r = run(
        "procedure F(): Text var R: Record \"PG RO Row\"; TP: TestPage \"PG RO Card\"; begin
            R.\"No.\" := 'A'; R.Insert();
            TP.OpenEdit();
            exit(Format(TP.NoCtl.Editable()) + Format(TP.LockedCtl.Editable()) + Format(TP.FreeCtl.Editable()));
        end;",
        "F",
    );
    assert_eq!(text(r), "YesNoYes");
}

// pages/lt-page-types-empty-table/PageTypesEmptyTable.al:ListPlus_OpenEdit_EmptyTable (also Card and Document): a page without a
// repeater opened for edit over an empty view runs OnNewRecord and OnAfterGetCurrRecord on a blank record after OnOpenPage.
// Microsoft's Suggest Price Lines UT.T212_CopyLinesEnabledOnPurchPriceListPageIfFixedUpgradeCompleted needs it.
#[test]
fn a_list_plus_page_over_an_empty_view_runs_new_record_and_get_curr_record() {
    let r = run(
        "procedure F(): Text var TP: TestPage \"PG ListPlus\"; T: Codeunit \"PG Trace\"; X: Text; begin
            TP.OpenEdit();
            exit(T.Take());
        end;",
        "F",
    );
    assert_eq!(text(r), "open;new;agcr;");
}

// pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:502-528 (SetValue_FCompat_VariantInts_FromBlank) and :2366
// (SetValue_VCompat_VariantInts_FromBlank): an Integer handed to an Enum control, bound to a table field or to a page
// variable, selects the value of that ordinal, also when the value's caption is ' ' (which trims to the refused '').
// Pinned on a card with a row.
#[test]
fn set_value_of_an_integer_selects_the_blank_captioned_enum_value() {
    al_builtins::assumptions::clear_hits();
    let r = run(
        "procedure F(): Text var TP: TestPage \"PG Blank Kind Page\"; V: Variant; begin
            TP.OpenEdit();
            TP.KindCtl.SetValue('Assets');
            TP.KindCtl.SetValue(0);
            exit('[' + TP.KindCtl.Value() + ']');
        end;",
        "F",
    );
    assert_eq!(text(r), "[ ]");
    // A page without a source table is not the pinned shape: assumption testpage-setvalue-blank-enum-member.
    assert!(
        al_builtins::assumptions::take_hits()
            .contains(&al_builtins::assumptions::Assumption::TestPageSetValueBlankEnumMember)
    );
    let r = run(
        "procedure F(): Text var R: Record \"PG Blank Kind Row\"; TP: TestPage \"PG Blank Kind Card\"; V: Variant; begin
            R.Id := 1; R.Kind := R.Kind::Assets; R.Insert();
            TP.OpenEdit();
            V := 0;
            TP.KindCtl.SetValue(V);
            TP.Close();
            R.Get(1);
            exit('[' + Format(R.Kind.AsInteger()) + ']');
        end;",
        "F",
    );
    assert_eq!(text(r), "[0]");
    assert!(al_builtins::assumptions::take_hits().is_empty());
}

// pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:532-597 (SetValue_FCompat_VariantTexts_FromBlank, BC 28 and later): a
// Text inside a Variant selects by ordinal digits; a caption or a blank text fails to evaluate and the control keeps its value.
#[test]
fn set_value_of_a_text_in_a_variant_goes_by_ordinal_digits() {
    let r = run(
        "procedure F(): Text var R: Record \"PG Blank Kind Row\"; TP: TestPage \"PG Blank Kind Card\"; V: Variant; T: Text; begin
            R.Id := 1; R.Insert();
            TP.OpenEdit();
            V := 'Assets';
            asserterror TP.KindCtl.SetValue(V);
            T := GetLastErrorText() + '|' + GetLastErrorCode() + '|[' + TP.KindCtl.Value() + ']|';
            V := ' ';
            asserterror TP.KindCtl.SetValue(V);
            T += GetLastErrorText() + '|';
            V := '1';
            TP.KindCtl.SetValue(V);
            exit(T + TP.KindCtl.Value());
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "The value \"Assets\" can't be evaluated into type Option.|Evaluate:Format|[ ]|The value \" \" can't be evaluated into type Option.|Assets"
    );
}

// pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:541-597 `member0` (the blank-captioned Enum value in a Variant selects it),
// :600-611 `space` (a Text ' ' selects the value whose caption is ' '), :1907 and :2138 (an Integer that is no Option member names
// the ordinal held before).
#[test]
fn set_value_selects_a_blank_captioned_value_by_member_and_by_space() {
    let r = run(
        "procedure F(): Text var R: Record \"PG Blank Kind Row\"; TP: TestPage \"PG Blank Kind Card\"; V: Variant; E: Enum \"PG Blank Kind\"; T: Text; begin
            R.Id := 1; R.Kind := R.Kind::Assets; R.Opt := R.Opt::Beta; R.Insert();
            TP.OpenEdit();
            E := E::\" \";
            V := E;
            TP.KindCtl.SetValue(V);
            T := '[' + TP.KindCtl.Value() + ']';
            TP.KindCtl.SetValue('Assets');
            T += '[' + TP.KindCtl.Value() + ']';
            TP.KindCtl.SetValue(' ');
            T += '[' + TP.KindCtl.Value() + ']';
            asserterror TP.OptCtl.SetValue(5);
            exit(T + GetLastErrorText());
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "[ ][Assets][ ]Validation error for Field: OptCtl,  Message = 'Your entry of '2' is not an acceptable value for 'Opt'. (Select Refresh to discard errors)'"
    );
}

// pages/lt-be-enum-setvalue-blank/BeEnumSetValueBlank.al:523-525 (int5 from blank) and :742-744 (int5 from Beta): an Integer that
// is no value of the Enum is refused naming the ordinal the control held before, not the one handed in.
#[test]
fn set_value_of_an_integer_that_is_no_enum_value_names_the_old_ordinal() {
    let r = run(
        "procedure F(): Text var R: Record \"PG Blank Kind Row\"; TP: TestPage \"PG Blank Kind Card\"; begin
            R.Id := 1; R.Kind := R.Kind::Assets; R.Insert();
            TP.OpenEdit();
            asserterror TP.KindCtl.SetValue(5);
            exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + TP.KindCtl.Value());
        end;",
        "F",
    );
    assert_eq!(
        text(r),
        "Validation error for Field: KindCtl,  Message = 'Your entry of '1' is not an acceptable value for 'Kind'. (Select Refresh to discard errors)'|TestValidation|1"
    );
}
