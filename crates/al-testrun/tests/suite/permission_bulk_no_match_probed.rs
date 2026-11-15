//! A ModifyAll or DeleteAll whose filter matches no row of a table with rows, in the shapes
//! tests/al-language/record/Y19PermNoMatchShapes.al:139-269 and record/Y19PermNoMatchTriggers.al:180-303 pin: BC denies
//! it as one that matches rows. The OnDelete trigger of the reporter's shape (al-runner issue 8068) on a target with an
//! Integer primary key is denied, unmarked; each neighbor below differs in one fact the probe did not measure and keeps
//! main's refusal.

use super::permission_writes::{BASE, run_main_on};
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
        field(5; FF; Code[10]) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

// As "NP Target", shared by all companies (the probe's tables were per company).
table 51710 "NP Shared Target"
{
    DataPerCompany = false;
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Parent Code"; Code[10]) { }
        field(3; Value; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

// The test codeunit's own subscriber is raised by its Insert(true).
table 51711 "NP Parent Ins"
{
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
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

// The probe's sets granted only what Mock.Set assigned: All Objects and one set. This one stands for the reporter's
// D365 BASIC, assigned before the set of the target (al-runner issue 8068).
permissionset 51712 "NP Other"
{
    Assignable = true;
    Permissions = tabledata "NP Parent Ins" = R;
}

// R and D on the target: the probe measured R and RI.
permissionset 51709 "NP RD"
{
    Assignable = true;
    Permissions = tabledata "NP Target" = RD, tabledata "NP Parent" = RIMD;
}

permissionset 51713 "NP R Wide"
{
    Assignable = true;
    Permissions = tabledata "NP Target" = R, tabledata "NP Shared Target" = R, tabledata "NP Parent Ins" = RIMD;
}
"#;

const MAIN: &str = r#"
// The set of the target declared by another app than the target's (the probe's sets were declared with the table).
permissionset 51730 "NP Main R"
{
    Assignable = true;
    Permissions = tabledata "NP Target" = R;
}

codeunit 51720 "NP Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        PermissionsMock: Codeunit "Permissions Mock";
        HandlerWritesPinned: Boolean;

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

    // The pinned shape (record/Y19PermWrites.al:294-307) and a measured shape in a TryFunction of the test codeunit.
    [Test]
    procedure PinnedShapeInTryFunction()
    begin
        PermissionsMock.Start();
        Seed(2);
        PermissionsMock.Set('NP R');
        if not TryWritePinned() then
            Error(GetLastErrorText());
    end;

    [Test]
    procedure MeasuredShapeInTryFunction()
    begin
        PermissionsMock.Start();
        Seed(1);
        PermissionsMock.Set('NP R');
        if not TryWriteMeasured() then
            Error(GetLastErrorText());
    end;

    [TryFunction]
    local procedure TryWritePinned()
    var
        Target: Record "NP Target";
    begin
        Target.SetRange(Id, 100);
        Target.ModifyAll(Value, 99);
    end;

    [TryFunction]
    local procedure TryWriteMeasured()
    var
        Target: Record "NP Target";
    begin
        Target.SetRange("Parent Code", 'P1');
        Target.ModifyAll("Parent Code", '');
    end;

    // Both shapes in a MessageHandler of the test codeunit.
    [Test]
    [HandlerFunctions('MsgHandler')]
    procedure PinnedShapeInMessageHandler()
    begin
        PermissionsMock.Start();
        Seed(2);
        PermissionsMock.Set('NP R');
        HandlerWritesPinned := true;
        Message('go');
    end;

    [Test]
    [HandlerFunctions('MsgHandler')]
    procedure MeasuredShapeInMessageHandler()
    begin
        PermissionsMock.Start();
        Seed(1);
        PermissionsMock.Set('NP R');
        HandlerWritesPinned := false;
        Message('go');
    end;

    [MessageHandler]
    procedure MsgHandler(Msg: Text[1024])
    var
        Target: Record "NP Target";
    begin
        if HandlerWritesPinned then begin
            Target.SetRange(Id, 100);
            Target.ModifyAll(Value, 99);
        end else begin
            Target.SetRange("Parent Code", 'P1');
            Target.ModifyAll("Parent Code", '');
        end;
    end;

    // Both shapes in the test codeunit's own subscriber, raised by Insert(true).
    [Test]
    procedure PinnedShapeInOwnSubscriber()
    var
        Parent: Record "NP Parent Ins";
    begin
        PermissionsMock.Start();
        Seed(2);
        PermissionsMock.Set('NP R Wide');
        Parent.Code := 'Q9';
        Parent.Insert(true);
    end;

    [Test]
    procedure MeasuredShapeInOwnSubscriber()
    var
        Parent: Record "NP Parent Ins";
    begin
        PermissionsMock.Start();
        Seed(1);
        PermissionsMock.Set('NP R Wide');
        Parent.Code := 'P9';
        Parent.Insert(true);
    end;

    [EventSubscriber(ObjectType::Table, Database::"NP Parent Ins", 'OnAfterInsertEvent', '', false, false)]
    local procedure OnAfterInsertParent(var Rec: Record "NP Parent Ins"; RunTrigger: Boolean)
    var
        Target: Record "NP Target";
    begin
        if Rec.Code = 'Q9' then begin
            Target.SetRange(Id, 100);
            Target.ModifyAll(Value, 99);
        end;
        if Rec.Code = 'P9' then begin
            Target.SetRange("Parent Code", 'P1');
            Target.ModifyAll("Parent Code", '');
        end;
    end;

    // The reporter's sets: Set of another set, then Assign of the target's set (al-runner issue 8068).
    [Test]
    procedure ReporterSetsInTrigger()
    var
        Parent: Record "NP Parent";
    begin
        PermissionsMock.Start();
        Seed(1);
        Parent.Code := 'P1';
        Parent.Insert();
        PermissionsMock.Set('NP Other');
        PermissionsMock.Assign('NP R');
        Parent.Delete(true);
    end;

    [Test]
    procedure ReporterSetsInTest()
    var
        Target: Record "NP Target";
    begin
        PermissionsMock.Start();
        Seed(1);
        PermissionsMock.Set('NP Other');
        PermissionsMock.Assign('NP R');
        Target.SetRange("Parent Code", 'P1');
        Target.DeleteAll();
    end;

    [Test]
    procedure SetOfAnotherApp()
    var
        Target: Record "NP Target";
    begin
        PermissionsMock.Start();
        Seed(1);
        PermissionsMock.Set('NP Main R');
        Target.SetRange("Parent Code", 'P1');
        Target.DeleteAll();
    end;

    [Test]
    procedure FlowFilterFilter()
    var
        Target: Record "NP Target";
    begin
        PermissionsMock.Start();
        Seed(1);
        PermissionsMock.Set('NP R');
        Target.SetRange(FF, 'P1');
        Target.ModifyAll("Parent Code", '');
    end;

    [Test]
    procedure TableSharedByCompanies()
    var
        Target: Record "NP Shared Target";
    begin
        PermissionsMock.Start();
        Target.DeleteAll();
        Target.Id := 1;
        Target."Parent Code" := 'X';
        Target.Insert();
        PermissionsMock.Set('NP R Wide');
        Target.SetRange("Parent Code", 'P1');
        Target.DeleteAll();
    end;

    // Integer values the probe did not measure (it filtered Id to 100 and wrote 99).
    [Test]
    procedure IntegerFilterValueNotMeasured()
    var
        Target: Record "NP Target";
    begin
        PermissionsMock.Start();
        Seed(1);
        PermissionsMock.Set('NP R');
        Target.SetRange(Id, 101);
        Target.ModifyAll(Value, 99);
    end;

    [Test]
    procedure IntegerWrittenValueNotMeasured()
    var
        Target: Record "NP Target";
    begin
        PermissionsMock.Start();
        Seed(1);
        PermissionsMock.Set('NP R');
        Target.SetRange(Id, 100);
        Target.ModifyAll(Value, 98);
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

// A test codeunit that binds its own subscriber (the probe's subscriber was in a codeunit of the app).
codeunit 51721 "NP Bound Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;
    EventSubscriberInstance = Manual;

    var
        PermissionsMock: Codeunit "Permissions Mock";

    [Test]
    procedure OwnBoundSubscriber()
    var
        Me: Codeunit "NP Bound Tests";
        Target: Record "NP Target";
        Parent: Record "NP Parent Sub";
    begin
        PermissionsMock.Start();
        Target.DeleteAll();
        Target.Id := 1;
        Target."Parent Code" := 'X';
        Target.Insert();
        Parent.Code := 'P1';
        Parent.Insert();
        BindSubscription(Me);
        PermissionsMock.Set('NP R');
        Parent.Delete(true);
    end;

    [EventSubscriber(ObjectType::Table, Database::"NP Parent Sub", 'OnBeforeDeleteEvent', '', false, false)]
    local procedure OnBeforeDeleteParent(var Rec: Record "NP Parent Sub"; RunTrigger: Boolean)
    var
        Target: Record "NP Target";
    begin
        Target.SetRange("Parent Code", Rec.Code);
        Target.ModifyAll("Parent Code", '');
    end;
}

// A test codeunit without TestPermissions (the reporter's had none, al-runner issue 8068).
codeunit 51722 "NP Default Perm Tests"
{
    Subtype = Test;

    var
        PermissionsMock: Codeunit "Permissions Mock";

    [Test]
    procedure TriggerWithoutTestPermissionsDisabled()
    var
        Target: Record "NP Target";
        Parent: Record "NP Parent";
    begin
        PermissionsMock.Start();
        Target.DeleteAll();
        Target.Id := 1;
        Target."Parent Code" := 'X';
        Target.Insert();
        Parent.Code := 'P1';
        Parent.Insert();
        PermissionsMock.Set('NP R');
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
fn measured_no_match_shapes_are_denied_unmarked() {
    for test in ["TriggerDenied", "DeleteAllInTheTestDenied"] {
        let r = result(test);
        assert_eq!(r.outcome, Outcome::Pass, "{test}");
        assert!(r.assumptions.is_empty(), "{test}: {:?}", r.assumptions);
    }
}

#[test]
fn neighbors_the_probe_did_not_measure_keep_the_refusal() {
    let mut wrong = Vec::new();
    for test in [
        "ThreeRows",
        "WrittenValueNotMeasured",
        "FilterFieldCode30",
        "OnModifyTrigger",
        "OnAfterDeleteSubscriber",
        "AppCodeunitWithAProperty",
        "SetsGrantReadAndDelete",
        "PinnedShapeInTryFunction",
        "MeasuredShapeInTryFunction",
        "PinnedShapeInMessageHandler",
        "MeasuredShapeInMessageHandler",
        "PinnedShapeInOwnSubscriber",
        "MeasuredShapeInOwnSubscriber",
        "OwnBoundSubscriber",
        "ReporterSetsInTrigger",
        "ReporterSetsInTest",
        "SetOfAnotherApp",
        "TriggerWithoutTestPermissionsDisabled",
        "FlowFilterFilter",
        "TableSharedByCompanies",
        "IntegerFilterValueNotMeasured",
        "IntegerWrittenValueNotMeasured",
    ] {
        match &result(test).outcome {
            Outcome::NotRun(why) if why.contains("a filter that matches no row") => {}
            other => wrong.push(format!("{test}: {other:?}")),
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");
}
