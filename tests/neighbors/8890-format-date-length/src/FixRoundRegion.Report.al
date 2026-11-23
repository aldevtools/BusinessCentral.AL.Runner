report 50810 "Fix Round Region"
{
    ProcessingOnly = true;
    UseRequestPage = false;

    trigger OnPreReport()
    var
        Log: Codeunit "Fix Round Region Log";
        D: Date;
    begin
        D := DMY2Date(15, 1, 2024);
        CurrReport.FormatRegion := 'de-DE';
        case Log.GetMode() of
            1:
                Log.Put(Format(D, 8, '<Standard Format,0>'));
            2:
                Log.Put(Format(D, 10, '<Year4>-<Month,2>-<Day,2>'));
            3:
                Log.Put(Format(D, 8, '<Year4><Month,2><Day,2>'));
            4:
                Log.Put(Format(134506T, 11, '<Standard Format,0>'));
        end;
    end;
}
