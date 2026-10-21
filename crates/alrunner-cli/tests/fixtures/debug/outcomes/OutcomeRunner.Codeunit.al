codeunit 50273 "Debug Outcome Runner"
{
    trigger OnRun()
    var
        Row: Record "Debug Outcome Row";
    begin
        Row."No." := 99;
        Row.Insert(true);
        Error('stopped in OnRun');
    end;
}
