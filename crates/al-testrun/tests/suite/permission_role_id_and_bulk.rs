//! Two permission reports of an outside reporter.
//!
//! al-runner issue 8067: the Role ID of a permission set is its whole upper-case name, up to 30 characters for a set that
//! is not assignable (corpus permissions/permission-sets/Y12PermSetNames.al:45). Sets whose names share their first 20
//! characters are different sets.
//!
//! al-runner issue 8068: a ModifyAll or DeleteAll whose filter matches no row of a table that holds rows is denied in the
//! shape record/Y19PermWrites.al:294-307 pins (Y19_P2_SetR, Y19_P2_SetRI) and in the shapes record/Y19PermNoMatchShapes.al and
//! record/Y19PermNoMatchTriggers.al pin (`permission_bulk_no_match_probed.rs`). The reporter's own repro and its neighbor shapes run in
//! `permission_bulk_no_match_repro.rs`. The no-match shapes here, on a target with a Code primary key or through a
//! RecordRef, are refused.

use super::permission_writes::{BASE, run_main_on};
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const DEP: &str = r#"
table 51600 "RI Item A" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { Clustered = true; } } }
table 51601 "RI Item B" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { Clustered = true; } } }

// The assigned set has 20 characters; the sets it names add " X1" / " X2" (23 characters). al-runner issue 8067.
permissionset 51600 "Repro Perm Set Aaaaa"
{
    Assignable = true;
    ExcludedPermissionSets = "Repro Perm Set Aaaaa X1";
    IncludedPermissionSets = "Repro Perm Set Aaaaa X2";
}

permissionset 51601 "Repro Perm Set Aaaaa X1"
{
    Assignable = false;
    Permissions = tabledata "RI Item B" = RIMD;
}

// Not assignable, 28 characters: its Role ID is the whole name (Y12PermSetNames.al:45), not its first 20 characters.
permissionset 51603 "RI Long Role Name Set Thirty"
{
    Assignable = false;
    Permissions = tabledata "RI Item A" = RIMD;
}

permissionset 51602 "Repro Perm Set Aaaaa X2"
{
    Assignable = false;
    Permissions = tabledata "RI Item A" = RIMD;
}

// Objects of the shape of al-runner issue 8068 (the reporter's own repro runs in `permission_bulk_no_match_repro.rs`).
table 51611 "Repro Other"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; "Parent Code"; Code[10]) { }
        field(3; Note; Code[10]) { }
    }
    keys { key(PK; Code) { Clustered = true; } }
}

// As "LT Y19 Row" of the pinned shape (Y19PermObjects.al:9-18).
table 51613 "RI Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Value; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

permissionset 51610 "Repro Parent Edit"
{
    Assignable = true;
    Permissions = tabledata "Repro Other" = R, tabledata "RI Row" = R;
}
"#;

const MAIN: &str = r#"
codeunit 51620 "RI Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        PermissionsMock: Codeunit "Permissions Mock";

    // al-runner issue 8067, the reporter's test: BC raises the plain Insert denial.
    [Test]
    procedure InsertDeniedUnderSetWithExcludedAndIncludedSets()
    var
        ItemB: Record "RI Item B";
    begin
        PermissionsMock.Start();
        PermissionsMock.Set('Repro Perm Set Aaaaa');
        ItemB.Code := 'B1';
        ItemB.Insert();
    end;

    // The included set's grant applies.
    [Test]
    procedure InsertAllowedByTheIncludedSet()
    var
        ItemA: Record "RI Item A";
    begin
        PermissionsMock.Start();
        PermissionsMock.Set('Repro Perm Set Aaaaa');
        ItemA.Code := 'A1';
        ItemA.Insert();
        PermissionsMock.Stop();
    end;

    // Set of the first 20 characters of a longer set's name: no set has that Role ID. Main took the longer set.
    [Test]
    procedure SetOfTheFirst20CharactersOfALongerName()
    var
        ItemA: Record "RI Item A";
    begin
        PermissionsMock.Start();
        PermissionsMock.Set('RI Long Role Name Se');
        ItemA.Code := 'A2';
        ItemA.Insert();
        PermissionsMock.Stop();
    end;

    local procedure SeedOther(Rows: Integer)
    var
        ReproOther: Record "Repro Other";
        i: Integer;
    begin
        ReproOther.DeleteAll();
        for i := 1 to Rows do begin
            ReproOther.Code := 'O' + Format(i);
            ReproOther."Parent Code" := 'X';
            ReproOther.Insert();
        end;
    end;

    // The reporter's ModifyAll made by the test codeunit itself, not by a trigger: not measured.
    [Test]
    procedure ModifyAllOfTheFilterFieldInTheTest()
    var
        ReproOther: Record "Repro Other";
    begin
        PermissionsMock.Start();
        SeedOther(1);
        PermissionsMock.Set('Repro Parent Edit');
        ReproOther.SetRange("Parent Code", 'P1');
        ReproOther.ModifyAll("Parent Code", '');
        PermissionsMock.Stop();
    end;

    // A non-key filter in the test codeunit, a field outside the filter written, two rows: the filter is not the pinned one.
    [Test]
    procedure DeleteAllOnANonKeyFilterInTheTest()
    var
        ReproOther: Record "Repro Other";
    begin
        PermissionsMock.Start();
        SeedOther(2);
        PermissionsMock.Set('Repro Parent Edit');
        ReproOther.SetRange("Parent Code", 'P1');
        asserterror ReproOther.ModifyAll(Note, 'N');
        PermissionsMock.Stop();
    end;

    // A filter that matches rows keeps main's denial.
    [Test]
    procedure ModifyAllMatchingRowsIsDenied()
    var
        ReproOther: Record "Repro Other";
    begin
        PermissionsMock.Start();
        SeedOther(1);
        PermissionsMock.Set('Repro Parent Edit');
        ReproOther.SetRange("Parent Code", 'X');
        asserterror ReproOther.ModifyAll("Parent Code", '');
        PermissionsMock.Stop();
        if GetLastErrorCode() <> 'DB:ClientModifyDenied' then
            Error('code: %1 %2', GetLastErrorCode(), GetLastErrorText());
    end;

    local procedure SeedRows()
    var
        Row: Record "RI Row";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Value := 10;
        Row.Insert();
        Row.Id := 2;
        Row.Value := 20;
        Row.Insert();
    end;

    // The pinned shape (Y19PermWrites.al:294-307) on two rows, from a local procedure of the test codeunit.
    [Test]
    procedure PinnedNoMatchOnTheKeyIsDenied()
    begin
        PermissionsMock.Start();
        SeedRows();
        PermissionsMock.Set('Repro Parent Edit');
        asserterror PinnedOp();
        PermissionsMock.Stop();
        if GetLastErrorText() <> 'Sorry, the current permissions prevented the action. (TableData 51613 RI Row Modify: main)' then
            Error('text: %1', GetLastErrorText());
        if GetLastErrorCode() <> 'DB:ClientModifyDenied' then
            Error('code: %1', GetLastErrorCode());
    end;

    local procedure PinnedOp()
    var
        Row: Record "RI Row";
    begin
        Row.SetRange(Id, 100);
        Row.ModifyAll(Value, 99);
    end;

    // The pinned shape through a RecordRef: not measured.
    [Test]
    procedure NoMatchDeleteAllThroughARecordRef()
    var
        RecRef: RecordRef;
    begin
        PermissionsMock.Start();
        SeedRows();
        PermissionsMock.Set('Repro Parent Edit');
        RecRef.Open(Database::"RI Row");
        RecRef.Field(1).SetRange(100);
        asserterror RecRef.DeleteAll();
        PermissionsMock.Stop();
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

fn refused(test: &str, reason: &str) {
    match &result(test).outcome {
        Outcome::NotRun(why) => assert!(why.contains(reason), "{test}: {why}"),
        other => panic!("{test}: {other:?}"),
    }
}

#[test]
fn sets_sharing_their_first_20_characters_are_different_sets() {
    match &result("InsertDeniedUnderSetWithExcludedAndIncludedSets").outcome {
        Outcome::Fail(msg) => assert!(
            msg.contains(
                "Sorry, the current permissions prevented the action. (TableData 51601 RI Item B Insert: main)"
            ),
            "{msg}"
        ),
        other => panic!("{other:?}"),
    }
    assert_eq!(result("InsertAllowedByTheIncludedSet").outcome, Outcome::Pass);
}

#[test]
fn a_set_of_the_first_20_characters_of_a_longer_name_is_refused() {
    refused(
        "SetOfTheFirst20CharactersOfALongerName",
        "has the Role ID 'RI LONG ROLE NAME SE'",
    );
}

#[test]
fn no_match_shapes_no_case_measures_are_refused() {
    for test in [
        "ModifyAllOfTheFilterFieldInTheTest",
        "DeleteAllOnANonKeyFilterInTheTest",
        "NoMatchDeleteAllThroughARecordRef",
    ] {
        refused(test, "a filter that matches no row");
    }
}

#[test]
fn the_pinned_no_match_shape_and_a_matching_filter_keep_the_denial() {
    assert_eq!(result("PinnedNoMatchOnTheKeyIsDenied").outcome, Outcome::Pass);
    assert_eq!(result("ModifyAllMatchingRowsIsDenied").outcome, Outcome::Pass);
    assert!(result("PinnedNoMatchOnTheKeyIsDenied").assumptions.is_empty());
}
