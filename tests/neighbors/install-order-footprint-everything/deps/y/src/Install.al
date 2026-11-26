codeunit 51510 "FP Everything Y Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "FP Everything TY";
        V: Variant;
    begin
        V := DaTi2Variant(20240115D, 123456T);
        R.Code := 'Y';
        R.Value := 'SET';
        if not R.Insert() then R.Modify();
    end;
}
