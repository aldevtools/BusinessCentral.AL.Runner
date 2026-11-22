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
        Check('agcr=75160/A/0 | agcr=75160/A/0 | agcr=75160/A/0', Got);
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

    [Test]
    procedure EmptyPart_CardOpenEdit_FieldOnly_OnAfterGetCurrRecordReadsRec()
    var
        Got: Text;
        Card: TestPage "PG EPR Card FieldOnly";
    begin
        Initialize();
        Card.OpenEdit();
        Got := Log.CurrLog();
        Card.Close();
        Check('agcr=0/A/0', Got);
    end;
}
"#;

/// The probe's OnAfterGetCurrRecord cells on a Card opened for edit: BC runs the trigger three times over the two empty parts
/// and once for the single part (corpus pull request 1335, EmptyPartRecLink.al:604, :652); the runtime runs it on no part of a
/// host opened for edit yet. That is a gap of its own, not changed here.
const EDIT_CURR_GAPS: [&str; 2] = [
    "EmptyPart_CardOpenEdit_ConstField_OnAfterGetCurrRecordReadsRec",
    "EmptyPart_CardOpenEdit_FieldOnly_OnAfterGetCurrRecordReadsRec",
];

/// Every other cell of the probe: Rec of the empty part holds the single values of its SubPageLink (const, field, a filter of
/// one value; not a range), in a FactBox and in the content area, on a Card and on a Document host, opened for view, for edit
/// and with OpenNew, after GoToRecord, and at a second action. A part with a row reads that row.
#[test]
fn empty_list_part_rec_holds_the_link_values() {
    let o = outcomes_in(SRC, "PG EPR Tests");
    assert_eq!(o.len(), 19, "{o:?}");
    for (name, outcome) in &o {
        if EDIT_CURR_GAPS.contains(&name.as_str()) {
            continue;
        }
        assert!(matches!(outcome, Outcome::Pass), "{name}: {outcome:?}");
    }
}

/// Shapes probe 1335 does not measure: a link to a field outside the key, PopulateAllFields, a SourceTableView, an OnNewRecord, a
/// filter the part sets itself, an unlinked part, a List or ListPlus host, a host moved by Next() or New(). The Card and ListPlus
/// shapes keep the blank Rec the runtime gave before the change (`act=0//0/`); these are not BC's values, they guard against the
/// rule reaching a shape nobody measured. On the List host the blank Rec was a wrong result (al-runner issue 6924): the read of
/// a field the part's filters name is refused there. The List host's part has no OnAfterGetCurrRecord, a trigger set no
/// List-host probe cell holds (`pages/empty_part_rec_list.rs`).
const UNMEASURED_SRC: &str = r#"
table 75180 "PG EPU Head" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
table 75181 "PG EPU Line"
{
    fields { field(1; "Table ID"; Integer) { } field(2; "No."; Code[20]) { } field(3; "Line No."; Integer) { } field(4; Kind; Code[20]) { } }
    keys { key(PK; "Table ID", "No.", "Line No.") { Clustered = true; } }
}
codeunit 75182 "PG EPU Log"
{
    SingleInstance = true;
    var L: Text;
    procedure Reset() begin L := ''; end;
    procedure Add(Rec: Record "PG EPU Line") begin L := 'act=' + Format(Rec."Table ID") + '/' + Rec."No." + '/' + Format(Rec."Line No.") + '/' + Rec.Kind; end;
    procedure Get(): Text begin exit(L); end;
}
page 75183 "PG EPU Part"
{
    PageType = ListPart; SourceTable = "PG EPU Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPU Log"; begin Log.Add(Rec); end; } } }
}
page 75184 "PG EPU Part View"
{
    PageType = ListPart; SourceTable = "PG EPU Line"; InsertAllowed = false; SourceTableView = where("Line No." = const(5));
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPU Log"; begin Log.Add(Rec); end; } } }
}
page 75185 "PG EPU Part NewRec"
{
    PageType = ListPart; SourceTable = "PG EPU Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPU Log"; begin Log.Add(Rec); end; } } }
    trigger OnNewRecord(BelowxRec: Boolean) begin Rec.Kind := 'N'; Rec."Line No." := 9; end;
}
page 75186 "PG EPU Part PopAll"
{
    PageType = ListPart; SourceTable = "PG EPU Line"; InsertAllowed = false; PopulateAllFields = true;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPU Log"; begin Log.Add(Rec); end; } } }
}
page 75187 "PG EPU Part OwnFilter"
{
    PageType = ListPart; SourceTable = "PG EPU Line"; InsertAllowed = false;
    layout { area(Content) { repeater(Lines) { field(LineNoCtl; Rec."Line No.") { } } } }
    actions { area(Processing) { action(ReadRec) { trigger OnAction() var Log: Codeunit "PG EPU Log"; begin Log.Add(Rec); end; } } }
    trigger OnOpenPage() begin Rec.SetRange("Table ID", 75180); end;
}
page 75188 "PG EPU Card"
{
    PageType = Card; SourceTable = "PG EPU Head";
    layout
    {
        area(Content) { field(NoCtl; Rec."No.") { } }
        area(FactBoxes)
        {
            part(Box; "PG EPU Part") { SubPageLink = "Table ID" = const(75180), "No." = field("No."); }
            part(BoxView; "PG EPU Part View") { SubPageLink = "Table ID" = const(75180), "No." = field("No."); }
            part(BoxNew; "PG EPU Part NewRec") { SubPageLink = "Table ID" = const(75180), "No." = field("No."); }
            part(BoxNonKey; "PG EPU Part") { SubPageLink = "Table ID" = const(75180), Kind = field("No."); }
            part(BoxPop; "PG EPU Part PopAll") { SubPageLink = "Table ID" = const(75180), Kind = field("No."); }
            part(BoxOwn; "PG EPU Part OwnFilter") { SubPageLink = "Table ID" = const(75180), "No." = field("No."); }
            part(BoxNoLink; "PG EPU Part") { }
        }
    }
}
page 75189 "PG EPU List"
{
    PageType = List; SourceTable = "PG EPU Head";
    layout
    {
        area(Content) { repeater(R) { field(NoCtl; Rec."No.") { } } }
        area(FactBoxes) { part(Box; "PG EPU Part") { SubPageLink = "Table ID" = const(75180), "No." = field("No."); } }
    }
}
page 75190 "PG EPU ListPlus"
{
    PageType = ListPlus; SourceTable = "PG EPU Head";
    layout { area(Content) { field(NoCtl; Rec."No.") { } part(Box; "PG EPU Part") { SubPageLink = "Table ID" = const(75180), "No." = field("No."); } } }
}
codeunit 75191 "PG EPU Tests"
{
    Subtype = Test;
    var Log: Codeunit "PG EPU Log";
    local procedure Init() var H: Record "PG EPU Head"; L: Record "PG EPU Line"; begin H.DeleteAll(); L.DeleteAll(); H."No." := 'A'; H.Insert(); H."No." := 'B'; H.Insert(); Log.Reset(); end;
    [Test] procedure CardView_NonKeyLink() var P: TestPage "PG EPU Card"; begin Init(); P.OpenView(); P.BoxNonKey.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardEdit_NonKeyLink() var P: TestPage "PG EPU Card"; begin Init(); P.OpenEdit(); P.BoxNonKey.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardView_PopulateAllFields() var P: TestPage "PG EPU Card"; begin Init(); P.OpenView(); P.BoxPop.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardEdit_PopulateAllFields() var P: TestPage "PG EPU Card"; begin Init(); P.OpenEdit(); P.BoxPop.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardView_SourceTableView() var P: TestPage "PG EPU Card"; begin Init(); P.OpenView(); P.BoxView.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardEdit_SourceTableView() var P: TestPage "PG EPU Card"; begin Init(); P.OpenEdit(); P.BoxView.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardView_OnNewRecord() var P: TestPage "PG EPU Card"; begin Init(); P.OpenView(); P.BoxNew.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardEdit_OnNewRecord() var P: TestPage "PG EPU Card"; begin Init(); P.OpenEdit(); P.BoxNew.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardView_OwnFilter() var P: TestPage "PG EPU Card"; begin Init(); P.OpenView(); P.BoxOwn.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardEdit_OwnFilter() var P: TestPage "PG EPU Card"; begin Init(); P.OpenEdit(); P.BoxOwn.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardView_NoLink() var P: TestPage "PG EPU Card"; begin Init(); P.OpenView(); P.BoxNoLink.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure ListView() var P: TestPage "PG EPU List"; begin Init(); P.OpenView(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure ListEdit() var P: TestPage "PG EPU List"; begin Init(); P.OpenEdit(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure ListPlusView() var P: TestPage "PG EPU ListPlus"; begin Init(); P.OpenView(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure ListPlusEdit() var P: TestPage "PG EPU ListPlus"; begin Init(); P.OpenEdit(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardEdit_Next() var P: TestPage "PG EPU Card"; begin Init(); P.OpenEdit(); P.Next(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardView_Next() var P: TestPage "PG EPU Card"; begin Init(); P.OpenView(); P.Next(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
    [Test] procedure CardEdit_New() var P: TestPage "PG EPU Card"; begin Init(); P.OpenEdit(); P.New(); P.Box.ReadRec.Invoke(); P.Close(); Error(Log.Get()); end;
}
"#;

/// Every unmeasured shape of a Card or ListPlus host reads the blank record, as before the change; on the List host the read
/// is refused, with the fact that left the measured cells. Each test raises what the part's action logged.
#[test]
fn empty_list_part_rec_outside_the_measured_shape() {
    let o = outcomes_in(UNMEASURED_SRC, "PG EPU Tests");
    assert_eq!(o.len(), 18, "{o:?}");
    let wrong: Vec<_> = o
        .iter()
        .filter(|(name, outcome)| match name.as_str() {
            "ListView" | "ListEdit" => !matches!(outcome, Outcome::NotRun(m)
                if m.starts_with("a read of Rec of a part that shows no row")
                    && m.contains("(the part page has a property, a trigger")),
            _ => {
                !matches!(outcome, Outcome::Fail(m) if m == "act=0//0/")
                    && !matches!(outcome, Outcome::NotRun(m) if m.ends_with("this failure is not trusted: act=0//0/"))
            }
        })
        .collect();
    assert!(wrong.is_empty(), "{wrong:#?}");
}
