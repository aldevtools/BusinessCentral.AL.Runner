dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; "FP SB1 C") { } } }

table 52220 "FP Dotnet Single TC"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52222 "FP Dotnet Single C Holder"
{
    SingleInstance = true;

    var
        Builder: DotNet "FP SB1 C";
        Made: Boolean;

    procedure Add(Part: Text): Text
    begin
        if not Made then
            Builder := Builder.StringBuilder();
        Made := true;
        Builder.Append(Part);
        exit(Builder.ToString());
    end;
}

codeunit 52221 "FP Dotnet Single C Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Holder: Codeunit "FP Dotnet Single C Holder";
        Row: Record "FP Dotnet Single TC";
    begin
        Holder.Add('C');
        Row.Code := 'C';
        Row.Value := Holder.Add('C');
        Row.Insert();
    end;
}
