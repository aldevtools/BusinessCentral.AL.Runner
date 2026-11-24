codeunit 56070 "RQ tr Tests"
{
    Subtype = Test;

    [Test]
    procedure T1RowIsConst()
    var
        R: Record "RQ tr Row";
    begin
        R.Get('T1');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure T2RowIsConst()
    var
        R: Record "RQ tr Row";
    begin
        R.Get('T2');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure T3RowIsConst()
    var
        R: Record "RQ tr Row";
    begin
        R.Get('T3');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure Control()
    begin
    end;
}
