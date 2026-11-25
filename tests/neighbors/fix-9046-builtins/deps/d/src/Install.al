table 52530 "FP Builtins TD"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(2; "Value"; Text[250]) { }
    }
    keys { key(PK; "Code") { Clustered = true; } }
}

codeunit 52531 "FP Builtins D Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Row: Record "FP Builtins TD";
        DT: DateTime;
        V: Variant;
        D: Date;
        T: Time;
    begin
        DT := CreateDateTime(20240115D, 123456T);
        D := DT2Date(DT);
        T := DT2Time(DT);
        D := DMY2Date(15, 1, 2024);
        D := DWY2Date(1, 1, 2024);
        V := D;
        D := Variant2Date(V);
        V := T;
        T := Variant2Time(V);
        Row.Code := 'D';
        Row.Value := Format(D, 0, 9) + Format(T, 0, 9) + Format(Date2DMY(D, 3)) + Format(Date2DWY(D, 2));
        Row.Insert();
    end;
}
