codeunit 50400 "Repro ModifyAll Tests"
{
    Subtype = Test;

    var
        PermissionsMock: Codeunit "Permissions Mock";

    [Test]
    procedure DeleteParentWhoseTriggerCallsModifyAllOnAnEmptyFilter()
    var
        ReproParent: Record "Repro Parent";
        ReproOther: Record "Repro Other";
    begin
        PermissionsMock.ClearAssignments();
        ReproParent.Code := 'P1';
        ReproParent.Insert();
        ReproOther.Code := 'O1';
        ReproOther."Parent Code" := 'X';
        ReproOther.Insert();
        PermissionsMock.Set('D365 BASIC');
        PermissionsMock.Assign('Repro Parent Edit');
        ReproParent.Delete(true);
    end;
}
