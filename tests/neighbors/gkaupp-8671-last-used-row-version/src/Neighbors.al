// Neighbor shapes of the row-version-write-after-insert path (al-vm records/stale.rs `row_version_after_write`) that corpus PR 1509
// does not measure, and the new read refusal. One codeunit per shape, so a refusal in one leaves the others alone. Not a pin.
table 76604 "RVW Child"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; RowId; Integer) { TableRelation = "RVW Row".Id; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// RowVersionWrites.al InsertTwoTables of corpus PR 1509 on its own: BC gives Row its row version when Row.Get sends it, after Other's.
codeunit 76606 "RVW N 76606"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ReadAfterOtherTableSent()
    var
        Row: Record "RVW Row";
        Other: Record "RVW Other Row";
        Prev: BigInteger;
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
        if Row.SystemRowVersion <= Other.SystemRowVersion then
            Error('row/other is not greater');
    end;
}

// A Delete while inserted rows of two tables are unsent.
codeunit 76607 "RVW N 76607"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure DeleteWithTwoTablesUnsent()
    var
        Row: Record "RVW Row";
        Other: Record "RVW Other Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Other.Id := 1;
        Other.Insert();
        Prev := LastUsedRowVersion();
        Row.Delete();
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// A Delete of a committed row of another table than the one with unsent inserted rows.
codeunit 76608 "RVW N 76608"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure DeleteOtherTableAfterInsert()
    var
        Row: Record "RVW Row";
        Other: Record "RVW Other Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Other.Id := 1;
        Other.Insert();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Other.Get(1);
        Other.Delete();
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// A Rename of a committed row of a table the open transaction did not insert into, after another table's Insert was sent.
codeunit 76609 "RVW N 76609"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure RenameNotInsertedTable()
    var
        Row: Record "RVW Row";
        Other: Record "RVW Other Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Commit();
        Other.Id := 1;
        Other.Insert();
        Other.Get(1);
        Row.Get(1);
        Prev := LastUsedRowVersion();
        Row.Rename(5);
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// A Rename of an unsent inserted row that rewrites a related row of another table.
codeunit 76610 "RVW N 76610"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure RenameRewritesRelatedRows()
    var
        Row: Record "RVW Row";
        Other: Record "RVW Other Row";
        Child: Record "RVW Child";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Child.DeleteAll();
        Child.Id := 1;
        Child.RowId := 2;
        Child.Insert();
        Commit();
        Row.Id := 2;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.Rename(3);
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
        Child.Get(1);
        if Child.RowId <> 3 then
            Error('the related row was not renamed');
    end;
}

// A ModifyAll of the table with unsent inserted rows whose filter matches no row.
codeunit 76611 "RVW N 76611"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ModifyAllModifiesNoRow()
    var
        Row: Record "RVW Row";
        Other: Record "RVW Other Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Row.SetRange(Id, 5);
        Row.ModifyAll(Name, 'y');
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// A ModifyAll of a committed row of another table than the one with unsent inserted rows.
codeunit 76612 "RVW N 76612"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ModifyAllOtherTableAfterInsert()
    var
        Row: Record "RVW Row";
        Other: Record "RVW Other Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Other.Id := 1;
        Other.Insert();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Prev := LastUsedRowVersion();
        Other.ModifyAll(Name, 'y');
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// A second Rename after the Rename of an unsent inserted row.
codeunit 76613 "RVW N 76613"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure RenameTwiceAfterInsert()
    var
        Row: Record "RVW Row";
        Other: Record "RVW Other Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Rename(2);
        Prev := LastUsedRowVersion();
        Row.Rename(3);
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// A Rename after the Modify of an inserted row.
codeunit 76614 "RVW N 76614"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure RenameAfterModifyOfInsert()
    var
        Row: Record "RVW Row";
        Other: Record "RVW Other Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Name := 'x';
        Row.Modify();
        Prev := LastUsedRowVersion();
        Row.Rename(2);
        if LastUsedRowVersion() <= Prev then
            Error('LastUsedRowVersion did not grow');
    end;
}

// A Delete after the Rename of an unsent inserted row.
codeunit 76615 "RVW N 76615"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure DeleteAfterRenameOfInsert()
    var
        Row: Record "RVW Row";
        Other: Record "RVW Other Row";
        Prev: BigInteger;
    begin
        Row.DeleteAll();
        Other.DeleteAll();
        Commit();
        Row.Id := 1;
        Row.Insert();
        Row.Rename(2);
        Prev := LastUsedRowVersion();
        Row.Delete();
        if LastUsedRowVersion() < Prev then
            Error('LastUsedRowVersion went down');
    end;
}
