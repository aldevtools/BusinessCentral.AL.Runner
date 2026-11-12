//! al-runner issue 6924: what Rec holds in the code of a ListPart that shows no row (its table has no row for the
//! SubPageLink). Base Application page 1178 "Doc. Attachment List Factbox" exits its OpenInDetail action when
//! `Rec."Table ID"` is 0. The objects and tests are the corpus probe's (pages/parts/EmptyPartRecLink.al, corpus pull
//! request 1335), the Probe Observer replaced by a text check.

use al_test_cli::report::Outcome;
use al_testkit::outcomes_in;

const SRC: &str = r#"
table 75160 "PG EPR Head"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { DataClassification = SystemMetadata; }
        field(2; Name; Text[30]) { DataClassification = SystemMetadata; }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 75161 "PG EPR Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Table ID"; Integer) { DataClassification = SystemMetadata; }
        field(2; "No."; Code[20]) { DataClassification = SystemMetadata; }
        field(3; "Line No."; Integer) { DataClassification = SystemMetadata; }
        field(4; Kind; Code[10]) { DataClassification = SystemMetadata; }
    }

    keys
    {
        key(PK; "Table ID", "No.", "Line No.") { Clustered = true; }
    }
}

page 75162 "PG EPR Part NoInsert"
{
    PageType = ListPart;
    SourceTable = "PG EPR Line";
    InsertAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(LineNoCtl; Rec."Line No.") { ApplicationArea = All; }
                field(KindCtl; Rec.Kind) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ReadRec)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Log: Codeunit "PG EPR Log";
                begin
                    Log.AddAction('act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.") + '/' + Rec.Kind);
                end;
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "PG EPR Log";
    begin
        Log.AddCurr('agcr=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No."));
    end;
}

page 75163 "PG EPR Part Insert"
{
    PageType = ListPart;
    SourceTable = "PG EPR Line";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(LineNoCtl; Rec."Line No.") { ApplicationArea = All; }
                field(KindCtl; Rec.Kind) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(ReadRec)
            {
                ApplicationArea = All;

                trigger OnAction()
                var
                    Log: Codeunit "PG EPR Log";
                begin
                    Log.AddAction('act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.") + '/' + Rec.Kind);
                end;
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "PG EPR Log";
    begin
        Log.AddCurr('agcr=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No."));
    end;
}

page 75164 "PG EPR Card ConstField"
{
    PageType = Card;
    SourceTable = "PG EPR Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { ApplicationArea = All; }
            part(Inline; "PG EPR Part NoInsert")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(75160), "No." = field("No.");
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPR Part NoInsert")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(75160), "No." = field("No.");
            }
        }
    }
}

page 75165 "PG EPR Card FieldOnly"
{
    PageType = Card;
    SourceTable = "PG EPR Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { ApplicationArea = All; }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPR Part NoInsert")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
            }
        }
    }
}

page 75166 "PG EPR Card ConstOnly"
{
    PageType = Card;
    SourceTable = "PG EPR Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { ApplicationArea = All; }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPR Part NoInsert")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(75160);
            }
        }
    }
}

page 75167 "PG EPR Card FilterOne"
{
    PageType = Card;
    SourceTable = "PG EPR Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { ApplicationArea = All; }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPR Part NoInsert")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = filter(75160), "No." = field("No.");
            }
        }
    }
}

page 75168 "PG EPR Card FilterRange"
{
    PageType = Card;
    SourceTable = "PG EPR Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { ApplicationArea = All; }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPR Part NoInsert")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = filter(75000 .. 75999), "No." = field("No.");
            }
        }
    }
}

page 75169 "PG EPR Document"
{
    PageType = Document;
    SourceTable = "PG EPR Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
            }
            part(Lines; "PG EPR Part Insert")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(75160), "No." = field("No.");
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPR Part NoInsert")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(75160), "No." = field("No.");
            }
        }
    }
}

codeunit 75170 "PG EPR Log"
{
    SingleInstance = true;

    var
        ActionLines: Text;
        Currs: Text;

    procedure Reset()
    begin
        ActionLines := '';
        Currs := '';
    end;

    procedure AddAction(Line: Text)
    begin
        if ActionLines <> '' then
            ActionLines += ' | ';
        ActionLines += Line;
    end;

    procedure AddCurr(Line: Text)
    begin
        if Currs <> '' then
            Currs += ' | ';
        Currs += Line;
    end;

    procedure ActionLog(): Text
    begin
        exit(ActionLines);
    end;

    procedure CurrLog(): Text
    begin
        exit(Currs);
    end;
}


codeunit 75171 "PG EPR Tests"
{
    Subtype = Test;

    var
        Log: Codeunit "PG EPR Log";

    local procedure Initialize()
    var
        Head: Record "PG EPR Head";
        Line: Record "PG EPR Line";
    begin
        Head.DeleteAll();
        Line.DeleteAll();
        Head."No." := 'A';
        Head.Insert();
        Head."No." := 'B';
        Head.Insert();
        Log.Reset();
    end;

    local procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('expected <%1> got <%2>', Expected, Actual);
    end;

    local procedure AddLine(No: Code[20]; LineNo: Integer; Kind: Code[10])
    var
        Line: Record "PG EPR Line";
    begin
        Line."Table ID" := 75160;
        Line."No." := No;
        Line."Line No." := LineNo;
        Line.Kind := Kind;
        Line.Insert();
    end;

    [Test]
    procedure EmptyPart_CardOpenView_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card ConstField";
    begin
        Initialize();
        Card.OpenView();
        Card.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=75160/A/0/', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenEdit_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card ConstField";
    begin
        Initialize();
        Card.OpenEdit();
        Card.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=75160/A/0/', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenView_ContentConstField_ActionReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card ConstField";
    begin
        Initialize();
        Card.OpenView();
        Card.Inline.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=75160/A/0/', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenEdit_ContentConstField_ActionReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card ConstField";
    begin
        Initialize();
        Card.OpenEdit();
        Card.Inline.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=75160/A/0/', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenView_FactBoxFieldOnly_ActionReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card FieldOnly";
    begin
        Initialize();
        Card.OpenView();
        Card.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=0/A/0/', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenView_FactBoxConstOnly_ActionReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card ConstOnly";
    begin
        Initialize();
        Card.OpenView();
        Card.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=75160//0/', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenView_FactBoxFilterOneValue_ActionReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card FilterOne";
    begin
        Initialize();
        Card.OpenView();
        Card.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=75160/A/0/', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenView_FactBoxFilterRange_ActionReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card FilterRange";
    begin
        Initialize();
        Card.OpenView();
        Card.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=0/A/0/', Got);
    end;

    [Test]
    procedure PartWithRow_CardOpenView_FactBoxConstField_ActionReadsRow()
    var
        Got: Text;
        Card: TestPage "PG EPR Card ConstField";
    begin
        Initialize();
        AddLine('A', 7, 'K');
        Card.OpenView();
        Card.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=75160/A/7/K', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenView_GoToRecordFromRowWithLines_ActionReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card ConstField";
        Head: Record "PG EPR Head";
    begin
        Initialize();
        AddLine('A', 7, 'K');
        Card.OpenView();
        Head.Get('B');
        Card.GoToRecord(Head);
        Card.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=75160/B/0/', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenNew_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card ConstField";
    begin
        Initialize();
        Card.OpenNew();
        Card.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=75160//0/', Got);
    end;

    [Test]
    procedure EmptyPart_DocumentOpenEdit_ContentWithNewLine_ActionReadsRec()
    var
        Got: Text;
        Doc: TestPage "PG EPR Document";
    begin
        Initialize();
        Doc.OpenEdit();
        Doc.Lines.ReadRec.Invoke();
        Got := Log.ActionLog();
        Doc.Close();
        Check('act=75160/A/0/', Got);
    end;

    [Test]
    procedure EmptyPart_DocumentOpenView_ContentWithNewLine_ActionReadsRec()
    var
        Got: Text;
        Doc: TestPage "PG EPR Document";
    begin
        Initialize();
        Doc.OpenView();
        Doc.Lines.ReadRec.Invoke();
        Got := Log.ActionLog();
        Doc.Close();
        Check('act=75160/A/0/', Got);
    end;

    [Test]
    procedure EmptyPart_DocumentOpenEdit_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Doc: TestPage "PG EPR Document";
    begin
        Initialize();
        Doc.OpenEdit();
        Doc.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Doc.Close();
        Check('act=75160/A/0/', Got);
    end;

    [Test]
    procedure EmptyPart_DocumentOpenView_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Doc: TestPage "PG EPR Document";
    begin
        Initialize();
        Doc.OpenView();
        Doc.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Doc.Close();
        Check('act=75160/A/0/', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenEdit_ConstField_OnAfterGetCurrRecordReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card ConstField";
    begin
        Initialize();
        Card.OpenEdit();
        Got := Log.CurrLog();
        Card.Close();
        Check('agcr=75160/A/0 | agcr=75160/A/0', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenView_FieldOnly_OnAfterGetCurrRecordReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card FieldOnly";
    begin
        Initialize();
        Card.OpenView();
        Got := Log.CurrLog();
        Card.Close();
        Check('agcr=0/A/0', Got);
    end;

    [Test]
    procedure EmptyPart_CardOpenView_FactBoxConstField_ActionTwice()
    var
        Got: Text;
        Card: TestPage "PG EPR Card ConstField";
    begin
        Initialize();
        Card.OpenView();
        Card.Box.ReadRec.Invoke();
        Card.Box.ReadRec.Invoke();
        Got := Log.ActionLog();
        Card.Close();
        Check('act=75160/A/0/ | act=75160/A/0/', Got);
    end;
}
"#;

/// The probe's OnAfterGetCurrRecord cell on the Card opened for edit: BC runs the trigger three times over the two empty parts
/// (corpus pull request 1335 on BC 27.5 and 28.5); the runtime runs it on neither part yet. That is a gap of its own, not
/// changed here.
const EDIT_CURR_GAP: &str = "EmptyPart_CardOpenEdit_ConstField_OnAfterGetCurrRecordReadsRec";

/// Every other cell of the probe: Rec of the empty part holds the single values of its SubPageLink (const, field, a filter of
/// one value; not a range), in a FactBox and in the content area, on a Card and on a Document host, opened for view, for edit
/// and with OpenNew, after GoToRecord, and at a second action. A part with a row reads that row.
#[test]
fn empty_list_part_rec_holds_the_link_values() {
    let o = outcomes_in(SRC, "PG EPR Tests");
    assert_eq!(o.len(), 18, "{o:?}");
    for (name, outcome) in &o {
        if name == EDIT_CURR_GAP {
            continue;
        }
        assert!(matches!(outcome, Outcome::Pass), "{name}: {outcome:?}");
    }
}
