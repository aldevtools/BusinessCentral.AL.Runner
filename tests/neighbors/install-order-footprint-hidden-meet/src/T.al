codeunit 53290 "RQ hz Tests"
{
    Subtype = Test;

    [Test]
    procedure XRowIsEmpty()
    var
        R: Record "RQ hz Row";
    begin
        R.Get('X');
        if R.Value <> 'EMPTY' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure XRowIsSeen()
    var
        R: Record "RQ hz Row";
    begin
        R.Get('X');
        if R.Value <> 'SEEN' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure DRowExists()
    var
        R: Record "RQ hz Row";
    begin
        R.Get('D');
    end;

    [Test]
    procedure Control()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
