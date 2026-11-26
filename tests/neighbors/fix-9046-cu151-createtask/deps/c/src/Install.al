// A later install session copies the branch row of app a into a table of its own.
table 52880 "CTC Mirror C"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52881 "CTC Mirror C Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Source: Record "CTC Row A";
        Mirror: Record "CTC Mirror C";
    begin
        if not Source.Get('BRANCH') then
            exit;
        Mirror.Code := 'MIRROR';
        Mirror.Value := Source.Value;
        Mirror.Insert();
    end;
}
