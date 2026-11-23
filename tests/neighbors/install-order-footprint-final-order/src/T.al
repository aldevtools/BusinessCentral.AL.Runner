codeunit 54490 "RQ fo Tests"
{
    Subtype = Test;

    [Test]
    procedure BRowIsConst()
    var
        R: Record "RQ fo Row";
    begin
        R.Get('B');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure Control()
    begin
    end;
}
