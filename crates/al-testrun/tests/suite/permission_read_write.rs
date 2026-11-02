//! ADR-062 stage 2: `ReadPermission` and `WritePermission` of a session whose permissions a test lowered.
//!
//! Values: corpus `tests/al-language/permissions/lt-y19-perm-probes/Y24PermReads.al` Y24_P19_ReadWritePermission (indirect
//! `r`, `i`, `im`, `imd`, `rimd`, inside an object whose Permissions names the table), and for the cases that file does not
//! pin (R alone, RI, RIM, RIMD direct, RecordRef; the `SetExactPermissionSet` variant needs Execute, stage 3) the hand-off of the Y75 probe
//! (`lt-y75-license/Y75PermReadWrite.al`, corpus PR 1031; BC 27.5, 28.5 and 29.0 agree). Issue 6756.

use super::permission_writes::run_main;
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions};

const MAIN: &str = r#"
table 50970 "RW Row"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50971 "RW Inherent"
{
    InherentPermissions = RX;
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 50972 "RW Temp Source"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50970 "RW Reader"
{
    procedure ReadWrite(): Text
    var
        Row: Record "RW Row";
    begin
        exit(Format(Row.ReadPermission()) + '/' + Format(Row.WritePermission()));
    end;
}

codeunit 50971 "RW Reader Perm R"
{
    Permissions = tabledata "RW Row" = r;
    procedure ReadWrite(): Text
    var
        Row: Record "RW Row";
    begin
        exit(Format(Row.ReadPermission()) + '/' + Format(Row.WritePermission()));
    end;
}

codeunit 50972 "RW Reader Perm RIMD"
{
    Permissions = tabledata "RW Row" = rimd;
    procedure ReadWrite(): Text
    var
        Row: Record "RW Row";
    begin
        exit(Format(Row.ReadPermission()) + '/' + Format(Row.WritePermission()));
    end;
}

permissionset 50970 "RW None" { Assignable = true; Permissions = tabledata "RW Temp Source" = R; }
permissionset 50971 "RW R" { Assignable = true; Permissions = tabledata "RW Row" = R; }
permissionset 50972 "RW RI" { Assignable = true; Permissions = tabledata "RW Row" = RI; }
permissionset 50973 "RW RIM" { Assignable = true; Permissions = tabledata "RW Row" = RIM; }
permissionset 50974 "RW I" { Assignable = true; Permissions = tabledata "RW Row" = I; }
permissionset 50975 "RW IM" { Assignable = true; Permissions = tabledata "RW Row" = IM; }
permissionset 50976 "RW IMD" { Assignable = true; Permissions = tabledata "RW Row" = IMD; }
permissionset 50977 "RW RIMD" { Assignable = true; Permissions = tabledata "RW Row" = RIMD; }
permissionset 50978 "RW Ind R" { Assignable = true; Permissions = tabledata "RW Row" = r; }
permissionset 50979 "RW Ind RIMD" { Assignable = true; Permissions = tabledata "RW Row" = rimd; }
permissionset 50980 "RW Inh R" { Assignable = true; Permissions = tabledata "RW Inherent" = R; }

codeunit 50973 "RW Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    local procedure Check(SetName: Code[20]; Expected: Text)
    var
        Row: Record "RW Row";
        RecRef: RecordRef;
        Reader: Codeunit "RW Reader";
    begin
        Mock.Start();
        Mock.ClearAssignments();
        Mock.Set(SetName);
        RecRef.Open(Database::"RW Row");
        if Format(Row.ReadPermission()) + '/' + Format(Row.WritePermission()) <> Expected then
            Error('Record %1: %2/%3', SetName, Row.ReadPermission(), Row.WritePermission());
        if Format(RecRef.ReadPermission()) + '/' + Format(RecRef.WritePermission()) <> Expected then
            Error('RecordRef %1: %2/%3', SetName, RecRef.ReadPermission(), RecRef.WritePermission());
        Mock.Stop();
    end;

    [Test]
    procedure NoGrant()
    begin
        Check('RW None', 'No/No');
    end;

    [Test]
    procedure ReadOnly()
    begin
        Check('RW R', 'Yes/No');
    end;

    [Test]
    procedure ReadInsertAndReadInsertModify()
    begin
        Check('RW RI', 'Yes/No');
        Check('RW RIM', 'Yes/No');
    end;

    [Test]
    procedure WriteLettersWithoutRead()
    begin
        Check('RW I', 'No/No');
        Check('RW IM', 'No/No');
        Check('RW IMD', 'No/Yes');
    end;

    [Test]
    procedure FullAccess()
    begin
        Check('RW RIMD', 'Yes/Yes');
    end;

    [Test]
    procedure IndirectSetAloneGrantsNothing()
    begin
        Check('RW Ind R', 'No/No');
        Check('RW Ind RIMD', 'No/No');
    end;

    [Test]
    procedure ObjectPermissionsCompleteAnIndirectSet()
    var
        Reader: Codeunit "RW Reader";
        PermR: Codeunit "RW Reader Perm R";
        PermRimd: Codeunit "RW Reader Perm RIMD";
    begin
        Mock.Start();
        Mock.Set('RW Ind R');
        if Reader.ReadWrite() <> 'No/No' then
            Error('no property: %1', Reader.ReadWrite());
        if PermR.ReadWrite() <> 'Yes/No' then
            Error('r: %1', PermR.ReadWrite());
        if PermRimd.ReadWrite() <> 'Yes/No' then
            Error('rimd over r: %1', PermRimd.ReadWrite());
        Mock.Set('RW Ind RIMD');
        if PermRimd.ReadWrite() <> 'Yes/Yes' then
            Error('rimd over rimd: %1', PermRimd.ReadWrite());
        if PermR.ReadWrite() <> 'Yes/No' then
            Error('r over rimd: %1', PermR.ReadWrite());
        Mock.Stop();
    end;

    [Test]
    procedure UnrestrictedSessionAnswersYes()
    var
        Row: Record "RW Row";
    begin
        if not (Row.ReadPermission() and Row.WritePermission()) then
            Error('unrestricted');
    end;

    // Issue 6756: the table's InherentPermissions = RX adds Read, so a set with R still cannot write.
    [Test]
    procedure InherentReadOnlyTableCannotBeWritten()
    var
        RecRef: RecordRef;
    begin
        Mock.Start();
        Mock.Set('RW Inh R');
        RecRef.Open(Database::"RW Inherent");
        if not RecRef.ReadPermission() then
            Error('read');
        if RecRef.WritePermission() then
            Error('write');
        Mock.Stop();
    end;

    // Not pinned: a temporary record under a denying set.
    [Test]
    procedure TemporaryRecordUnderADenyingSetIsRefused()
    var
        Row: Record "RW Row" temporary;
    begin
        Mock.Start();
        Mock.Set('RW None');
        if Row.ReadPermission() then;
        Mock.Stop();
    end;
}
"#;

fn results() -> &'static CorpusRuntime {
    static RT: std::sync::OnceLock<CorpusRuntime> = std::sync::OnceLock::new();
    RT.get_or_init(|| run_main(MAIN, &RunOptions::default()))
}

fn outcome(test: &str) -> Outcome {
    results()
        .results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
        .outcome
        .clone()
}

fn passes(test: &str) {
    assert_eq!(outcome(test), Outcome::Pass, "{test}");
}

#[test]
fn direct_grants_decide_read_and_write_permission() {
    for t in [
        "NoGrant",
        "ReadOnly",
        "ReadInsertAndReadInsertModify",
        "WriteLettersWithoutRead",
        "FullAccess",
        "UnrestrictedSessionAnswersYes",
    ] {
        passes(t);
    }
}

#[test]
fn indirect_grants_need_the_objects_permissions_property() {
    passes("IndirectSetAloneGrantsNothing");
    passes("ObjectPermissionsCompleteAnIndirectSet");
}

#[test]
fn inherent_permissions_merge_into_the_answer() {
    passes("InherentReadOnlyTableCannotBeWritten");
}

#[test]
fn a_temporary_record_under_a_denying_set_is_refused() {
    match outcome("TemporaryRecordUnderADenyingSetIsRefused") {
        Outcome::NotRun(why) => assert!(why.contains("not pinned"), "{why}"),
        other => panic!("{other:?}"),
    }
}
