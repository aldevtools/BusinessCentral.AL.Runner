// The facts are corpus PR 1657, LgnRunnerSessions.al (e58a5a05a): hub rows :380-:391, runner rows :422-:444 (87437),
// the same at +52 (87438) and +107 (87439).
codeunit 53910 "SRS H Hub Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    // :385, :386: one login row, of the hub's session; the hub commits its id for the runner codeunits.
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

codeunit 53920 "SRS N NonRestrictive"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    trigger OnRun()
    begin
        Memo.NoteRun(SessionId());
    end;

    var
        Memo: Codeunit "SRS Memo";
        Assert: Codeunit "SRS Assert";

    // :423 not the hub's session.
    [Test]
    procedure N_NotHubSession()
    begin
        Assert.IsTrue(SessionId() <> Assert.HubId(), 'not the hub session');
    end;

    // :424, :425: one login in this session, with this session's id.
    [Test]
    procedure N_MemoLogin()
    begin
        Assert.Equal(1, Memo.SeenLoginCalls(), 'login calls');
        Assert.Equal(SessionId(), Memo.SeenLoginId(), 'login id');
    end;

    // :428 the id in OnRun is the id in the test.
    [Test]
    procedure N_OnRunId()
    begin
        Assert.Equal(SessionId(), Memo.LastRunId(), 'OnRun id');
    end;

    // :429-:431: two login rows, one of the hub, one of this session.
    [Test]
    procedure N_LoginRows()
    var
        Row: Record "SRS Login Row";
    begin
        Assert.Equal(2, Row.Count(), 'login rows');
        Row.SetRange("Session Id", Assert.HubId());
        Assert.Equal(1, Row.Count(), 'hub login rows');
        Row.SetRange("Session Id", SessionId());
        Assert.Equal(1, Row.Count(), 'own login rows');
    end;

    // :438, :439.
    [Test]
    procedure N_ActiveSessions()
    begin
        Assert.IsTrue(Session.IsSessionActive(SessionId()), 'own session active');
        Assert.IsTrue(not Session.IsSessionActive(Assert.HubId()), 'hub session not active');
    end;

    // :440-:444: own rows filtered on the own id, none of the hub id.
    [Test]
    procedure N_SessionTables()
    var
        ActiveSession: Record "Active Session";
        SessionRow: Record Session;
    begin
        ActiveSession.SetRange("Session ID", SessionId());
        Assert.Equal(1, ActiveSession.Count(), 'own active session rows');
        ActiveSession.SetRange("Server Instance ID", ServiceInstanceId());
        Assert.Equal(1, ActiveSession.Count(), 'own active session rows of this instance');
        ActiveSession.Reset();
        ActiveSession.SetRange("Session ID", Assert.HubId());
        Assert.Equal(0, ActiveSession.Count(), 'hub active session rows');
        SessionRow.SetRange("Connection ID", SessionId());
        Assert.Equal(1, SessionRow.Count(), 'own session rows');
        SessionRow.SetRange("Connection ID", Assert.HubId());
        Assert.Equal(0, SessionRow.Count(), 'hub session rows');
    end;
}

codeunit 53921 "SRS R Restrictive"
{
    Subtype = Test;

    trigger OnRun()
    begin
        Memo.NoteRun(SessionId());
    end;

    var
        Memo: Codeunit "SRS Memo";
        Assert: Codeunit "SRS Assert";

    // :475 (= :423 + 52).
    [Test]
    procedure R_NotHubSession()
    begin
        Assert.IsTrue(SessionId() <> Assert.HubId(), 'not the hub session');
    end;

    // :476, :477: no second login in the shared session.
    [Test]
    procedure R_MemoLogin()
    begin
        Assert.Equal(1, Memo.SeenLoginCalls(), 'login calls');
        Assert.Equal(SessionId(), Memo.SeenLoginId(), 'login id');
    end;

    // :488 (= :436 + 52): the earlier runner codeunit noted the same id.
    [Test]
    procedure R_EarlierRunnersSameId()
    begin
        Assert.Equal(0, Memo.RunsWithOtherId(SessionId()), 'runner codeunits with another id');
    end;

    // :481-:486: still one login row of this session after the hub's.
    [Test]
    procedure R_LoginRows()
    var
        Row: Record "SRS Login Row";
        Mark: Record "SRS Hub Mark";
    begin
        Assert.Equal(2, Row.Count(), 'login rows');
        Mark.Get('HUB');
        Row.SetFilter("Entry No.", '>%1', Mark."Max Login Entry");
        Assert.Equal(1, Row.Count(), 'login rows after the hub');
        Row.SetRange("Session Id", SessionId());
        Assert.Equal(1, Row.Count(), 'own login rows after the hub');
    end;

    // :490, :491.
    [Test]
    procedure R_ActiveSessions()
    begin
        Assert.IsTrue(Session.IsSessionActive(SessionId()), 'own session active');
        Assert.IsTrue(not Session.IsSessionActive(Assert.HubId()), 'hub session not active');
    end;
}

codeunit 53922 "SRS M Handler"
{
    Subtype = Test;
    TestPermissions = Disabled;

    trigger OnRun()
    begin
        Memo.NoteRun(SessionId());
    end;

    var
        Memo: Codeunit "SRS Memo";
        Assert: Codeunit "SRS Assert";

    // :531, :532, :535, :543 (= +107): one login, the id of OnRun, every earlier runner codeunit with this id.
    [Test]
    [HandlerFunctions('MsgHandler')]
    procedure M_SharedSession()
    begin
        Message('hello');
        Assert.Equal(1, Memo.SeenLoginCalls(), 'login calls');
        Assert.Equal(SessionId(), Memo.LastRunId(), 'OnRun id');
        Assert.Equal(0, Memo.RunsWithOtherId(SessionId()), 'runner codeunits with another id');
    end;

    [MessageHandler]
    procedure MsgHandler(Msg: Text[1024])
    begin
    end;
}

// Not measured: unfiltered reads of Active Session and Session (probe hand-off, "What stays unmeasured"). Main's answer.
codeunit 53923 "SRS U Unfiltered"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    [Test]
    procedure U_ActiveSessionUnfiltered()
    var
        ActiveSession: Record "Active Session";
        Assert: Codeunit "SRS Assert";
    begin
        Assert.Equal(1, ActiveSession.Count(), 'active session rows');
    end;

    [Test]
    procedure U_SessionUnfiltered()
    var
        SessionRow: Record Session;
        Assert: Codeunit "SRS Assert";
    begin
        Assert.Equal(1, SessionRow.Count(), 'session rows');
    end;
}

// Not measured: a runner codeunit that changes the login rows; the later ones may not read them.
codeunit 53930 "SRS W Changes Login Rows"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    trigger OnRun()
    var
        Row: Record "SRS Login Row";
    begin
        Row.Init();
        Row."Session Id" := -1;
        Row.Insert();
    end;

    [Test]
    procedure W_SeesOwnInsert()
    var
        Row: Record "SRS Login Row";
        Assert: Codeunit "SRS Assert";
    begin
        Assert.Equal(3, Row.Count(), 'login rows');
    end;
}

codeunit 53931 "SRS Y After Changed Rows"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    [Test]
    procedure Y_LoginRows()
    var
        Row: Record "SRS Login Row";
        Assert: Codeunit "SRS Assert";
    begin
        Assert.Equal(3, Row.Count(), 'login rows');
    end;

    [Test]
    procedure Y_OtherTableOnly()
    var
        Other: Record "SRS Other";
        Assert: Codeunit "SRS Assert";
    begin
        Assert.Equal(0, Other.Count(), 'other rows');
    end;
}

// Not measured: StopSession of the hub session's id from the runner session.
codeunit 53940 "SRS S Stop Hub"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    [Test]
    procedure S_StopHubSession()
    var
        Assert: Codeunit "SRS Assert";
    begin
        if Session.StopSession(Assert.HubId()) then;
    end;
}
