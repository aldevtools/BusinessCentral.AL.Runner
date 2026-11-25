table 52600 "CT Task TA"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52601 "CT Task A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
        Note('INSTALL');
    end;

    procedure Note(Tag: Code[20])
    var
        Row: Record "CT Task TA";
    begin
        Row.Code := Tag;
        Row.Value := Format(TaskScheduler.CanCreateTask());
        if not Row.Insert() then
            Row.Modify();
    end;
}
