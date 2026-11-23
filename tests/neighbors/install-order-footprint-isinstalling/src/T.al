codeunit 52490 "RP nav2 Tests"
{
    Subtype = Test;

    [Test]
    procedure N1RowExists()
    var
        R: Record "RP Nav2 Row";
    begin
        R.Get('N1');
    end;

    [Test]
    procedure Control()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
