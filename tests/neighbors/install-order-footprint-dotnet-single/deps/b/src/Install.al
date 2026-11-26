dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; "FP SB1 B") { } } }

table 52210 "FP Dotnet Single TB"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52212 "FP Dotnet Single B Holder"
{
    SingleInstance = true;

    var
        Builder: DotNet "FP SB1 B";
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

codeunit 52211 "FP Dotnet Single B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Holder: Codeunit "FP Dotnet Single B Holder";
        Row: Record "FP Dotnet Single TB";
    begin
        Holder.Add('B');
        Row.Code := 'B';
        Row.Value := Holder.Add('B');
        Row.Insert();
    end;
}
