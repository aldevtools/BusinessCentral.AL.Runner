codeunit 52190 "RP mixed Tests"
{
    Subtype = Test;

    [Test]
    procedure ReaderWhoseRowDependsOnTheOrder()
    var
        R: Record "RP Mixed Differs";
    begin
        R.Get('X');
    end;

    [Test]
    procedure SharedRowExists()
    var
        R: Record "RP Mixed Collide";
    begin
        R.Get('ROW');
    end;

    [Test]
    procedure Control()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
