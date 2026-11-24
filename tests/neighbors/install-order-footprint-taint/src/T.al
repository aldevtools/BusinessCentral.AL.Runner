codeunit 52590 "RP taint Tests"
{
    Subtype = Test;

    [Test]
    procedure DRowDependsOnTheOrder()
    var
        R: Record "RP Taint D Row";
    begin
        R.Get('D');
    end;

    [Test]
    procedure XRowCopiedFromDRow()
    var
        R: Record "RP Taint X Row";
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
