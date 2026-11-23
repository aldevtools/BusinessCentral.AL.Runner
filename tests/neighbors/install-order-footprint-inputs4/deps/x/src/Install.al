codeunit 51250 "FP Inputs X Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        T1: Record "FP Inputs T1";
        T2: Record "FP Inputs T2";
        T3: Record "FP Inputs T3";
        T4: Record "FP Inputs T4";
        Own: Record "FP Inputs TX";
    begin
        if T1.IsEmpty() then;
        if T2.IsEmpty() then;
        if T3.IsEmpty() then;
        if T4.IsEmpty() then;
        Own.Code := 'X';
        Own.Value := 'SET';
        if not Own.Insert() then Own.Modify();
    end;
}
