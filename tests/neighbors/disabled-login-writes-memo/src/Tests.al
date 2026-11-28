codeunit 52610 "R5 A Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure A_MemoThenWrite()
    var
        Data: Record "DLW Data";
        Memo: Codeunit "DLW Memo";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(0, Memo.Get(), 'memo at login');
        Data.Code := 'A';
        Data.Insert();
        Commit();
    end;
}

codeunit 52612 "R5 C Codeunit After Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Codeunit;

    [Test]
    procedure C_MemoOfTheOneLogin()
    var
        Data: Record "DLW Data";
        Row: Record "DLW Login Row";
        Memo: Codeunit "DLW Memo";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Data.Count(), 'rows A left');
        Assert.Equal(1, Row.Count(), 'login rows');
        Assert.Equal(0, Memo.Get(), 'memo the one login filled');
    end;
}

table 52604 "R5 Own"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Code; Code[10]) { }
    }

    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

// Runner 130450 runs it after A; its OnRun writes, so it starts from what A left. Its test reads the memo the one login
// filled: refused (what BC runs for it is not measured).
codeunit 52620 "R5 F Memo 130450"
{
    Subtype = Test;

    trigger OnRun()
    var
        Own: Record "R5 Own";
    begin
        Own.Code := 'F';
        Own.Insert();
    end;

    [Test]
    procedure F_ReadsTheMemo()
    var
        Memo: Codeunit "DLW Memo";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(0, Memo.Get(), 'memo');
    end;
}

// As F, but its test reads the table the login wrote: refused.
codeunit 52621 "R5 G Login Rows 130450"
{
    Subtype = Test;

    trigger OnRun()
    var
        Own: Record "R5 Own";
    begin
        Own.Code := 'G';
        Own.Insert();
    end;

    [Test]
    procedure G_ReadsTheLoginRows()
    var
        Row: Record "DLW Login Row";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Row.Count(), 'login rows');
    end;
}
