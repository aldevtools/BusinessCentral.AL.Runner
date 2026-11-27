table 52610 "CT Task TB"
{
    DataPerCompany = false;

    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52611 "CT Task B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerDatabase()
    var
        Row: Record "CT Task TB";
        Tag: Code[20];
    begin
        Tag := 'DATABASE';
        Row.Code := Tag;
        Row.Value := Format(TaskScheduler.CanCreateTask());
        if not Row.Insert() then
            Row.Modify();
    end;
}
