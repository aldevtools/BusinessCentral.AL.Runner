// Runner PR 9426, review round 1: two codeunits runner 130450 runs in its one session after the Disabled hub codeunit
// (corpus PR 1657, LgnRunnerSessions.al:436: one session id for all of them) each run a page background task, whose
// worker runs in a child session (pages/background-tasks/PbtChildSession.al:209-229: not the test session's id). Each
// worker's id is one no earlier worker of that session had. The numbering is not pinned.
codeunit 53910 "SRS H Hub Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure H_CommitHubId()
    var
        Row: Record "SRS Login Row";
        Mark: Record "SRS Hub Mark";
        Assert: Codeunit "SRS Assert";
    begin
        Assert.Equal(1, Row.Count(), 'login rows');
        Row.FindLast();
        Mark.Code := 'HUB';
        Mark."Hub Session Id" := SessionId();
        Mark."Max Login Entry" := Row."Entry No.";
        Mark.Insert();
        Commit();
    end;
}

codeunit 53950 "SRS B1 First Task"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    [Test]
    procedure B1_FirstCodeunitTask()
    var
        Starter: Codeunit "SRS Bg Start";
    begin
        Starter.RunTask();
    end;
}

codeunit 53951 "SRS B2 Second Task"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    [Test]
    procedure B2_SecondCodeunitTaskAnotherId()
    var
        Starter: Codeunit "SRS Bg Start";
        Memo: Codeunit "SRS Bg Memo";
        Assert: Codeunit "SRS Assert";
    begin
        Starter.RunTask();
        Assert.Equal(2, Memo.Count(), 'workers of both codeunits');
    end;
}

