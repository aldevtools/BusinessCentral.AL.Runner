//! The TestPage filter `'=0'` / `'=1'` on a temporary Worksheet the test opened with OpenEdit (`filter_eq_text.rs`, the cell of
//! corpus PR 1401 testpage/filters/WorksheetOpenEditTemp.al): the measured walks run, and the shapes next to them refuse (review of
//! al-runner PR 8108, round 1).

const SRC: &str = r#"
table 69960 "OE Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; IsHist; Boolean) { } }
    keys { key(PK; Id) { } }
    procedure FourRows()
    var i: Integer;
    begin
        Rec.Reset(); Rec.DeleteAll();
        for i := 1 to 4 do begin Rec.Init(); Rec.Id := i; Rec.Name := 'R' + Format(i); Rec.IsHist := i > 2; Rec.Insert(); end;
    end;
}
table 69961 "OE TRow"
{
    TableType = Temporary;
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; IsHist; Boolean) { } }
    keys { key(PK; Id) { } }
    procedure FourRows()
    var i: Integer;
    begin
        Rec.Reset(); Rec.DeleteAll();
        for i := 1 to 4 do begin Rec.Init(); Rec.Id := i; Rec.Name := 'R' + Format(i); Rec.IsHist := i > 2; Rec.Insert(); end;
    end;
}
page 69962 "OE Act"
{
    PageType = Worksheet; SourceTable = "OE Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(NameCtl; Rec.Name) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.FourRows(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
}
page 69963 "OE Agr"
{
    PageType = Worksheet; SourceTable = "OE Row"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(NameCtl; Rec.Name) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.FourRows(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
    trigger OnAfterGetRecord() begin end;
}
page 69964 "OE TT"
{
    PageType = Worksheet; SourceTable = "OE TRow"; SourceTableTemporary = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(NameCtl; Rec.Name) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.FourRows(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
}
page 69965 "OE DI"
{
    PageType = Worksheet; SourceTable = "OE Row"; SourceTableTemporary = true; DelayedInsert = true; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(IdCtl; Rec.Id) { } field(NameCtl; Rec.Name) { } } } }
    actions { area(Processing) { action(Fill) { trigger OnAction() begin Rec.FourRows(); if Rec.FindFirst() then; CurrPage.Update(false); end; } } }
}
codeunit 69966 C
{
    [TryFunction]
    local procedure GoKey(var P: TestPage "OE Act"; KeyValue: Integer)
    begin
        P.GoToKey(KeyValue);
    end;
    procedure AfterFill(): Text
    var P: TestPage "OE Act"; R: Text;
    begin
        P.OpenEdit(); P.Fill.Invoke(); P.Filter.SetFilter(IsHist, '=1');
        R := 'filter=' + P.Filter.GetFilter(IsHist);
        if GoKey(P, 3) then R += ' cur=' + P.IdCtl.Value();
        P.First(); R += ' first=' + P.IdCtl.Value(); P.Close(); exit(R);
    end;
    procedure BeforeFill(): Text
    var P: TestPage "OE Act"; R: Text;
    begin
        P.OpenEdit(); P.Filter.SetFilter(IsHist, '=1'); P.Fill.Invoke();
        R := 'filter=' + P.Filter.GetFilter(IsHist); P.First(); R += ' first=' + P.IdCtl.Value();
        P.Next(); R += ' next=' + P.IdCtl.Value(); P.Close(); exit(R);
    end;
    procedure DirtyRow(): Text
    var P: TestPage "OE Act";
    begin
        P.OpenEdit(); P.Fill.Invoke(); P.NameCtl.SetValue('Dirty'); P.Filter.SetFilter(IsHist, '=1');
    end;
    procedure NextNext(): Text
    var P: TestPage "OE Act";
    begin
        P.OpenEdit(); P.Fill.Invoke(); P.Next(); P.Next(); P.Filter.SetFilter(IsHist, '=0');
    end;
    procedure NewLine(): Text
    var P: TestPage "OE Act";
    begin
        P.OpenEdit(); P.Fill.Invoke(); P.New(); P.Filter.SetFilter(IsHist, '=1');
    end;
    procedure OtherFieldFirst(): Text
    var P: TestPage "OE Act";
    begin
        P.OpenEdit(); P.Fill.Invoke(); P.Filter.SetFilter(Name, 'R1'); P.Filter.SetFilter(IsHist, '=1');
    end;
    procedure SetValueAfter(): Text
    var P: TestPage "OE Act";
    begin
        P.OpenEdit(); P.Fill.Invoke(); P.Filter.SetFilter(IsHist, '=1'); P.First(); P.NameCtl.SetValue('x');
    end;
    procedure SecondInvoke(): Text
    var P: TestPage "OE Act";
    begin
        P.OpenEdit(); P.Fill.Invoke(); P.Filter.SetFilter(IsHist, '=1'); P.Fill.Invoke();
    end;
    procedure AgrPage(): Text
    var P: TestPage "OE Agr";
    begin
        P.OpenEdit(); P.Fill.Invoke(); P.Filter.SetFilter(IsHist, '=1');
    end;
    procedure TemporaryTable(): Text
    var P: TestPage "OE TT";
    begin
        P.OpenEdit(); P.Fill.Invoke(); P.Filter.SetFilter(IsHist, '=1');
    end;
    procedure DelayedInsertPage(): Text
    var P: TestPage "OE DI";
    begin
        P.OpenEdit(); P.Fill.Invoke(); P.Filter.SetFilter(IsHist, '=1');
    end;
}
"#;

fn refusal(proc: &str) -> String {
    match al_testkit::run_in(SRC, "C", proc) {
        Err(al_builtins::native::Signal::Unsupported(why)) => why.into_reason(),
        other => panic!("{proc}: expected a refusal, got {other:?}"),
    }
}

// WorksheetOpenEditTemp.al:618 GoToKey_Act_OpenEdit and :605 BeforeFill_Act_OpenEdit: the text sets no filter, every row
// shows (rows 1 and 2 have IsHist No, a filter Yes would start at 3), and First, Next, GoToKey and the Invoke after the filter follow.
#[test]
fn the_measured_open_edit_walks_run() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "AfterFill")),
        "filter= cur=3 first=1"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "BeforeFill")),
        "filter= first=1 next=2"
    );
}

// Test calls before the filter that no cell makes: a SetValue, moves, New, a filter on another field.
#[test]
fn other_calls_before_the_filter_refuse() {
    for proc in ["DirtyRow", "NextNext", "NewLine", "OtherFieldFirst"] {
        let why = refusal(proc);
        assert!(why.contains("after test calls on the page no cell"), "{proc}: {why}");
    }
}

// After the filter only the calls of the measured walks follow.
#[test]
fn calls_after_the_filter_outside_the_walks_refuse() {
    for proc in ["SetValueAfter", "SecondInvoke"] {
        let why = refusal(proc);
        assert!(why.contains("after the '=0' or '=1' filter"), "{proc}: {why}");
    }
}

// Pages no cell has: a trigger other than OnOpenPage, TableType = Temporary, DelayedInsert.
#[test]
fn other_pages_refuse() {
    for proc in ["AgrPage", "TemporaryTable", "DelayedInsertPage"] {
        let why = refusal(proc);
        assert!(
            why.contains("on a page whose triggers, properties, controls or source table"),
            "{proc}: {why}"
        );
    }
}
