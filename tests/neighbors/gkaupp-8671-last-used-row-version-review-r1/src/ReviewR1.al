// Round 1 review shapes of the row-version-write-after-insert path (al-vm records/stale.rs). One codeunit per shape, so a
// refusal in one leaves the others alone. Not a pin.
// HIGH: rows whose Insert row version is below a row version an earlier send handed out (`note_read_for_row_versions`): BC gives
// them their row version when it sends them (corpus PR 1509, RowVersionWrites.al:392 InsertTwoTables `row/other` greater), the
// runtime keeps the Insert's. A read of their SystemRowVersion, and a find that filters or sorts on it, is refused; other reads run.
// MED: a Delete, Rename or ModifyAll of a committed row while the table holds inserted rows (not measured by PR 1509).
table 76620 "RVR Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 76621 "RVR Other"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 76622 "RVR Inst A"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 76623 "RVR Inst B"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// Install code in the InsertTwoTables shape: A inserted, B inserted, B read, A read.
codeunit 76624 "RVR Install"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    var
        A: Record "RVR Inst A";
        B: Record "RVR Inst B";
    begin
        A.Id := 1;
        A.Name := 'a';
        A.Insert();
        B.Id := 1;
        B.Name := 'b';
        B.Insert();
        B.Get(1);
        A.Get(1);
    end;
}

// The issue's watermark after InsertTwoTables: the mark read after Other was sent, then Row sent. BC finds Row above the mark.
codeunit 76625 "RVR N 76625"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure WatermarkFilterAfterTwoTables()
    var
        Row: Record "RVR Row";
        Other: Record "RVR Other";
        Mark: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Other.Id := 1;
        Other.Insert();
        Other.Get(1);
        Mark := LastUsedRowVersion();
        Row.Get(1);
        Row.Reset();
        Row.SetFilter(SystemRowVersion, '>%1', Mark);
        if not Row.FindFirst() then
            Error('the row above the mark was not found');
    end;
}

// Insert A, Insert B, read B, read A, no read of SystemRowVersion: LastUsedRowVersion grows, as on main.
codeunit 76626 "RVR N 76626"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TwoTablesReadBackwardsNoRowVersionRead()
    var
        Row: Record "RVR Row";
        Other: Record "RVR Other";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Prev := LastUsedRowVersion();
        Row.Id := 1;
        Row.Name := 'r';
        Row.Insert();
        Other.Id := 1;
        Other.Insert();
        Other.Get(1);
        Row.Get(1);
        if Row.Name <> 'r' then
            Error('wrong name');
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// InsertTwoTables with the row version read through FieldRef.Value.
codeunit 76627 "RVR N 76627"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TwoTablesRowVersionThroughFieldRef()
    var
        Row: Record "RVR Row";
        Other: Record "RVR Other";
        RecRef: RecordRef;
        RowVersion: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Other.Id := 1;
        Other.Insert();
        Other.Get(1);
        Row.Get(1);
        RecRef.GetTable(Row);
        RowVersion := RecRef.Field(Row.FieldNo(SystemRowVersion)).Value;
        if RowVersion <= Other.SystemRowVersion then
            Error('row/other is not greater');
    end;
}

// A row inserted before another table's rows were sent, sent after them by a read of its own table (not measured).
codeunit 76628 "RVR N 76628"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure InterleavedInsertsSentAfterOtherTable()
    var
        Row: Record "RVR Row";
        Other: Record "RVR Other";
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Other.Id := 1;
        Other.Insert();
        Row.Id := 2;
        Row.Insert();
        Other.Get(1);
        Row.Get(1);
        if Row.SystemRowVersion <= Other.SystemRowVersion then
            Error('row/other is not greater');
    end;
}

// InsertTwoTables, then a find sorted on SystemRowVersion.
codeunit 76629 "RVR N 76629"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure TwoTablesFindSortedOnRowVersion()
    var
        Row: Record "RVR Row";
        Other: Record "RVR Other";
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Other.Id := 1;
        Other.Insert();
        Other.Get(1);
        Row.Get(1);
        Row.Reset();
        Row.SetCurrentKey(SystemRowVersion);
        if not Row.FindLast() then
            Error('no row');
        if Row.Id <> 1 then
            Error('wrong row');
    end;
}

// A row install code inserted before another table's rows were sent, its SystemRowVersion read by a test.
codeunit 76630 "RVR N 76630"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure InstallRowsRowVersionRead()
    var
        A: Record "RVR Inst A";
        B: Record "RVR Inst B";
    begin
        A.Get(1);
        B.Get(1);
        if A.SystemRowVersion <= B.SystemRowVersion then
            Error('a/b is not greater');
    end;
}

// The same install rows read without SystemRowVersion: as on main.
codeunit 76631 "RVR N 76631"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure InstallRowsPlainRead()
    var
        A: Record "RVR Inst A";
        B: Record "RVR Inst B";
    begin
        A.Get(1);
        B.Get(1);
        if (A.Name <> 'a') or (B.Name <> 'b') then
            Error('wrong install rows');
    end;
}

// MED: a Delete of a committed row of the table that holds an unsent inserted row.
codeunit 76632 "RVR N 76632"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure DeleteCommittedRowOfInsertedTable()
    var
        Row: Record "RVR Row";
        Committed: Record "RVR Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        Commit();
        Row.Id := 2;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Committed.Id := 1;
        Committed.Delete();
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// MED: a Rename of a committed row of the table that holds an unsent inserted row.
codeunit 76633 "RVR N 76633"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure RenameCommittedRowOfInsertedTable()
    var
        Row: Record "RVR Row";
        Committed: Record "RVR Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        Commit();
        Row.Id := 2;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Committed.Id := 1;
        Committed.Rename(5);
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// MED: a ModifyAll that modifies a committed row besides the unsent inserted one.
codeunit 76634 "RVR N 76634"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ModifyAllWithCommittedRow()
    var
        Row: Record "RVR Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        Commit();
        Row.Id := 2;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.Reset();
        Row.ModifyAll(Name, 'y');
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}
