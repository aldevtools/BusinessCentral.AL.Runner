//! `TestAction.Enabled()` right after a GoToKey on the List of al-runner issue 7916: the action's Enabled expression reads the
//! variables as OnAfterGetRecord left them for the row GoToKey landed on (assumption `action-enabled-follows-row-after-go-to-key`,
//! the reporter's repro on BC 29). Other shapes keep the variables' current value. The platform table Field is declared here by its
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
