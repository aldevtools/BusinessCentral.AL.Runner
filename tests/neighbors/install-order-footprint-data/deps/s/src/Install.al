codeunit 51330 "FP Data S Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        Src: Record "FP Data Source";
        Out: Record "FP Data Same";
    begin
        Out.Code := 'X';
        if Src.IsEmpty() then
            Out.Value := 'EMPTY'
        else
            Out.Value := 'EMPTY';
        if not Out.Insert() then Out.Modify();
    end;
}
