dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; "FP SB2 A") { } } }

table 52300 "FP Dotnet NotKept TA"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52301 "FP Dotnet NotKept A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Builder: DotNet "FP SB2 A";
        Row: Record "FP Dotnet NotKept TA";
    begin
        Builder := Builder.StringBuilder();
        Builder.Insert(0, 'A');
        Row.Code := 'A';
        Row.Value := Builder.ToString();
        Row.Insert();
    end;
}
