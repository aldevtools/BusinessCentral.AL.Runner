dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; "FP SB1 D") { } } }

table 52230 "FP Dotnet Single TD"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52232 "FP Dotnet Single D Holder"
{
    SingleInstance = true;

    var
        Builder: DotNet "FP SB1 D";
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

codeunit 52231 "FP Dotnet Single D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Holder: Codeunit "FP Dotnet Single D Holder";
        Row: Record "FP Dotnet Single TD";
    begin
        Holder.Add('D');
        Row.Code := 'D';
        Row.Value := Holder.Add('D');
        Row.Insert();
    end;
}
