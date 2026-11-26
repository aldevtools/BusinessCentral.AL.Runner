dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; "FP SB C") { } } }

table 52120 "FP Dotnet Local TC"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52121 "FP Dotnet Local C Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Builder: DotNet "FP SB C";
        Row: Record "FP Dotnet Local TC";
    begin
        Builder := Builder.StringBuilder();
        Builder.Append('C');
        Row.Code := 'C';
        Row.Value := Builder.ToString();
        Row.Insert();
    end;
}
