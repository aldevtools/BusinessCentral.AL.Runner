codeunit 50220 "Debug Broken Tests"
{
    Subtype = Test;

    [Test]
    procedure DoesNotCompile()
    var
        Counter: Integer;
    begin
        Counter := NoSuchProcedure(1);
    end;
}
