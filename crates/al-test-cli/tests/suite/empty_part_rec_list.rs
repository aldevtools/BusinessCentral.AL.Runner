//! al-runner issue 6924 on a List host: what Rec holds in the action of a ListPart that shows no row, when the host is a
//! PageType = List page. The objects and tests are the corpus probe's (pages/parts/EmptyPartRecLinkList.al), each test raising
//! what it observed in place of the Probe Observer. The expected values are the probe's local observation on BC 28.5 (corpus
//! pull request 1339); the List-host cells run through the assumption `empty-part-rec-list-host` until it is pinned.

use al_builtins::assumptions::Assumption;
use al_testkit::{blocked, compile};
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;

/// Each test's outcome and the assumptions it went through.
fn run(src: &str) -> Vec<(String, Outcome, bool)> {
    let l = compile(src);
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
    let oi = l.program.objects.iter().position(|o| o.name == "PG EPL Tests").unwrap();
    let mut hits = Vec::new();
    let o = al_testrun::runtime::run_codeunit_full(
        &engine,
        &l,
        oi,
        None,
        TestIsolation::Codeunit,
        None,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut hits,
        None,
    );
    o.into_iter()
        .map(|(name, outcome)| {
            let flagged = hits
                .iter()
                .any(|(n, h): &(String, Vec<Assumption>)| *n == name && h.contains(&Assumption::EmptyPartRecListHost));
            (name, outcome, flagged)
        })
        .collect()
}

const SRC: &str = r#"
table 76840 "PG EPL Head"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { DataClassification = SystemMetadata; }
        field(2; "Ref Code"; Code[20]) { DataClassification = SystemMetadata; }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 76841 "PG EPL Line"
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

// The shape of page 1178: a ListPart without the new-row line.
page 76842 "PG EPL Part"
{
    PageType = ListPart;
    SourceTable = "PG EPL Line";
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
                    Log: Codeunit "PG EPL Log";
                begin
                    Log.AddAction('act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.") + '/' + Rec.Kind);
                end;
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "PG EPL Log";
    begin
        Log.AddCurr('agcr=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No."));
    end;
}

page 76843 "PG EPL List ConstField"
{
    PageType = List;
    SourceTable = "PG EPL Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
            part(Inline; "PG EPL Part")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(76840), "No." = field("No.");
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPL Part")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(76840), "No." = field("No.");
            }
        }
    }
}

// The link fields of the following Lists differ from the first one; each has only the FactBox.
page 76844 "PG EPL List FieldOnly"
{
    PageType = List;
    SourceTable = "PG EPL Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPL Part")
            {
                ApplicationArea = All;
                SubPageLink = "No." = field("No.");
            }
        }
    }
}

page 76845 "PG EPL List ConstOnly"
{
    PageType = List;
    SourceTable = "PG EPL Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPL Part")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(76840);
            }
        }
    }
}

page 76846 "PG EPL List FilterOne"
{
    PageType = List;
    SourceTable = "PG EPL Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPL Part")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = filter(76840), "No." = field("No.");
            }
        }
    }
}

page 76847 "PG EPL List FilterRange"
{
    PageType = List;
    SourceTable = "PG EPL Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPL Part")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = filter(76000 .. 76999), "No." = field("No.");
            }
        }
    }
}

// The reporter's link: the field of the host is outside its key.
page 76848 "PG EPL List RefCode"
{
    PageType = List;
    SourceTable = "PG EPL Head";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Heads)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
        area(FactBoxes)
        {
            part(Box; "PG EPL Part")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(76840), "No." = field("Ref Code");
            }
        }
    }
}

codeunit 76849 "PG EPL Log"
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

codeunit 76850 "PG EPL Tests"
{
    Subtype = Test;

    var
        Log: Codeunit "PG EPL Log";

    // Heads A and B (Ref Code RA and RB), no line.
    local procedure Initialize()
    var
        Head: Record "PG EPL Head";
        Line: Record "PG EPL Line";
    begin
        Head.DeleteAll();
        Line.DeleteAll();
        Head."No." := 'A';
        Head."Ref Code" := 'RA';
        Head.Insert();
        Head."No." := 'B';
        Head."Ref Code" := 'RB';
        Head.Insert();
        Log.Reset();
    end;

    local procedure AddLine(No: Code[20]; LineNo: Integer; Kind: Code[10])
    var
        Line: Record "PG EPL Line";
    begin
        Line."Table ID" := 76840;
        Line."No." := No;
        Line."Line No." := LineNo;
        Line.Kind := Kind;
        Line.Insert();
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_ContentConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Host.Inline.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToRecord_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToRecord_ContentConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Inline.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToRecordFromRowWithLines_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        AddLine('A', 7, 'K');
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_Next_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Got += 'next=' + Format(Host.Next()) + ';';
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_NextFromRowWithLines_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        AddLine('A', 7, 'K');
        Host.OpenEdit();
        Got += 'next=' + Format(Host.Next()) + ';';
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_ContentConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Host.Inline.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToRecord_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToRecord_ContentConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Inline.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToRecordFromRowWithLines_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        AddLine('A', 7, 'K');
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_Next_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Got += 'next=' + Format(Host.Next()) + ';';
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_NextFromRowWithLines_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        AddLine('A', 7, 'K');
        Host.OpenView();
        Got += 'next=' + Format(Host.Next()) + ';';
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_FactBoxFieldOnly_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FieldOnly";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToRecord_FactBoxFieldOnly_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FieldOnly";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_FactBoxConstOnly_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstOnly";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToRecord_FactBoxConstOnly_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstOnly";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_FactBoxFilterOneValue_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FilterOne";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToRecord_FactBoxFilterOneValue_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FilterOne";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_FactBoxFilterRange_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FilterRange";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToRecord_FactBoxFilterRange_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FilterRange";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_FactBoxFieldOnly_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FieldOnly";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToRecord_FactBoxFieldOnly_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FieldOnly";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_FactBoxConstOnly_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstOnly";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToRecord_FactBoxConstOnly_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstOnly";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_FactBoxFilterOneValue_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FilterOne";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToRecord_FactBoxFilterOneValue_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FilterOne";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_FactBoxFilterRange_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FilterRange";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToRecord_FactBoxFilterRange_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List FilterRange";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    // The reporter's shape: OpenEdit, GoToRecord, then the FactBox action.
    [Test]
    procedure EmptyPart_ListOpenEdit_GoToRecord_FactBoxRefCode_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List RefCode";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToRecord_FactBoxRefCode_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List RefCode";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_FactBoxRefCode_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List RefCode";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure PartWithRow_ListOpenEdit_FactBoxConstField_ActionReadsRow()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        AddLine('A', 7, 'K');
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure PartWithRow_ListOpenEdit_GoToRecord_FactBoxConstField_ActionReadsRow()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        AddLine('B', 8, 'L');
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    // The List shows no row.
    [Test]
    procedure EmptyPart_ListOpenEdit_NoHeads_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Head.DeleteAll();
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_NoHeads_FactBoxConstField_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Head.DeleteAll();
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToRecord_FactBoxConstField_ActionTwice()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Head.Get('B');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_ConstField_OnAfterGetCurrRecordReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Got += 'curr=' + Log.CurrLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToRecord_ConstField_OnAfterGetCurrRecordReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Log.Reset();
        Head.Get('B');
        Host.GoToRecord(Head);
        Got += 'curr=' + Log.CurrLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_ConstField_OnAfterGetCurrRecordReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Got += 'curr=' + Log.CurrLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToRecord_ConstField_OnAfterGetCurrRecordReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Log.Reset();
        Head.Get('B');
        Host.GoToRecord(Head);
        Got += 'curr=' + Log.CurrLog() + ';';
        Host.Close();
        Error(Got);
    end;
}
"#;

/// Each cell: what BC 28.5 raised (corpus pull request 1339, local observation), and whether the cell goes through the
/// assumption (a List host whose part takes the link values; the cells of a List that shows no row do not).
const CELLS: [(&str, &str, bool); 42] = [
    (
        "EmptyPart_ListOpenEdit_FactBoxConstField_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_ContentConstField_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_FactBoxConstField_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_ContentConstField_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecordFromRowWithLines_FactBox_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_Next_FactBoxConstField_ActionReadsRec",
        "next=Yes;action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_NextFromRowWithLines_FactBox_ActionReadsRec",
        "next=Yes;action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_FactBoxConstField_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_ContentConstField_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToRecord_FactBoxConstField_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToRecord_ContentConstField_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToRecordFromRowWithLines_FactBox_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_Next_FactBoxConstField_ActionReadsRec",
        "next=Yes;action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_NextFromRowWithLines_FactBox_ActionReadsRec",
        "next=Yes;action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_FactBoxFieldOnly_ActionReadsRec",
        "action=act=0/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_FactBoxFieldOnly_ActionReadsRec",
        "action=act=0/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_FactBoxConstOnly_ActionReadsRec",
        "action=act=76840//0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_FactBoxConstOnly_ActionReadsRec",
        "action=act=76840//0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_FactBoxFilterOneValue_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_FactBoxFilterOneValue_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_FactBoxFilterRange_ActionReadsRec",
        "action=act=0/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_FactBoxFilterRange_ActionReadsRec",
        "action=act=0/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_FactBoxFieldOnly_ActionReadsRec",
        "action=act=0/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToRecord_FactBoxFieldOnly_ActionReadsRec",
        "action=act=0/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_FactBoxConstOnly_ActionReadsRec",
        "action=act=76840//0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToRecord_FactBoxConstOnly_ActionReadsRec",
        "action=act=76840//0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_FactBoxFilterOneValue_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToRecord_FactBoxFilterOneValue_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_FactBoxFilterRange_ActionReadsRec",
        "action=act=0/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToRecord_FactBoxFilterRange_ActionReadsRec",
        "action=act=0/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_FactBoxRefCode_ActionReadsRec",
        "action=act=76840/RB/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToRecord_FactBoxRefCode_ActionReadsRec",
        "action=act=76840/RB/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_FactBoxRefCode_ActionReadsRec",
        "action=act=76840/RA/0/;",
        true,
    ),
    (
        "PartWithRow_ListOpenEdit_FactBoxConstField_ActionReadsRow",
        "action=act=76840/A/7/K;",
        false,
    ),
    (
        "PartWithRow_ListOpenEdit_GoToRecord_FactBoxConstField_ActionReadsRow",
        "action=act=76840/B/8/L;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_NoHeads_FactBoxConstField_ActionReadsRec",
        "action=act=76840//0/;",
        false,
    ),
    (
        "EmptyPart_ListOpenView_NoHeads_FactBoxConstField_ActionReadsRec",
        "action=act=0//0/;",
        false,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_FactBoxConstField_ActionTwice",
        "action=act=76840/B/0/ | act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_ConstField_OnAfterGetCurrRecordReadsRec",
        "curr=agcr=76840/A/0 | agcr=76840/A/0 | agcr=76840/A/0;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_ConstField_OnAfterGetCurrRecordReadsRec",
        "curr=agcr=76840/B/0 | agcr=76840/B/0 | agcr=76840/B/0;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_ConstField_OnAfterGetCurrRecordReadsRec",
        "curr=agcr=76840/A/0 | agcr=76840/A/0;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToRecord_ConstField_OnAfterGetCurrRecordReadsRec",
        "curr=agcr=76840/B/0 | agcr=76840/B/0;",
        true,
    ),
];

/// The cells the runtime does not answer as BC does, with what it raises: a List opened for edit that shows no row (BC stamps
/// the const of the link: `act=76840//0/`; the rule needs a host on a stored row), and OnAfterGetCurrRecord of the parts of a
/// List opened for edit (BC runs it three times; the runtime runs it on no part of a host opened for edit, the gap
/// `suite/empty_part_rec_link.rs` names for a Card).
const GAPS: [(&str, &str); 3] = [
    (
        "EmptyPart_ListOpenEdit_NoHeads_FactBoxConstField_ActionReadsRec",
        "action=act=0//0/;",
    ),
    (
        "EmptyPart_ListOpenEdit_ConstField_OnAfterGetCurrRecordReadsRec",
        "curr=;",
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_ConstField_OnAfterGetCurrRecordReadsRec",
        "curr=;",
    ),
];

/// Every cell raises BC's value, except the gaps, which keep the runtime's outcome from before this change.
#[test]
fn empty_list_part_rec_list_host_cells() {
    let o = run(SRC);
    assert_eq!(o.len(), CELLS.len(), "{o:?}");
    for (name, outcome, flagged) in &o {
        let (_, bc, flag) = CELLS
            .iter()
            .find(|(n, _, _)| n == name)
            .unwrap_or_else(|| panic!("no cell {name}"));
        let want = GAPS.iter().find(|(n, _)| n == name).map_or(*bc, |(_, v)| *v);
        assert!(
            matches!(outcome, Outcome::Fail(m) if m == want),
            "{name}: {outcome:?}, want {want}"
        );
        assert_eq!(flagged, flag, "{name}: flagged {flagged}");
    }
}

/// List-host shapes probe 1339 does not measure: a `field(filter(..))` link, a SubPageView, a part with an OnNewRecord, a part a
/// page extension adds, a host
/// moved by Previous, Last or GoToKey, and the new row of OpenNew. Each keeps the blank Rec the runtime gave before this change
/// (`act=0//0/`). The static shapes sit on Lists of their own and go through no assumption; the moves use a List whose part
/// takes the assumed path as the host opens. These are not BC's values; they guard against the rule reaching a shape nobody
/// measured.
const UNMEASURED_SRC: &str = r#"
table 76860 "PG EPLU Head" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 76861 "PG EPLU Line"
{
    fields { field(1; "Table ID"; Integer) { } field(2; "No."; Code[20]) { } field(3; "Line No."; Integer) { } }
    keys { key(PK; "Table ID", "No.", "Line No.") { Clustered = true; } }
}
codeunit 76862 "PG EPLU Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Reset() begin L := ''; end;
    procedure Add(Rec: Record "PG EPLU Line") begin L := 'act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No."); end;
    procedure Get(): Text begin exit(L); end;
}
page 76863 "PG EPLU Part"
{
    PageType = ListPart; SourceTable = "PG EPLU Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPLU Log"; begin Log.Add(Rec); end; } } }
}
page 76864 "PG EPLU Part NewRec"
{
    PageType = ListPart; SourceTable = "PG EPLU Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPLU Log"; begin Log.Add(Rec); end; } } }
    trigger OnNewRecord(BelowxRec: Boolean) begin Rec."Line No." := 9; end;
}
page 76865 "PG EPLU List"
{
    PageType = List; SourceTable = "PG EPLU Head";
    layout
    {
        area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } }
        area(FactBoxes) { part(Box; "PG EPLU Part") { SubPageLink = "Table ID" = const(76860), "No." = field("No."); } }
    }
}
page 76866 "PG EPLU List FieldFilter"
{
    PageType = List; SourceTable = "PG EPLU Head";
    layout
    {
        area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } }
        area(FactBoxes) { part(Box; "PG EPLU Part") { SubPageLink = "Table ID" = const(76860), "No." = field(filter("No.")); } }
    }
}
page 76867 "PG EPLU List View"
{
    PageType = List; SourceTable = "PG EPLU Head";
    layout
    {
        area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } }
        area(FactBoxes) { part(Box; "PG EPLU Part") { SubPageLink = "Table ID" = const(76860), "No." = field("No."); SubPageView = sorting("Table ID", "No.", "Line No.") order(descending); } }
    }
}
page 76868 "PG EPLU List NewRec"
{
    PageType = List; SourceTable = "PG EPLU Head";
    layout
    {
        area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } }
        area(FactBoxes) { part(Box; "PG EPLU Part NewRec") { SubPageLink = "Table ID" = const(76860), "No." = field("No."); } }
    }
}
page 76870 "PG EPLU List Bare"
{
    PageType = List; SourceTable = "PG EPLU Head";
    layout { area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } } }
}
pageextension 76871 "PG EPLU List Bare Ext" extends "PG EPLU List Bare"
{
    layout { addfirst(FactBoxes) { part(ExtBox; "PG EPLU Part") { SubPageLink = "Table ID" = const(76860), "No." = field("No."); } } }
}
codeunit 76869 "PG EPL Tests"
{
    Subtype = Test;
    var Log: Codeunit "PG EPLU Log";
    local procedure Init() var H: Record "PG EPLU Head"; L: Record "PG EPLU Line"; begin H.DeleteAll(); L.DeleteAll(); H."No." := 'A'; H.Insert(); H."No." := 'B'; H.Insert(); H."No." := 'C'; H.Insert(); Log.Reset(); end;
    [Test] procedure Static_FieldFilter() var P: TestPage "PG EPLU List FieldFilter"; H: Record "PG EPLU Head"; begin Init(); P.OpenEdit(); H.Get('B'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Static_SubPageView() var P: TestPage "PG EPLU List View"; H: Record "PG EPLU Head"; begin Init(); P.OpenEdit(); H.Get('B'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Static_OnNewRecord() var P: TestPage "PG EPLU List NewRec"; H: Record "PG EPLU Head"; begin Init(); P.OpenEdit(); H.Get('B'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Static_ExtensionPart() var P: TestPage "PG EPLU List Bare"; H: Record "PG EPLU Head"; begin Init(); P.OpenEdit(); H.Get('B'); P.GoToRecord(H); P.ExtBox.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Static_OpenNew() var P: TestPage "PG EPLU List"; begin Init(); P.OpenNew(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Move_Previous() var P: TestPage "PG EPLU List"; H: Record "PG EPLU Head"; begin Init(); P.OpenEdit(); P.Last(); P.Previous(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Move_Last() var P: TestPage "PG EPLU List"; begin Init(); P.OpenEdit(); P.Last(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Move_GoToKey() var P: TestPage "PG EPLU List"; begin Init(); P.OpenView(); P.GoToKey('B'); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
}
"#;

#[test]
fn empty_list_part_rec_list_host_unmeasured_shapes_stay_blank() {
    let o = run(UNMEASURED_SRC);
    assert_eq!(o.len(), 8, "{o:?}");
    for (name, outcome, flagged) in &o {
        assert!(
            matches!(outcome, Outcome::Fail(m) if m == "act=0//0"),
            "{name}: {outcome:?}"
        );
        assert_eq!(*flagged, name.starts_with("Move_"), "{name}: flagged {flagged}");
    }
}
