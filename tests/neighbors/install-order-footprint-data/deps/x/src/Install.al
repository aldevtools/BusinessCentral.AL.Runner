codeunit 51320 "FP Data X Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Src: Record "FP Data Source";
        Out: Record "FP Data Differs";
    begin
        Out.Code := 'X';
        if Src.IsEmpty() then
            Out.Value := 'EMPTY'
        else
            Out.Value := 'SEEN';
        if not Out.Insert() then Out.Modify();
    end;
}
