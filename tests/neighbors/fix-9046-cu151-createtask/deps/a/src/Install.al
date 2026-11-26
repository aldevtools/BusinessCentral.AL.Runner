table 52850 "CTC Row A"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
        field(3; "Task"; Guid) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52851 "CTC Writer A"
{
    trigger OnRun()
    var
        Row: Record "CTC Row A";
    begin
        Row.Code := 'WRITER';
        Row.Value := 'ran';
        if not Row.Insert() then
            Row.Modify();
    end;
}

// The checked shape: with the Yes the install code creates a task, else it runs the writer itself.
codeunit 52852 "CTC Task A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "CTC Row A";
    begin
        Row.Code := 'BRANCH';
        if TaskScheduler.CanCreateTask() then begin
            Row.Task := TaskScheduler.CreateTask(Codeunit::"CTC Writer A", 0);
            Row.Value := 'task';
        end else begin
            Codeunit.Run(Codeunit::"CTC Writer A");
            Row.Value := 'fallback';
        end;
        Row.Insert();
    end;
}
