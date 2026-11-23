codeunit 52090 "RP chain Tests"
{
    Subtype = Test;

    [Test]
    procedure DRowExists()
    var
        R: Record "RP Chain Row";
    begin
        R.Get('D');
    end;

    [Test]
    procedure XRowExists()
    var
        R: Record "RP Chain Row";
    begin
        R.Get('X');
    end;

    [Test]
    procedure Control()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
