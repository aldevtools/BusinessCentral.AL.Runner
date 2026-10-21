codeunit 50230 "Debug Pause Tests"
{
    Subtype = Test;

    [Test]
    procedure LoopsForever()
    var
        I: Integer;
    begin
        while true do
            I := (I + 1) mod 1000;
    end;
}
