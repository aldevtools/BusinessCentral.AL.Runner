//! al-runner issue 8068, the reporter's own repro (`repro8068/`, unchanged): `ReproParent.Delete(true)` from a test method
//! runs the OnDelete trigger of "Repro Parent", whose ModifyAll on "Repro Other" matches no row while the sets give R on
//! it. The reporter's run on BC 28.5 raised no error: it runs under assumption `bulk-write-no-match-in-delete-trigger` on
//! BC 28 only. Each neighbor below differs from it in one fact the reporter's run does not measure, and is refused.
//!
//! The fixture stands in for what the reporter's run took from Microsoft's apps: the Permissions Mock with its OnRun (the
//! test runner runs it around each test method), the All Objects set, and a "D365 BASIC" set that grants nothing on the
//! repro's tables.

use super::permission_writes::run_main_on_target;
use al_builtins::assumptions::Assumption;
use al_testrun::Outcome;
use al_testrun::isolation::TestIsolation;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const REPRO_APP: &str = include_str!("repro8068/Repro.al");
const REPRO_TEST: &str = include_str!("repro8068/Tests.Codeunit.al");
const REPRO: &str = "DeleteParentWhoseTriggerCallsModifyAllOnAnEmptyFilter";

const PLATFORM: &str = r#"
dotnet
{
    assembly("Microsoft.Dynamics.Nav.PermissionTestHelper")
    {
        type("Microsoft.Dynamics.Nav.Runtime.PermissionTestHelper"; "PermissionTestHelper")
        {
        }
    }
}

codeunit 131006 "Permissions Mock"
{
    SingleInstance = true;

    // The test runner runs this codeunit before and after each test method: it starts, then stops the mock.
    trigger OnRun()
    begin
        if not Started then Start() else Stop();
    end;

    var
        PermissionTestHelper: DotNet PermissionTestHelper;
        Started: Boolean;

    procedure Start()
    begin
        PermissionTestHelper := PermissionTestHelper.PermissionTestHelper();
        Started := true;
    end;

    procedure Stop()
    begin
        if not Started then
            exit;
        Started := false;
        PermissionTestHelper.Dispose();
    end;

    procedure ClearAssignments()
    begin
        if not Started then
            exit;
        PermissionTestHelper.Clear();
    end;

    procedure Assign(RoleID: Code[20])
    begin
        if not Started then
            exit;
        PermissionTestHelper.AddEffectivePermissionSet(RoleID);
    end;

    procedure Set(RoleID: Code[20])
    begin
        if not Started then
            exit;
        PermissionTestHelper.Clear();
        Assign('All Objects');
        Assign(RoleID);
    end;
}

permissionset 131006 "All Objects"
{
    Assignable = true;
    Permissions = table * = X, report * = X, codeunit * = X, page * = X, xmlport * = X, query * = X;
}

permissionset 50390 "D365 BASIC"
{
    Assignable = true;
}
"#;

/// Objects of the neighbor shapes: each parent's OnDelete differs from "Repro Parent"'s in one fact, or its target table
/// differs from "Repro Other" in one fact.
const NEIGHBOR_OBJECTS: &str = r#"
table 50310 "Nb Parent Key"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        ReproOther: Record "Repro Other";
    begin
        ReproOther.SetCurrentKey(Code);
        ReproOther.Ascending(false);
        ReproOther.SetRange("Parent Code", Code);
        ReproOther.ModifyAll("Parent Code", '');
    end;
}

table 50311 "Nb Parent Lock"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        ReproOther: Record "Repro Other";
    begin
        ReproOther.LockTable();
        ReproOther.SetRange("Parent Code", Code);
        ReproOther.ModifyAll("Parent Code", '');
    end;
}

table 50312 "Nb Parent Find"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        ReproOther: Record "Repro Other";
    begin
        ReproOther.SetRange("Parent Code", Code);
        if ReproOther.FindFirst() then;
        ReproOther.ModifyAll("Parent Code", '');
    end;
}

table 50313 "Nb Parent Filter Removed"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        ReproOther: Record "Repro Other";
    begin
        ReproOther.SetRange(Code, 'Z');
        ReproOther.SetRange(Code);
        ReproOther.SetRange("Parent Code", Code);
        ReproOther.ModifyAll("Parent Code", '');
    end;
}

table 50314 "Nb Other Keyed"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; "Parent Code"; Code[10]) { }
    }
    keys { key(PK; Code) { Clustered = true; } key(ByParent; "Parent Code") { } }
}

table 50315 "Nb Parent Keyed"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        Other: Record "Nb Other Keyed";
    begin
        Other.SetRange("Parent Code", Code);
        Other.ModifyAll("Parent Code", '');
    end;
}

table 50316 "Nb Other Wide"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; "Parent Code"; Code[20]) { }
    }
    keys { key(PK; Code) { Clustered = true; } }
}

table 50317 "Nb Parent Wide"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        Other: Record "Nb Other Wide";
    begin
        Other.SetRange("Parent Code", Code);
        Other.ModifyAll("Parent Code", '');
    end;
}

table 50318 "Nb Parent Literal"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        ReproOther: Record "Repro Other";
    begin
        ReproOther.SetRange("Parent Code", 'P1');
        ReproOther.ModifyAll("Parent Code", '');
    end;
}

table 50319 "Nb Parent Other Field"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; "Primary Child No."; Code[10]) { }
    }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        ReproOther: Record "Repro Other";
    begin
        ReproOther.SetRange("Parent Code", "Primary Child No.");
        ReproOther.ModifyAll("Parent Code", '');
    end;
}

table 50320 "Nb Parent Extended"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }

    trigger OnDelete()
    var
        ReproOther: Record "Repro Other";
    begin
        ReproOther.SetRange("Parent Code", Code);
        ReproOther.ModifyAll("Parent Code", '');
    end;
}

tableextension 50320 "Nb Parent Extension" extends "Nb Parent Extended"
{
    trigger OnDelete()
    begin
    end;
}

permissionset 50392 "Nb Edit 2"
{
    Assignable = true;
    Permissions = tabledata "Nb Parent Literal" = RIMD, tabledata "Nb Parent Other Field" = RIMD, tabledata "Nb Parent Extended" = RIMD;
}

permissionset 50391 "Nb Edit"
{
    Assignable = true;
    Permissions = tabledata "Nb Parent Key" = RIMD, tabledata "Nb Parent Lock" = RIMD, tabledata "Nb Parent Find" = RIMD,
        tabledata "Nb Parent Filter Removed" = RIMD, tabledata "Nb Parent Keyed" = RIMD, tabledata "Nb Parent Wide" = RIMD,
        tabledata "Nb Other Keyed" = R, tabledata "Nb Other Wide" = R;
}
"#;

/// Each test seeds as the reporter's test does (one parent row P1, one "Repro Other" row O1 with Parent Code X), then
/// changes one fact.
const NEIGHBORS: &str = r#"
codeunit 50410 "Nb Tests"
{
    Subtype = Test;

    var
        PermissionsMock: Codeunit "Permissions Mock";

    local procedure SeedOther()
    var
        ReproOther: Record "Repro Other";
    begin
        ReproOther.Code := 'O1';
        ReproOther."Parent Code" := 'X';
        ReproOther.Insert();
    end;

    local procedure Restrict()
    begin
        PermissionsMock.Set('D365 BASIC');
        PermissionsMock.Assign('Repro Parent Edit');
        PermissionsMock.Assign('Nb Edit');
        PermissionsMock.Assign('Nb Edit 2');
    end;

    // MED1: the trigger reached by DeleteAll(true).
    [Test]
    procedure TriggerReachedByDeleteAllTrue()
    var
        ReproParent: Record "Repro Parent";
    begin
        PermissionsMock.ClearAssignments();
        ReproParent.Code := 'P1';
        ReproParent.Insert();
        SeedOther();
        Restrict();
        ReproParent.SetRange(Code, 'P1');
        ReproParent.DeleteAll(true);
    end;

    // MED1: the trigger reached by RecordRef.Delete(true).
    [Test]
    procedure TriggerReachedByRecordRefDeleteTrue()
    var
        ReproParent: Record "Repro Parent";
        RecRef: RecordRef;
    begin
        PermissionsMock.ClearAssignments();
        ReproParent.Code := 'P1';
        ReproParent.Insert();
        SeedOther();
        Restrict();
        RecRef.GetTable(ReproParent);
        RecRef.Delete(true);
    end;

    // MED1: the trigger of a temporary parent.
    [Test]
    procedure TriggerOfATemporaryParent()
    var
        TempParent: Record "Repro Parent" temporary;
    begin
        PermissionsMock.ClearAssignments();
        TempParent.Code := 'P1';
        TempParent.Insert();
        SeedOther();
        Restrict();
        TempParent.Delete(true);
    end;

    // MED3: SetCurrentKey and Ascending(false) on the target before the filter.
    [Test]
    procedure TargetInDescendingOrder()
    var
        Parent: Record "Nb Parent Key";
    begin
        PermissionsMock.ClearAssignments();
        Parent.Code := 'P1';
        Parent.Insert();
        SeedOther();
        Restrict();
        Parent.Delete(true);
    end;

    // MED3: LockTable on the target first.
    [Test]
    procedure TargetLockedFirst()
    var
        Parent: Record "Nb Parent Lock";
    begin
        PermissionsMock.ClearAssignments();
        Parent.Code := 'P1';
        Parent.Insert();
        SeedOther();
        Restrict();
        Parent.Delete(true);
    end;

    // MED3: a FindFirst on the target first.
    [Test]
    procedure TargetReadFirst()
    var
        Parent: Record "Nb Parent Find";
    begin
        PermissionsMock.ClearAssignments();
        Parent.Code := 'P1';
        Parent.Insert();
        SeedOther();
        Restrict();
        Parent.Delete(true);
    end;

    // MED3: a filter set and removed on the target first.
    [Test]
    procedure TargetFilterSetAndRemoved()
    var
        Parent: Record "Nb Parent Filter Removed";
    begin
        PermissionsMock.ClearAssignments();
        Parent.Code := 'P1';
        Parent.Insert();
        SeedOther();
        Restrict();
        Parent.Delete(true);
    end;

    // MED4: the filter value is blank (a parent with a blank Code).
    [Test]
    procedure BlankFilterValue()
    var
        ReproParent: Record "Repro Parent";
    begin
        PermissionsMock.ClearAssignments();
        ReproParent.Code := '';
        ReproParent.Insert();
        SeedOther();
        Restrict();
        ReproParent.Delete(true);
    end;

    // MED4: the filter field is in a secondary key of the target.
    [Test]
    procedure FilterFieldInASecondaryKey()
    var
        Parent: Record "Nb Parent Keyed";
        Other: Record "Nb Other Keyed";
    begin
        PermissionsMock.ClearAssignments();
        Parent.Code := 'P1';
        Parent.Insert();
        Other.Code := 'O1';
        Other."Parent Code" := 'X';
        Other.Insert();
        Restrict();
        Parent.Delete(true);
    end;

    // MED4: the filter field is a Code[20].
    [Test]
    procedure FilterFieldOfAnotherLength()
    var
        Parent: Record "Nb Parent Wide";
        Other: Record "Nb Other Wide";
    begin
        PermissionsMock.ClearAssignments();
        Parent.Code := 'P1';
        Parent.Insert();
        Other.Code := 'O1';
        Other."Parent Code" := 'X';
        Other.Insert();
        Restrict();
        Parent.Delete(true);
    end;

    // MED5: a Commit between the seeding and Set.
    [Test]
    procedure CommitBeforeSet()
    var
        ReproParent: Record "Repro Parent";
    begin
        PermissionsMock.ClearAssignments();
        ReproParent.Code := 'P1';
        ReproParent.Insert();
        SeedOther();
        Commit();
        Restrict();
        ReproParent.Delete(true);
    end;

    // Round 2 B1: the SetRange value is a literal, not the trigger record's Code.
    [Test]
    procedure SetRangeWithALiteral()
    var
        Parent: Record "Nb Parent Literal";
    begin
        PermissionsMock.ClearAssignments();
        Parent.Code := 'P1';
        Parent.Insert();
        SeedOther();
        Restrict();
        Parent.Delete(true);
    end;

    // Round 2 B1: the SetRange value is another field of the trigger record.
    [Test]
    procedure SetRangeWithAnotherField()
    var
        Parent: Record "Nb Parent Other Field";
    begin
        PermissionsMock.ClearAssignments();
        Parent.Code := 'P1';
        Parent."Primary Child No." := 'Z';
        Parent.Insert();
        SeedOther();
        Restrict();
        Parent.Delete(true);
    end;

    // Round 2 F1: the parent has a table extension.
    [Test]
    procedure ParentWithATableExtension()
    var
        Parent: Record "Nb Parent Extended";
    begin
        PermissionsMock.ClearAssignments();
        Parent.Code := 'P1';
        Parent.Insert();
        SeedOther();
        Restrict();
        Parent.Delete(true);
    end;

    // Round 2 F2: the parent had a Delete(false) and an Insert before the Delete(true).
    [Test]
    procedure ParentDeletedAndInsertedBefore()
    var
        ReproParent: Record "Repro Parent";
    begin
        PermissionsMock.ClearAssignments();
        ReproParent.Code := 'P1';
        ReproParent.Insert();
        ReproParent.Delete(false);
        ReproParent.Insert();
        SeedOther();
        Restrict();
        ReproParent.Delete(true);
    end;

    // The target holds two rows.
    [Test]
    procedure TwoRowsInTheTarget()
    var
        ReproParent: Record "Repro Parent";
        ReproOther: Record "Repro Other";
    begin
        PermissionsMock.ClearAssignments();
        ReproParent.Code := 'P1';
        ReproParent.Insert();
        SeedOther();
        ReproOther.Code := 'O2';
        ReproOther."Parent Code" := 'X';
        ReproOther.Insert();
        Restrict();
        ReproParent.Delete(true);
    end;
}
"#;

fn target(major: u16) -> al_version::Target {
    al_version::Target::for_bc(al_version::BcVersion::new(major, 5))
}

fn run_on(major: Option<u16>) -> CorpusRuntime {
    let t = major.map(target);
    run_main_on_target(
        &format!("{REPRO_TEST}\n{NEIGHBORS}"),
        &format!("{PLATFORM}\n{REPRO_APP}\n{NEIGHBOR_OBJECTS}"),
        // Each neighbor seeds the rows the reporter's test seeds: the rows of one test are rolled back when it ends.
        &RunOptions {
            isolation: Some(TestIsolation::Function),
            ..RunOptions::default()
        },
        t.as_ref(),
    )
}

fn on_28() -> &'static CorpusRuntime {
    static RT: std::sync::OnceLock<CorpusRuntime> = std::sync::OnceLock::new();
    RT.get_or_init(|| run_on(Some(28)))
}

fn result<'a>(rt: &'a CorpusRuntime, test: &str) -> &'a al_testrun::runtime::TestResult {
    rt.results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
}

fn refused(rt: &CorpusRuntime, test: &str) {
    match &result(rt, test).outcome {
        Outcome::NotRun(why) => assert!(why.contains("a filter that matches no row"), "{test}: {why}"),
        other => panic!("{test}: {other:?}"),
    }
}

#[test]
fn the_reporters_repro_runs_flagged_on_bc_28() {
    let r = result(on_28(), REPRO);
    assert_eq!(r.outcome, Outcome::Pass);
    assert_eq!(r.assumptions, vec![Assumption::BulkWriteNoMatchInDeleteTrigger]);
}

#[test]
fn the_reporters_repro_is_refused_on_majors_it_was_not_run_on() {
    for major in [Some(27), Some(29), None] {
        refused(&run_on(major), REPRO);
    }
}

#[test]
fn a_trigger_reached_another_way_is_refused() {
    for test in [
        "TriggerReachedByDeleteAllTrue",
        "TriggerReachedByRecordRefDeleteTrue",
        "TriggerOfATemporaryParent",
    ] {
        refused(on_28(), test);
    }
}

#[test]
fn a_target_used_before_the_modify_all_is_refused() {
    for test in [
        "TargetInDescendingOrder",
        "TargetLockedFirst",
        "TargetReadFirst",
        "TargetFilterSetAndRemoved",
    ] {
        refused(on_28(), test);
    }
}

#[test]
fn another_filter_value_or_table_shape_is_refused() {
    for test in [
        "BlankFilterValue",
        "FilterFieldInASecondaryKey",
        "FilterFieldOfAnotherLength",
        "TwoRowsInTheTarget",
    ] {
        refused(on_28(), test);
    }
}

#[test]
fn a_parent_or_set_range_the_reporter_did_not_run_is_refused() {
    for test in [
        "SetRangeWithALiteral",
        "SetRangeWithAnotherField",
        "ParentWithATableExtension",
        "ParentDeletedAndInsertedBefore",
    ] {
        refused(on_28(), test);
    }
}

#[test]
fn a_commit_before_the_set_is_refused() {
    refused(on_28(), "CommitBeforeSet");
}
