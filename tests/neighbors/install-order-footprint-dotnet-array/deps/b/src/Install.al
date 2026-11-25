dotnet
{
    assembly("mscorlib")
    {
        type("System.Text.Encoding"; "FP Enc B") { }
        type("System.Array"; "FP Arr B") { }
    }
}

table 52710 "FP Dotnet Array TB"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52711 "FP Dotnet Array B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Encoding: DotNet "FP Enc B";
        Bytes: DotNet "FP Arr B";
        Row: Record "FP Dotnet Array TB";
    begin
        Bytes := Encoding.UTF8().GetBytes('B');
        Row.Code := 'B';
        Row.Value := Format(Bytes.Length()) + '|' + Format(Bytes.GetValue(0));
        Row.Insert();
    end;
}
