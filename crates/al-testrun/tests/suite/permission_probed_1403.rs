//! The permission cells tests/al-language/permissions/testpermissions/Y19InherentAttrIndirect.al:153-185 and
//! Y19PermExtAndCompose.al:265-453 pin: each measured shape passes with no assumption marked, and each neighbor below differs
//! in one fact the probe did not measure and keeps main's refusal.

use super::permission_writes::{BASE, run_main_on_target};
use al_testrun::Outcome;
use al_testrun::runtime::{CorpusRuntime, RunOptions, TestResult};

const DEP: &str = r#"
table 52400 "PQ T"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Value; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

table 52401 "PQ B" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }

table 52402 "PQ E"
{
    DataClassification = SystemMetadata;
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

table 52403 "PQ E Perm"
{
    DataClassification = SystemMetadata;
    Permissions = tabledata "PQ B" = imd;
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }
}

tableextension 52400 "PQ X" extends "PQ E"
{
    trigger OnInsert()
    var
        B: Record "PQ B";
    begin
        B.Id := Rec.Id + 100;
        B.Insert();
    end;

    // A Modify from extension code: not measured.
    procedure ModifyB()
    var
        B: Record "PQ B";
    begin
        B.Get(1);
        B.Modify();
    end;
}

tableextension 52401 "PQ X Perm" extends "PQ E Perm"
{
    trigger OnInsert()
    var
        B: Record "PQ B";
    begin
        B.Id := Rec.Id + 100;
        B.Insert();
    end;
}

page 52400 "PQ Page"
{
    PageType = Card;
    SourceTable = "PQ E";
    layout { area(Content) { field(Id; Rec.Id) { ApplicationArea = All; } } }
}

pageextension 52400 "PQ PX" extends "PQ Page"
{
    trigger OnOpenPage()
    var
        B: Record "PQ B";
    begin
        B.Id := 300;
        B.Insert();
    end;
}

permissionset 52410 "PQ Only B" { Assignable = true; Permissions = tabledata "PQ B" = R; }
permissionset 52411 "PQ Ext Imd B"
{
    Assignable = true;
    Permissions = tabledata "PQ E" = RIMD, tabledata "PQ E Perm" = RIMD, tabledata "PQ B" = imd;
}
permissionset 52412 "PQ Ext No B"
{
    Assignable = true;
    Permissions = tabledata "PQ E" = RIMD, tabledata "PQ E Perm" = RIMD;
}

// Composition (Y19PermExtAndCompose.al:40-71).
permissionset 52413 "PQ Grant T" { Assignable = false; Permissions = tabledata "PQ T" = RIMD; }
permissionset 52414 "PQ Grant T Again" { Assignable = false; Permissions = tabledata "PQ T" = RIMD; }
permissionset 52415 "PQ Narrow"
{
    Assignable = false;
    Permissions = tabledata "PQ B" = R;
    ExcludedPermissionSets = "PQ Grant T";
}
permissionset 52416 "PQ Inc Narrow"
{
    Assignable = true;
    IncludedPermissionSets = "PQ Narrow", "PQ Grant T Again";
}
// The including set grants something itself: not measured.
permissionset 52417 "PQ Inc Narrow Own"
{
    Assignable = true;
    Permissions = tabledata "PQ E" = R;
    IncludedPermissionSets = "PQ Narrow", "PQ Grant T Again";
}
permissionset 52418 "PQ X Set" { Assignable = true; Permissions = tabledata "PQ T" = RIMD; }
permissionset 52419 "PQ Y Set"
{
    Assignable = true;
    Permissions = tabledata "PQ B" = R;
    ExcludedPermissionSets = "PQ X Set";
}
permissionset 52420 "PQ None" { Assignable = true; Permissions = tabledata "PQ B" = R; }

// Lower-case attribute letters (Y19InherentAttrIndirect.al:35-140).
codeunit 52421 "PQ IA With"
{
    Permissions = tabledata "PQ T" = rimd;

    procedure InsertOnWriter() begin InsertI(5); end;
    procedure ModifyOnWriter() begin ModifyM(); end;
    procedure InsertTwoLetters() begin InsertIM(6); end;
    procedure ReadPlain() begin GetPlain(); end;

    [InherentPermissions(PermissionObjectType::TableData, Database::"PQ T", 'i')]
    local procedure InsertI(NewId: Integer)
    var
        Row: Record "PQ T";
    begin
        Row.Id := NewId;
        Row.Insert();
    end;

    [InherentPermissions(PermissionObjectType::TableData, Database::"PQ T", 'm')]
    local procedure ModifyM()
    var
        Row: Record "PQ T";
    begin
        Row.Get(1);
        Row.Value := 7;
        Row.Modify();
    end;

    [InherentPermissions(PermissionObjectType::TableData, Database::"PQ T", 'im')]
    local procedure InsertIM(NewId: Integer)
    var
        Row: Record "PQ T";
    begin
        Row.Id := NewId;
        Row.Insert();
    end;

    local procedure GetPlain()
    var
        Row: Record "PQ T";
    begin
        Row.Get(1);
    end;
}

codeunit 52422 "PQ IA No"
{
    procedure InsertOnWriter() begin InsertI(5); end;

    [InherentPermissions(PermissionObjectType::TableData, Database::"PQ T", 'i')]
    local procedure InsertI(NewId: Integer)
    var
        Row: Record "PQ T";
    begin
        Row.Id := NewId;
        Row.Insert();
    end;
}
// Round-1 neighbors: each differs from a measured shape in one fact.
permissionset 52423 "PQ R T" { Assignable = true; Permissions = tabledata "PQ T" = R; }

// A table with a trigger, written under the letter 'i' (the measured table runs no trigger).
table 52404 "PQ Trig"
{
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }

    trigger OnInsert()
    begin
    end;
}

codeunit 52424 "PQ IA Trig"
{
    Permissions = tabledata "PQ Trig" = rimd;

    procedure InsertOnWriter() begin InsertI(5); end;

    [InherentPermissions(PermissionObjectType::TableData, Database::"PQ Trig", 'i')]
    local procedure InsertI(NewId: Integer)
    var
        Row: Record "PQ Trig";
    begin
        Row.Id := NewId;
        Row.Insert();
    end;
}

// The Permissions property grants the read, no attribute on the stack.
codeunit 52425 "PQ Plain"
{
    Permissions = tabledata "PQ T" = r;

    procedure NextOuter() begin NextInner(); end;
    procedure NoMatchFirstOuter() begin NoMatchFirstInner(); end;
    procedure FindEqOuter() begin FindEqInner(); end;
    procedure SysIdOuter(Id: Guid) begin SysIdInner(Id); end;

    local procedure NextInner()
    var
        Row: Record "PQ T";
    begin
        if Row.Next() = 0 then;
    end;

    local procedure NoMatchFirstInner()
    var
        Row: Record "PQ T";
    begin
        Row.SetRange(Id, 100);
        Row.FindFirst();
    end;

    local procedure FindEqInner()
    var
        Row: Record "PQ T";
    begin
        Row.Id := 1;
        Row.Find();
    end;

    local procedure SysIdInner(Id: Guid)
    var
        Row: Record "PQ T";
    begin
        Row.GetBySystemId(Id);
    end;
}

// A base table and a base page that declare their own trigger (the measured ones declare no routine).
table 52405 "PQ E Code"
{
    DataClassification = SystemMetadata;
    fields { field(1; Id; Integer) { } }
    keys { key(PK; Id) { Clustered = true; } }

    trigger OnInsert()
    begin
    end;
}

tableextension 52402 "PQ X Code" extends "PQ E Code"
{
    trigger OnInsert()
    var
        B: Record "PQ B";
    begin
        B.Id := Rec.Id + 100;
        B.Insert();
    end;
}

page 52401 "PQ Page Code"
{
    PageType = Card;
    SourceTable = "PQ E";
    layout { area(Content) { field(Id; Rec.Id) { ApplicationArea = All; } } }

    trigger OnOpenPage()
    begin
    end;
}

pageextension 52401 "PQ PX Code" extends "PQ Page Code"
{
    trigger OnOpenPage()
    var
        B: Record "PQ B";
    begin
        B.Id := 301;
        B.Insert();
    end;
}

permissionset 52426 "PQ Ext Code Imd B"
{
    Assignable = true;
    Permissions = tabledata "PQ E" = RIMD, tabledata "PQ E Code" = RIMD, tabledata "PQ B" = imd;
}

// An excluding included set that grants what it excludes.
permissionset 52427 "PQ Narrow Ov"
{
    Assignable = false;
    Permissions = tabledata "PQ T" = RIMD;
    ExcludedPermissionSets = "PQ Grant T";
}
permissionset 52428 "PQ Inc Ov"
{
    Assignable = true;
    IncludedPermissionSets = "PQ Narrow Ov", "PQ Grant T Again";
}
"#;

const MAIN: &str = r#"
codeunit 52430 "PQ Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    local procedure Seed(Rows: Integer)
    var
        T: Record "PQ T";
        B: Record "PQ B";
        i: Integer;
    begin
        T.DeleteAll();
        B.DeleteAll();
        for i := 1 to Rows do begin
            T.Id := i;
            T.Insert();
        end;
        B.Id := 1;
        B.Insert();
    end;

    local procedure Expect(Text: Text; Code: Text)
    begin
        Mock.Stop();
        if GetLastErrorText() <> Text then
            Error('text: %1', GetLastErrorText());
        if GetLastErrorCode() <> Code then
            Error('code: %1', GetLastErrorCode());
    end;

    // The reads run in a procedure of the test codeunit, as the probe's Op (Y19PermExtAndCompose.al:561).
    local procedure FindSetNoMatch()
    var
        T: Record "PQ T";
    begin
        T.SetRange(Id, 100);
        T.FindSet();
    end;

    local procedure FindLastAt(Value: Integer)
    var
        T: Record "PQ T";
    begin
        T.SetRange(Id, Value);
        T.FindLast();
    end;

    local procedure IsEmptyNoMatch(): Boolean
    var
        T: Record "PQ T";
    begin
        T.SetRange(Id, 100);
        exit(T.IsEmpty());
    end;

    [Test]
    procedure NoMatchFindSetDenied()
    begin
        Mock.Start();
        Seed(2);
        Mock.Set('PQ Only B');
        asserterror FindSetNoMatch();
        Expect('Sorry, the current permissions prevented the action. (TableData 52400 PQ T Read: main)', 'DB:ClientReadDenied');
    end;

    [Test]
    procedure NoMatchIsEmptyAllowed()
    begin
        Mock.Start();
        Seed(2);
        Mock.Set('PQ Only B');
        if not IsEmptyNoMatch() then
            Error('not empty');
        Mock.Stop();
    end;

    // The read in the test method itself (the probe read in a procedure below it).
    [Test]
    procedure NoMatchInTestMethod()
    var
        T: Record "PQ T";
    begin
        Mock.Start();
        Seed(2);
        Mock.Set('PQ Only B');
        T.SetRange(Id, 100);
        if T.FindSet() then;
    end;

    // Three rows (the probe seeded two).
    [Test]
    procedure NoMatchThreeRows()
    begin
        Mock.Start();
        Seed(3);
        Mock.Set('PQ Only B');
        FindSetNoMatch();
    end;

    // Another filter value (the probe filtered on 100).
    [Test]
    procedure NoMatchOtherValue()
    begin
        Mock.Start();
        Seed(2);
        Mock.Set('PQ Only B');
        FindLastAt(99);
    end;

    [Test]
    procedure LetterIWithPermissionsAllowed()
    var
        C: Codeunit "PQ IA With";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ None');
        C.InsertOnWriter();
        Mock.Stop();
    end;

    [Test]
    procedure LetterIWithoutPermissionsDenied()
    var
        C: Codeunit "PQ IA No";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ None');
        asserterror C.InsertOnWriter();
        Expect('Sorry, the current permissions prevented the action. (TableData 52400 PQ T Insert: base)', 'DB:ClientInsertDenied');
    end;

    [Test]
    procedure LetterMReadDeniedIndirect()
    var
        C: Codeunit "PQ IA With";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ None');
        asserterror C.ModifyOnWriter();
        Expect('Sorry, the current permissions prevented the action. (TableData 52400 PQ T IndirectRead: base)', 'DB:ClientReadDenied');
    end;

    // Two letters in one attribute: not measured.
    [Test]
    procedure TwoLetters()
    var
        C: Codeunit "PQ IA With";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ None');
        C.InsertTwoLetters();
    end;

    // A read denied to code whose Permissions grants it, no attribute on the stack: main named `Read`; not measured.
    [Test]
    procedure IndirectReadWithoutAttribute()
    var
        C: Codeunit "PQ IA With";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ None');
        asserterror C.ReadPlain();
    end;

    [Test]
    procedure IncludedExclusionStaysInSet()
    var
        T: Record "PQ T";
        B: Record "PQ B";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ Inc Narrow');
        T.FindFirst();
        B.FindFirst();
        Mock.Stop();
    end;

    [Test]
    procedure AssignedExclusionStaysInSet()
    var
        T: Record "PQ T";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ Y Set');
        Mock.Assign('PQ X Set');
        T.FindFirst();
        Mock.Stop();
    end;

    // The including set grants something itself: not measured, so the read of what the excluding set grants refuses.
    [Test]
    procedure IncludingSetWithOwnGrants()
    var
        B: Record "PQ B";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ Inc Narrow Own');
        B.FindFirst();
    end;

    [Test]
    procedure TableExtensionTriggerInsert()
    var
        E: Record "PQ E";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ Ext Imd B');
        E.Id := 2;
        asserterror E.Insert(true);
        Expect('Sorry, the current permissions prevented the action. (TableData 52401 PQ B Insert: base)', 'DB:ClientInsertDenied');
    end;

    // A Modify from table extension code: not measured.
    [Test]
    procedure TableExtensionModify()
    var
        E: Record "PQ E";
    begin
        Mock.Start();
        Seed(1);
        E.Id := 1;
        E.Insert();
        Mock.Set('PQ Ext Imd B');
        E.ModifyB();
    end;

    [Test]
    procedure PageExtensionOpenViewCloses()
    var
        P: TestPage "PQ Page";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ Ext Imd B');
        asserterror P.OpenView();
        Expect('Page PQ Page has to close.', 'FormAbort:Permission');
    end;

    // OpenEdit: not measured.
    [Test]
    procedure PageExtensionOpenEdit()
    var
        P: TestPage "PQ Page";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ Ext Imd B');
        P.OpenEdit();
    end;
    // The set grants Read on the table (the measured set gives nothing on it).
    [Test]
    procedure LetterISetGrantsRead()
    var
        C: Codeunit "PQ IA With";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ R T');
        C.InsertOnWriter();
    end;

    [Test]
    procedure LetterINoPermsSetGrantsRead()
    var
        C: Codeunit "PQ IA No";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ R T');
        C.InsertOnWriter();
    end;

    // The written table has a trigger.
    [Test]
    procedure LetterITableWithTrigger()
    var
        C: Codeunit "PQ IA Trig";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ None');
        C.InsertOnWriter();
    end;

    [Test]
    procedure IndirectNext()
    var
        C: Codeunit "PQ Plain";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ None');
        C.NextOuter();
    end;

    [Test]
    procedure IndirectNoMatchFindFirst()
    var
        C: Codeunit "PQ Plain";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ None');
        C.NoMatchFirstOuter();
    end;

    [Test]
    procedure IndirectFindEq()
    var
        C: Codeunit "PQ Plain";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ None');
        C.FindEqOuter();
    end;

    [Test]
    procedure IndirectGetBySystemId()
    var
        C: Codeunit "PQ Plain";
        T: Record "PQ T";
    begin
        Mock.Start();
        Seed(1);
        T.Get(1);
        Mock.Set('PQ None');
        C.SysIdOuter(T.SystemId);
    end;

    // The excluding included set grants what it excludes: not measured, so that set adds nothing (main's lower bound).
    [Test]
    procedure ExcludingSetGrantsWhatItExcludes()
    var
        T: Record "PQ T";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ Inc Ov');
        T.Id := 5;
        T.Insert();
    end;

    // The base table declares its own OnInsert.
    [Test]
    procedure TableExtensionBaseWithTrigger()
    var
        E: Record "PQ E Code";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ Ext Code Imd B');
        E.Id := 2;
        E.Insert(true);
    end;

    // The base page declares its own OnOpenPage.
    [Test]
    procedure PageExtensionBaseWithTrigger()
    var
        P: TestPage "PQ Page Code";
    begin
        Mock.Start();
        Seed(1);
        Mock.Set('PQ Ext Code Imd B');
        P.OpenView();
    end;
}

// A failing test, apart from the refused neighbors above (a failure after a refusal in one codeunit is not trusted).
codeunit 52431 "PQ Failing Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Mock: Codeunit "Permissions Mock";

    [Test]
    procedure TableExtensionTriggerInsertBasePermissions()
    var
        E: Record "PQ E Perm";
    begin
        Mock.Start();
        Mock.Set('PQ Ext No B');
        E.Id := 2;
        asserterror E.Insert(true);
        Mock.Stop();
        Error('%1|%2', GetLastErrorText(), GetLastErrorCode());
    end;

}
"#;

fn results(major: u16) -> &'static CorpusRuntime {
    static RT: [std::sync::OnceLock<CorpusRuntime>; 2] = [std::sync::OnceLock::new(), std::sync::OnceLock::new()];
    let i = usize::from(major == 29);
    RT[i].get_or_init(|| {
        let target = al_version::Target::for_bc(al_version::BcVersion::new(major, if major == 29 { 0 } else { 5 }));
        run_main_on_target(MAIN, &format!("{BASE}{DEP}"), &RunOptions::default(), Some(&target))
    })
}

fn result(major: u16, test: &str) -> &'static TestResult {
    results(major)
        .results()
        .iter()
        .find(|r| r.test == test)
        .unwrap_or_else(|| panic!("{test} did not run"))
}

fn assert_clean(major: u16, test: &str) {
    let r = result(major, test);
    assert_eq!(r.outcome, Outcome::Pass, "{test} on {major}");
    assert!(r.assumptions.is_empty(), "{test} on {major}: {:?}", r.assumptions);
}

fn assert_refused(major: u16, test: &str, reason: &str) {
    match &result(major, test).outcome {
        Outcome::NotRun(why) if why.contains(reason) => {}
        other => panic!("{test} on {major}: {other:?}"),
    }
}

#[test]
fn measured_no_match_reads_pass_and_neighbors_refuse() {
    assert_clean(28, "NoMatchFindSetDenied");
    assert_clean(28, "NoMatchIsEmptyAllowed");
    for test in ["NoMatchThreeRows", "NoMatchOtherValue", "NoMatchInTestMethod"] {
        assert_refused(28, test, "a read other than FindFirst that finds no row");
    }
}

#[test]
fn measured_indirect_letters_pass_and_neighbors_refuse() {
    for test in [
        "LetterIWithPermissionsAllowed",
        "LetterIWithoutPermissionsDenied",
        "LetterMReadDeniedIndirect",
    ] {
        assert_clean(28, test);
    }
    assert_refused(28, "TwoLetters", "an indirect [InherentPermissions] attribute");
    for test in [
        "LetterISetGrantsRead",
        "LetterINoPermsSetGrantsRead",
        "LetterITableWithTrigger",
    ] {
        assert_refused(28, test, "an indirect [InherentPermissions] attribute");
    }
    for test in [
        "IndirectReadWithoutAttribute",
        "IndirectNext",
        "IndirectNoMatchFindFirst",
        "IndirectFindEq",
        "IndirectGetBySystemId",
    ] {
        assert_refused(28, test, "by code whose Permissions property grants it");
    }
}

#[test]
fn measured_exclusions_stay_in_their_set_and_neighbors_refuse() {
    for test in ["IncludedExclusionStaysInSet", "AssignedExclusionStaysInSet"] {
        assert_clean(28, test);
    }
    assert_refused(28, "IncludingSetWithOwnGrants", "not fully composed");
    // The excluding included set adds nothing, as on main; the other included set grants the Insert, unflagged.
    let r = result(28, "ExcludingSetGrantsWhatItExcludes");
    assert_eq!((&r.outcome, r.assumptions.as_slice()), (&Outcome::Pass, &[][..]));
}

#[test]
fn extension_code_grants_follow_the_measured_majors() {
    for major in [28, 29] {
        assert_clean(major, "TableExtensionTriggerInsert");
        assert_clean(major, "PageExtensionOpenViewCloses");
        assert_refused(major, "TableExtensionModify", "which Permissions property covers");
        assert_refused(major, "PageExtensionOpenEdit", "which Permissions property covers");
        assert_refused(
            major,
            "TableExtensionBaseWithTrigger",
            "which Permissions property covers",
        );
        assert_refused(
            major,
            "PageExtensionBaseWithTrigger",
            "which Permissions property covers",
        );
    }
    // The base table's Permissions: IndirectInsert on 28 (the grant holds, the sets give nothing), Insert on 29.
    let text = |major| match &result(major, "TableExtensionTriggerInsertBasePermissions").outcome {
        Outcome::Fail(f) => f.clone(),
        other => panic!("{major}: {other:?}"),
    };
    assert!(
        text(28).contains("PQ B IndirectInsert: base)|DB:ClientInsertDenied"),
        "{}",
        text(28)
    );
    assert!(
        text(29).contains("PQ B Insert: base)|DB:ClientInsertDenied"),
        "{}",
        text(29)
    );
}
