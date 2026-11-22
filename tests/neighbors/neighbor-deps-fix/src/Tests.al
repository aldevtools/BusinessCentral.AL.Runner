// The app under test does not include the sources under deps/ (tools/neighbor-diff.sh stages it without them).
codeunit 60930 "ND Tests"
{
    Subtype = Test;

    [Test]
    procedure DependencyIsReachable()
    var
        Lib: Codeunit "ND Lib";
    begin
        if Lib.Answer() <> 42 then
            Error('dependency answer');
    end;
}
