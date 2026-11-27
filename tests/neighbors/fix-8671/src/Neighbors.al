// Neighbors of the checked-Insert rule (assumption row-version-checked-insert, corpus PR 1621) and of the write shapes
// RowVersionWriteShapes.al does not measure. Each test reads LastUsedRowVersion() after its shape; none asserts a number.
table 76650 "RVN Blob Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 76651 "RVN Trigger Row"
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
    trigger OnDelete()
    begin
    end;

    trigger OnModify()
    begin
    end;
}

codeunit 76652 "RVN Neighbors"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Start()
    var
        Row: Record "RVS Row";
        Blob: Record "RVN Blob Row";
        Trig: Record "RVN Trigger Row";
    begin
        Row.DeleteAll();
        Blob.DeleteAll();
        Trig.DeleteAll();
        Row.Id := 1;
        Row.Insert();
        Trig.Id := 1;
        Trig.Insert();
        Commit();
    end;

    [Test]
    procedure CheckedInsert_AfterPlainInsertSameTable()
    var
        Row: Record "RVS Row";
        V: BigInteger;
    begin
        Start();
        Row.Id := 10;
        Row.Insert();
        Row.Id := 11;
        if not Row.Insert() then
            Error('insert failed');
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure CheckedInsert_AfterLockTable()
    var
        Row: Record "RVS Row";
        V: BigInteger;
    begin
        Start();
        Row.LockTable();
        Row.Id := 10;
        if not Row.Insert() then
            Error('insert failed');
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure CheckedInsert_ThroughRecordRef()
    var
        RecRef: RecordRef;
        V: BigInteger;
    begin
        Start();
        RecRef.Open(Database::"RVS Row");
        RecRef.Field(1).Value := 10;
        if not RecRef.Insert() then
            Error('insert failed');
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure CheckedInsert_BlobTable()
    var
        Blob: Record "RVN Blob Row";
        V: BigInteger;
    begin
        Start();
        Blob.Id := 10;
        if not Blob.Insert() then
            Error('insert failed');
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure CheckedInsert_Temporary()
    var
        Row: Record "RVS Row" temporary;
        V: BigInteger;
    begin
        Start();
        Row.Id := 10;
        if not Row.Insert() then
            Error('insert failed');
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure StatementInsert_ThenRead()
    var
        Row: Record "RVS Row";
        V: BigInteger;
    begin
        Start();
        Row.Id := 10;
        Row.Insert();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure DeleteAllTrue_OnDelete_AfterInsert()
    var
        Trig: Record "RVN Trigger Row";
        V: BigInteger;
    begin
        Start();
        Trig.Id := 10;
        Trig.Insert();
        Trig.DeleteAll(true);
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure ModifyAllTrue_AfterInsert()
    var
        Trig: Record "RVN Trigger Row";
        V: BigInteger;
    begin
        Start();
        Trig.Id := 10;
        Trig.Insert();
        Trig.ModifyAll(Name, 'x', true);
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure TruncateFiltered_AfterInsert()
    var
        Row: Record "RVS Row";
        V: BigInteger;
    begin
        Start();
        Row.Id := 10;
        Row.Insert();
        Row.SetRange(Id, 10);
        Row.Truncate();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure RenameCommittedRow()
    var
        Row: Record "RVS Row";
        V: BigInteger;
    begin
        Start();
        Row.Get(1);
        Row.Rename(5);
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure RecordRefModify_AfterInsert()
    var
        Row: Record "RVS Row";
        RecRef: RecordRef;
        V: BigInteger;
    begin
        Start();
        Row.Id := 10;
        Row.Insert();
        RecRef.Open(Database::"RVS Row");
        RecRef.Get(Row.RecordId);
        RecRef.Field(2).Value := 'x';
        RecRef.Modify();
        V := LastUsedRowVersion();
    end;
}
