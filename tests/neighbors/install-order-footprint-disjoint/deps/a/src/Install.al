codeunit 51610 "FP Disjoint A Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Disjoint TA";
    begin
        R.Code := 'A';
        R.Value := 'SET';
        if not R.Insert() then R.Modify();
    end;
}
