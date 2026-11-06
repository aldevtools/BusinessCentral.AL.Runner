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

/// The objects of testpage/R6977ListAfterView.al with Assert.AreEqual replaced by Error on a difference.
const AFTER_VIEW: &str = r##"
table 96060 "TP R6977B Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Description; Text[30]) { }
    }

    keys
    {
        key(PK; "Line No.") { Clustered = true; }
    }
}

page 96061 "TP R6977B Lines"
{
    PageType = List;
    SourceTable = "TP R6977B Line";
    ApplicationArea = All;
    UsageCategory = None;

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

codeunit 96062 "TP R6977B After View"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Initialize()
    var
        Line: Record "TP R6977B Line";
    begin
        Line.DeleteAll();
        Line."Line No." := 10000;
        Line.Description := 'a';
        Line.Insert();
        Line."Line No." := 20000;
        Line.Description := 'b';
        Line.Insert();
    end;

    local procedure AreEqual(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('expected %1 got %2', Expected, Actual);
    end;

    local procedure Stored(LineNo: Integer): Text
    var
        Line: Record "TP R6977B Line";
    begin
        if not Line.Get(LineNo) then
            exit('<none>');
        exit(Line.Description);
    end;

    local procedure StoredCount(): Text
    var
        Line: Record "TP R6977B Line";
    begin
        exit(Format(Line.Count()));
    end;

    local procedure Observe(Name: Text; Shown: Text): Text
    begin
        exit(Name + '=' + Shown + ';');
    end;

    local procedure OpenOnFirst(var Lines: TestPage "TP R6977B Lines")
    begin
        Lines.OpenEdit();
        Lines.First();
    end;

    local procedure TypeDescription(var Lines: TestPage "TP R6977B Lines"; var Log: Text; Prefix: Text)
    begin
        ClearLastError();
        Lines.Description.SetValue('x');
        Log += Observe(Prefix + 'err', GetLastErrorText());
        Log += Observe(Prefix + 'valErrs', Format(Lines.ValidationErrorCount()));
        Log += Observe(Prefix + 'shown', Lines.Description.Value());
        Log += Observe(Prefix + 'now', Stored(10000));
    end;

    [Test]
    procedure TP6977B_View_FieldEditable()
    var
        Lines: TestPage "TP R6977B Lines";
        Log: Text;
    begin
        Initialize();
        OpenOnFirst(Lines);
        Log += Observe('before|page', Format(Lines.Editable()));
        Log += Observe('before|description', Format(Lines.Description.Editable()));
        Log += Observe('before|lineNo', Format(Lines."Line No.".Editable()));
        Lines.View().Invoke();
        Log += Observe('after|page', Format(Lines.Editable()));
        Log += Observe('after|description', Format(Lines.Description.Editable()));
        Log += Observe('after|lineNo', Format(Lines."Line No.".Editable()));
        Log += Observe('after|row', Lines."Line No.".Value());
        Lines.Close();
        AreEqual('before|page=Yes;before|description=Yes;before|lineNo=Yes;after|page=No;after|description=No;after|lineNo=No;after|row=10000;', Log);
    end;

    [Test]
    procedure TP6977B_View_SetValue()
    var
        Lines: TestPage "TP R6977B Lines";
        Log: Text;
    begin
        Initialize();
        OpenOnFirst(Lines);
        Lines.View().Invoke();
        TypeDescription(Lines, Log, 'after|');
        Lines.Close();
        Log += Observe('closed', Stored(10000));
        Log += Observe('stored', StoredCount());
        AreEqual('after|err=;after|valErrs=0;after|shown=x;after|now=a;closed=x;stored=2;', Log);
    end;

    [Test]
    procedure TP6977B_View_EditViewActions()
    var
        Lines: TestPage "TP R6977B Lines";
        Log: Text;
    begin
        Initialize();
        OpenOnFirst(Lines);
        Log += Observe('before|editVisible', Format(Lines.Edit().Visible()));
        Log += Observe('before|editEnabled', Format(Lines.Edit().Enabled()));
        Log += Observe('before|viewVisible', Format(Lines.View().Visible()));
        Log += Observe('before|viewEnabled', Format(Lines.View().Enabled()));
        Lines.View().Invoke();
        Log += Observe('after|editVisible', Format(Lines.Edit().Visible()));
        Log += Observe('after|editEnabled', Format(Lines.Edit().Enabled()));
        Log += Observe('after|viewVisible', Format(Lines.View().Visible()));
        Log += Observe('after|viewEnabled', Format(Lines.View().Enabled()));
        Lines.Close();
        AreEqual('before|editVisible=Yes;before|editEnabled=No;before|viewVisible=Yes;before|viewEnabled=Yes;after|editVisible=Yes;after|editEnabled=Yes;after|viewVisible=Yes;after|viewEnabled=No;', Log);
    end;

    [Test]
    procedure TP6977B_View_ThenEditInvoke()
    var
        Lines: TestPage "TP R6977B Lines";
        Log: Text;
    begin
        Initialize();
        OpenOnFirst(Lines);
        Lines.View().Invoke();
        Lines.Edit().Invoke();
        Log += Observe('back|page', Format(Lines.Editable()));
        Log += Observe('back|description', Format(Lines.Description.Editable()));
        Log += Observe('back|editEnabled', Format(Lines.Edit().Enabled()));
        Log += Observe('back|viewEnabled', Format(Lines.View().Enabled()));
        Log += Observe('back|row', Lines."Line No.".Value());
        TypeDescription(Lines, Log, 'back|');
        Lines.Close();
        Log += Observe('closed', Stored(10000));
        Log += Observe('stored', StoredCount());
        AreEqual('back|page=Yes;back|description=Yes;back|editEnabled=No;back|viewEnabled=Yes;back|row=10000;back|err=;back|valErrs=0;back|shown=x;back|now=a;closed=x;stored=2;', Log);
    end;
}
"##;

/// R6977ListAfterView.al:125, :142, :164, :187: Editable, SetValue and the Edit and View actions after View, and Edit back.
#[test]
fn probe_6977b_after_view_on_a_list_without_card() {
    let o = outcomes_in(AFTER_VIEW, "TP R6977B After View");
    assert_eq!(
        format!("{o:?}"),
        r#"[("TP6977B_View_FieldEditable", Pass), ("TP6977B_View_SetValue", Pass), ("TP6977B_View_EditViewActions", Pass), ("TP6977B_View_ThenEditInvoke", Pass)]"#
    );
}

/// What no probe pins after View and then Edit on such a list: whether the new-row line comes back for a walk, and a second
/// switch. Each is refused, never guessed.
const AFTER_EDIT_BACK: &str = r##"
codeunit 96063 "TP R6977B After Edit Back"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure OpenBack(var Lines: TestPage "TP R6977B Lines")
    var
        Line: Record "TP R6977B Line";
    begin
        Line.DeleteAll();
        Line."Line No." := 10000;
        Line.Insert();
        Lines.OpenEdit();
        Lines.First();
        Lines.View().Invoke();
        Lines.Edit().Invoke();
    end;

    [Test]
    procedure WalkNext()
    var
        Lines: TestPage "TP R6977B Lines";
    begin
        OpenBack(Lines);
        if Lines.Next() then;
    end;

    [Test]
    procedure ViewAgain()
    var
        Lines: TestPage "TP R6977B Lines";
    begin
        OpenBack(Lines);
        Lines.View().Invoke();
    end;

    [Test]
    procedure NewRow()
    var
        Lines: TestPage "TP R6977B Lines";
    begin
        OpenBack(Lines);
        Lines.New();
    end;
}
"##;

#[test]
fn after_view_then_edit_on_a_list_without_card_unpinned_steps_are_refused() {
    let src = format!("{AFTER_VIEW}\n{AFTER_EDIT_BACK}");
    let o = outcomes_in(&src, "TP R6977B After Edit Back");
    for (name, outcome) in &o {
        match outcome {
            al_testrun::Outcome::NotRun(why) => assert!(
                why.contains("after the built-in View and then Edit switched a list without a card"),
                "{name}: {why}"
            ),
            other => panic!("{name}: expected a refusal, got {other:?}"),
        }
    }
    assert_eq!(o.len(), 3, "{o:?}");
}
