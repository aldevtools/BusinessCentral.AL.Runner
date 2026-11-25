// Inserts the setup row in every company it installs in: after Y, also in the copy Y made.
codeunit 53855 "IO9039S X Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "IO9039S Setup";
    begin
        R.Code := 'ROW';
        R.Value := 'SAME';
        if not R.Insert() then R.Modify();
    end;
}
