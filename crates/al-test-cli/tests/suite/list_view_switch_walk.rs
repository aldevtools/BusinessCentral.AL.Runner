//! A List with no card (AutoSplitKey, InsertAllowed by default), opened by Page.Run on a filtered record or by OpenEdit: after
//! the built-in View action a First/Next walk visits only the rows; without View the list opened by Page.Run also shows the
//! blank new-row line. al-runner issue 6977 and the probe testpage/R6977ListViewNext.al of the corpus.

use al_testkit::outcomes_in;

/// The objects of testpage/R6977ListViewNext.al with the Probe Observer replaced by Error on a difference.
const PROBE: &str = r##"
table 95986 "TP R6977 Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Header No."; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Description; Text[30]) { }
    }

    keys
    {
        key(PK; "Header No.", "Line No.") { Clustered = true; }
    }
}

page 95987 "TP R6977 Lines"
{
    PageType = List;
    SourceTable = "TP R6977 Line";
    ApplicationArea = All;
    UsageCategory = None;
    AutoSplitKey = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
            }
        }
    }
}

page 95988 "TP R6977 Lines No Insert"
{
    PageType = List;
    SourceTable = "TP R6977 Line";
    ApplicationArea = All;
    UsageCategory = None;
    AutoSplitKey = true;
    InsertAllowed = false;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
            }
        }
    }
}

codeunit 95989 "TP R6977 Probe"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Visited: Text;
        UseView: Boolean;

    local procedure Initialize()
    var
        Line: Record "TP R6977 Line";
    begin
        Line.DeleteAll();
        InsertLine('H1', 10000, 'a');
        InsertLine('H1', 20000, 'b');
        InsertLine('H1', 30000, 'c');
        InsertLine('H2', 10000, 'z');
        Visited := '';
    end;

    local procedure InsertLine(HeaderNo: Code[20]; LineNo: Integer; Descr: Text[30])
    var
        Line: Record "TP R6977 Line";
    begin
        Line."Header No." := HeaderNo;
        Line."Line No." := LineNo;
        Line.Description := Descr;
        Line.Insert();
    end;

    local procedure WalkLines(var Lines: TestPage "TP R6977 Lines")
    var
        Counter: Integer;
    begin
        if Lines.First() then
            repeat
                Counter += 1;
                Visited += Lines."Line No.".Value() + '/' + Lines.Description.Value() + ';';
            until not Lines.Next();
        Visited += ' n=' + Format(Counter);
    end;

    local procedure WalkNoInsert(var Lines: TestPage "TP R6977 Lines No Insert")
    var
        Counter: Integer;
    begin
        if Lines.First() then
            repeat
                Counter += 1;
                Visited += Lines."Line No.".Value() + '/' + Lines.Description.Value() + ';';
            until not Lines.Next();
        Visited += ' n=' + Format(Counter);
    end;

    local procedure Check(Expected: Text)
    var
        Line: Record "TP R6977 Line";
    begin
        if Visited <> Expected then
            Error('visited %1', Visited);
        if Line.Count() <> 4 then
            Error('stored %1', Line.Count());
    end;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure TP6977_PageRun_View_FirstNextWalk()
    var
        Line: Record "TP R6977 Line";
    begin
        Initialize();
        UseView := true;
        Line.SetRange("Header No.", 'H1');
        Page.Run(Page::"TP R6977 Lines", Line);
        Check('10000/a;20000/b;30000/c; n=3');
    end;

    [Test]
    [HandlerFunctions('LinesHandler')]
    procedure TP6977_PageRun_NoView_FirstNextWalk()
    var
        Line: Record "TP R6977 Line";
    begin
        Initialize();
        UseView := false;
        Line.SetRange("Header No.", 'H1');
        Page.Run(Page::"TP R6977 Lines", Line);
        Check('10000/a;20000/b;30000/c;0/; n=4');
    end;

    [Test]
    [HandlerFunctions('NoInsertHandler')]
    procedure TP6977_PageRun_InsertNotAllowed_View_FirstNextWalk()
    var
        Line: Record "TP R6977 Line";
    begin
        Initialize();
        UseView := true;
        Line.SetRange("Header No.", 'H1');
        Page.Run(Page::"TP R6977 Lines No Insert", Line);
        Check('10000/a;20000/b;30000/c; n=3');
    end;

    [Test]
    procedure TP6977_OpenEditFiltered_View_FirstNextWalk()
    var
        Lines: TestPage "TP R6977 Lines";
    begin
        Initialize();
        Lines.OpenEdit();
        Lines.Filter.SetFilter("Header No.", 'H1');
        Lines.View().Invoke();
        WalkLines(Lines);
        Lines.Close();
        Check('10000/a;20000/b;30000/c; n=3');
    end;

    [Test]
    procedure TP6977_OpenViewFiltered_FirstNextWalk()
    var
        Lines: TestPage "TP R6977 Lines";
    begin
        Initialize();
        Lines.OpenView();
        Lines.Filter.SetFilter("Header No.", 'H1');
        WalkLines(Lines);
        Lines.Close();
        Check('10000/a;20000/b;30000/c; n=3');
    end;

    [PageHandler]
    procedure LinesHandler(var Lines: TestPage "TP R6977 Lines")
    begin
        if UseView then
            Lines.View().Invoke();
        WalkLines(Lines);
    end;

    [PageHandler]
    procedure NoInsertHandler(var Lines: TestPage "TP R6977 Lines No Insert")
    begin
        if UseView then
            Lines.View().Invoke();
        WalkNoInsert(Lines);
    end;
}
"##;

/// R6977ListViewNext.al:138, :156, :174, :191 and :210: each test names its expected walk.
#[test]
fn probe_6977_view_on_a_list_without_card_drops_the_new_row_line() {
    let o = outcomes_in(PROBE, "TP R6977 Probe");
    assert_eq!(
        format!("{o:?}"),
        r#"[("TP6977_PageRun_View_FirstNextWalk", Pass), ("TP6977_PageRun_NoView_FirstNextWalk", Pass), ("TP6977_PageRun_InsertNotAllowed_View_FirstNextWalk", Pass), ("TP6977_OpenEditFiltered_View_FirstNextWalk", Pass), ("TP6977_OpenViewFiltered_FirstNextWalk", Pass)]"#
    );
}
