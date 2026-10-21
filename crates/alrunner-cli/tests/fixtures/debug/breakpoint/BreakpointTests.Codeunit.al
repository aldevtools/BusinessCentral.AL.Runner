codeunit 50200 "Debug Breakpoint Tests"
{
    Subtype = Test;

    [Test]
    procedure ThreeAssignments()
    var
        Counter: Integer;
        Name: Text;
    begin
        Counter := 1;
        Name := 'abc';
        Counter := 2;
        Counter := 3;
        if Counter <> 3 then
            Error('Counter is %1', Counter);
    end;
}
