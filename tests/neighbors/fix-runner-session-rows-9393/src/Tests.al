codeunit 54010 "FRS H Hub Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure H_OneLoginRow()
    var
        Row: Record "FRS Login Row";
        Assert: Codeunit "FRS Assert";
    begin
        Assert.Equal(1, Row.Count(), 'login rows');
    end;
}

// The measured shape with Commit() (LgnRunnerSessions.al:804-:843): its own tests see the row (:840).
codeunit 54020 "FRS C Insert Commit"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    trigger OnRun()
    var
        Rows: Codeunit "FRS Rows";
    begin
        Rows.Insert('C');
        Commit();
    end;

    [Test]
    procedure C_Own()
    var
        Assert: Codeunit "FRS Assert";
        Rows: Codeunit "FRS Rows";
    begin
        Assert.Equal(1, Rows.Written('C'), 'rows C inserted');
    end;
}

// Not measured: OnRun modifies the hub session's login row.
codeunit 54021 "FRS M Modify"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    trigger OnRun()
    var
        Row: Record "FRS Login Row";
    begin
        Row.SetRange(Writer, '');
        Row.FindFirst();
        Row.Note := 999;
        Row.Modify();
    end;

    [Test]
    procedure M_Own()
    var
        Assert: Codeunit "FRS Assert";
        Rows: Codeunit "FRS Rows";
    begin
        Assert.Equal(1, Rows.Modified(), 'modified login rows');
    end;
}

// Not measured: OnRun inserts into a table the login does not write.
codeunit 54022 "FRS O Other Table"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    trigger OnRun()
    var
        Other: Record "FRS Other";
    begin
        Other.Code := 'O';
        Other.Insert();
    end;

    [Test]
    procedure O_Own()
    var
        Assert: Codeunit "FRS Assert";
        Rows: Codeunit "FRS Rows";
    begin
        Assert.Equal(1, Rows.Other('O'), 'other rows O');
    end;
}

// Not measured: OnRun inserts into the login table and into another table.
codeunit 54023 "FRS B Both Tables"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    trigger OnRun()
    var
        Other: Record "FRS Other";
        Rows: Codeunit "FRS Rows";
    begin
        Rows.Insert('B');
        Other.Code := 'B';
        Other.Insert();
    end;

    [Test]
    procedure B_Own()
    var
        Assert: Codeunit "FRS Assert";
        Rows: Codeunit "FRS Rows";
    begin
        Assert.Equal(1, Rows.Written('B'), 'rows B inserted');
    end;
}

// Not measured: OnRun inserts into the login table and writes isolated storage.
codeunit 54024 "FRS S Storage"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    trigger OnRun()
    var
        Rows: Codeunit "FRS Rows";
    begin
        Rows.Insert('S');
        IsolatedStorage.Set('FRS S', 'x', DataScope::Module);
    end;

    [Test]
    procedure S_Own()
    var
        Assert: Codeunit "FRS Assert";
        Rows: Codeunit "FRS Rows";
    begin
        Assert.Equal(1, Rows.Written('S'), 'rows S inserted');
    end;
}

// After every writer: the measured shape is gone (LgnRunnerSessions.al:878, :918). The values of the other tests are
// main's answer (what OnRun wrote reaches the later codeunits), not BC's: not measured.
codeunit 54030 "FRS R Reader"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    [Test]
    procedure R_CommitRows()
    var
        Assert: Codeunit "FRS Assert";
        Rows: Codeunit "FRS Rows";
    begin
        Assert.Equal(0, Rows.Written('C'), 'rows C inserted');
    end;

    [Test]
    procedure R_Modified()
    var
        Assert: Codeunit "FRS Assert";
        Rows: Codeunit "FRS Rows";
    begin
        Assert.Equal(1, Rows.Modified(), 'modified login rows');
    end;

    [Test]
    procedure R_OtherO()
    var
        Assert: Codeunit "FRS Assert";
        Rows: Codeunit "FRS Rows";
    begin
        Assert.Equal(1, Rows.Other('O'), 'other rows O');
    end;

    [Test]
    procedure R_BothRows()
    var
        Assert: Codeunit "FRS Assert";
        Rows: Codeunit "FRS Rows";
    begin
        Assert.Equal(2, Rows.Written('B') + Rows.Other('B'), 'rows B inserted, login table and other');
    end;

    [Test]
    procedure R_StorageRows()
    var
        Assert: Codeunit "FRS Assert";
        Rows: Codeunit "FRS Rows";
    begin
        Assert.Equal(1, Rows.Written('S'), 'rows S inserted');
    end;
}
