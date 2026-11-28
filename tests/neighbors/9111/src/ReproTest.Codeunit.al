// al-runner issue 9111: the reporter's repro, unchanged but for one app (app and test together) and app.json: versions 27 (to
// run on every major) and System Application instead of the application (the neighbor check's memory limit), so the enum
// values Uncategorized of System Application in place of Extensions. The logo is the 535 x 534 baseline JPEG of corpus PR 1594
// (fixture G), the size of the reporter's logo.
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
