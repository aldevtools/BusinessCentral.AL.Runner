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
        Other: Record "RVS Other";
    begin
        Row.DeleteAll();
        Other.DeleteAll();
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

    // Neighbors of the DeleteAll rule (assumption row-version-delete-all): each reads the value after a DeleteAll and an Insert.
    [Test]
    procedure DeleteAllFalseArgument_NoInsert_ThenInsert()
    var
        Row: Record "RVS Row";
        Other: Record "RVS Other";
        V: BigInteger;
    begin
        Start();
        Row.DeleteAll(false);
        Other.Id := 5;
        Other.Insert();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure DeleteAllRecordRef_NoInsert_ThenInsert()
    var
        Other: Record "RVS Other";
        RecRef: RecordRef;
        V: BigInteger;
    begin
        Start();
        RecRef.Open(Database::"RVS Row");
        RecRef.DeleteAll();
        Other.Id := 5;
        Other.Insert();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure DeleteAll_OtherTableUnsent()
    var
        Row: Record "RVS Row";
        Other: Record "RVS Other";
        V: BigInteger;
    begin
        Start();
        Other.Id := 5;
        Other.Insert();
        Row.DeleteAll();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure DeleteAllTrue_TriggerWrites_NoInsert()
    var
        Writing: Record "RVN Writing Row";
        Other: Record "RVS Other";
        V: BigInteger;
    begin
        Start();
        Writing.Id := 1;
        Writing.Insert();
        Commit();
        Writing.DeleteAll(true);
        Other.Id := 6;
        Other.Insert();
        V := LastUsedRowVersion();
    end;

    // Gates of the DeleteAll rule after an Insert: a filter that deletes an inserted row, DeleteAll(true), MarkedOnly.
    [Test]
    procedure DeleteAllFiltered_DeletesInsertedRow()
    var
        Row: Record "RVS Row";
        V: BigInteger;
    begin
        Start();
        Row.Id := 7;
        Row.Insert();
        Row.Reset();
        Row.SetRange(Id, 7);
        Row.DeleteAll();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure DeleteAllTrue_AfterInsert()
    var
        Row: Record "RVS Row";
        V: BigInteger;
    begin
        Start();
        Row.Id := 7;
        Row.Insert();
        Row.DeleteAll(true);
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure DeleteAllMarkedOnly_NoInsert_ThenInsert()
    var
        Row: Record "RVS Row";
        Other: Record "RVS Other";
        V: BigInteger;
    begin
        Start();
        Row.Get(1);
        Row.Mark(true);
        Row.MarkedOnly(true);
        Row.DeleteAll();
        Other.Id := 5;
        Other.Insert();
        V := LastUsedRowVersion();
    end;
    [Test]
    procedure CheckedInsert_OtherTableUnsent()
    var
        Row: Record "RVS Row";
        Other: Record "RVS Other";
        V: BigInteger;
    begin
        Start();
        Other.Id := 10;
        Other.Insert();
        Row.Id := 10;
        if not Row.Insert() then
            Error('insert failed');
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure CheckedInsert_OtherCompany()
    var
        Row: Record "RVS Row";
        V: BigInteger;
    begin
        Start();
        Row.ChangeCompany(OtherCompany());
        Row.Id := 10;
        if not Row.Insert() then
            Error('insert failed');
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure DeleteAllAfterInsert_OtherCompany()
    var
        Row: Record "RVS Row";
        V: BigInteger;
    begin
        Start();
        Row.ChangeCompany(OtherCompany());
        Row.Id := 10;
        Row.Insert();
        Row.DeleteAll();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure TwoTables_OneRowEach()
    var
        Row: Record "RVS Row";
        Other: Record "RVS Other";
    begin
        Start();
        Row.Id := 10;
        Row.Insert();
        Other.Id := 10;
        Other.Insert();
        if LastUsedRowVersion() <> Other.SystemRowVersion then
            Error('LastUsedRowVersion is not the second table''s row version');
    end;

    [Test]
    procedure TwoTables_ThirdTableInsert()
    var
        Row: Record "RVS Row";
        Other: Record "RVS Other";
        Trig: Record "RVN Trigger Row";
        V: BigInteger;
    begin
        Start();
        Row.Id := 10;
        Row.Insert();
        Other.Id := 10;
        Other.Insert();
        Trig.Id := 10;
        Trig.Insert();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure TwoTables_SecondInsertSecondTable()
    var
        Row: Record "RVS Row";
        Other: Record "RVS Other";
        V: BigInteger;
    begin
        Start();
        Row.Id := 10;
        Row.Insert();
        Other.Id := 10;
        Other.Insert();
        Other.Id := 11;
        Other.Insert();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure TwoTables_TwoRowsFirstTable()
    var
        Row: Record "RVS Row";
        Other: Record "RVS Other";
        V: BigInteger;
    begin
        Start();
        Row.Id := 10;
        Row.Insert();
        Row.Id := 11;
        Row.Insert();
        Other.Id := 10;
        Other.Insert();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure TwoTables_DeleteAfter()
    var
        Row: Record "RVS Row";
        Other: Record "RVS Other";
        V: BigInteger;
    begin
        Start();
        Row.Id := 10;
        Row.Insert();
        Other.Id := 10;
        Other.Insert();
        Row.Delete();
        V := LastUsedRowVersion();
    end;

    [Test]
    procedure TwoTables_SecondThroughRecordRef()
    var
        Row: Record "RVS Row";
        RecRef: RecordRef;
        V: BigInteger;
    begin
        Start();
        Row.Id := 10;
        Row.Insert();
        RecRef.Open(Database::"RVS Other");
        RecRef.Field(1).Value := 10;
        RecRef.Insert();
        V := LastUsedRowVersion();
    end;

    local procedure OtherCompany(): Text[30]
    begin
        exit('RVN Other Company');
    end;
}

table 76653 "RVN Writing Row"
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
    trigger OnDelete()
    var
        Other: Record "RVS Other";
    begin
        Other.Id := 900 + Id;
        Other.Insert();
    end;
}

codeunit 76654 "RVN Isolation Function"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    [Test]
    procedure CheckedInsert_IsolationFunction()
    var
        Row: Record "RVS Row";
        V: BigInteger;
    begin
        Row.SetRange(Id, 10);
        Row.DeleteAll();
        Commit();
        Row.Id := 10;
        if not Row.Insert() then
            Error('insert failed');
        V := LastUsedRowVersion();
    end;
}
