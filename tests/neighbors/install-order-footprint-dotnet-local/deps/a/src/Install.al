dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; "FP SB A") { } } }

table 52100 "FP Dotnet Local TA"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52101 "FP Dotnet Local A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Builder: DotNet "FP SB A";
        Row: Record "FP Dotnet Local TA";
    begin
        Builder := Builder.StringBuilder();
        Builder.Append('A');
        Row.Code := 'A';
        Row.Value := Builder.ToString();
        Row.Insert();
    end;
}
