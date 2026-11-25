dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; "FP SB B") { } } }

table 52110 "FP Dotnet Local TB"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52111 "FP Dotnet Local B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Builder: DotNet "FP SB B";
        Row: Record "FP Dotnet Local TB";
    begin
        Builder := Builder.StringBuilder();
        Builder.Append('B');
        Row.Code := 'B';
        Row.Value := Builder.ToString();
        Row.Insert();
    end;
}
