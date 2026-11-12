codeunit 50899 T
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure DependentExtensionTargetsRowsAddedByDependency()
    var
        Trace: Codeunit "Cross Report Trace";
        Rows: Record "Cross Report Number";
        Number: Integer;
    begin
        for Number := 1 to 4 do begin
            Rows.Number := Number;
            Rows.Insert();
        end;
        Report.RunModal(Report::"Cross Report Base", false);
        if Trace.Read() <> 'owner=2|dependent=2|before|footer|after=2|' then
            Error('Cross report observation: %1', Trace.Read());
    end;
}
