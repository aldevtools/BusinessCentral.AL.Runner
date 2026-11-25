dotnet
{
    assembly("mscorlib")
    {
        type("System.Text.Encoding"; "FP Enc D") { }
        type("System.Array"; "FP Arr D") { }
    }
}

table 52730 "FP Dotnet Array TD"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52731 "FP Dotnet Array D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Encoding: DotNet "FP Enc D";
        Bytes: DotNet "FP Arr D";
        Row: Record "FP Dotnet Array TD";
    begin
        Bytes := Encoding.UTF8().GetBytes('D');
        Row.Code := 'D';
        Row.Value := Format(Bytes.Length()) + '|' + Format(Bytes.GetValue(0));
        Row.Insert();
    end;
}
