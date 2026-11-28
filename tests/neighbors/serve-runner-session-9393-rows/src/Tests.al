// Corpus PR 1657, LgnRunnerSessions.al (1d9af7cd): hub rows :380-:391, runner rows :422-:444. The marker row W's OnRun
// inserts is seen by W's tests and gone in every later runner codeunit (:795, :839, :877, :917); the login rows stay as the
// login left them, 2 (:792-:794). As in the probe, markers count only rows of this session (the corpus harness also runs
// each OnRun once in another session while it fills the suite, and those rows stay; not asserted there, not modelled here).
codeunit 53910 "SRS H Hub Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    // :385, :386: one login row, of the hub's session.
    [Test]
    procedure H_OneLoginRow()
    var
        Row: Record "SRS Login Row";
        Assert: Codeunit "SRS Assert";
    begin
        Assert.Equal(1, Row.Count(), 'login rows');
    end;
}

// A runner codeunit before W: no marker row yet in the runner's order. In the default order it starts from what the hub
// codeunits left, outside W's carry chain, and sees the hub's login row only on main and head alike (the entry's "not
// covered, main's behavior").
codeunit 53925 "SRS X Before W"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    [Test]
    procedure X_Total()
    var
        Assert: Codeunit "SRS Assert";
        Rows: Codeunit "SRS Login Rows";
    begin
        Assert.Equal(2, Rows.LoginRows(), 'login rows');
    end;
}

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
        Row."Writer Session" := SessionId();
        Row.Insert();
    end;

    [Test]
    procedure W_Total()
    var
        Assert: Codeunit "SRS Assert";
        Rows: Codeunit "SRS Login Rows";
    begin
        Assert.Equal(1, Rows.Markers(), 'marker rows');
        Assert.Equal(2, Rows.LoginRows(), 'login rows');
    end;
}

// After W: the marker row W's OnRun wrote is gone (:839, :877), and no second login row of the session (:424, :792-:794).
codeunit 53931 "SRS Y After Changed Rows"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    [Test]
    procedure Y_Marker()
    var
        Assert: Codeunit "SRS Assert";
        Rows: Codeunit "SRS Login Rows";
    begin
        Assert.Equal(0, Rows.Markers(), 'the row the OnRun of W inserted');
    end;

    [Test]
    procedure Y_Total()
    var
        Assert: Codeunit "SRS Assert";
        Rows: Codeunit "SRS Login Rows";
    begin
        Assert.Equal(2, Rows.LoginRows(), 'login rows');
    end;

    // :424: one login call in the session.
    [Test]
    procedure Y_MemoLoginCalls()
    var
        Assert: Codeunit "SRS Assert";
        Memo: Codeunit "SRS Memo";
    begin
        Assert.Equal(1, Memo.SeenLoginCalls(), 'login calls');
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

codeunit 53903 "SRS Login Rows"
{
    // The marker rows a runner codeunit of this session wrote (LgnRunnerSessions.al:707-713, rows of this session).
    procedure Markers(): Integer
    var
        Row: Record "SRS Login Row";
    begin
        Row.SetRange("Session Id", -1);
        Row.SetRange("Writer Session", SessionId());
        exit(Row.Count());
    end;

    // The login rows: the hub's and the runner session's (LgnRunnerSessions.al:698-704, :792).
    procedure LoginRows(): Integer
    var
        Row: Record "SRS Login Row";
    begin
        Row.SetRange("Session Id", 0);
        exit(Row.Count());
    end;
}
