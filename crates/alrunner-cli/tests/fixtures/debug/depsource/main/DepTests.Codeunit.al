codeunit 50260 "Debug Dep Tests"
{
    Subtype = Test;

    [Test]
    procedure CallsTheDependency()
    var
        Subject: Codeunit "Debug Dep Subject";
        Got: Integer;
    begin
        Got := Subject.Twice(3);
        if Got <> 6 then
            Error('Got %1', Got);
    end;
}
