codeunit 50800 "Repro Date Writer"
{
    procedure CompactDate(Value: Date): Text
    begin
        exit(Format(Value, 8, '<Year4><Month,2><Day,2>'));
    end;

    procedure CompactDateNoLength(Value: Date): Text
    begin
        exit(Format(Value, 0, '<Year4><Month,2><Day,2>'));
    end;

    procedure DashedDate(Value: Date): Text
    begin
        exit(Format(Value, 10, '<Year4>-<Month,2>-<Day,2>'));
    end;
}
