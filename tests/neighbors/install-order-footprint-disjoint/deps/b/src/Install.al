codeunit 51620 "FP Disjoint B Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Disjoint TB";
    begin
        R.Code := 'B';
        R.Value := 'SET';
        if not R.Insert() then R.Modify();
    end;
}
