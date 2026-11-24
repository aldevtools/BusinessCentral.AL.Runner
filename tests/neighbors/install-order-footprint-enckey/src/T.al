codeunit 52290 "RP enc Tests"
{
    Subtype = Test;

    [Test]
    procedure RowWrittenFromTheKeyState()
    var
        R: Record "RP Enc Row";
    begin
        R.Get('Y');
    end;

    [Test]
    procedure Control()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
