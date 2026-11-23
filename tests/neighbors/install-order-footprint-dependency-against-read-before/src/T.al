codeunit 54190 "RQ dc2 Tests"
{
    Subtype = Test;

    [Test]
    procedure PRowIsEmpty()
    var
        R: Record "RQ dc2 Row";
    begin
        R.Get('P');
        if R.Value <> 'EMPTY' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure PRowIsSeen()
    var
        R: Record "RQ dc2 Row";
    begin
        R.Get('P');
        if R.Value <> 'SEEN' then
            Error('value %1', R.Value);
    end;

    [Test]
    procedure GRowExists()
    var
        R: Record "RQ dc2 Row";
    begin
        R.Get('G');
    end;

    [Test]
    procedure Control()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
