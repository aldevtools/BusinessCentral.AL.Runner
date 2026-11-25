dotnet
{
    assembly("mscorlib")
    {
        type("System.Text.Encoding"; "FP Enc C") { }
        type("System.Array"; "FP Arr C") { }
    }
}

table 52720 "FP Dotnet Array TC"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52721 "FP Dotnet Array C Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Encoding: DotNet "FP Enc C";
        Bytes: DotNet "FP Arr C";
        Row: Record "FP Dotnet Array TC";
    begin
        Bytes := Encoding.UTF8().GetBytes('C');
        Row.Code := 'C';
        Row.Value := Format(Bytes.Length()) + '|' + Format(Bytes.GetValue(0));
        Row.Insert();
    end;
}
