codeunit 51110 "FP Cycle X1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Own: Record "FP Cycle T1";
        Next: Record "FP Cycle T2";
    begin
        Own.Code := 'X1';
        Own.Value := 'SET';
        if not Own.Insert() then Own.Modify();
        if Next.IsEmpty() then;
    end;
}
