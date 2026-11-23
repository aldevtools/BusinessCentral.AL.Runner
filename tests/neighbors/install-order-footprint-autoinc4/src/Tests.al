// Four unordered apps insert into one AutoIncrement log in their install code (ADR-079 decision 5).
codeunit 51490 "FP autoinc4 Tests"
{
    Subtype = Test;

    [Test]
    procedure LogHasFourEntries()
    var
        Log: Record "FP AutoInc Log";
    begin
        if Log.Count() <> 4 then Error('count %1', Log.Count());
    end;

    [Test]
    procedure ControlTouchesNoInstalledTable()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
