codeunit 51120 "FP Cycle X2 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Own: Record "FP Cycle T2";
        Next: Record "FP Cycle T3";
    begin
        Own.Code := 'X2';
        Own.Value := 'SET';
        if not Own.Insert() then Own.Modify();
        if Next.IsEmpty() then;
    end;
}
