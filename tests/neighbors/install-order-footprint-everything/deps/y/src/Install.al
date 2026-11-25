codeunit 51510 "FP Everything Y Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Everything TY";
    begin
        R.Code := 'Y';
        R.Value := 'SET';
        if not R.Insert() then R.Modify();
    end;
}
