//! The reporters' real-BC repros of al-runner issues 5785, 5808, 5809 and 5811 (writer RT's page batch), run as al-runner runs
//! them. Each passes on BC 29 (the issue texts); writer RT pins the same values in pages/lt-outside-worksheet-2/ (RtFillOnOpen.al,
//! RtRowEditable.al, RtFilteredGoneRow.al, RtLinkRange.al). Shapes next to them that no test pins stay refused.

use al_testkit::outcomes_in;
use al_testrun::Outcome;

fn outcome(src: &str, codeunit: &str, test: &str) -> Outcome {
    let out = outcomes_in(src, codeunit);
    out.into_iter()
        .find(|(t, _)| t == test)
        .map(|(_, o)| o)
        .unwrap_or_else(|| panic!("no test {test}"))
}

/// al-runner issue 5785.
const WORKSHEET_FILLED_ON_OPEN: &str = r##"
table 50100 "Repro Plan Line 50100"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; Description; Text[50]) { }
    }
    keys
    {
        key(PK; "Code") { }
    }
}

page 50101 "Repro Plan Worksheet 50101"
{
    PageType = Worksheet;
    SourceTable = "Repro Plan Line 50100";
    InsertAllowed = false;
    DeleteAllowed = false;
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
            }
        }
    }

    trigger OnOpenPage()
    begin
        // Fill the worksheet for this run: Rec is left on the last row inserted.
        Rec.DeleteAll();
        AddLine('A', 'Alpha');
        AddLine('B', 'Bravo');
        AddLine('C', 'Charlie');
    end;

    local procedure AddLine(NewCode: Code[20]; NewDescription: Text[50])
    begin
        Rec.Init();
        Rec."Code" := NewCode;
        Rec.Description := NewDescription;
        Rec.Insert();
    end;
}

codeunit 50102 "Repro Plan Runner 50102"
{
    procedure RunWorksheetLookup(): Action
    var
        PlanWorksheet: Page "Repro Plan Worksheet 50101";
    begin
        PlanWorksheet.LookupMode(true);
        exit(PlanWorksheet.RunModal());
    end;
}

codeunit 50103 "Repro Plan Worksheet Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        ShownCode: Text;

    [Test]
    [HandlerFunctions('PickShownRowHandler')]
    procedure WorksheetFilledByOnOpenPageOpensOnRowRecWasLeftOn()
    var
        PlanRunner: Codeunit "Repro Plan Runner 50102";
        CloseAction: Action;
    begin
        ShownCode := '';
        CloseAction := PlanRunner.RunWorksheetLookup();
        if (CloseAction <> Action::LookupOK) or (ShownCode <> 'C') then
            Error('RunModal returned %1, the worksheet opened on row %2.', CloseAction, ShownCode);
    end;

    [ModalPageHandler]
    procedure PickShownRowHandler(var PlanWorksheet: TestPage "Repro Plan Worksheet 50101")
    begin
        ShownCode := PlanWorksheet."Code".Value();
        PlanWorksheet.OK().Invoke();
    end;
}
"##;

/// al-runner issue 5808.
const REPEATER_EDITABLE: &str = r##"
table 50112 "Repro Plan Row 50112"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Sequence No."; Integer) { }
        field(3; "New Start"; DateTime) { }
    }
    keys
    {
        key(PK; "Code") { Clustered = true; }
        key(Seq; "Sequence No.") { }
    }
}

page 50113 "Repro Plan Worksheet 50113"
{
    PageType = Worksheet;
    SourceTable = "Repro Plan Row 50112";
    SourceTableView = sorting("Sequence No.");
    ApplicationArea = All;
    UsageCategory = None;
    InsertAllowed = false;
    DelayedInsert = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Sequence No."; Rec."Sequence No.")
                {
                    ApplicationArea = All;
                    Editable = SequenceEditable;
                }
                field("Code"; Rec."Code") { ApplicationArea = All; }
                field("New Start"; Rec."New Start") { ApplicationArea = All; }
            }
        }
    }

    trigger OnAfterGetRecord()
    begin
        SequenceEditable := Rec."New Start" = 0DT;
    end;

    var
        SequenceEditable: Boolean;
}

codeunit 50114 "Repro Plan Tests 50114"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure SequenceEditableFollowsTheRowGoToRecordMovesTo()
    var
        RowA: Record "Repro Plan Row 50112";
        RowB: Record "Repro Plan Row 50112";
        Worksheet: TestPage "Repro Plan Worksheet 50113";
        Seen: Text;
    begin
        RowA.DeleteAll();
        RowA."Code" := 'A';
        RowA."Sequence No." := 1;
        RowA."New Start" := CreateDateTime(DMY2Date(1, 1, 2030), 080000T);
        RowA.Insert();
        RowB."Code" := 'B';
        RowB."Sequence No." := 2;
        RowB.Insert();

        Worksheet.OpenEdit();
        Worksheet.GoToRecord(RowA);
        Seen := 'A:' + Format(Worksheet."Sequence No.".Editable());
        Worksheet.GoToRecord(RowB);
        Seen += ';B:' + Format(Worksheet."Sequence No.".Editable());
        Worksheet.Close();

        if Seen <> 'A:No;B:Yes' then
            Error('Editable of Sequence No. per row: %1', Seen);
    end;
}
"##;

/// al-runner issue 5809.
const FILTERED_GONE_ROW: &str = r##"
table 50120 "Repro Work Row 50120"
{
    fields
    {
        field(1; "User Security ID"; Guid) { }
        field(2; "Entry No."; Integer) { }
        field(3; Description; Text[50]) { }
    }
    keys
    {
        key(PK; "User Security ID", "Entry No.") { Clustered = true; }
    }
}

codeunit 50121 "Repro Work Log 50121"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Entries;
        Entries := '';
        exit(Result);
    end;
}

codeunit 50122 "Repro Work Apply 50122"
{
    procedure ApplyRowsOfUser(UserSecId: Guid)
    var
        Row: Record "Repro Work Row 50120";
    begin
        Row.SetRange("User Security ID", UserSecId);
        Row.DeleteAll();
    end;
}

page 50123 "Repro Work Worksheet 50123"
{
    PageType = Worksheet;
    SourceTable = "Repro Work Row 50120";
    ApplicationArea = All;
    UsageCategory = None;
    InsertAllowed = false;
    DelayedInsert = true;

    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field("Entry No."; Rec."Entry No.") { ApplicationArea = All; }
                field(Description; Rec.Description) { ApplicationArea = All; }
            }
        }
    }

    actions
    {
        area(Processing)
        {
            action(Apply)
            {
                ApplicationArea = All;
                Caption = 'Apply';

                trigger OnAction()
                var
                    WorkApply: Codeunit "Repro Work Apply 50122";
                begin
                    WorkApply.ApplyRowsOfUser(UserSecurityId());
                    CurrPage.Update(false);
                end;
            }
        }
    }

    trigger OnOpenPage()
    begin
        Rec.FilterGroup(2);
        Rec.SetRange("User Security ID", UserSecurityId());
        Rec.FilterGroup(0);
    end;

    trigger OnNewRecord(BelowxRec: Boolean)
    var
        WorkLog: Codeunit "Repro Work Log 50121";
    begin
        WorkLog.Add('N:' + Format(Rec."User Security ID" = UserSecurityId()));
    end;

    trigger OnAfterGetCurrRecord()
    var
        WorkLog: Codeunit "Repro Work Log 50121";
    begin
        WorkLog.Add('C:' + Format(Rec."Entry No.") + ':' + Format(Rec."User Security ID" = UserSecurityId()));
    end;
}

codeunit 50124 "Repro Work Tests 50124"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ActionThatAppliesTheOnlyRowOfAUserFilteredWorksheet()
    var
        Row: Record "Repro Work Row 50120";
        WorkLog: Codeunit "Repro Work Log 50121";
        Worksheet: TestPage "Repro Work Worksheet 50123";
        Seen: Text;
    begin
        Row.DeleteAll();
        Row."Entry No." := 1;
        Row."User Security ID" := UserSecurityId();
        Row.Description := 'Only row';
        Row.Insert();

        Worksheet.OpenEdit();
        WorkLog.Take();
        Worksheet.Apply.Invoke();
        Seen := WorkLog.Take();
        Worksheet.Close();

        if not Row.IsEmpty() then
            Error('The row is still there.');
        if Seen <> 'N:Yes;C:0:Yes;' then
            Error('Triggers after the action: %1', Seen);
    end;
}
"##;

/// al-runner issue 5811.
const RUN_PAGE_LINK_RANGE: &str = r##"
table 50132 "Repro Order Head 50132"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Status; Option) { OptionMembers = Planned,Released; }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 50133 "Repro Order Step 50133"
{
    fields
    {
        field(1; "Order No."; Code[20]) { }
        field(2; "Step No."; Code[10]) { }
        field(3; Status; Option) { OptionMembers = Planned,Released; }
    }
    keys
    {
        key(PK; "Order No.", "Step No.") { Clustered = true; }
    }
}

codeunit 50134 "Repro Order Log 50134"
{
    SingleInstance = true;

    var
        Entries: Text;

    procedure Add(Entry: Text)
    begin
        Entries += Entry + ';';
    end;

    procedure Take(): Text
    var
        Result: Text;
    begin
        Result := Entries;
        Entries := '';
        exit(Result);
    end;
}

page 50135 "Repro Order Steps 50135"
{
    PageType = List;
    SourceTable = "Repro Order Step 50133";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            repeater(Steps)
            {
                field("Order No."; Rec."Order No.")
                {
                    ApplicationArea = All;
                    Visible = OrderNoVisible;
                }
                field("Step No."; Rec."Step No.") { ApplicationArea = All; }
            }
        }
    }

    trigger OnOpenPage()
    var
        OrderLog: Codeunit "Repro Order Log 50134";
    begin
        OrderNoVisible := true;
        if Rec.GetFilter("Order No.") <> '' then begin
            OrderLog.Add('Min:' + Rec.GetRangeMin("Order No."));
            OrderLog.Add('Max:' + Rec.GetRangeMax("Order No."));
            OrderNoVisible := Rec.GetRangeMin("Order No.") <> Rec.GetRangeMax("Order No.");
        end;
        OrderLog.Add('Visible:' + Format(OrderNoVisible));
    end;

    var
        OrderNoVisible: Boolean;
}

page 50136 "Repro Order Card 50136"
{
    PageType = Card;
    SourceTable = "Repro Order Head 50132";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field("No."; Rec."No.") { ApplicationArea = All; }
        }
    }

    actions
    {
        area(Navigation)
        {
            action(Steps)
            {
                ApplicationArea = All;
                Caption = 'Steps';
                RunObject = page "Repro Order Steps 50135";
                RunPageLink = Status = field(Status),
                              "Order No." = field("No.");
            }
        }
    }
}

codeunit 50137 "Repro Order Tests 50137"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure GetRangeMinInOnOpenPageOfAPageOpenedThroughRunPageLink()
    var
        Head: Record "Repro Order Head 50132";
        Step: Record "Repro Order Step 50133";
        OrderLog: Codeunit "Repro Order Log 50134";
        Card: TestPage "Repro Order Card 50136";
        Steps: TestPage "Repro Order Steps 50135";
        Seen: Text;
        StepNo: Text;
    begin
        Step.DeleteAll();
        Head.DeleteAll();
        Head."No." := 'H1';
        Head.Status := Head.Status::Released;
        Head.Insert();
        Step."Order No." := 'H1';
        Step."Step No." := '10';
        Step.Status := Step.Status::Released;
        Step.Insert();
        OrderLog.Take();

        Card.OpenView();
        Card.GoToRecord(Head);
        Steps.Trap();
        Card.Steps.Invoke();
        StepNo := Steps."Step No.".Value();
        Steps.Close();
        Seen := OrderLog.Take();

        if StepNo <> '10' then
            Error('Step No. is <%1>.', StepNo);
        if Seen <> 'Min:H1;Max:H1;Visible:No;' then
            Error('OnOpenPage saw: %1', Seen);
    end;
}
"##;

#[test]
fn a_database_worksheet_onopenpage_fills_opens_in_lookup_mode_on_the_row_rec_was_left_on() {
    assert_eq!(
        outcome(
            WORKSHEET_FILLED_ON_OPEN,
            "Repro Plan Worksheet Tests",
            "WorksheetFilledByOnOpenPageOpensOnRowRecWasLeftOn"
        ),
        Outcome::Pass
    );
}

#[test]
fn a_database_worksheet_with_onnextrecord_whose_onopenpage_moved_rec_stays_refused() {
    let src = WORKSHEET_FILLED_ON_OPEN.replace(
        "    local procedure AddLine(",
        "    trigger OnNextRecord(Steps: Integer): Integer
    begin
        exit(Rec.Next(Steps));
    end;

    local procedure AddLine(",
    );
    let o = outcome(
        &src,
        "Repro Plan Worksheet Tests",
        "WorksheetFilledByOnOpenPageOpensOnRowRecWasLeftOn",
    );
    assert!(matches!(o, Outcome::NotRun(_)), "{o:?}");
}

#[test]
fn editable_of_a_repeater_control_follows_the_row_gotorecord_moves_to() {
    assert_eq!(
        outcome(
            REPEATER_EDITABLE,
            "Repro Plan Tests 50114",
            "SequenceEditableFollowsTheRowGoToRecordMovesTo"
        ),
        Outcome::Pass
    );
}

#[test]
fn a_blank_rec_after_the_only_row_is_gone_takes_a_key_field_filter_of_filter_group_2() {
    assert_eq!(
        outcome(
            FILTERED_GONE_ROW,
            "Repro Work Tests 50124",
            "ActionThatAppliesTheOnlyRowOfAUserFilteredWorksheet"
        ),
        Outcome::Pass
    );
}

#[test]
fn a_key_field_filter_in_another_filter_group_stays_refused() {
    let src = FILTERED_GONE_ROW.replace("Rec.FilterGroup(2);", "Rec.FilterGroup(4);");
    let o = outcome(
        &src,
        "Repro Work Tests 50124",
        "ActionThatAppliesTheOnlyRowOfAUserFilteredWorksheet",
    );
    assert!(matches!(o, Outcome::NotRun(_)), "{o:?}");
}

#[test]
fn a_key_field_filter_whose_field_validates_stays_refused() {
    let src = FILTERED_GONE_ROW.replace(
        r#"field(1; "User Security ID"; Guid) { }"#,
        r#"field(1; "User Security ID"; Guid) { trigger OnValidate() begin end; }"#,
    );
    let o = outcome(
        &src,
        "Repro Work Tests 50124",
        "ActionThatAppliesTheOnlyRowOfAUserFilteredWorksheet",
    );
    assert!(matches!(o, Outcome::NotRun(_)), "{o:?}");
}

#[test]
fn a_read_of_xrec_s_key_field_on_the_filtered_blank_rec_stays_refused() {
    // Whether xRec takes the filter value with Rec is not pinned (RtFilteredGoneRow.al reads only Rec).
    let src = FILTERED_GONE_ROW.replace(
        "WorkLog.Add('N:' + Format(Rec.\"User Security ID\" = UserSecurityId()));",
        "WorkLog.Add('N:' + Format(Rec.\"User Security ID\" = UserSecurityId()) + Format(xRec.\"User Security ID\" = UserSecurityId()));",
    );
    assert_ne!(src, FILTERED_GONE_ROW);
    let o = outcome(
        &src,
        "Repro Work Tests 50124",
        "ActionThatAppliesTheOnlyRowOfAUserFilteredWorksheet",
    );
    assert!(matches!(o, Outcome::NotRun(_)), "{o:?}");
}

#[test]
fn getrangemin_and_getrangemax_in_onopenpage_read_the_run_page_link_value() {
    assert_eq!(
        outcome(
            RUN_PAGE_LINK_RANGE,
            "Repro Order Tests 50137",
            "GetRangeMinInOnOpenPageOfAPageOpenedThroughRunPageLink"
        ),
        Outcome::Pass
    );
}
