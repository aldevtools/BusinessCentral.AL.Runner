dotnet { assembly("mscorlib") { type("System.Text.StringBuilder"; "FP SB1 A") { } } }

table 52200 "FP Dotnet Single TA"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[100]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52202 "FP Dotnet Single A Holder"
{
    SingleInstance = true;

    var
        Builder: DotNet "FP SB1 A";
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

codeunit 52201 "FP Dotnet Single A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Holder: Codeunit "FP Dotnet Single A Holder";
        Row: Record "FP Dotnet Single TA";
    begin
        Holder.Add('A');
        Row.Code := 'A';
        Row.Value := Holder.Add('A');
        Row.Insert();
    end;
}
