dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; "FP SB2 C") { } } }

table 52320 "FP Dotnet NotKept TC"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52321 "FP Dotnet NotKept C Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Builder: DotNet "FP SB2 C";
        Row: Record "FP Dotnet NotKept TC";
    begin
        Builder := Builder.StringBuilder();
        Builder.Append('C');
        Row.Code := 'C';
        Row.Value := Builder.ToString();
        Row.Insert();
    end;
}
