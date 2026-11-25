dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; "FP SB2 B") { } } }

table 52310 "FP Dotnet NotKept TB"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52311 "FP Dotnet NotKept B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Builder: DotNet "FP SB2 B";
        Row: Record "FP Dotnet NotKept TB";
    begin
        Builder := Builder.StringBuilder();
        Builder.Append('B');
        Row.Code := 'B';
        Row.Value := Builder.ToString();
        Row.Insert();
    end;
}
