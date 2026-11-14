// BC Documentation: https://learn.microsoft.com/en-us/dynamics365/business-central/dev-itpro/developer/devenv-system-tables
// Scope: in-scope
// Fixtures used: Probe Observer (codeunit 69160); test codeunit 75191, subscriber codeunit 75192.
// Probe: al-runner issue 7854. What a Delete of a TENANT permission set does to its Tenant Permission and Tenant Permission Set Rel.
//   rows. System Application's Permission Set page deletes only the set row; Microsoft's Permission Import Export Tests read the
//   set's permissions after TenantPermissionSet.DeleteAll(). Each test uses its own role ids (prefix TSDC) and cleans them first.
//   Measured: Delete, Delete(true), DeleteAll, DeleteAll(true), RecordRef.Delete and RecordRef.DeleteAll (filtered to one set) of a
//   set that has its own Tenant Permission rows, a Rel. row that includes another set, and a Rel. row of another set that includes
//   it; the rows left in both tables for the deleted set and for a second set that is not deleted. Also: DeleteAll of a set whose
//   App ID is not zero, and how often a bound OnAfterDeleteEvent subscriber on Tenant Permission runs during a set DeleteAll.
// BC versions: 27, 28, 29 (values observed on a local BC 27.5 and 28.5, identical; the corpus CI run checks all three)

codeunit 75191 "XP TSDC Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Observer: Codeunit "Probe Observer";

    local procedure AddSet(RoleId: Code[20])
    begin
        AddSetApp('00000000-0000-0000-0000-000000000000', RoleId);
    end;

    local procedure AddSetApp(AppId: Guid; RoleId: Code[20])
    var
        TenantSet: Record "Tenant Permission Set";
    begin
        TenantSet."App ID" := AppId;
        TenantSet."Role ID" := RoleId;
        TenantSet.Name := CopyStr(RoleId, 1, 30);
        TenantSet.Insert();
    end;

    local procedure AddPerm(RoleId: Code[20]; TableId: Integer)
    begin
        AddPermApp('00000000-0000-0000-0000-000000000000', RoleId, TableId);
    end;

    local procedure AddPermApp(AppId: Guid; RoleId: Code[20]; TableId: Integer)
    var
        TenantPerm: Record "Tenant Permission";
    begin
        TenantPerm."App ID" := AppId;
        TenantPerm."Role ID" := RoleId;
        TenantPerm."Object Type" := TenantPerm."Object Type"::"Table Data";
        TenantPerm."Object ID" := TableId;
        TenantPerm."Read Permission" := TenantPerm."Read Permission"::Yes;
        TenantPerm.Insert();
    end;

    local procedure AddRel(RoleId: Code[20]; Related: Code[20])
    var
        Rel: Record "Tenant Permission Set Rel.";
    begin
        Rel."App ID" := '00000000-0000-0000-0000-000000000000';
        Rel."Role ID" := RoleId;
        Rel."Related App ID" := '00000000-0000-0000-0000-000000000000';
        Rel."Related Role ID" := Related;
        Rel."Related Scope" := Rel."Related Scope"::Tenant;
        Rel.Insert();
    end;

    local procedure Clean()
    var
        TenantSet: Record "Tenant Permission Set";
        TenantPerm: Record "Tenant Permission";
        Rel: Record "Tenant Permission Set Rel.";
    begin
        TenantPerm.SetFilter("Role ID", 'TSDC*');
        TenantPerm.DeleteAll();
        Rel.SetFilter("Role ID", 'TSDC*');
        Rel.DeleteAll();
        TenantSet.SetFilter("Role ID", 'TSDC*');
        TenantSet.DeleteAll();
    end;

    local procedure Setup()
    begin
        Clean();
        AddSet('TSDC GONE');
        AddSet('TSDC KEPT');
        AddPerm('TSDC GONE', 18);
        AddPerm('TSDC GONE', 23);
        AddPerm('TSDC KEPT', 18);
        AddRel('TSDC GONE', 'TSDC KEPT');
        AddRel('TSDC KEPT', 'TSDC GONE');
    end;

    local procedure Left(): Text
    var
        TenantSet: Record "Tenant Permission Set";
        TenantPerm: Record "Tenant Permission";
        Rel: Record "Tenant Permission Set Rel.";
        Result: Text;
    begin
        TenantSet.SetFilter("Role ID", 'TSDC*');
        Result := 'sets=' + Format(TenantSet.Count());
        TenantPerm.SetRange("Role ID", 'TSDC GONE');
        Result += ' perms gone=' + Format(TenantPerm.Count());
        TenantPerm.SetRange("Role ID", 'TSDC KEPT');
        Result += ' perms kept=' + Format(TenantPerm.Count());
        Rel.SetRange("Role ID", 'TSDC GONE');
        Result += ' rel of gone=' + Format(Rel.Count());
        Rel.SetRange("Role ID", 'TSDC KEPT');
        Result += ' rel of kept=' + Format(Rel.Count());
        exit(Result);
    end;

    [Test]
    procedure TSDC_Delete()
    var
        TenantSet: Record "Tenant Permission Set";
    begin
        Setup();
        TenantSet.Get('00000000-0000-0000-0000-000000000000', 'TSDC GONE');
        TenantSet.Delete();
        Observer.Observe('after Delete', Left());
        Clean();
        Observer.Expect('after Delete', 'sets=1 perms gone=0 perms kept=1 rel of gone=0 rel of kept=0');
        Observer.Verify();
    end;

    [Test]
    procedure TSDC_DeleteTrue()
    var
        TenantSet: Record "Tenant Permission Set";
    begin
        Setup();
        TenantSet.Get('00000000-0000-0000-0000-000000000000', 'TSDC GONE');
        TenantSet.Delete(true);
        Observer.Observe('after Delete(true)', Left());
        Clean();
        Observer.Expect('after Delete(true)', 'sets=1 perms gone=0 perms kept=1 rel of gone=0 rel of kept=0');
        Observer.Verify();
    end;

    [Test]
    procedure TSDC_DeleteAll()
    var
        TenantSet: Record "Tenant Permission Set";
    begin
        Setup();
        TenantSet.SetRange("Role ID", 'TSDC GONE');
        TenantSet.DeleteAll();
        Observer.Observe('after DeleteAll', Left());
        Clean();
        Observer.Expect('after DeleteAll', 'sets=1 perms gone=0 perms kept=1 rel of gone=0 rel of kept=0');
        Observer.Verify();
    end;

    [Test]
    procedure TSDC_DeleteAllTrue()
    var
        TenantSet: Record "Tenant Permission Set";
    begin
        Setup();
        TenantSet.SetRange("Role ID", 'TSDC GONE');
        TenantSet.DeleteAll(true);
        Observer.Observe('after DeleteAll(true)', Left());
        Clean();
        Observer.Expect('after DeleteAll(true)', 'sets=1 perms gone=0 perms kept=1 rel of gone=0 rel of kept=0');
        Observer.Verify();
    end;

    [Test]
    procedure TSDC_RecRefDelete()
    var
        TenantSet: Record "Tenant Permission Set";
        RecRef: RecordRef;
    begin
        Setup();
        TenantSet.Get('00000000-0000-0000-0000-000000000000', 'TSDC GONE');
        RecRef.GetTable(TenantSet);
        RecRef.Delete();
        Observer.Observe('after RecordRef.Delete', Left());
        Clean();
        Observer.Expect('after RecordRef.Delete', 'sets=1 perms gone=0 perms kept=1 rel of gone=0 rel of kept=0');
        Observer.Verify();
    end;

    [Test]
    procedure TSDC_RecRefDeleteAll()
    var
        TenantSet: Record "Tenant Permission Set";
        RecRef: RecordRef;
    begin
        Setup();
        TenantSet.SetRange("Role ID", 'TSDC GONE');
        RecRef.GetTable(TenantSet);
        RecRef.DeleteAll();
        Observer.Observe('after RecordRef.DeleteAll', Left());
        Clean();
        Observer.Expect('after RecordRef.DeleteAll', 'sets=1 perms gone=0 perms kept=1 rel of gone=0 rel of kept=0');
        Observer.Verify();
    end;

    [Test]
    procedure TSDC_DeleteAllOtherApp()
    var
        TenantSet: Record "Tenant Permission Set";
        TenantPerm: Record "Tenant Permission";
        AppId: Guid;
    begin
        Clean();
        AppId := '{7A1C0E55-3B2D-4E6F-8A90-1B2C3D4E5F60}';
        AddSetApp(AppId, 'TSDC APP');
        AddPermApp(AppId, 'TSDC APP', 18);
        AddPermApp(AppId, 'TSDC APP', 23);
        AddPerm('TSDC APP', 27);
        TenantSet.SetRange("App ID", AppId);
        TenantSet.SetRange("Role ID", 'TSDC APP');
        TenantSet.DeleteAll();
        TenantSet.Reset();
        TenantSet.SetFilter("Role ID", 'TSDC*');
        Observer.Observe('sets left', Format(TenantSet.Count()));
        TenantPerm.SetRange("App ID", AppId);
        TenantPerm.SetRange("Role ID", 'TSDC APP');
        Observer.Observe('perms of that app left', Format(TenantPerm.Count()));
        TenantPerm.SetRange("App ID", '00000000-0000-0000-0000-000000000000');
        Observer.Observe('perms of app zero, same role left', Format(TenantPerm.Count()));
        TenantPerm.SetRange("App ID");
        TenantPerm.SetFilter("Role ID", 'TSDC*');
        TenantPerm.DeleteAll();
        Clean();
        Observer.Expect('sets left', '0');
        Observer.Expect('perms of that app left', '2');
        Observer.Expect('perms of app zero, same role left', '1');
        Observer.Verify();
    end;

    [Test]
    procedure TSDC_DeleteAllPermSubscriber()
    var
        TenantSet: Record "Tenant Permission Set";
        Sub: Codeunit "XP TSDC Perm Delete Sub";
    begin
        Setup();
        BindSubscription(Sub);
        TenantSet.SetRange("Role ID", 'TSDC GONE');
        TenantSet.DeleteAll();
        Observer.Observe('Tenant Permission OnAfterDeleteEvent calls', Format(Sub.Calls()));
        Observer.Observe('after DeleteAll', Left());
        UnbindSubscription(Sub);
        Clean();
        Observer.Expect('Tenant Permission OnAfterDeleteEvent calls', '2');
        Observer.Expect('after DeleteAll', 'sets=1 perms gone=0 perms kept=1 rel of gone=0 rel of kept=0');
        Observer.Verify();
    end;
}

codeunit 75192 "XP TSDC Perm Delete Sub"
{
    EventSubscriberInstance = Manual;

    var
        CallCount: Integer;

    [EventSubscriber(ObjectType::Table, Database::"Tenant Permission", 'OnAfterDeleteEvent', '', false, false)]
    local procedure OnAfterDeletePerm(var Rec: Record "Tenant Permission"; RunTrigger: Boolean)
    begin
        CallCount += 1;
    end;

    procedure Calls(): Integer
    begin
        exit(CallCount);
    end;
}
