//! The row triggers and the DataCaptionExpression runs of a Worksheet the test opens itself (`ws_trigger_order.rs`, corpus PR
//! 1401 pages/properties/WorksheetTriggersOpen.al, al-runner issue 8109). These tests hold the refusals of the new path (the probe's
//! own pages, copied, with calls the probe does not make), the refusals of the same shape outside the cells, and the shapes next to
//! them to main's outcome. The measured cells themselves are the corpus cases.

use al_testkit::outcomes_in;

/// The probe's objects (WorksheetTriggersOpen.al:16-191) and a test codeunit of calls it does not make.
const PROBE: &str = r#"
table 78920 "TRG Row"
{
    DataClassification = CustomerContent;

    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Name; Text[50]) { }
    }

    keys
    {
        key(PK; "Code") { Clustered = true; }
    }
}

page 78922 "TRG WS"
{
    PageType = Worksheet;
    SourceTable = "TRG Row";
    Caption = 'TRG WS';
    DataCaptionExpression = Cap();
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(CodeCtl; Rec."Code") { ApplicationArea = All; }
                field(NameCtl; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    var
        Log: Codeunit "TRG Log";

    local procedure Cap(): Text
    var
        V: Text;
    begin
        V := 'Cap ' + Rec.Name;
        Log.Add('DC=' + V);
        exit(V);
    end;

    trigger OnInit()
    begin
        Log.Add('INIT');
    end;

    trigger OnOpenPage()
    begin
        Log.Add('OPEN');
    end;

    trigger OnAfterGetRecord()
    begin
        Log.Add('AGR ' + Rec."Code");
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('AGC ' + Rec."Code");
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('NEW');
    end;

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        Log.Add('QCP ' + Format(CloseAction));
        exit(true);
    end;

    trigger OnClosePage()
    begin
        Log.Add('CLOSE');
    end;
}

page 78923 "TRG WS NoCap"
{
    PageType = Worksheet;
    SourceTable = "TRG Row";
    Caption = 'TRG WS NoCap';
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(CodeCtl; Rec."Code") { ApplicationArea = All; }
                field(NameCtl; Rec.Name) { ApplicationArea = All; }
            }
        }
    }

    var
        Log: Codeunit "TRG Log";

    trigger OnInit()
    begin
        Log.Add('INIT');
    end;

    trigger OnOpenPage()
    begin
        Log.Add('OPEN');
    end;

    trigger OnAfterGetRecord()
    begin
        Log.Add('AGR ' + Rec."Code");
    end;

    trigger OnAfterGetCurrRecord()
    begin
        Log.Add('AGC ' + Rec."Code");
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    begin
        Log.Add('NEW');
    end;

    trigger OnQueryClosePage(CloseAction: Action): Boolean
    begin
        Log.Add('QCP ' + Format(CloseAction));
        exit(true);
    end;

    trigger OnClosePage()
    begin
        Log.Add('CLOSE');
    end;
}

codeunit 78921 "TRG Log"
{
    SingleInstance = true;

    var
        Lines: List of [Text];

    procedure Wipe()
    begin
        Clear(Lines);
    end;

    procedure Add(Line: Text)
    begin
        Lines.Add(Line);
    end;

    procedure Joined(): Text
    var
        Line: Text;
        Result: Text;
    begin
        foreach Line in Lines do begin
            if Result <> '' then
                Result += ' | ';
            Result += Line;
        end;
        exit(Result);
    end;
}
codeunit 78925 "TRG Seed"
{
    var Log: Codeunit "TRG Log";
    procedure Seed(View: Text; Suffix: Text)
    var Seeded: Record "TRG Row"; i: Integer;
    begin
        Seeded.DeleteAll(); Log.Wipe();
        if View = 'Three' then
            for i := 1 to 3 do begin
                Seeded.Init(); Seeded."Code" := CopyStr('ABC', i, 1); Seeded.Name := 'Row' + CopyStr('ABC', i, 1) + Suffix; Seeded.Insert();
            end;
    end;
}
codeunit 78926 "TRG Values"
{
    Subtype = Test;
    var Log: Codeunit "TRG Log"; S: Codeunit "TRG Seed";
    [Test] procedure Cell_Edit_Three_Log()
    var TP: TestPage "TRG WS";
    begin S.Seed('Three', ''); TP.OpenEdit(); Error('log=%1 cap=%2', Log.Joined(), TP.Caption()); end;
    [Test] procedure NoCap_Edit_Three_Last_KeepsMain()
    var TP: TestPage "TRG WS NoCap";
    begin S.Seed('Three', ''); TP.OpenEdit(); TP.Last(); Error('last=%1', TP.CodeCtl.Value); end;
    [Test] procedure Cell_Close_Ends_The_Limit()
    var TP: TestPage "TRG WS"; V: Text;
    begin
        S.Seed('Three', ''); TP.OpenEdit();
        V := TP.Caption(); V := TP.CodeCtl.Value; TP.First(); V := TP.CodeCtl.Value; TP.Next(); V := TP.CodeCtl.Value; TP.Close();
        asserterror TP.Last(); Error('after close=%1', GetLastErrorText());
    end;
}
codeunit 78927 "TRG Last"
{
    Subtype = Test;
    var S: Codeunit "TRG Seed";
    [Test] procedure Cell_Edit_Last_Refused()
    var TP: TestPage "TRG WS";
    begin S.Seed('Three', ''); TP.OpenEdit(); TP.Last(); end;
}
codeunit 78928 "TRG Filter"
{
    Subtype = Test;
    var S: Codeunit "TRG Seed";
    [Test] procedure Cell_View_Empty_SetFilter_Refused()
    var TP: TestPage "TRG WS NoCap";
    begin S.Seed('Empty', ''); TP.OpenView(); TP.Filter.SetFilter("Code", 'A'); end;
}
codeunit 78929 "TRG Blanks"
{
    Subtype = Test;
    var S: Codeunit "TRG Seed";
    [Test] procedure Cell_Caption_TwoTrailingBlanks_Refused()
    var TP: TestPage "TRG WS";
    begin S.Seed('Three', '  '); TP.OpenEdit(); Error('cap=%1', TP.Caption()); end;
}
"#;

/// Worksheets of the probe pages' shape outside the cells, and pages next to them.
const NEIGHBORS: &str = r#"
table 78930 "TRN Row" { fields { field(1; "Code"; Code[20]) { } field(2; Name; Text[50]) { } } keys { key(PK; "Code") { } } }
table 78931 "TRN Other" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { } } }
codeunit 78932 "TRN Log"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Wipe() begin Lines := ''; end;
    procedure Add(Line: Text) begin if Lines <> '' then Lines += ' | '; Lines += Line; end;
    procedure Joined(): Text begin exit(Lines); end;
}
page 78933 "TRN Risky"
{
    PageType = Worksheet; SourceTable = "TRN Row"; Caption = 'TRN Risky'; DataCaptionExpression = Cap(); UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "TRN Log";
    local procedure Cap(): Text var O: Record "TRN Other"; begin Log.Add('DC'); if O.Get('X') then; exit('Cap ' + Rec.Name); end;
    trigger OnAfterGetRecord() begin Log.Add('AGR ' + Rec."Code"); end;
}
page 78934 "TRN New"
{
    PageType = Worksheet; SourceTable = "TRN Row"; Caption = 'TRN New'; UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "TRN Log";
    trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('NEW'); end;
}
page 78935 "TRN New Delayed"
{
    PageType = Worksheet; SourceTable = "TRN Row"; Caption = 'TRN New Delayed'; UsageCategory = None; DelayedInsert = true;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "TRN Log";
    trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('NEW'); end;
}
page 78936 "TRN List"
{
    PageType = List; SourceTable = "TRN Row"; Caption = 'TRN List'; UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "TRN Log";
    trigger OnNewRecord(BelowxRec: Boolean) begin Log.Add('NEW'); end;
}
page 78937 "TRN Pure"
{
    PageType = Worksheet; SourceTable = "TRN Row"; Caption = 'TRN Pure'; DataCaptionExpression = Rec.Name; UsageCategory = None;
    layout { area(Content) { repeater(L) { field(CodeCtl; Rec."Code") { } } } }
    var Log: Codeunit "TRN Log";
    trigger OnAfterGetRecord() begin Log.Add('AGR ' + Rec."Code"); end;
}
codeunit 78938 "TRN Seed"
{
    var Log: Codeunit "TRN Log";
    procedure Seed(Rows: Integer)
    var R: Record "TRN Row"; i: Integer;
    begin
        R.DeleteAll(); Log.Wipe();
        for i := 1 to Rows do begin R.Init(); R."Code" := Format(i); R.Name := 'N' + Format(i); R.Insert(); end;
    end;
}
codeunit 78939 "TRN Main"
{
    Subtype = Test;
    var Log: Codeunit "TRN Log"; S: Codeunit "TRN Seed";
    [Test] procedure NewDelayed_View_Empty_KeepsMain()
    var TP: TestPage "TRN New Delayed";
    begin S.Seed(0); TP.OpenView(); Error('log=[%1]', Log.Joined()); end;
    [Test] procedure List_View_Empty_KeepsMain()
    var TP: TestPage "TRN List";
    begin S.Seed(0); TP.OpenView(); Error('log=[%1]', Log.Joined()); end;
    [Test] procedure Pure_Edit_KeepsMain()
    var TP: TestPage "TRN Pure";
    begin S.Seed(2); TP.OpenEdit(); TP.Last(); Error('log=[%1] cap=%2', Log.Joined(), TP.Caption()); end;
    [Test] procedure New_Edit_Empty_KeepsMain()
    var TP: TestPage "TRN New";
    begin S.Seed(0); TP.OpenEdit(); Error('log=[%1]', Log.Joined()); end;
}
codeunit 78940 "TRN Risky Tests"
{
    Subtype = Test;
    var S: Codeunit "TRN Seed";
    [Test] procedure Risky_Edit_Refused()
    var TP: TestPage "TRN Risky";
    begin S.Seed(2); TP.OpenEdit(); end;
}
codeunit 78941 "TRN New Tests"
{
    Subtype = Test;
    var S: Codeunit "TRN Seed";
    [Test] procedure New_View_Empty_Refused()
    var TP: TestPage "TRN New";
    begin S.Seed(0); TP.OpenView(); end;
}
"#;

fn outcome(outcomes: &[(String, al_testrun::Outcome)], name: &str) -> String {
    let o = outcomes.iter().find(|(n, _)| n == name).map(|(_, o)| o);
    format!("{o:?}")
}

#[test]
fn calls_after_a_measured_open_that_the_probe_does_not_make_refuse() {
    for (codeunit, t) in [
        ("TRG Last", "Cell_Edit_Last_Refused"),
        ("TRG Filter", "Cell_View_Empty_SetFilter_Refused"),
    ] {
        let o = outcome(&outcomes_in(PROBE, codeunit), t);
        assert!(
            o.contains("NotRun") && o.contains("other than the probe's calls"),
            "{t}: {o}"
        );
    }
    let o = outcome(
        &outcomes_in(PROBE, "TRG Blanks"),
        "Cell_Caption_TwoTrailingBlanks_Refused",
    );
    assert!(
        o.contains("NotRun") && o.contains("blanks the probe did not measure"),
        "{o}"
    );
    let outcomes = outcomes_in(PROBE, "TRG Values");
    // The open of the measured cell (Cap_Three_Edit, WorksheetTriggersOpen.al:315) with the caption read right after it.
    let o = outcome(&outcomes, "Cell_Edit_Three_Log");
    assert!(
        o.contains(
            "log=INIT | OPEN | AGR A | DC=Cap RowA | AGR B | DC=Cap RowB | AGR C | DC=Cap RowC | AGR A | AGR A | DC=Cap RowA | \
             AGC A | DC=Cap RowA cap=TRG WS - Cap RowA"
        ),
        "{o}"
    );
    // TRG WS NoCap with rows keeps main's path: no limit on the calls after the open.
    let o = outcome(&outcomes, "NoCap_Edit_Three_Last_KeepsMain");
    assert!(o.contains("last=C") && !o.contains("NotRun"), "{o}");
    // The close ends the limit: a call on the closed page gets main's error, not the refusal.
    let o = outcome(&outcomes, "Cell_Close_Ends_The_Limit");
    assert!(o.contains("after close=") && !o.contains("NotRun"), "{o}");
}

#[test]
fn the_probe_shape_outside_the_cells_refuses_and_other_pages_keep_main() {
    let o = outcome(&outcomes_in(NEIGHBORS, "TRN Risky Tests"), "Risky_Edit_Refused");
    assert!(
        o.contains("NotRun") && o.contains("can raise an error or leave something behind"),
        "{o}"
    );
    let o = outcome(&outcomes_in(NEIGHBORS, "TRN New Tests"), "New_View_Empty_Refused");
    assert!(
        o.contains("NotRun") && o.contains("BC runs both twice at the open, main neither"),
        "{o}"
    );
    // A property the probe pages do not declare, another page type, an expression that can do nothing, and OpenEdit of the empty
    // view: main's outcome.
    let outcomes = outcomes_in(NEIGHBORS, "TRN Main");
    for (t, want) in [
        ("NewDelayed_View_Empty_KeepsMain", "log=[]"),
        ("List_View_Empty_KeepsMain", "log=[]"),
        ("Pure_Edit_KeepsMain", "cap=TRN Pure - N2"),
        ("New_Edit_Empty_KeepsMain", "log=[NEW | NEW]"),
    ] {
        let o = outcome(&outcomes, t);
        assert!(o.contains(want) && !o.contains("NotRun"), "{t}: {o}");
    }
}
