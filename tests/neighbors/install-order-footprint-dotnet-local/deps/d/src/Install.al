dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; "FP SB D") { } } }

table 52130 "FP Dotnet Local TD"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52131 "FP Dotnet Local D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Builder: DotNet "FP SB D";
        Row: Record "FP Dotnet Local TD";
    begin
        Builder := Builder.StringBuilder();
        Builder.Append('D');
        Row.Code := 'D';
        Row.Value := Builder.ToString();
        Row.Insert();
    end;
}
