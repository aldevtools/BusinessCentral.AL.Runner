dotnet
{
    assembly("mscorlib")
    {
        type("System.Text.Encoding"; "FP Enc A") { }
        type("System.Array"; "FP Arr A") { }
    }
}

table 52700 "FP Dotnet Array TA"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52701 "FP Dotnet Array A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Encoding: DotNet "FP Enc A";
        Bytes: DotNet "FP Arr A";
        Row: Record "FP Dotnet Array TA";
    begin
        Bytes := Encoding.UTF8().GetBytes('A');
        Row.Code := 'A';
        Row.Value := Format(Bytes.Length()) + '|' + Format(Bytes.GetValue(0));
        Row.Insert();
    end;
}
