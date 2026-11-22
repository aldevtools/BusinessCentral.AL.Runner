// Round 1 fix of the row-version-write-after-insert path (al-vm records/restamped.rs, records/stale.rs). One codeunit per
// shape, so a refusal in one leaves the others alone. Not a pin.
// Every other place that sends inserted rows notes them as the read does: a Commit and a Modify of their table. The other
// readers of the row version: an SQLTimestamp field and a query column. The write gate: a Delete after another table's rows
// were sent keeps main's refusal; a Delete, Rename or ModifyAll of the inserted rows only, beside a committed row, is assumed.
table 76640 "RVF Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; Name; Text[30]) { }
        field(3; Stamp; BigInteger) { SQLTimestamp = true; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 76641 "RVF Other"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

query 76642 "RVF Versions"
{
    elements
    {
        dataitem(Row; "RVF Row")
        {
            column(Id; Id) { }
            column(RowVersion; SystemRowVersion) { }
        }
    }
}

query 76643 "RVF Names"
{
    elements
    {
        dataitem(Row; "RVF Row")
        {
            column(Id; Id) { }
            column(Name; Name) { }
        }
    }
}

codeunit 76644 "RVF Setup"
{
    // Row inserted, Other inserted, Other sent by a read: Row's rows hold an Insert row version below Other's.
    procedure TwoTables()
    var
        Row: Record "RVF Row";
        Other: Record "RVF Other";
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Name := 'r';
        Row.Insert();
        Other.Id := 1;
        Other.Insert();
        Other.Get(1);
    end;
}

// A Commit sends Row's rows after Other's.
codeunit 76645 "RVF N 76645"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure CommitSendsAfterOtherTable()
    var
        Row: Record "RVF Row";
        Other: Record "RVF Other";
        Setup: Codeunit "RVF Setup";
    begin
        Setup.TwoTables();
        Commit();
        Row.Get(1);
        Other.Get(1);
        if Row.SystemRowVersion <= Other.SystemRowVersion then
            Error('row/other is not greater');
    end;
}

// A Modify of one inserted row, with no read before it, sends the other inserted row of its table after Other's.
codeunit 76646 "RVF N 76646"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ModifySendsAfterOtherTable()
    var
        Row: Record "RVF Row";
        Second: Record "RVF Row";
        Other: Record "RVF Other";
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Second.Id := 2;
        Second.Insert();
        Other.Id := 1;
        Other.Insert();
        Other.Get(1);
        Row.Name := 'm';
        Row.Modify();
        Second.Get(2);
        if Second.SystemRowVersion <= Other.SystemRowVersion then
            Error('second/other is not greater');
    end;
}

// The SQLTimestamp field of such a row holds its row version.
codeunit 76647 "RVF N 76647"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure SqlTimestampFieldRead()
    var
        Row: Record "RVF Row";
        Other: Record "RVF Other";
        Setup: Codeunit "RVF Setup";
    begin
        Setup.TwoTables();
        Row.Get(1);
        Other.Get(1);
        if Row.Stamp <= Other.SystemRowVersion then
            Error('stamp/other is not greater');
    end;
}

// A query column over SystemRowVersion of such a table.
codeunit 76648 "RVF N 76648"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure QueryRowVersionColumn()
    var
        Other: Record "RVF Other";
        Setup: Codeunit "RVF Setup";
        Q: Query "RVF Versions";
    begin
        Setup.TwoTables();
        Other.Get(1);
        Q.Open();
        if not Q.Read() then
            Error('no row');
        if Q.RowVersion <= Other.SystemRowVersion then
            Error('row/other is not greater');
    end;
}

// A query over the same table without the row version: as on main.
codeunit 76649 "RVF N 76649"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure QueryWithoutRowVersion()
    var
        Setup: Codeunit "RVF Setup";
        Q: Query "RVF Names";
    begin
        Setup.TwoTables();
        Q.Open();
        if not Q.Read() then
            Error('no row');
        if Q.Name <> 'r' then
            Error('wrong row');
    end;
}

// A Delete of the inserted row after another table's rows were sent (not measured): main's refusal.
codeunit 76650 "RVF N 76650"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure DeleteAfterOtherTableSent()
    var
        Row: Record "RVF Row";
        Setup: Codeunit "RVF Setup";
        Prev: BigInteger;
    begin
        Setup.TwoTables();
        Prev := LastUsedRowVersion();
        Row.Id := 1;
        Row.Delete();
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// A Delete of the inserted row beside a committed row of the table.
codeunit 76651 "RVF N 76651"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure DeleteInsertedRowBesideCommitted()
    var
        Row: Record "RVF Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        Commit();
        Row.Id := 2;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.Delete();
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// A Rename of the inserted row beside a committed row of the table.
codeunit 76652 "RVF N 76652"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure RenameInsertedRowBesideCommitted()
    var
        Row: Record "RVF Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        Commit();
        Row.Id := 2;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.Rename(5);
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// A ModifyAll of the inserted rows only, the committed row filtered out.
codeunit 76653 "RVF N 76653"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ModifyAllInsertedRowsBesideCommitted()
    var
        Row: Record "RVF Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        Commit();
        Row.Id := 2;
        Row.Insert();
        Row.Id := 3;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.Reset();
        Row.SetRange(Id, 2, 3);
        Row.ModifyAll(Name, 'y');
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// A Rename of an inserted row that a Modify wrote after its Insert (the last write of the key is no Insert): main's refusal.
codeunit 76654 "RVF N 76654"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure RenameOfModifiedInsertedRow()
    var
        Row: Record "RVF Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Name := 'm';
        Row.Modify();
        Row.Id := 2;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.Get(1);
        Row.Rename(5);
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}
