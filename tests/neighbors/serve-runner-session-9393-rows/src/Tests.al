// Corpus PR 1657, LgnRunnerSessions.al (e58a5a05a): hub rows :380-:391, runner rows :422-:444. The marker row an OnRun
// inserts is not measured yet: the facts below hold in any order of the runner codeunits (marker row there: 3 login
// table rows, else 2), the cell this app asks corpus PR 1657 for.
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
        Assert.Equal(Rows.Expected(), Rows.Total(), 'login table rows');
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
        Row.Insert();
    end;

    [Test]
    procedure W_Total()
    var
        Assert: Codeunit "SRS Assert";
        Rows: Codeunit "SRS Login Rows";
    begin
        Assert.Equal(1, Rows.Markers(), 'marker rows');
        Assert.Equal(Rows.Expected(), Rows.Total(), 'login table rows');
    end;
}

// After W: the marker row W's OnRun committed, and no second login row of the session (:424, :429-:434).
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
        Assert.Equal(1, Rows.Markers(), 'the row the OnRun of W inserted');
    end;

    [Test]
    procedure Y_Total()
    var
        Assert: Codeunit "SRS Assert";
        Rows: Codeunit "SRS Login Rows";
    begin
        Assert.Equal(Rows.Expected(), Rows.Total(), 'login table rows');
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
    procedure Markers(): Integer
    var
        Row: Record "SRS Login Row";
    begin
        Row.SetRange("Session Id", -1);
        exit(Row.Count());
    end;

    procedure Total(): Integer
    var
        Row: Record "SRS Login Row";
    begin
        exit(Row.Count());
    end;

    // The hub's login row and the runner session's one, plus the marker row when W's OnRun ran before.
    procedure Expected(): Integer
    begin
        if Markers() = 1 then
            exit(3);
        exit(2);
    end;
}
