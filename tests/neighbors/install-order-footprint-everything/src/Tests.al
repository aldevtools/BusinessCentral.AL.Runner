// One install calls a builtin the footprint does not attribute; one other unordered app writes its own table.
codeunit 51590 "FP everything Tests"
{
    Subtype = Test;

    [Test]
    procedure OtherAppRowExists()
    var
        R: Record "FP Everything TY";
    begin
        R.Get('Y');
    end;

    [Test]
    procedure OwnRowExists()
    var
        R: Record "FP Everything TE";
    begin
        R.Get('E');
    end;

    [Test]
    procedure ControlTouchesNoInstalledTable()
    var
        Ok: Boolean;
    begin
        Ok := true;
    end;
}
