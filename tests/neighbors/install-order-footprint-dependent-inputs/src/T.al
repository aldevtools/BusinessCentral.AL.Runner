codeunit 54490 "RQ di Tests"
{
    Subtype = Test;

    [Test]
    procedure PRowIsConst()
    var
        R: Record "RQ di Row";
    begin
        R.Get('P');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure Control()
    begin
    end;
}
