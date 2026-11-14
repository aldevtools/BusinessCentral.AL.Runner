//! A ModifyAll or DeleteAll whose filter matches no row of a table with rows, in the shapes corpus PR 1403 measured
//! (record/Y19PermNoMatchShapes.al, record/Y19PermNoMatchTriggers.al; assumption `bulk-write-no-match-denied`): BC denies
//! it as one that matches rows. The OnDelete trigger of the reporter's shape (al-runner issue 8068) on a target with an
//! Integer primary key is denied, flagged; each neighbor below differs in one fact the probe did not measure and keeps
//! main's refusal.

use super::permission_writes::{BASE, run_main_on};
use al_builtins::assumptions::Assumption;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const DEP: &str = r#"
// As "LT NM Target" (Y19PermNoMatchShapes.al:15-26), with a Code[30] field the probe did not measure.
table 51700 "NP Target"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Parent Code"; Code[10]) { }
        field(3; Value; Integer) { }
        field(4; Wide; Code[30]) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

// As "LT NM Parent" (Y19PermNoMatchShapes.al:29-42).
table 51701 "NP Parent"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        Target: Record "NP Target";
    begin
        Target.SetRange("Parent Code", Code);
        Target.ModifyAll("Parent Code", '');
    end;
}

// The written value is 'Z' (the probe wrote '' and 'Y').
table 51702 "NP Parent Value"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        Target: Record "NP Target";
    begin
        Target.SetRange("Parent Code", Code);
        Target.ModifyAll("Parent Code", 'Z');
    end;
}

// The filter field is a Code[30].
table 51703 "NP Parent Wide"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        Target: Record "NP Target";
    begin
        Target.SetRange(Wide, Code);
        Target.ModifyAll(Wide, '');
    end;
}

// The write is in OnModify, reached by Modify(true).
table 51704 "NP Parent Modify"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnModify()
    var
        Target: Record "NP Target";
    begin
        Target.SetRange("Parent Code", Code);
        Target.ModifyAll("Parent Code", '');
    end;
}

table 51705 "NP Parent Sub"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

// A subscriber to OnAfterDeleteEvent (the probe measured OnBeforeDeleteEvent, Y19PermNoMatchTriggers.al:146-156).
codeunit 51706 "NP After Delete Subscriber"
{
    [EventSubscriber(ObjectType::Table, Database::"NP Parent Sub", 'OnAfterDeleteEvent', '', false, false)]
    local procedure OnAfterDeleteParent(var Rec: Record "NP Parent Sub"; RunTrigger: Boolean)
    var
        Target: Record "NP Target";
    begin
        Target.SetRange("Parent Code", Rec.Code);
        Target.ModifyAll("Parent Code", '');
    end;
}

// A codeunit of the app with a property (the probe's had none, Y19PermNoMatchShapes.al:93-110).
codeunit 51707 "NP Single Writer"
{
    SingleInstance = true;

    procedure ClearParentCode(ParentCode: Code[10])
    var
        Target: Record "NP Target";
    begin
        Target.SetRange("Parent Code", ParentCode);
        Target.ModifyAll("Parent Code", '');
    end;
}

permissionset 51708 "NP R"
{
    Assignable = true;
    Permissions = tabledata "NP Target" = R, tabledata "NP Parent" = RIMD, tabledata "NP Parent Value" = RIMD,
        tabledata "NP Parent Wide" = RIMD, tabledata "NP Parent Modify" = RIMD, tabledata "NP Parent Sub" = RIMD;
}

// R and D on the target: the probe measured R and RI.
permissionset 51709 "NP RD"
{
    Assignable = true;
    Permissions = tabledata "NP Target" = RD, tabledata "NP Parent" = RIMD;
}
"#;

const MAIN: &str = r#"
codeunit 51720 "NP Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        PermissionsMock: Codeunit "Permissions Mock";

    local procedure Seed(Rows: Integer)
    var
        Target: Record "NP Target";
        i: Integer;
    begin
        Target.DeleteAll();
        for i := 1 to Rows do begin
            Target.Id := i;
            Target."Parent Code" := 'X';
            Target.Wide := 'X';
            Target.Insert();
        end;
    end;

    local procedure Expect(Text: Text; Code: Text)
    begin
        PermissionsMock.Stop();
        if GetLastErrorText() <> Text then
            Error('text: %1', GetLastErrorText());
        if GetLastErrorCode() <> Code then
            Error('code: %1', GetLastErrorCode());
    end;

    // Y19PermNoMatchShapes.al op 11 on one row: denied.
    [Test]
    procedure TriggerDenied()
    var
        Parent: Record "NP Parent";
    begin
        PermissionsMock.Start();
        Seed(1);
        Parent.Code := 'P1';
        Parent.Insert();
        PermissionsMock.Set('NP R');
        asserterror Parent.Delete(true);
        Expect('Sorry, the current permissions prevented the action. (TableData 51700 NP Target Modify: base)', 'DB:ClientModifyDenied');
    end;

    // Y19PermNoMatchShapes.al op 6 on two rows: DeleteAll denied with Delete.
    [Test]
    procedure DeleteAllInTheTestDenied()
    var
        Target: Record "NP Target";
    begin
        PermissionsMock.Start();
        Seed(2);
        PermissionsMock.Set('NP R');
        Target.SetRange("Parent Code", 'P1');
        asserterror Target.DeleteAll();
        Expect('Sorry, the current permissions prevented the action. (TableData 51700 NP Target Delete: main)', 'DB:ClientDeleteDenied');
    end;

    // Three rows (the probe seeded one or two).
    [Test]
    procedure ThreeRows()
    var
        Parent: Record "NP Parent";
    begin
        PermissionsMock.Start();
        Seed(3);
        Parent.Code := 'P1';
        Parent.Insert();
        PermissionsMock.Set('NP R');
        Parent.Delete(true);
    end;

    [Test]
    procedure WrittenValueNotMeasured()
    var
        Parent: Record "NP Parent Value";
    begin
        PermissionsMock.Start();
        Seed(1);
        Parent.Code := 'P1';
        Parent.Insert();
        PermissionsMock.Set('NP R');
        Parent.Delete(true);
    end;

    [Test]
    procedure FilterFieldCode30()
    var
        Parent: Record "NP Parent Wide";
    begin
        PermissionsMock.Start();
        Seed(1);
        Parent.Code := 'P1';
        Parent.Insert();
        PermissionsMock.Set('NP R');
        Parent.Delete(true);
    end;

    [Test]
    procedure OnModifyTrigger()
    var
        Parent: Record "NP Parent Modify";
    begin
        PermissionsMock.Start();
        Seed(1);
        Parent.Code := 'P1';
        Parent.Insert();
        PermissionsMock.Set('NP R');
        Parent.Modify(true);
    end;

    [Test]
    procedure OnAfterDeleteSubscriber()
    var
        Parent: Record "NP Parent Sub";
    begin
        PermissionsMock.Start();
        Seed(1);
        Parent.Code := 'P1';
        Parent.Insert();
        PermissionsMock.Set('NP R');
        Parent.Delete(true);
    end;

    [Test]
    procedure AppCodeunitWithAProperty()
    var
        Writer: Codeunit "NP Single Writer";
    begin
        PermissionsMock.Start();
        Seed(1);
        PermissionsMock.Set('NP R');
        Writer.ClearParentCode('P1');
    end;

    [Test]
    procedure SetsGrantReadAndDelete()
    var
        Parent: Record "NP Parent";
    begin
        PermissionsMock.Start();
        Seed(1);
        Parent.Code := 'P1';
        Parent.Insert();
        PermissionsMock.Set('NP RD');
        Parent.Delete(true);
    end;
}
"#;

fn results() -> &'static CorpusRuntime {
    static RT: std::sync::OnceLock<CorpusRuntime> = std::sync::OnceLock::new();
    RT.get_or_init(|| run_main_on(MAIN, &format!("{BASE}{DEP}"), &RunOptions::default()))
}

fn result(test: &str) -> &'static al_testrun::runtime::TestResult {
    results()
        .results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
}

#[test]
fn measured_no_match_shapes_are_denied_flagged() {
    for test in ["TriggerDenied", "DeleteAllInTheTestDenied"] {
        let r = result(test);
        assert_eq!(r.outcome, Outcome::Pass, "{test}");
        assert_eq!(r.assumptions, vec![Assumption::BulkWriteNoMatchDenied], "{test}");
    }
}

#[test]
fn neighbors_the_probe_did_not_measure_keep_the_refusal() {
    for test in [
        "ThreeRows",
        "WrittenValueNotMeasured",
        "FilterFieldCode30",
        "OnModifyTrigger",
        "OnAfterDeleteSubscriber",
        "AppCodeunitWithAProperty",
        "SetsGrantReadAndDelete",
    ] {
        match &result(test).outcome {
            Outcome::NotRun(why) => assert!(why.contains("a filter that matches no row"), "{test}: {why}"),
            other => panic!("{test}: {other:?}"),
        }
    }
}
