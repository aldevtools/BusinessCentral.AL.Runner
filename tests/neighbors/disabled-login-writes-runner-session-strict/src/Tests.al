// Hub, Disabled: notes the hub session's id; leaves the login row alone.
codeunit 52610 "RS A Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure A_NoteSession()
    var
        Lib: Codeunit "RS Lib";
    begin
        Lib.Note('HUB');
        Commit();
    end;
}

// Runner 130450 (NonRestrictive) with an OnRun write, like corpus PR 1644 cell 8 (LgnHeldLoginRunners.al:656).
codeunit 52620 "RS P1 Runner"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    trigger OnRun()
    var
        Own: Record "RS Own";
        Lib: Codeunit "RS Lib";
    begin
        Own.Code := 'P1';
        Own.Insert();
        Lib.Note('P1');
    end;

    // 1644 :669: one login row of this session.
    [Test]
    procedure P1_OwnSessionRows()
    var
        Lib: Codeunit "RS Lib";
    begin
        Lib.Equal(1, Lib.OwnRows(), 'login rows of this session');
    end;

    // 1644 :664: not the hub codeunits' session.
    [Test]
    procedure P1_OtherSessionThanHub()
    var
        Lib: Codeunit "RS Lib";
    begin
        if Lib.Mark('HUB') = SessionId() then
            Error('same session as the hub codeunit: %1', SessionId());
    end;

    // The Active Session row of the running session is keyed by SessionId() (session-system/TestActiveSessionTable.al).
    [Test]
    procedure P1_ActiveSessionRow()
    var
        ActiveSession: Record "Active Session";
    begin
        if not ActiveSession.Get(ServiceInstanceId(), SessionId()) then
            Error('no Active Session row for SessionId() %1', SessionId());
    end;

    // The hub session's id seen from this session: whether BC still has the hub session is not measured.
    [Test]
    procedure P1_HubSessionActive()
    var
        Lib: Codeunit "RS Lib";
    begin
        if Session.IsSessionActive(Lib.Mark('HUB')) then
            Error('the hub session is active');
    end;
}

// A second runner-130450 codeunit (no TestPermissions: Restrictive), like 1644 :697 after :656.
codeunit 52621 "RS P2 Runner"
{
    Subtype = Test;

    trigger OnRun()
    var
        Own: Record "RS Own";
    begin
        Own.Code := 'P2';
        Own.Insert();
    end;

    // 1644 :710: one login row of this session.
    [Test]
    procedure P2_OwnSessionRows()
    var
        Lib: Codeunit "RS Lib";
    begin
        Lib.Equal(1, Lib.OwnRows(), 'login rows of this session');
    end;

    // Whether two codeunits runner 130450 runs share one session is not measured: the runner gives each its own.
    [Test]
    procedure P2_OtherSessionThanP1()
    var
        Lib: Codeunit "RS Lib";
    begin
        if Lib.Mark('P1') = SessionId() then
            Error('same session as P1: %1', SessionId());
    end;
}
