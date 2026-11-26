table 52860 "CTC Row B"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Task"; Guid) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

// CreateTask in install code without CanCreateTask (main ran it too).
codeunit 52861 "CTC Task B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerDatabase()
    var
        Row: Record "CTC Row B";
    begin
        Row.Code := 'CREATED';
        Row.Task := TaskScheduler.CreateTask(0, 0);
        Row.Insert();
    end;
}
