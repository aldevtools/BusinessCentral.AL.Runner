// al-runner issue 9111: the reporter's repro, unchanged but for one app (app and test together) and the versions in app.json
// (27, to run on every major). The logo is the 535 x 534 baseline JPEG of corpus PR 1594 (fixture G), the size of the
// reporter's logo.
codeunit 50156 "JL Logo Test"
{
    Subtype = Test;

    var
        Assert: Codeunit "Library Assert";

    [Test]
    procedure TheTestRuns()
    begin
        Assert.IsTrue(true, 'the test runs');
    end;
}
