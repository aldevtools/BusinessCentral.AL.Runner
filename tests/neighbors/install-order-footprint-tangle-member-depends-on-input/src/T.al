codeunit 55840 "RQ tm Tests"
{
    Subtype = Test;

    [Test]
    procedure T1RowIsConst()
    var
        R: Record "RQ tm Row";
    begin
        R.Get('T1');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure T2RowIsConst()
    var
        R: Record "RQ tm Row";
    begin
        R.Get('T2');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure Control()
    begin
    end;
}
