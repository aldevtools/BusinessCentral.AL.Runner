// CreateTask inside a try function in install code (refused, al-runner issue 9134): the row after it.
table 52890 "CTC Try Row D"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52891 "CTC Try Target D"
{
    trigger OnRun()
    begin
    end;
}

codeunit 52892 "CTC Try Install D"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "CTC Try Row D";
    begin
        Row.Code := 'TRY';
        Row.Value := Format(TryCreate());
        Row.Insert();
    end;

    [TryFunction]
    local procedure TryCreate()
    begin
        TaskScheduler.CreateTask(Codeunit::"CTC Try Target D", 0);
    end;
}
