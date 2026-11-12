//! `TestAction.Enabled()` right after a GoToKey on the List of al-runner issue 7916: the action's Enabled expression reads the
//! variables as OnAfterGetRecord left them for the row GoToKey landed on (assumption `action-enabled-follows-row-after-go-to-key`,
//! the reporter's repro on real BC). Other shapes keep the variables' current value. The platform table Field is declared here by its
//! field list, as the System app declares it (Options as Integer).

use al_builtins::assumptions::{Assumption, take_hits};
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 2000000041 "Field"
{
    fields
    {
        field(1; TableNo; Integer) { } field(2; "No."; Integer) { } field(3; TableName; Text[30]) { }
        field(4; FieldName; Text[30]) { } field(5; Type; Option) { OptionMembers = {OPT}; } field(6; Len; Integer) { } field(7; Class; Option) { OptionMembers = {OPT}; }
        field(8; Enabled; Boolean) { } field(9; "Type Name"; Text[30]) { } field(10; ExternalName; Text[100]) { }
        field(20; "Field Caption"; Text[80]) { } field(21; RelationTableNo; Integer) { } field(22; RelationFieldNo; Integer) { }
        field(23; SQLDataType; Option) { OptionMembers = {OPT}; } field(24; OptionString; Text[2047]) { } field(25; ObsoleteState; Option) { OptionMembers = {OPT}; }
        field(26; ObsoleteReason; Text[248]) { } field(27; DataClassification; Option) { OptionMembers = {OPT}; } field(28; IsPartOfPrimaryKey; Boolean) { }
        field(60; "App Package ID"; Guid) { } field(61; "App Runtime Package ID"; Guid) { } field(62; OptimizeForTextSearch; Boolean) { }
        field(63; Access; Option) { OptionMembers = {OPT}; } field(64; IsAllowedInCustomizations; Boolean) { }
    }
    keys { key(PK; TableNo, "No.") { } }
}
table 50310 "PX Target" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { } } }
table 50311 "PX Source"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; Rel; Code[10]) { TableRelation = "PX Target"; }
        field(3; Other; Code[10]) { }
    }
    keys { key(PK; Code) { } }
}
page 50313 "PX Enabled Walk"
{
    PageType = List;
    SourceTable = Field;
    Editable = false;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Rows)
            {
                field(NoCtl; Rec."No.") { ApplicationArea = All; }
                field(RelCtl; Rec.RelationTableNo) { ApplicationArea = All; }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(NextAction)
            {
                ApplicationArea = All;
                Enabled = NextEnabled;
                trigger OnAction()
                begin
                end;
            }
        }
    }
    trigger OnAfterGetRecord()
    begin
        NextEnabled := Rec.RelationTableNo <> 0;
    end;
    var
        NextEnabled: Boolean;
}
page 50317 "PX Blocked"
{
    PageType = List;
    SourceTable = Field;
    Editable = false;
    UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } field(RelCtl; Rec.RelationTableNo) { } } } }
    actions { area(Processing) { action(NextAction) { Enabled = NextEnabled and not Blocked; trigger OnAction() begin Blocked := true; end; } } }
    trigger OnAfterGetRecord() begin NextEnabled := Rec.RelationTableNo <> 0; end;
    var NextEnabled: Boolean; Blocked: Boolean;
}
codeunit 50309 "PX Flip" { SingleInstance = true; procedure Set(V: Boolean) begin F := V; end; procedure Get(): Boolean begin exit(F); end; var F: Boolean; }
page 50318 "PX Flipped"
{
    PageType = List;
    SourceTable = Field;
    Editable = false;
    UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } field(RelCtl; Rec.RelationTableNo) { } } } }
    actions { area(Processing) { action(NextAction) { Enabled = NextEnabled; trigger OnAction() begin end; } } }
    trigger OnAfterGetRecord() begin NextEnabled := (Rec.RelationTableNo <> 0) xor Flip.Get(); end;
    var NextEnabled: Boolean; Flip: Codeunit "PX Flip";
}
page 50321 "PX Counted"
{
    PageType = List;
    SourceTable = Field;
    Editable = false;
    UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } field(RelCtl; Rec.RelationTableNo) { } } } }
    actions { area(Processing) { action(NextAction) { Enabled = NextEnabled; trigger OnAction() begin end; } } }
    trigger OnAfterGetRecord() begin N += 1; NextEnabled := N mod 2 = 0; end;
    var NextEnabled: Boolean; N: Integer;
}
page 50322 "PX Action Sets"
{
    PageType = List;
    SourceTable = Field;
    Editable = false;
    UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } field(RelCtl; Rec.RelationTableNo) { } } } }
    actions { area(Processing) { action(NextAction) { Enabled = NextEnabled; trigger OnAction() begin NextEnabled := false; end; } } }
    trigger OnAfterGetRecord() begin NextEnabled := Rec.RelationTableNo <> 0; end;
    var NextEnabled: Boolean;
}
page 50323 "PX Reporter Walk"
{
    PageType = List;
    SourceTable = Field;
    Editable = false;
    UsageCategory = None;
    layout { area(Content) { repeater(Rows) { field(NoCtl; Rec."No.") { } field(RelCtl; Rec.RelationTableNo) { } } } }
    actions
    {
        area(Processing)
        {
            action(NextAction)
            {
                Enabled = NextEnabled;
                trigger OnAction()
                begin
                    Rec.SetRange(TableNo, Rec.RelationTableNo);
                    if Rec.FindFirst() then;
                    CurrPage.Update(false);
                end;
            }
        }
    }
    trigger OnAfterGetRecord() begin NextEnabled := Rec.RelationTableNo <> 0; end;
    var NextEnabled: Boolean;
}
codeunit 50312 T
{
    procedure Trapped(): Text
    var
        F: Record Field;
        Walk: TestPage "PX Enabled Walk";
        Steps: Text;
    begin
        F.SetRange(TableNo, 50311);
        Walk.Trap();
        Page.Run(Page::"PX Enabled Walk", F);
        Walk.GoToKey(50311, 2);
        Steps += Format(Walk.NextAction.Enabled()) + ' ';
        Walk.GoToKey(50311, 1);
        Steps += Format(Walk.NextAction.Enabled()) + ' ';
        Walk.GoToKey(50311, 2);
        Steps += Format(Walk.NextAction.Enabled());
        exit(Steps);
    end;

    procedure ReadBetween(): Text
    var
        F: Record Field;
        Walk: TestPage "PX Enabled Walk";
        Rel: Text;
    begin
        F.SetRange(TableNo, 50311);
        Walk.Trap();
        Page.Run(Page::"PX Enabled Walk", F);
        Walk.GoToKey(50311, 2);
        Rel := Walk.RelCtl.Value();
        exit(Format(Walk.NextAction.Enabled()));
    end;

    procedure OpenView(): Text
    var
        Walk: TestPage "PX Enabled Walk";
    begin
        Walk.OpenView();
        Walk.Filter.SetFilter(TableNo, '50311');
        Walk.GoToKey(50311, 2);
        exit(Format(Walk.NextAction.Enabled()));
    end;
    procedure ActionSetsOther(): Text
    var
        F: Record Field;
        Walk: TestPage "PX Blocked";
    begin
        F.SetRange(TableNo, 50311);
        Walk.Trap();
        Page.Run(Page::"PX Blocked", F);
        Walk.GoToKey(50311, 1);
        Walk.NextAction.Invoke();
        Walk.GoToKey(50311, 2);
        exit(Format(Walk.NextAction.Enabled()));
    end;

    procedure TriggerReadsOtherState(): Text
    var
        F: Record Field;
        Walk: TestPage "PX Flipped";
        Flip: Codeunit "PX Flip";
        Steps: Text;
    begin
        F.SetRange(TableNo, 50311);
        Walk.Trap();
        Page.Run(Page::"PX Flipped", F);
        Flip.Set(true);
        Walk.GoToKey(50311, 2);
        Steps := Format(Walk.NextAction.Enabled());
        Flip.Set(false);
        exit(Steps);
    end;

    procedure TriggerCounts(): Text
    var
        F: Record Field;
        Walk: TestPage "PX Counted";
        Steps: Text;
    begin
        F.SetRange(TableNo, 50311);
        Walk.Trap();
        Page.Run(Page::"PX Counted", F);
        Walk.GoToKey(50311, 2);
        Steps += Format(Walk.NextAction.Enabled()) + ' ';
        Walk.GoToKey(50311, 1);
        Steps += Format(Walk.NextAction.Enabled()) + ' ';
        Walk.GoToKey(50311, 2);
        Steps += Format(Walk.NextAction.Enabled());
        exit(Steps);
    end;

    procedure ActionSetsSameVariable(): Text
    var
        F: Record Field;
        Walk: TestPage "PX Action Sets";
    begin
        F.SetRange(TableNo, 50311);
        Walk.Trap();
        Page.Run(Page::"PX Action Sets", F);
        Walk.GoToKey(50311, 2);
        exit(Format(Walk.NextAction.Enabled()));
    end;

    procedure PageVariableRun(): Text
    var
        F: Record Field;
        Walk: TestPage "PX Enabled Walk";
        P: Page "PX Enabled Walk";
        Steps: Text;
    begin
        F.SetRange(TableNo, 50311);
        P.SetTableView(F);
        Walk.Trap();
        P.Run();
        Walk.GoToKey(50311, 2);
        Steps += Format(Walk.NextAction.Enabled()) + ' ';
        Walk.GoToKey(50311, 1);
        Steps += Format(Walk.NextAction.Enabled()) + ' ';
        Walk.GoToKey(50311, 2);
        Steps += Format(Walk.NextAction.Enabled());
        exit(Steps);
    end;

    procedure ReporterOnAction(): Text
    var
        F: Record Field;
        Walk: TestPage "PX Reporter Walk";
        Steps: Text;
    begin
        F.SetRange(TableNo, 50311);
        Walk.Trap();
        Page.Run(Page::"PX Reporter Walk", F);
        Walk.GoToKey(50311, 2);
        Steps += Format(Walk.NextAction.Enabled()) + ' ';
        Walk.GoToKey(50311, 1);
        Steps += Format(Walk.NextAction.Enabled()) + ' ';
        Walk.GoToKey(50311, 2);
        Steps += Format(Walk.NextAction.Enabled());
        exit(Steps);
    end;
}
"#;

/// The source with the Options of Field given enough members for the ordinals the platform rows hold.
fn src() -> String {
    let members: Vec<String> = (0..64).map(|i| format!("M{i}")).collect();
    SRC.replace("{OPT}", &members.join(","))
}

#[test]
fn enabled_after_go_to_key_reads_the_rows_variables() {
    take_hits();
    let r = run_in(&src(), "T", "Trapped");
    assert_eq!(text(r), "Yes No Yes");
    assert!(take_hits().contains(&Assumption::ActionEnabledFollowsRowAfterGoToKey));
}

/// A control read between the GoToKey and Enabled() is no call the repro makes: the current value stays, as on main.
#[test]
fn a_read_between_keeps_the_current_value() {
    take_hits();
    assert_eq!(text(run_in(&src(), "T", "ReadBetween")), "No");
    assert!(!take_hits().contains(&Assumption::ActionEnabledFollowsRowAfterGoToKey));
}

/// OpenView is not the repro's open: the current value stays, as on main.
#[test]
fn open_view_keeps_the_current_value() {
    take_hits();
    assert_eq!(text(run_in(&src(), "T", "OpenView")), "No");
    assert!(!take_hits().contains(&Assumption::ActionEnabledFollowsRowAfterGoToKey));
}

/// Runs `proc` and asserts its answer and that the path of the assumption did not run (the variables' current value, as on
/// main).
fn keeps_main(proc: &str, want: &str) {
    take_hits();
    assert_eq!(text(run_in(&src(), "T", proc)), want, "{proc}");
    assert!(
        !take_hits().contains(&Assumption::ActionEnabledFollowsRowAfterGoToKey),
        "{proc}"
    );
}

/// The Enabled expression reads a variable OnAction sets and OnAfterGetRecord does not: no row read resets it.
#[test]
fn a_variable_an_action_sets_keeps_the_current_value() {
    keeps_main("ActionSetsOther", "No");
}

/// OnAfterGetRecord reads state other than Rec (a SingleInstance codeunit the test changed after the open).
#[test]
fn a_trigger_that_reads_other_state_keeps_the_current_value() {
    keeps_main("TriggerReadsOtherState", "No");
}

/// OnAfterGetRecord counts its runs: the answer would depend on how often BC reads the rows.
#[test]
fn a_trigger_that_counts_its_runs_keeps_the_current_value() {
    keeps_main("TriggerCounts", "No No No");
}

/// The variable OnAfterGetRecord sets is also set by OnAction.
#[test]
fn a_variable_an_action_also_sets_keeps_the_current_value() {
    keeps_main("ActionSetsSameVariable", "No");
}

/// A Page variable's Run() (after SetTableView) is not the static Page.Run(Id, Rec) of the repro.
#[test]
fn a_page_variable_run_keeps_the_current_value() {
    keeps_main("PageVariableRun", "No No No");
}

/// The reporter's OnAction (SetRange, FindFirst, CurrPage.Update(false)) sets no page variable: the path runs.
#[test]
fn the_reporters_on_action_takes_the_path() {
    take_hits();
    assert_eq!(text(run_in(&src(), "T", "ReporterOnAction")), "Yes No Yes");
    assert!(take_hits().contains(&Assumption::ActionEnabledFollowsRowAfterGoToKey));
}
