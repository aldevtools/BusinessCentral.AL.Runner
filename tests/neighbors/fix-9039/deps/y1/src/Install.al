// Empties the table with a DeleteAll without filters only when X1 ran first, then writes the same row back: both orders
// leave the same row, and only one order leaves the emptied mark.
codeunit 53815 "IO9039 Y1 Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        R: Record "IO9039 Emptied Same";
    begin
        if R.Get('ROW') then begin
            R.DeleteAll();
            R.Code := 'ROW';
            R.Value := 'SAME';
            R.Insert();
        end;
    end;
}
