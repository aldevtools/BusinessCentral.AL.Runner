codeunit 55950 "RQ ct Tests"
{
    Subtype = Test;

    [Test]
    procedure A1RowIsConst()
    var
        R: Record "RQ ct Row";
    begin
        R.Get('A1');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure A2RowIsConst()
    var
        R: Record "RQ ct Row";
    begin
        R.Get('A2');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure B1RowIsConst()
    var
        R: Record "RQ ct Row";
    begin
        R.Get('B1');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure B2RowIsConst()
    var
        R: Record "RQ ct Row";
    begin
        R.Get('B2');
        if R.Value <> 'CONST' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure Control()
    begin
    end;
}
