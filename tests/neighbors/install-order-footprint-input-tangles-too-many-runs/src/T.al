codeunit 56280 "RQ itm Tests"
{
    Subtype = Test;

    [Test]
    procedure XRowIsConst()
    var
        R: Record "RQ itm Row";
    begin
        R.Get('X');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure Control()
    begin
    end;
}
