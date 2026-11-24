codeunit 55640 "RQ ti Tests"
{
    Subtype = Test;

    [Test]
    procedure XIsZero()
    var
        R: Record "RQ ti Row";
    begin
        R.Get('X');
        if R.Value <> '0' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure Control()
    begin
    end;
}
