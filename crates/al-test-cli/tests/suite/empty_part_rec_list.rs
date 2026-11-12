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

// The triggers and properties of page 1178 beside the shape of "PG EPL Part": DeleteAllowed, DelayedInsert, OnInit, OnDeleteRecord,
// a field with OnDrillDown and Editable = false.
page 76851 "PG EPL Part Doc"
{
    PageType = ListPart;
    SourceTable = "PG EPL Line";
    DeleteAllowed = true;
    DelayedInsert = true;
    InsertAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(LineNoCtl; Rec."Line No.")
                {
                    ApplicationArea = All;

                    trigger OnDrillDown()
                    begin
                        DrillDowns += 1;
                    end;
                }
                field(KindCtl; Rec.Kind)
                {
                    ApplicationArea = All;
                    Editable = false;
                }
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

    trigger OnInit()
    begin
        DrillDowns := 0;
    end;

    trigger OnDeleteRecord(): Boolean
    begin
        exit(true);
    end;

    trigger OnAfterGetCurrRecord()
    var
        Log: Codeunit "PG EPL Log";
    begin
        Log.AddCurr('agcr=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No."));
    end;

    var
        DrillDowns: Integer;
}

page 76852 "PG EPL List Doc"
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
            part(Box; "PG EPL Part Doc")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(76840), "No." = field("No.");
            }
        }
    }
}

// A host with triggers that do not touch the part.
page 76853 "PG EPL List Triggers"
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
                SubPageLink = "Table ID" = const(76840), "No." = field("No.");
            }
        }
    }

    trigger OnOpenPage()
    begin
        Opens += 1;
    end;

    trigger OnAfterGetRecord()
    begin
        Reads += 1;
    end;

    trigger OnAfterGetCurrRecord()
    begin
        CurrReads += 1;
    end;

    var
        Opens: Integer;
        Reads: Integer;
        CurrReads: Integer;
}

// Page 1178's host form: the FactBox has UpdatePropagation = Both.
page 76854 "PG EPL List DocBoth"
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
            part(Box; "PG EPL Part Doc")
            {
                ApplicationArea = All;
                SubPageLink = "Table ID" = const(Database::"PG EPL Head"), "No." = field("No.");
                UpdatePropagation = Both;
            }
        }
    }
}

// The reporter's host form (issue 6924): a List page with a Caption whose FactBox part sets ApplicationArea, Caption,
// UpdatePropagation = Both and links "No." to a host field outside the host key, like the Base Application hosts of page 1178.
page 76855 "PG EPL List Rep"
{
    PageType = List;
    SourceTable = "PG EPL Head";
    Caption = 'EPL Reporter List';
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
            part(Box; "PG EPL Part Doc")
            {
                ApplicationArea = All;
                Caption = 'Attachments';
                SubPageLink = "Table ID" = const(Database::"PG EPL Head"), "No." = field("Ref Code");
                UpdatePropagation = Both;
            }
        }
    }
}

// As "PG EPL List Rep", but the part control has only Caption besides ApplicationArea and SubPageLink.
page 76856 "PG EPL List RepCap"
{
    PageType = List;
    SourceTable = "PG EPL Head";
    Caption = 'EPL Reporter List Caption';
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
            part(Box; "PG EPL Part Doc")
            {
                ApplicationArea = All;
                Caption = 'Attachments';
                SubPageLink = "Table ID" = const(Database::"PG EPL Head"), "No." = field("Ref Code");
            }
        }
    }
}

// As "PG EPL List Rep", on a host with Editable = false.
page 76857 "PG EPL List RepRO"
{
    PageType = List;
    SourceTable = "PG EPL Head";
    Caption = 'EPL Reporter List Read Only';
    Editable = false;
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
            part(Box; "PG EPL Part Doc")
            {
                ApplicationArea = All;
                Caption = 'Attachments';
                SubPageLink = "Table ID" = const(Database::"PG EPL Head"), "No." = field("Ref Code");
                UpdatePropagation = Both;
            }
        }
    }
}

// The card page of "PG EPL List RepCard".
page 76858 "PG EPL Head Card"
{
    PageType = Card;
    SourceTable = "PG EPL Head";
    Caption = 'EPL Head Card';
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            group(General)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RefCtl; Rec."Ref Code") { ApplicationArea = All; }
            }
        }
    }
}

// As "PG EPL List Rep", on a host with CardPageId.
page 76859 "PG EPL List RepCard"
{
    PageType = List;
    SourceTable = "PG EPL Head";
    Caption = 'EPL Reporter List Card';
    CardPageId = "PG EPL Head Card";
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
            part(Box; "PG EPL Part Doc")
            {
                ApplicationArea = All;
                Caption = 'Attachments';
                SubPageLink = "Table ID" = const(Database::"PG EPL Head"), "No." = field("Ref Code");
                UpdatePropagation = Both;
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

    // Head A only (Ref Code RA), no line: a GoToRecord to A stays on the row the host stands on.
    local procedure InitializeOneHead()
    var
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Head.Get('B');
        Head.Delete();
    end;

    // Heads A, B and C (Ref Code RA, RB and RC), no line: C is the last row of the host.
    local procedure InitializeThree()
    var
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Head."No." := 'C';
        Head."Ref Code" := 'RC';
        Head.Insert();
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

    [Test]
    procedure EmptyPart_ListOpenEdit_OneHead_GoToSameRow_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        InitializeOneHead();
        Host.OpenEdit();
        Head.Get('A');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();

        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_OneHead_GoToSameRow_Content_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        InitializeOneHead();
        Host.OpenEdit();
        Head.Get('A');
        Host.GoToRecord(Head);
        Host.Inline.ReadRec.Invoke();

        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_OneHead_GoToSameRow_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        InitializeOneHead();
        Host.OpenView();
        Head.Get('A');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();

        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_OneHead_GoToSameRow_Content_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        InitializeOneHead();
        Host.OpenView();
        Head.Get('A');
        Host.GoToRecord(Head);
        Host.Inline.ReadRec.Invoke();

        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToFirstRow_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Head.Get('A');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();

        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToFirstRow_Content_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        Head.Get('A');
        Host.GoToRecord(Head);
        Host.Inline.ReadRec.Invoke();

        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToFirstRow_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenView();
        Head.Get('A');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();

        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_OneHead_GoToSameRow_OnAfterGetCurrRecordReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        InitializeOneHead();
        Host.OpenEdit();
        Log.Reset();
        Head.Get('A');
        Host.GoToRecord(Head);

        Got += 'curr=' + Log.CurrLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_OneHead_GoToSameRow_OnAfterGetCurrRecordReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List ConstField";
        Head: Record "PG EPL Head";
    begin
        InitializeOneHead();
        Host.OpenView();
        Log.Reset();
        Head.Get('A');
        Host.GoToRecord(Head);

        Got += 'curr=' + Log.CurrLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_PartDoc_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Doc";
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
    procedure EmptyPart_ListOpenView_PartDoc_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Doc";
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
    procedure EmptyPart_ListOpenEdit_GoToRecord_PartDoc_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Doc";
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
    procedure EmptyPart_ListOpenEdit_OneHead_GoToSameRow_PartDoc_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Doc";
        Head: Record "PG EPL Head";
    begin
        InitializeOneHead();
        Host.OpenEdit();
        Head.Get('A');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();

        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_PartDoc_OnAfterGetCurrRecordReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Doc";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();

        Got += 'curr=' + Log.CurrLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_HostTriggers_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Triggers";
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
    procedure EmptyPart_ListOpenView_HostTriggers_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Triggers";
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
    procedure EmptyPart_ListOpenEdit_GoToRecord_HostTriggers_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Triggers";
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
    procedure EmptyPart_ListOpenEdit_Next_HostTriggers_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Triggers";
        Head: Record "PG EPL Head";
    begin
        Initialize();
        Host.OpenEdit();
        if Host.Next() then;
        Host.Box.ReadRec.Invoke();

        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_OneHead_GoToSameRow_HostTriggers_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Triggers";
        Head: Record "PG EPL Head";
    begin
        InitializeOneHead();
        Host.OpenEdit();
        Head.Get('A');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();

        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_PartDocBoth_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List DocBoth";
    begin
        Initialize();
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToRecord_PartDocBoth_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List DocBoth";
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
    procedure EmptyPart_ListOpenEdit_OneHead_GoToSameRow_PartDocBoth_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List DocBoth";
        Head: Record "PG EPL Head";
    begin
        InitializeOneHead();
        Host.OpenEdit();
        Head.Get('A');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_OneHead_GoToSameRow_PartDocBoth_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List DocBoth";
        Head: Record "PG EPL Head";
    begin
        InitializeOneHead();
        Host.OpenView();
        Head.Get('A');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    // The reporter's host form (issue 6924): heads A, B and C, every part empty; GoToRecord moves to the last row C.
    [Test]
    procedure EmptyPart_ListOpenEdit_Reporter_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Rep";
        Head: Record "PG EPL Head";
    begin
        InitializeThree();
        Host.OpenEdit();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_Reporter_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Rep";
        Head: Record "PG EPL Head";
    begin
        InitializeThree();
        Host.OpenView();
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToLastRow_Reporter_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Rep";
        Head: Record "PG EPL Head";
    begin
        InitializeThree();
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToLastRow_Reporter_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List Rep";
        Head: Record "PG EPL Head";
    begin
        InitializeThree();
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToLastRow_ReporterCaptionOnly_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List RepCap";
        Head: Record "PG EPL Head";
    begin
        InitializeThree();
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToLastRow_ReporterCaptionOnly_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List RepCap";
        Head: Record "PG EPL Head";
    begin
        InitializeThree();
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToLastRow_ReporterNotEditable_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List RepRO";
        Head: Record "PG EPL Head";
    begin
        InitializeThree();
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToLastRow_ReporterNotEditable_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List RepRO";
        Head: Record "PG EPL Head";
    begin
        InitializeThree();
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenEdit_GoToLastRow_ReporterCardPage_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List RepCard";
        Head: Record "PG EPL Head";
    begin
        InitializeThree();
        Host.OpenEdit();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;

    [Test]
    procedure EmptyPart_ListOpenView_GoToLastRow_ReporterCardPage_FactBox_ActionReadsRec()
    var
        Got: Text;
        Host: TestPage "PG EPL List RepCard";
        Head: Record "PG EPL Head";
    begin
        InitializeThree();
        Host.OpenView();
        Head.Get('C');
        Host.GoToRecord(Head);
        Host.Box.ReadRec.Invoke();
        Got += 'action=' + Log.ActionLog() + ';';
        Host.Close();
        Error(Got);
    end;
}
"#;

/// Each cell: what BC 28.5 raised (corpus pull request 1339, local observation), and whether the cell goes through the
/// assumption (a List host whose part takes the link values; the cells of a List that shows no row do not).
const CELLS: &[(&str, &str, bool)] = &[
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
    (
        "EmptyPart_ListOpenEdit_OneHead_GoToSameRow_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_OneHead_GoToSameRow_Content_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_OneHead_GoToSameRow_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_OneHead_GoToSameRow_Content_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToFirstRow_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToFirstRow_Content_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToFirstRow_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_OneHead_GoToSameRow_OnAfterGetCurrRecordReadsRec",
        "curr=;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_OneHead_GoToSameRow_OnAfterGetCurrRecordReadsRec",
        "curr=;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_PartDoc_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_PartDoc_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_PartDoc_FactBox_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_OneHead_GoToSameRow_PartDoc_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_PartDoc_OnAfterGetCurrRecordReadsRec",
        "curr=agcr=76840/A/0;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_HostTriggers_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_HostTriggers_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_HostTriggers_FactBox_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_Next_HostTriggers_FactBox_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_OneHead_GoToSameRow_HostTriggers_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    // The cells of page 76854 "PG EPL List DocBoth" and of the reporter's host form (pages 76855-76859), measured on BC 27.5,
    // 28.5 and 29.0 alike (corpus pull request 1339, commit 831ae544, EmptyPartRecLinkList.al:1800-2031).
    (
        "EmptyPart_ListOpenEdit_PartDocBoth_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_PartDocBoth_FactBox_ActionReadsRec",
        "action=act=76840/B/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_OneHead_GoToSameRow_PartDocBoth_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_OneHead_GoToSameRow_PartDocBoth_FactBox_ActionReadsRec",
        "action=act=76840/A/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_Reporter_FactBox_ActionReadsRec",
        "action=act=76840/RA/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_Reporter_FactBox_ActionReadsRec",
        "action=act=76840/RA/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToLastRow_Reporter_FactBox_ActionReadsRec",
        "action=act=76840/RC/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToLastRow_Reporter_FactBox_ActionReadsRec",
        "action=act=76840/RC/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToLastRow_ReporterCaptionOnly_FactBox_ActionReadsRec",
        "action=act=76840/RC/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToLastRow_ReporterCaptionOnly_FactBox_ActionReadsRec",
        "action=act=76840/RC/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToLastRow_ReporterNotEditable_FactBox_ActionReadsRec",
        "action=act=76840/RC/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToLastRow_ReporterNotEditable_FactBox_ActionReadsRec",
        "action=act=76840/RC/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenEdit_GoToLastRow_ReporterCardPage_FactBox_ActionReadsRec",
        "action=act=76840/RC/0/;",
        true,
    ),
    (
        "EmptyPart_ListOpenView_GoToLastRow_ReporterCardPage_FactBox_ActionReadsRec",
        "action=act=76840/RC/0/;",
        true,
    ),
];

/// The cell whose read of the part's Rec is refused: a List opened for edit that shows no row (BC stamps the const of the link,
/// `act=76840//0/`; the rule needs a host on a stored row, and the runtime gave the blank Rec before, `act=0//0/`).
const REFUSED: &[&str] = &["EmptyPart_ListOpenEdit_NoHeads_FactBoxConstField_ActionReadsRec"];

/// The cells the runtime does not answer as BC does, with what it raises: OnAfterGetCurrRecord of the parts of a
/// List opened for edit (BC runs it three times on page 76842 and once on page 76851; the runtime runs it on no part of a host
/// opened for edit, the gap `suite/empty_part_rec_link.rs` names for a Card), and OnAfterGetCurrRecord after a GoToRecord that
/// stays on the row of a host opened for view (BC runs it on no part; the runtime runs it twice, as it did before this change,
/// and the part's Rec holds the link values there).
const GAPS: &[(&str, &str)] = &[
    (
        "EmptyPart_ListOpenEdit_ConstField_OnAfterGetCurrRecordReadsRec",
        "curr=;",
    ),
    (
        "EmptyPart_ListOpenEdit_GoToRecord_ConstField_OnAfterGetCurrRecordReadsRec",
        "curr=;",
    ),
    ("EmptyPart_ListOpenEdit_PartDoc_OnAfterGetCurrRecordReadsRec", "curr=;"),
    (
        "EmptyPart_ListOpenView_OneHead_GoToSameRow_OnAfterGetCurrRecordReadsRec",
        "curr=agcr=76840/A/0 | agcr=76840/A/0;",
    ),
];

/// Every cell raises BC's value (`raised`), except the gaps, which keep the runtime's outcome from before this change, and the
/// refused cells, whose read of the part's Rec is refused.
#[test]
fn empty_list_part_rec_list_host_cells() {
    let o = run(SRC);
    assert_eq!(o.len(), CELLS.len(), "{o:?}");
    let mut wrong = Vec::new();
    for (name, outcome, flagged) in &o {
        if REFUSED.contains(&name.as_str()) {
            if !matches!(outcome, Outcome::NotRun(m) if m.contains("a read of Rec of a part that shows no row"))
                || *flagged
            {
                wrong.push((name, outcome, flagged));
            }
            continue;
        }
        let (_, bc, flag) = CELLS
            .iter()
            .find(|(n, _, _)| n == name)
            .unwrap_or_else(|| panic!("no cell {name}"));
        let want = GAPS.iter().find(|(n, _)| n == name).map_or(*bc, |(_, v)| *v);
        if !raised(outcome, want) || flagged != flag {
            wrong.push((name, outcome, flagged));
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The test raised `want`: as its failure, or as the failure the runner does not trust because an earlier test of the codeunit
/// was refused (the log is a SingleInstance codeunit's; the refused cell comes early in the probe's order).
fn raised(outcome: &Outcome, want: &str) -> bool {
    match outcome {
        Outcome::Fail(m) => m == want,
        Outcome::NotRun(m) => m.ends_with(&format!("this failure is not trusted: {want}")),
        Outcome::Pass => false,
    }
}

/// List-host shapes probe 1339 does not measure: a `field(filter(..))` link, a SubPageView, a part with an OnNewRecord, a part a
/// page extension adds, a host
/// moved by Previous, Last or GoToKey, and the new row of OpenNew. The runtime gave the blank Rec there before (`act=0//0`), a
/// guess (al-runner issue 6924); now the read of the part's Rec is refused. The static shapes sit on Lists of their own; the
/// moves use a List whose part takes the assumed path as the host opens, and the read after the move is refused.
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
    trigger OnAfterGetCurrRecord() begin end;
}
page 76864 "PG EPLU Part NewRec"
{
    PageType = ListPart; SourceTable = "PG EPLU Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPLU Log"; begin Log.Add(Rec); end; } } }
    trigger OnNewRecord(BelowxRec: Boolean) begin Rec."Line No." := 9; end;
    trigger OnAfterGetCurrRecord() begin end;
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
fn empty_list_part_rec_list_host_unmeasured_shapes_refuse() {
    let o = run(UNMEASURED_SRC);
    assert_eq!(o.len(), 8, "{o:?}");
    let wrong: Vec<_> = o.iter().filter(|(_, outcome, _)| !refused(outcome)).collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The test was refused at a read of the empty part's Rec, with the fact that left the measured cells.
fn refused(outcome: &Outcome) -> bool {
    matches!(outcome, Outcome::NotRun(m) if m.starts_with("a read of Rec of a part that shows no row"))
}

/// Neighbors of the List-host cells that differ from a probe cell in one fact the probe held fixed (review of corpus pull
/// request 1339): the part's page extension with OnNewRecord, a subscriber of the part, the part's OnFindRecord, a temporary
/// part, a List page as part, a part with Editable = false or with the new-row line, the part control with Editable = false, a
/// part that is the Provider of another, a host with Editable = false, a SourceTableView, a page extension or an
/// OnAfterGetCurrRecord of its own, a host run by Page.Run or Page.RunModal, a const on the last key field and a Code const; and
/// a Card host opened for view moved by a GoToRecord that stays on its row (no List rule; the part keeps the values the open
/// stamped, `act=76880/A/0` on main 57481ae65 too).
/// Main gave each the blank Rec (`act=0//0`, a guess); each now refuses the read of the part's Rec and goes through no
/// assumption. The Card keeps main's result.
const NEIGHBOR_SRC: &str = r#"
table 76880 "PG EPLN Head" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 76881 "PG EPLN Line"
{
    fields { field(1; "Table ID"; Integer) { } field(2; "No."; Code[20]) { } field(3; "Line No."; Integer) { } }
    keys { key(PK; "Table ID", "No.", "Line No.") { Clustered = true; } }
}
codeunit 76882 "PG EPLN Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Reset() begin L := ''; end;
    procedure Add(Rec: Record "PG EPLN Line") begin L := 'act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No."); end;
    procedure Get(): Text begin exit(L); end;
    [EventSubscriber(ObjectType::Page, Page::"PG EPLN Part Sub", 'OnAfterGetCurrRecordEvent', '', false, false)]
    local procedure OnCurr(var Rec: Record "PG EPLN Line") begin end;
}
page 76883 "PG EPLN Part"
{
    PageType = ListPart; SourceTable = "PG EPLN Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPLN Log"; begin Log.Add(Rec); end; } } }
    trigger OnAfterGetCurrRecord() begin end;
}
page 76884 "PG EPLN Part Ext"
{
    PageType = ListPart; SourceTable = "PG EPLN Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPLN Log"; begin Log.Add(Rec); end; } } }
    trigger OnAfterGetCurrRecord() begin end;
}
pageextension 76885 "PG EPLN Part Ext New" extends "PG EPLN Part Ext"
{
    trigger OnNewRecord(BelowxRec: Boolean) begin Rec."Line No." := 9; end;
}
page 76886 "PG EPLN Part Sub"
{
    PageType = ListPart; SourceTable = "PG EPLN Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPLN Log"; begin Log.Add(Rec); end; } } }
    trigger OnAfterGetCurrRecord() begin end;
}
page 76887 "PG EPLN Part Find"
{
    PageType = ListPart; SourceTable = "PG EPLN Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPLN Log"; begin Log.Add(Rec); end; } } }
    trigger OnAfterGetCurrRecord() begin end;
    trigger OnFindRecord(Which: Text): Boolean begin exit(Rec.Find(Which)); end;
}
page 76888 "PG EPLN Part Temp"
{
    PageType = ListPart; SourceTable = "PG EPLN Line"; InsertAllowed = false; SourceTableTemporary = true;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPLN Log"; begin Log.Add(Rec); end; } } }
    trigger OnAfterGetCurrRecord() begin end;
}
page 76889 "PG EPLN Part AsList"
{
    PageType = List; SourceTable = "PG EPLN Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPLN Log"; begin Log.Add(Rec); end; } } }
    trigger OnAfterGetCurrRecord() begin end;
}
page 76890 "PG EPLN Part NoEdit"
{
    PageType = ListPart; SourceTable = "PG EPLN Line"; InsertAllowed = false; Editable = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPLN Log"; begin Log.Add(Rec); end; } } }
    trigger OnAfterGetCurrRecord() begin end;
}
page 76891 "PG EPLN Part Ins"
{
    PageType = ListPart; SourceTable = "PG EPLN Line";
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPLN Log"; begin Log.Add(Rec); end; } } }
    trigger OnAfterGetCurrRecord() begin end;
}
page 76892 "PG EPLN Parts"
{
    PageType = List; SourceTable = "PG EPLN Head";
    layout
    {
        area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } }
        area(FactBoxes)
        {
            part(PExt; "PG EPLN Part Ext") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); }
            part(PSub; "PG EPLN Part Sub") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); }
            part(PFind; "PG EPLN Part Find") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); }
            part(PTemp; "PG EPLN Part Temp") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); }
            part(PAsList; "PG EPLN Part AsList") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); }
            part(PNoEdit; "PG EPLN Part NoEdit") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); }
            part(PIns; "PG EPLN Part Ins") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); }
            part(CtlNoEdit; "PG EPLN Part") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); Editable = false; }
            part(AllKey; "PG EPLN Part") { SubPageLink = "Table ID" = const(76880), "No." = field("No."), "Line No." = const(5); }
            part(CodeConst; "PG EPLN Part") { SubPageLink = "No." = const('A'); }
        }
    }
}
page 76893 "PG EPLN Host NoEdit"
{
    PageType = List; SourceTable = "PG EPLN Head"; Editable = false;
    layout
    {
        area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } }
        area(FactBoxes) { part(Box; "PG EPLN Part") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); } }
    }
}
page 76894 "PG EPLN Host View"
{
    PageType = List; SourceTable = "PG EPLN Head"; SourceTableView = sorting("No.") order(descending);
    layout
    {
        area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } }
        area(FactBoxes) { part(Box; "PG EPLN Part") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); } }
    }
}
page 76895 "PG EPLN Host Curr"
{
    PageType = List; SourceTable = "PG EPLN Head";
    layout
    {
        area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } }
        area(FactBoxes) { part(Box; "PG EPLN Part") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); } }
    }
    trigger OnAfterGetCurrRecord() begin end;
}
page 76896 "PG EPLN Host"
{
    PageType = List; SourceTable = "PG EPLN Head";
    layout
    {
        area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } }
        area(FactBoxes) { part(Box; "PG EPLN Part") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); } }
    }
}
page 76897 "PG EPLN Host Ext"
{
    PageType = List; SourceTable = "PG EPLN Head";
    layout
    {
        area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } }
        area(FactBoxes) { part(Box; "PG EPLN Part") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); } }
    }
}
page 76879 "PG EPLN Host Prov"
{
    PageType = List; SourceTable = "PG EPLN Head";
    layout
    {
        area(Content)
        {
            repeater(R) { field(NoCtl; Rec."No.") { } }
            part(Prov; "PG EPLN Part") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); }
            part(Det; "PG EPLN Part") { Provider = Prov; SubPageLink = "No." = field("No."); }
        }
    }
}
page 76878 "PG EPLN Card"
{
    PageType = Card; SourceTable = "PG EPLN Head";
    layout
    {
        area(Content)
        {
            field(NoCtl; Rec."No.") { }
            part(Box; "PG EPLN Part") { SubPageLink = "Table ID" = const(76880), "No." = field("No."); }
        }
    }
}
pageextension 76898 "PG EPLN Host Ext Trig" extends "PG EPLN Host Ext"
{
    trigger OnOpenPage() begin end;
}
codeunit 76899 "PG EPL Tests"
{
    Subtype = Test;
    var Log: Codeunit "PG EPLN Log";
    local procedure Init() var H: Record "PG EPLN Head"; L: Record "PG EPLN Line"; begin H.DeleteAll(); L.DeleteAll(); H."No." := 'A'; H.Insert(); H."No." := 'B'; H.Insert(); Log.Reset(); end;
    [Test] procedure PartExtNewRec() var P: TestPage "PG EPLN Parts"; begin Init(); P.OpenEdit(); P.PExt.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure PartSubscriber() var P: TestPage "PG EPLN Parts"; begin Init(); P.OpenEdit(); P.PSub.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure PartFindRecord() var P: TestPage "PG EPLN Parts"; begin Init(); P.OpenEdit(); P.PFind.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure PartTemporary() var P: TestPage "PG EPLN Parts"; begin Init(); P.OpenEdit(); P.PTemp.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure PartPageTypeList() var P: TestPage "PG EPLN Parts"; begin Init(); P.OpenEdit(); P.PAsList.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure PartNotEditable() var P: TestPage "PG EPLN Parts"; begin Init(); P.OpenEdit(); P.PNoEdit.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure PartNewLineOnView() var P: TestPage "PG EPLN Parts"; begin Init(); P.OpenView(); P.PIns.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure PartControlNotEditable() var P: TestPage "PG EPLN Parts"; begin Init(); P.OpenEdit(); P.CtlNoEdit.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure LinkConstOnLastKeyField() var P: TestPage "PG EPLN Parts"; begin Init(); P.OpenEdit(); P.AllKey.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure LinkCodeConst() var P: TestPage "PG EPLN Parts"; begin Init(); P.OpenEdit(); P.CodeConst.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure PartIsProvider() var P: TestPage "PG EPLN Host Prov"; begin Init(); P.OpenEdit(); P.Prov.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure HostNotEditable() var P: TestPage "PG EPLN Host NoEdit"; begin Init(); P.OpenView(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure HostSourceTableView() var P: TestPage "PG EPLN Host View"; begin Init(); P.OpenEdit(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure HostOwnCurrTrigger() var P: TestPage "PG EPLN Host Curr"; begin Init(); P.OpenEdit(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure HostPageExtension() var P: TestPage "PG EPLN Host Ext"; begin Init(); P.OpenEdit(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] [HandlerFunctions('HostHandler')] procedure HostPageRun() begin Init(); Page.Run(Page::"PG EPLN Host"); Error(Log.Get()); end;
    [Test] [HandlerFunctions('HostModalHandler')] procedure HostRunModal() begin Init(); Page.RunModal(Page::"PG EPLN Host"); Error(Log.Get()); end;
    [Test] procedure CardViewGoToSameRow() var P: TestPage "PG EPLN Card"; H: Record "PG EPLN Head"; begin Init(); P.OpenView(); H.Get('A'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [PageHandler] procedure HostHandler(var P: TestPage "PG EPLN Host") begin P.Box.ReadRec.Invoke(); end;
    [ModalPageHandler] procedure HostModalHandler(var P: TestPage "PG EPLN Host") begin P.Box.ReadRec.Invoke(); end;
}
"#;

/// Every neighbor refuses the read of the part's Rec, with no assumption on its run; the Card keeps main's result.
#[test]
fn empty_list_part_rec_list_host_neighbors_refuse() {
    let o = run(NEIGHBOR_SRC);
    assert_eq!(o.len(), 18, "{o:?}");
    let wrong: Vec<_> = o
        .iter()
        .filter(|(name, outcome, flagged)| {
            *flagged
                || if name == "CardViewGoToSameRow" {
                    !raised(outcome, "act=76880/A/0")
                } else {
                    !refused(outcome)
                }
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The reporter's shape (al-runner issue 6924, the List host of DOCMAIL): a List host over a table keyed by ID, opened for edit
/// and moved by GoToRecord to its last row, whose FactBox ListPart is linked by a const and by a field outside the host's key and
/// shows no row; the part's action exits on `Rec."Table ID" = 0`, else runs a modal page filtered on it, whose handler reads the
/// filter. Here the host has a page extension, a fact no probe cell holds: BC's Rec is not measured there, so the read of the
/// part's Rec is refused (main gave the blank Rec and the test failed with an empty filter, a wrong result).
const REPORTER_SRC: &str = r#"
table 76900 "PG EPLR Body"
{
    fields { field(1; ID; Integer) { } field(2; "ID Code"; Code[20]) { } }
    keys { key(PK; ID) { Clustered = true; } }
}
table 76901 "PG EPLR Att"
{
    fields { field(1; "Table ID"; Integer) { } field(2; "No."; Code[20]) { } field(3; "Line No."; Integer) { } }
    keys { key(PK; "Table ID", "No.", "Line No.") { Clustered = true; } }
}
page 76902 "PG EPLR Details"
{
    PageType = List; SourceTable = "PG EPLR Att";
    layout { area(Content) { repeater(R) { field(LineNoCtl; Rec."Line No.") { } } } }
}
page 76903 "PG EPLR Part"
{
    PageType = ListPart; SourceTable = "PG EPLR Att"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions
    {
        area(Processing)
        {
            action(OpenInDetail)
            {
                trigger OnAction()
                var
                    Att: Record "PG EPLR Att";
                    Details: Page "PG EPLR Details";
                begin
                    if Rec."Table ID" = 0 then
                        exit;
                    Att.SetRange("Table ID", Rec."Table ID");
                    Details.SetTableView(Att);
                    Details.RunModal();
                end;
            }
        }
    }
    trigger OnAfterGetCurrRecord() begin end;
}
page 76904 "PG EPLR Bodies"
{
    PageType = List; SourceTable = "PG EPLR Body";
    layout
    {
        area(Content) { repeater(R) { field(IDCtl; Rec.ID) { } field(CodeCtl; Rec."ID Code") { } } }
        area(FactBoxes) { part(Attachments; "PG EPLR Part") { SubPageLink = "Table ID" = const(76900), "No." = field("ID Code"); } }
    }
}
pageextension 76905 "PG EPLR Bodies Ext" extends "PG EPLR Bodies"
{
    actions { addfirst(Processing) { action(Extra) { trigger OnAction() begin end; } } }
}
codeunit 76906 "PG EPL Tests"
{
    Subtype = Test;
    var TableFilterText: Text;
    [Test]
    [HandlerFunctions('DetailsHandler')]
    procedure AttachedDocumentsDrillDown()
    var
        Body: Record "PG EPLR Body";
        Att: Record "PG EPLR Att";
        P: TestPage "PG EPLR Bodies";
    begin
        Att.DeleteAll();
        Body.DeleteAll();
        Body.ID := 1; Body."ID Code" := 'C1'; Body.Insert();
        Body.ID := 2; Body."ID Code" := 'C2'; Body.Insert();
        TableFilterText := '';
        Body.FindLast();
        P.OpenEdit();
        P.GoToRecord(Body);
        P.Attachments.OpenInDetail.Invoke();
        Error('filter=' + TableFilterText);
    end;
    [ModalPageHandler]
    procedure DetailsHandler(var D: TestPage "PG EPLR Details")
    begin
        TableFilterText := D.Filter.GetFilter("Table ID");
    end;
}
"#;

/// The reporter's shape outside the measured cells is refused at the read of the part's Rec, through no assumption.
#[test]
fn empty_list_part_rec_reporter_shape_unmeasured_refuses() {
    let o = run(REPORTER_SRC);
    assert_eq!(o.len(), 1, "{o:?}");
    let (name, outcome, flagged) = &o[0];
    assert!(
        matches!(outcome, Outcome::NotRun(m) if m.contains("part that shows no row") && m.contains("page extension")),
        "{name}: {outcome:?}"
    );
    assert!(!flagged, "{name}: flagged");
}

/// Neighbors of the cells of page 76854 "PG EPL List DocBoth" and of the reporter's host form (corpus pull request 1339, commit
/// 831ae544, EmptyPartRecLinkList.al:1800-2031) that differ from each in one fact no probe cell of that form holds: Next, a
/// GoToRecord to a middle row or to the first of several rows, an open form or a landing the form was not measured in, a part
/// control with UpdatePropagation = SubPart, a host with Editable = false whose part control has only a SubPageLink, a host with
/// two of the new properties, a part without page 1178's triggers, a link without its const, and a CardPart. Each refuses the
/// read of the part's Rec. A Card host whose part control has the reporter's properties keeps
/// main's result (the Card rule, EmptyPartRecLink.al:558), and so does the part's OnAfterGetCurrRecord on a DocBoth host opened
/// for view and moved by a GoToRecord that stays on its row (no probe cell reads it there; main runs it once at the move,
/// `agcr=76910/A`).
const WIDEN_NEIGHBOR_SRC: &str = r#"
table 76910 "PG EPLW Head"
{
    fields { field(1; "No."; Code[20]) { } field(2; "Ref Code"; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 76911 "PG EPLW Line"
{
    fields { field(1; "Table ID"; Integer) { } field(2; "No."; Code[20]) { } field(3; "Line No."; Integer) { } }
    keys { key(PK; "Table ID", "No.", "Line No.") { Clustered = true; } }
}
codeunit 76912 "PG EPLW Log"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Reset() begin Lines := ''; end;
    procedure Add(Line: Text) begin if Lines <> '' then Lines += ' | '; Lines += Line; end;
    procedure Get(): Text begin exit(Lines); end;
}
page 76913 "PG EPLW Part Doc"
{
    PageType = ListPart; SourceTable = "PG EPLW Line"; DeleteAllowed = true; DelayedInsert = true; InsertAllowed = false;
    ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { ApplicationArea = All; trigger OnDrillDown() begin end; } } } }
    actions { area(Processing) { action(ReadRec) { ApplicationArea = All; trigger OnAction() var Log: Codeunit "PG EPLW Log"; begin Log.Add('act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.")); end; } } }
    trigger OnInit() begin end;
    trigger OnDeleteRecord(): Boolean begin exit(true); end;
    trigger OnAfterGetCurrRecord() var Log: Codeunit "PG EPLW Log"; begin Log.Add('agcr=' + Format(Rec."Table ID") + '/' + Rec."No."); end;
}
page 76914 "PG EPLW Part Plain"
{
    PageType = ListPart; SourceTable = "PG EPLW Line"; InsertAllowed = false; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { ApplicationArea = All; } } } }
    actions { area(Processing) { action(ReadRec) { ApplicationArea = All; trigger OnAction() var Log: Codeunit "PG EPLW Log"; begin Log.Add('act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.")); end; } } }
    trigger OnAfterGetCurrRecord() begin end;
}
page 76915 "PG EPLW Card Part"
{
    PageType = CardPart; SourceTable = "PG EPLW Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(LineNoCtl; Rec."Line No.") { ApplicationArea = All; } } }
    actions { area(Processing) { action(ReadRec) { ApplicationArea = All; trigger OnAction() var Log: Codeunit "PG EPLW Log"; begin Log.Add('act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.")); end; } } }
}
page 76916 "PG EPLW DocBoth"
{
    PageType = List; SourceTable = "PG EPLW Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLW Part Doc") { ApplicationArea = All; SubPageLink = "Table ID" = const(76910), "No." = field("No."); UpdatePropagation = Both; } }
    }
}
page 76917 "PG EPLW Rep"
{
    PageType = List; SourceTable = "PG EPLW Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLW Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(76910), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
}
page 76918 "PG EPLW SubPart"
{
    PageType = List; SourceTable = "PG EPLW Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLW Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(76910), "No." = field("Ref Code"); UpdatePropagation = SubPart; } }
    }
}
page 76919 "PG EPLW Partner"
{
    PageType = List; SourceTable = "PG EPLW Head"; Caption = 'Rep'; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLW Part Doc") { SubPageLink = "Table ID" = const(76910), "No." = field("Ref Code"); } }
    }
}
page 76920 "PG EPLW Head Card"
{
    PageType = Card; SourceTable = "PG EPLW Head"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { group(G) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } } }
}
page 76921 "PG EPLW TwoProps"
{
    PageType = List; SourceTable = "PG EPLW Head"; Caption = 'Rep'; Editable = false; CardPageId = "PG EPLW Head Card";
    ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLW Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(76910), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
}
page 76922 "PG EPLW Rep Plain"
{
    PageType = List; SourceTable = "PG EPLW Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLW Part Plain") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(76910), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
}
page 76923 "PG EPLW Rep FieldOnly"
{
    PageType = List; SourceTable = "PG EPLW Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLW Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
}
page 76924 "PG EPLW CardPart Host"
{
    PageType = List; SourceTable = "PG EPLW Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLW Card Part") { ApplicationArea = All; SubPageLink = "Table ID" = const(76910), "No." = field("No."); } }
    }
}
page 76925 "PG EPLW Card Host"
{
    PageType = Card; SourceTable = "PG EPLW Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { group(G) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLW Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(76910), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
}
codeunit 76926 "PG EPL Tests"
{
    Subtype = Test;
    var Log: Codeunit "PG EPLW Log";
    local procedure Init(Rows: Integer)
    var H: Record "PG EPLW Head"; L: Record "PG EPLW Line";
    begin
        H.DeleteAll(); L.DeleteAll();
        H."No." := 'A'; H."Ref Code" := 'RA'; H.Insert();
        if Rows > 1 then begin H."No." := 'B'; H."Ref Code" := 'RB'; H.Insert(); end;
        if Rows > 2 then begin H."No." := 'C'; H."Ref Code" := 'RC'; H.Insert(); end;
        Log.Reset();
    end;
    [Test] procedure DocBoth_OpenEdit_Next() var P: TestPage "PG EPLW DocBoth"; begin Init(2); P.OpenEdit(); P.Next(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure DocBoth_OpenView_AsOpened() var P: TestPage "PG EPLW DocBoth"; begin Init(2); P.OpenView(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure DocBoth_OpenView_GoToLastRow() var P: TestPage "PG EPLW DocBoth"; H: Record "PG EPLW Head"; begin Init(2); P.OpenView(); H.Get('B'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_OpenEdit_GoToMiddleRow() var P: TestPage "PG EPLW Rep"; H: Record "PG EPLW Head"; begin Init(3); P.OpenEdit(); H.Get('B'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_OpenEdit_GoToFirstOfThree() var P: TestPage "PG EPLW Rep"; H: Record "PG EPLW Head"; begin Init(3); P.OpenEdit(); H.Get('A'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_OpenEdit_Next() var P: TestPage "PG EPLW Rep"; begin Init(3); P.OpenEdit(); P.Next(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure SubPart_OpenEdit_GoToLastRow() var P: TestPage "PG EPLW SubPart"; H: Record "PG EPLW Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Partner_OpenEdit_GoToLastRow() var P: TestPage "PG EPLW Partner"; H: Record "PG EPLW Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure TwoProps_OpenEdit_GoToLastRow() var P: TestPage "PG EPLW TwoProps"; H: Record "PG EPLW Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure RepPlainPart_OpenEdit_GoToLastRow() var P: TestPage "PG EPLW Rep Plain"; H: Record "PG EPLW Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure RepFieldOnly_OpenEdit_GoToLastRow() var P: TestPage "PG EPLW Rep FieldOnly"; H: Record "PG EPLW Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardPart_OpenEdit_GoToLastRow() var P: TestPage "PG EPLW CardPart Host"; H: Record "PG EPLW Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardHost_OpenEdit() var P: TestPage "PG EPLW Card Host"; begin Init(1); P.OpenEdit(); Log.Reset(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure DocBoth_OpenView_GoToSameRow_Curr() var P: TestPage "PG EPLW DocBoth"; H: Record "PG EPLW Head"; begin Init(1); P.OpenView(); Log.Reset(); H.Get('A'); P.GoToRecord(H); P.Close(); Error(Log.Get()); end;
}
"#;

/// The neighbors of the widened cells: each refuses, and a CardPart's reason names a part, not a ListPart; the Card host and the
/// OnAfterGetCurrRecord read keep main's result. A test goes through the assumption only where a landing before the one the
/// read follows was a measured cell (`LANDED_IN_A_CELL`: the open of a DocBoth or Rep host for edit, the GoToRecord to the only
/// row of a DocBoth host opened for view).
#[test]
fn empty_list_part_rec_widened_cells_neighbors_refuse() {
    const LANDED_IN_A_CELL: &[&str] = &[
        "DocBoth_OpenEdit_Next",
        "Rep_OpenEdit_GoToMiddleRow",
        "Rep_OpenEdit_GoToFirstOfThree",
        "Rep_OpenEdit_Next",
        "DocBoth_OpenView_GoToSameRow_Curr",
    ];
    let o = run(WIDEN_NEIGHBOR_SRC);
    assert_eq!(o.len(), 14, "{o:?}");
    let main_result: &[(&str, &str)] = &[
        ("CardHost_OpenEdit", "act=76910/RA/0"),
        ("DocBoth_OpenView_GoToSameRow_Curr", "agcr=76910/A"),
    ];
    let wrong: Vec<_> = o
        .iter()
        .filter(|(name, outcome, flagged)| {
            *flagged != LANDED_IN_A_CELL.contains(&name.as_str())
                || match main_result.iter().find(|(n, _)| n == name) {
                    Some((_, want)) => !raised(outcome, want),
                    None => !matches!(outcome, Outcome::NotRun(m)
                        if m.starts_with("a read of Rec of a part that shows no row") && !m.contains("ListPart")),
                }
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// Neighbors of the measured cells of the reporter's host form and of page 76854 "PG EPL List DocBoth" (corpus pull request 1339,
/// commit 831ae544, EmptyPartRecLinkList.al:1800-2031) that review round 1 of al-runner PR 7932 found answering:
/// - a fact every cell holds and the allowlist did not require: the part in the content area, host triggers, a `filter(..)` of
///   one value in place of the `const(..)`, a part without the field's OnDrillDown;
/// - a call sequence before the landing no cell has: Next, three moves, a host filter (on the reporter's form, on DocBoth and on a
///   probe's first host);
/// - a step after the landing that the landing does not include: the part's `CurrPage.Update(false)` before the read, the part's
///   First or Next, a GoToRecord that answered No, a line or a host row written through a Record variable, a second read;
/// - the test's filter on the part of a form outside the cells (a host with Editable = false and a bare part control, a part
///   control with UpdatePropagation = SubPart): the filter re-stamped the link values there with no assumption.
/// Each refuses the read of the part's Rec. The measured landings stay flagged passes with BC's values, on the reporter's form, on
/// DocBoth and on a probe's first host (`Base_*`, two reads after a GoToRecord as EmptyPartRecLinkList.al:1370 measures); a Card
/// host keeps main's result for a second read and for the test's filter on the part (EmptyPartRecLink.al:634 pins the second
/// action on a Card).
const TIED_SRC: &str = r#"
table 76930 "PG EPLT Head"
{
    fields { field(1; "No."; Code[20]) { } field(2; "Ref Code"; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
table 76931 "PG EPLT Line"
{
    fields { field(1; "Table ID"; Integer) { } field(2; "No."; Code[20]) { } field(3; "Line No."; Integer) { } }
    keys { key(PK; "Table ID", "No.", "Line No.") { Clustered = true; } }
}
codeunit 76932 "PG EPLT Log"
{
    SingleInstance = true;
    var Lines: Text;
    procedure Reset() begin Lines := ''; end;
    procedure Add(Line: Text) begin if Lines <> '' then Lines += ' | '; Lines += Line; end;
    procedure Get(): Text begin exit(Lines); end;
}
page 76933 "PG EPLT Part Doc"
{
    PageType = ListPart; SourceTable = "PG EPLT Line"; DeleteAllowed = true; DelayedInsert = true; InsertAllowed = false;
    ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { ApplicationArea = All; trigger OnDrillDown() begin end; } } } }
    actions
    {
        area(Processing)
        {
            action(ReadRec) { ApplicationArea = All; trigger OnAction() var Log: Codeunit "PG EPLT Log"; begin Log.Add('act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.")); end; }
            action(UpdRead) { ApplicationArea = All; trigger OnAction() var Log: Codeunit "PG EPLT Log"; begin CurrPage.Update(false); Log.Add('upd=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.")); end; }
        }
    }
    trigger OnInit() begin end;
    trigger OnDeleteRecord(): Boolean begin exit(true); end;
    trigger OnAfterGetCurrRecord() begin end;
}
page 76934 "PG EPLT Part NoDrill"
{
    PageType = ListPart; SourceTable = "PG EPLT Line"; DeleteAllowed = true; DelayedInsert = true; InsertAllowed = false;
    ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { ApplicationArea = All; } } } }
    actions { area(Processing) { action(ReadRec) { ApplicationArea = All; trigger OnAction() var Log: Codeunit "PG EPLT Log"; begin Log.Add('act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.")); end; } } }
    trigger OnInit() begin end;
    trigger OnDeleteRecord(): Boolean begin exit(true); end;
    trigger OnAfterGetCurrRecord() begin end;
}
page 76935 "PG EPLT Rep"
{
    PageType = List; SourceTable = "PG EPLT Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLT Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(76930), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
}
page 76936 "PG EPLT DocBoth"
{
    PageType = List; SourceTable = "PG EPLT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLT Part Doc") { ApplicationArea = All; SubPageLink = "Table ID" = const(76930), "No." = field("No."); UpdatePropagation = Both; } }
    }
}
page 76937 "PG EPLT Rep Content"
{
    PageType = List; SourceTable = "PG EPLT Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } }
            part(Box; "PG EPLT Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(76930), "No." = field("Ref Code"); UpdatePropagation = Both; }
        }
    }
}
page 76938 "PG EPLT Rep Trig"
{
    PageType = List; SourceTable = "PG EPLT Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLT Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(76930), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
    trigger OnOpenPage() begin end;
    trigger OnAfterGetRecord() begin end;
    trigger OnAfterGetCurrRecord() begin end;
}
page 76939 "PG EPLT Rep FilterLink"
{
    PageType = List; SourceTable = "PG EPLT Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLT Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = filter(76930), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
}
page 76940 "PG EPLT Rep NoDrill"
{
    PageType = List; SourceTable = "PG EPLT Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLT Part NoDrill") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(76930), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
}
page 76941 "PG EPLT Partner"
{
    PageType = List; SourceTable = "PG EPLT Head"; Caption = 'Rep'; Editable = false; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLT Part Doc") { SubPageLink = "Table ID" = const(76930), "No." = field("Ref Code"); } }
    }
}
page 76942 "PG EPLT SubPart"
{
    PageType = List; SourceTable = "PG EPLT Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLT Part Doc") { ApplicationArea = All; SubPageLink = "Table ID" = const(76930), "No." = field("Ref Code"); UpdatePropagation = SubPart; } }
    }
}
page 76943 "PG EPLT Base"
{
    PageType = List; SourceTable = "PG EPLT Head"; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { repeater(Heads) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLT Part Doc") { ApplicationArea = All; SubPageLink = "Table ID" = const(76930), "No." = field("No."); } }
    }
}
page 76944 "PG EPLT Card Host"
{
    PageType = Card; SourceTable = "PG EPLT Head"; Caption = 'Rep'; ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content) { group(G) { field(NoCtl; Rec."No.") { ApplicationArea = All; } } }
        area(FactBoxes) { part(Box; "PG EPLT Part Doc") { ApplicationArea = All; Caption = 'Attachments'; SubPageLink = "Table ID" = const(76930), "No." = field("Ref Code"); UpdatePropagation = Both; } }
    }
}
codeunit 76945 "PG EPL Tests"
{
    Subtype = Test;
    var Log: Codeunit "PG EPLT Log";
    local procedure Init(Rows: Integer)
    var H: Record "PG EPLT Head"; L: Record "PG EPLT Line";
    begin
        H.DeleteAll(); L.DeleteAll();
        H."No." := 'A'; H."Ref Code" := 'RA'; H.Insert();
        if Rows > 1 then begin H."No." := 'B'; H."Ref Code" := 'RB'; H.Insert(); end;
        if Rows > 2 then begin H."No." := 'C'; H."Ref Code" := 'RC'; H.Insert(); end;
        Log.Reset();
    end;
    local procedure AddLine(No: Code[20]) var L: Record "PG EPLT Line"; begin L."Table ID" := 76930; L."No." := No; L."Line No." := 1; L.Insert(); end;
    [Test] procedure Rep_GoToLast() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_View_GoToLast() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenView(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_AsOpened() var P: TestPage "PG EPLT Rep"; begin Init(3); P.OpenEdit(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure DocBoth_OnlyRow() var P: TestPage "PG EPLT DocBoth"; H: Record "PG EPLT Head"; begin Init(1); P.OpenEdit(); H.Get('A'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Base_AsOpened() var P: TestPage "PG EPLT Base"; begin Init(3); P.OpenEdit(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Card_TwoReads() var P: TestPage "PG EPLT Card Host"; begin Init(1); P.OpenEdit(); P.Box.ReadRec.Invoke(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Card_PartFilter() var P: TestPage "PG EPLT Card Host"; begin Init(1); P.OpenEdit(); P.Box.Filter.SetFilter("Line No.", '5'); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Content_GoToLast() var P: TestPage "PG EPLT Rep Content"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Trig_GoToLast() var P: TestPage "PG EPLT Rep Trig"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure FilterLink_GoToLast() var P: TestPage "PG EPLT Rep FilterLink"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure NoDrill_GoToLast() var P: TestPage "PG EPLT Rep NoDrill"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_Next_GoToLast() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); P.Next(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_ThreeMoves() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); H.Get('A'); P.GoToRecord(H); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_HostFilter_GoToLastOfFilter() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); P.Filter.SetFilter("No.", 'A|B'); H.Get('B'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure DocBoth_HostFilter_OnlyRow() var P: TestPage "PG EPLT DocBoth"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); P.Filter.SetFilter("No.", 'B'); H.Get('B'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_GoToLast_PartUpdate() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.UpdRead.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_GoToLast_PartFirst() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); if P.Box.First() then; P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_GoToLast_PartNext() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); if P.Box.Next() then; P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_AsOpened_GoToNoRow() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Init(); H."No." := 'Z'; if P.GoToRecord(H) then; P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_GoToLast_LineAdded() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); AddLine('RC'); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_GoToLast_HostModified() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); H."Ref Code" := 'RX'; H.Modify(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_GoToLast_HostDeleted() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); H.Delete(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_GoToLast_RowAdded() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); H."No." := 'D'; H."Ref Code" := 'RD'; H.Insert(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Rep_GoToLast_TwoReads() var P: TestPage "PG EPLT Rep"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Base_AsOpened_TwoReads() var P: TestPage "PG EPLT Base"; begin Init(3); P.OpenEdit(); P.Box.ReadRec.Invoke(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Base_AsOpened_PartFirst() var P: TestPage "PG EPLT Base"; begin Init(3); P.OpenEdit(); if P.Box.First() then; P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Base_GoToLast_TwoReads() var P: TestPage "PG EPLT Base"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Base_Next_GoToLast() var P: TestPage "PG EPLT Base"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); P.Next(); H.Get('C'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Base_HostFilter_GoToLastOfFilter() var P: TestPage "PG EPLT Base"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); P.Filter.SetFilter("No.", 'A|B'); H.Get('B'); P.GoToRecord(H); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Partner_GoToLast_PartFilter() var P: TestPage "PG EPLT Partner"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.Filter.SetFilter("Line No.", '5'); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure Partner_AsOpened_PartFilter() var P: TestPage "PG EPLT Partner"; begin Init(3); P.OpenEdit(); P.Box.Filter.SetFilter("Line No.", '5'); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure SubPart_GoToLast_PartFilter() var P: TestPage "PG EPLT SubPart"; H: Record "PG EPLT Head"; begin Init(3); P.OpenEdit(); H.Get('C'); P.GoToRecord(H); P.Box.Filter.SetFilter("Line No.", '5'); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
}
"#;

/// The measured landings answer BC's values through the assumption; the Card host keeps main's result through none; every
/// neighbor refuses the read of the part's Rec.
#[test]
fn empty_list_part_rec_tied_to_its_landing() {
    let o = run(TIED_SRC);
    assert_eq!(o.len(), 32, "{o:?}");
    let kept: &[(&str, &str, bool)] = &[
        ("Rep_GoToLast", "act=76930/RC/0", true),
        ("Rep_View_GoToLast", "act=76930/RC/0", true),
        ("Rep_AsOpened", "act=76930/RA/0", true),
        ("DocBoth_OnlyRow", "act=76930/A/0", true),
        ("Base_AsOpened", "act=76930/A/0", true),
        ("Base_GoToLast_TwoReads", "act=76930/C/0 | act=76930/C/0", true),
        ("Card_TwoReads", "act=76930/RA/0 | act=76930/RA/0", false),
        ("Card_PartFilter", "act=76930/RA/5", false),
    ];
    let wrong: Vec<_> = o
        .iter()
        .filter(
            |(name, outcome, flagged)| match kept.iter().find(|(n, _, _)| n == name) {
                Some((_, want, flag)) => !raised(outcome, want) || flagged != flag,
                None => !refused(outcome),
            },
        )
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}
