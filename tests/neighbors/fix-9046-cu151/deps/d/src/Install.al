table 52630 "CT Task TD"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52631 "CT Task D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
        Note('INSTALL');
    end;

    procedure Note(Tag: Code[20])
    var
        Row: Record "CT Task TD";
    begin
        Row.Code := Tag;
        Row.Value := Format(TaskScheduler.CanCreateTask());
        if not Row.Insert() then
            Row.Modify();
    end;
}
