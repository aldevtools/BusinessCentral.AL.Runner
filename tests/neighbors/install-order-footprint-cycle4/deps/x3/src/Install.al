codeunit 51130 "FP Cycle X3 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Own: Record "FP Cycle T3";
        Next: Record "FP Cycle T4";
    begin
        Own.Code := 'X3';
        Own.Value := 'SET';
        if not Own.Insert() then Own.Modify();
        if Next.IsEmpty() then;
    end;
}
