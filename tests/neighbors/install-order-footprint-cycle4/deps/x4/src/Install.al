codeunit 51140 "FP Cycle X4 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Own: Record "FP Cycle T4";
        Next: Record "FP Cycle T1";
    begin
        Own.Code := 'X4';
        Own.Value := 'SET';
        if not Own.Insert() then Own.Modify();
        if Next.IsEmpty() then;
    end;
}
