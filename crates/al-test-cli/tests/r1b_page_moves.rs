//! Three page moves al-runner refused, each from a reporter's repro that passes on real BC 29 (al-runner issues 5824, 5825,
//! 5826). What the corpus pins runs clean; the rest runs on a registered assumption that awaits the writer's files in
//! pages/lt-rz-outside-page-moves/.

use al_testkit::{blocked, compile};
use al_testrun::Outcome;
use al_testrun::assumptions::Assumption;
use al_testrun::isolation::TestIsolation;

type Run = (Vec<(String, Outcome)>, Vec<(String, Vec<Assumption>)>);

fn run(src: &str, codeunit: &str) -> Run {
    let l = compile(src);
    let engine = al_vm::Engine::new(l.program.clone(), blocked(&l));
    let oi = l.program.objects.iter().position(|o| o.name == codeunit).unwrap();
    let mut hits = Vec::new();
    let out = al_testrun::runtime::run_codeunit_full(
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
    (out, hits)
}

fn of(out: &[(String, Outcome)], n: &str) -> Outcome {
    out.iter().find(|(t, _)| t == n).map(|(_, o)| o.clone()).unwrap()
}

fn hits_of(hits: &[(String, Vec<Assumption>)], n: &str) -> Vec<Assumption> {
    hits.iter().find(|(t, _)| t == n).map(|(_, h)| h.clone()).unwrap_or_default()
}

// al-runner issue 5826: RunObject + RunPageLink to a List that declares SourceTableView.
const LINK_VIEW: &str = r#"
table 50125 "Repro Stock Item" { fields { field(1; "Code"; Code[20]) { } } keys { key(PK; "Code") { } } }
table 50126 "Repro Stock Entry"
{
    fields { field(1; "Entry No."; Integer) { } field(2; "Item Code"; Code[20]) { } field(3; Quantity; Decimal) { } field(4; Positive; Boolean) { } }
    keys { key(PK; "Entry No.") { } }
}
page 50127 "Repro Stock Items"
{
    PageType = List; SourceTable = "Repro Stock Item"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Items) { field("Code"; Rec."Code") { ApplicationArea = All; } } } }
    actions
    {
        area(Processing)
        {
            action(EntriesNewestFirst) { ApplicationArea = All; RunObject = page "Repro Entries Newest First"; RunPageLink = "Item Code" = field("Code"); }
            action(PositiveEntries) { ApplicationArea = All; RunObject = page "Repro Positive Entries"; RunPageLink = "Item Code" = field("Code"); }
        }
    }
}
page 50128 "Repro Entries Newest First"
{
    PageType = List; SourceTable = "Repro Stock Entry"; SourceTableView = sorting("Entry No.") order(descending); Editable = false;
    ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Entries) { field("Entry No."; Rec."Entry No.") { ApplicationArea = All; } field("Item Code"; Rec."Item Code") { ApplicationArea = All; } } } }
}
page 50129 "Repro Positive Entries"
{
    PageType = List; SourceTable = "Repro Stock Entry"; SourceTableView = where(Positive = const(true)); Editable = false;
    ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { repeater(Entries) { field("Entry No."; Rec."Entry No.") { ApplicationArea = All; } field("Item Code"; Rec."Item Code") { ApplicationArea = All; } } } }
}
codeunit 50130 "Repro Link To View Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        CapturedFilter: Text;
        CapturedCurrent: Text;
        CapturedRows: Text;

    [Test]
    [HandlerFunctions('NewestFirstHandler')]
    procedure LinkToDescendingView()
    var
        StockItems: TestPage "Repro Stock Items";
    begin
        Setup();
        StockItems.OpenView();
        StockItems.GoToKey('I1');
        StockItems.EntriesNewestFirst.Invoke();
        Check('I1', '4', '4,3,1,');
    end;

    [Test]
    [HandlerFunctions('PositiveHandler')]
    procedure LinkToWhereView()
    var
        StockItems: TestPage "Repro Stock Items";
    begin
        Setup();
        StockItems.OpenView();
        StockItems.GoToKey('I1');
        StockItems.PositiveEntries.Invoke();
        Check('I1', '1', '1,4,');
    end;

    [PageHandler]
    procedure NewestFirstHandler(var Entries: TestPage "Repro Entries Newest First")
    begin
        CapturedFilter := Entries.Filter.GetFilter("Item Code");
        CapturedCurrent := Entries."Entry No.".Value();
        CapturedRows := '';
        if Entries.First() then
            repeat
                CapturedRows += Entries."Entry No.".Value() + ',';
            until not Entries.Next();
    end;

    [PageHandler]
    procedure PositiveHandler(var Entries: TestPage "Repro Positive Entries")
    begin
        CapturedFilter := Entries.Filter.GetFilter("Item Code");
        CapturedCurrent := Entries."Entry No.".Value();
        CapturedRows := '';
        if Entries.First() then
            repeat
                CapturedRows += Entries."Entry No.".Value() + ',';
            until not Entries.Next();
    end;

    local procedure Check(ExpectedFilter: Text; ExpectedCurrent: Text; ExpectedRows: Text)
    begin
        if (CapturedFilter <> ExpectedFilter) or (CapturedCurrent <> ExpectedCurrent) or (CapturedRows <> ExpectedRows) then
            Error('Expected filter %1, current entry %2 and rows %3; got filter %4, current entry %5 and rows %6.', ExpectedFilter, ExpectedCurrent, ExpectedRows, CapturedFilter, CapturedCurrent, CapturedRows);
    end;

    local procedure Setup()
    var
        StockItem: Record "Repro Stock Item";
        StockEntry: Record "Repro Stock Entry";
    begin
        StockEntry.DeleteAll();
        StockItem.DeleteAll();
        StockItem."Code" := 'I1';
        StockItem.Insert();
        StockItem."Code" := 'I2';
        StockItem.Insert();
        AddEntry(1, 'I1', 5);
        AddEntry(2, 'I2', 3);
        AddEntry(3, 'I1', -2);
        AddEntry(4, 'I1', 1);
        AddEntry(5, 'I2', -1);
    end;

    local procedure AddEntry(EntryNo: Integer; ItemCode: Code[20]; Qty: Decimal)
    var
        StockEntry: Record "Repro Stock Entry";
    begin
        StockEntry."Entry No." := EntryNo;
        StockEntry."Item Code" := ItemCode;
        StockEntry.Quantity := Qty;
        StockEntry.Positive := Qty > 0;
        StockEntry.Insert();
    end;
}
"#;

#[test]
fn run_page_link_to_a_list_with_a_source_table_view_applies_both() {
    let (out, hits) = run(LINK_VIEW, "Repro Link To View Tests");
    assert_eq!(of(&out, "LinkToDescendingView"), Outcome::Pass);
    assert_eq!(of(&out, "LinkToWhereView"), Outcome::Pass);
    // The rows and the open row of a List reached this way are the assumption (the Page.Run one is not marked).
    for t in ["LinkToDescendingView", "LinkToWhereView"] {
        assert_eq!(hits_of(&hits, t), vec![Assumption::RunPageLinkTargetView], "{t}");
    }
}

// al-runner issue 5824: CurrPage.Update(false) after a header field's OnValidate set a filter that left the row.
const UPDATE_VALIDATE: &str = r#"
table 50115 "Repro Plan Line"
{
    fields { field(1; "No."; Code[20]) { } field(2; "Group Code"; Code[20]) { } }
    keys { key(PK; "No.") { Clustered = true; } }
}
page 50116 "Repro Plan Worksheet"
{
    PageType = Worksheet; SourceTable = "Repro Plan Line"; ApplicationArea = All; UsageCategory = Tasks;
    InsertAllowed = false; DeleteAllowed = false;
    layout
    {
        area(Content)
        {
            group(Filters)
            {
                field(GroupFilter; GroupFilter)
                {
                    Caption = 'Group';
                    trigger OnValidate()
                    begin
                        if GroupFilter = '' then
                            Rec.SetRange("Group Code")
                        else
                            Rec.SetRange("Group Code", GroupFilter);
                        CurrPage.Update(false);
                    end;
                }
            }
            repeater(Lines)
            {
                Editable = false;
                field("No."; Rec."No.") { }
                field("Group Code"; Rec."Group Code") { }
            }
        }
    }
    var
        GroupFilter: Code[20];
}
codeunit 50117 "Repro Update Filter Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FilterLeavesMiddleRow()
    begin
        Check('B', 'G1', 'C', 'A,C,E,');
    end;

    [Test]
    procedure FilterLeavesLaterRow()
    begin
        Check('D', 'G1', 'E', 'A,C,E,');
    end;

    [Test]
    procedure FilterLeavesFirstRow()
    begin
        Check('A', 'G2', 'B', 'B,D,F,');
    end;

    [Test]
    procedure FilterLeavesLastRow()
    begin
        Check('F', 'G1', 'E', 'A,C,E,');
    end;

    local procedure Check(StandOn: Code[20]; GroupFilter: Code[20]; ExpectedCurrent: Text; ExpectedRows: Text)
    var
        Worksheet: TestPage "Repro Plan Worksheet";
        Current: Text;
        Rows: Text;
    begin
        Setup();
        Worksheet.OpenEdit();
        Worksheet.GoToKey(StandOn);
        Worksheet.GroupFilter.SetValue(GroupFilter);
        Current := Worksheet."No.".Value();
        if Worksheet.First() then
            repeat
                Rows += Worksheet."No.".Value() + ',';
            until not Worksheet.Next();
        Worksheet.Close();
        if (Current <> ExpectedCurrent) or (Rows <> ExpectedRows) then
            Error('Expected current %1 and rows %2; got current %3 and rows %4.', ExpectedCurrent, ExpectedRows, Current, Rows);
    end;

    local procedure Setup()
    var
        Line: Record "Repro Plan Line";
    begin
        Line.DeleteAll();
        Add('A', 'G1');
        Add('B', 'G2');
        Add('C', 'G1');
        Add('D', 'G2');
        Add('E', 'G1');
        Add('F', 'G2');
    end;

    local procedure Add(No: Code[20]; Grp: Code[20])
    var
        Line: Record "Repro Plan Line";
    begin
        Line."No." := No;
        Line."Group Code" := Grp;
        Line.Insert();
    end;
}
"#;

#[test]
fn update_after_a_validate_filter_left_the_row_settles_as_after_an_action() {
    let (out, hits) = run(UPDATE_VALIDATE, "Repro Update Filter Tests");
    for t in ["FilterLeavesMiddleRow", "FilterLeavesLaterRow", "FilterLeavesFirstRow", "FilterLeavesLastRow"] {
        assert_eq!(of(&out, t), Outcome::Pass, "{t}");
        assert_eq!(hits_of(&hits, t), vec![Assumption::UpdateFromValidateSettlesAsAfterAction], "{t}");
    }
}

// al-runner issue 5825: a database Worksheet in lookup mode whose OnOpenPage left Rec on a row under a secondary key.
const WORKSHEET_KEY: &str = r#"
table 50120 "Repro Demand Line"
{
    fields { field(1; "Line No."; Integer) { } field(2; Level; Integer) { } field(3; "Item Code"; Code[20]) { } }
    keys { key(PK; "Line No.") { } key(ByItem; "Item Code") { } }
}
page 50121 "Repro Demand Worksheet"
{
    PageType = Worksheet; SourceTable = "Repro Demand Line"; InsertAllowed = false; DeleteAllowed = false;
    ApplicationArea = All; UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Line No."; Rec."Line No.") { ApplicationArea = All; }
                field(Level; Rec.Level) { ApplicationArea = All; }
                field("Item Code"; Rec."Item Code") { ApplicationArea = All; }
            }
        }
    }
    var
        Mode: Integer;
    procedure SetMode(NewMode: Integer)
    begin
        Mode := NewMode;
    end;
    trigger OnOpenPage()
    begin
        Rec.DeleteAll();
        AddLine(10000, 0, 'HDR');
        AddLine(20000, 1, 'X');
        AddLine(30000, 1, 'Y');
        AddLine(40000, 0, 'TRL');
        Rec.Reset();
        case Mode of
            1:
                begin
                    Rec.SetCurrentKey("Item Code");
                    Rec.FindFirst();
                end;
            2:
                begin
                    Rec.SetCurrentKey("Item Code");
                    Rec.Get(30000);
                end;
            3:
                Rec.FindFirst();
        end;
        Rec.SetRange(Level, 1);
    end;
    local procedure AddLine(LineNo: Integer; NewLevel: Integer; ItemCode: Code[20])
    begin
        Rec.Init();
        Rec."Line No." := LineNo;
        Rec.Level := NewLevel;
        Rec."Item Code" := ItemCode;
        Rec.Insert();
    end;
}
codeunit 50122 "Repro Demand Runner"
{
    procedure RunLookup(NewMode: Integer; var Picked: Record "Repro Demand Line"): Action
    var
        DemandWorksheet: Page "Repro Demand Worksheet";
        Result: Action;
    begin
        DemandWorksheet.SetMode(NewMode);
        DemandWorksheet.LookupMode(true);
        Result := DemandWorksheet.RunModal();
        if Result = Action::LookupOK then
            DemandWorksheet.GetRecord(Picked);
        exit(Result);
    end;
}
codeunit 50123 "Repro Open Outside View Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;
    var
        ShownCode: Text;

    [Test]
    [HandlerFunctions('PickShownRowHandler')]
    procedure SecondaryKeyRecBeforeTheView()
    begin
        Check(1, 'X');
    end;

    [Test]
    [HandlerFunctions('PickShownRowHandler')]
    procedure SecondaryKeyRecInsideTheView()
    begin
        Check(2, 'Y');
    end;

    [Test]
    [HandlerFunctions('PickShownRowHandler')]
    procedure PrimaryKeyRecBeforeTheView()
    begin
        Check(3, 'X');
    end;

    local procedure Check(Mode: Integer; Expected: Text)
    var
        Picked: Record "Repro Demand Line";
        DemandRunner: Codeunit "Repro Demand Runner";
        CloseAction: Action;
    begin
        ShownCode := '';
        CloseAction := DemandRunner.RunLookup(Mode, Picked);
        if (CloseAction <> Action::LookupOK) or (ShownCode <> Expected) or (Picked."Item Code" <> Expected) then
            Error('Expected the worksheet to open on %1; RunModal returned %2, it showed %3 and returned %4.', Expected, CloseAction, ShownCode, Picked."Item Code");
    end;

    [ModalPageHandler]
    procedure PickShownRowHandler(var DemandWorksheet: TestPage "Repro Demand Worksheet")
    begin
        ShownCode := DemandWorksheet."Item Code".Value();
        DemandWorksheet.OK().Invoke();
    end;
}
"#;

#[test]
fn database_worksheet_in_a_secondary_key_opens_where_the_rec_lands() {
    let (out, hits) = run(WORKSHEET_KEY, "Repro Open Outside View Tests");
    for t in ["SecondaryKeyRecBeforeTheView", "SecondaryKeyRecInsideTheView", "PrimaryKeyRecBeforeTheView"] {
        assert_eq!(of(&out, t), Outcome::Pass, "{t}");
    }
    for t in ["SecondaryKeyRecBeforeTheView", "SecondaryKeyRecInsideTheView"] {
        assert_eq!(hits_of(&hits, t), vec![Assumption::DbWorksheetSecondaryKeyOpensOnRec], "{t}");
    }
    // The primary key is pinned (pages/lt-outside-worksheet-2/RtFillOnOpen.al:263-417).
    assert_eq!(hits_of(&hits, "PrimaryKeyRecBeforeTheView"), vec![]);
}
