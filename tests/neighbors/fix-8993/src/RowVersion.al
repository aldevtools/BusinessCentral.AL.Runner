table 98993 "RV8993 Item"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Note; Text[50]) { }
    }
    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

table 98994 "RV8993 Setup"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; Note; Text[50]) { }
        field(3; Watermark; BigInteger) { }
    }
    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

codeunit 98993 "RV8993 Reader"
{
    procedure ReadFirstRowVersion(): BigInteger
    var
        RecRef: RecordRef;
        FldRef: FieldRef;
    begin
        RecRef.Open(Database::"RV8993 Item");
        RecRef.FindFirst();
        FldRef := RecRef.Field(0);
        exit(FldRef.Value());
    end;
}

codeunit 98994 "RV8993 Tests"
{
    Subtype = Test;

    var
        Reader: Codeunit "RV8993 Reader";

    local procedure InsertThenOtherTable()
    var
        Item: Record "RV8993 Item";
        Setup: Record "RV8993 Setup";
    begin
        Item.DeleteAll();
        Setup.DeleteAll();
        Item.Code := 'A';
        Item.Insert();
        Setup.Code := 'S';
        Setup.Insert();
        Setup.Note := 'changed';
        Setup.Modify();
    end;

    [Test]
    procedure Control_InsertThenRead_FieldRefAboveZero()
    var
        Item: Record "RV8993 Item";
    begin
        Item.DeleteAll();
        Item.Code := 'A';
        Item.Insert();
        if Reader.ReadFirstRowVersion() <= 0 then
            Error('not above 0');
    end;

    [Test]
    procedure Reporter_FieldRefValue_LessOrEqualZero()
    begin
        InsertThenOtherTable();
        if Reader.ReadFirstRowVersion() <= 0 then
            Error('not above 0');
    end;

    [Test]
    procedure RecordField_GreaterThanZero()
    var
        Item: Record "RV8993 Item";
    begin
        InsertThenOtherTable();
        Item.Get('A');
        if not (Item.SystemRowVersion > 0) then
            Error('not above 0');
    end;

    [Test]
    procedure Refused_VariantLocalIntoBigInteger()
    var
        RecRef: RecordRef;
        V: Variant;
        RV: BigInteger;
    begin
        InsertThenOtherTable();
        RecRef.Open(Database::"RV8993 Item");
        RecRef.FindFirst();
        V := RecRef.Field(0).Value();
        RV := V;
        if RV = 0 then
            Error('zero');
    end;

    [Test]
    procedure StoredInField_ThenComparedWithZero()
    var
        Setup: Record "RV8993 Setup";
    begin
        InsertThenOtherTable();
        Setup.Get('S');
        Setup.Watermark := Reader.ReadFirstRowVersion();
        Setup.Modify();
        Setup.Get('S');
        if Setup.Watermark <= 0 then
            Error('not above 0');
    end;

    [Test]
    procedure Refused_Format()
    begin
        InsertThenOtherTable();
        Error(Format(Reader.ReadFirstRowVersion()));
    end;

    [Test]
    procedure Refused_ComparedWithOtherNumber()
    begin
        InsertThenOtherTable();
        if Reader.ReadFirstRowVersion() > 5 then
            Error('above 5');
    end;

    [Test]
    procedure Refused_Arithmetic()
    var
        RV: BigInteger;
    begin
        InsertThenOtherTable();
        RV := Reader.ReadFirstRowVersion() + 1;
    end;

    [Test]
    procedure Refused_TwoReadsCompared()
    var
        Setup: Record "RV8993 Setup";
    begin
        InsertThenOtherTable();
        Setup.Get('S');
        if Reader.ReadFirstRowVersion() > Setup.SystemRowVersion then
            Error('item after setup');
    end;

    [Test]
    procedure Refused_ConvertedToInteger()
    var
        I: Integer;
    begin
        InsertThenOtherTable();
        I := Reader.ReadFirstRowVersion();
    end;

    [Test]
    procedure Refused_FilterOnStoredValue()
    var
        Setup: Record "RV8993 Setup";
    begin
        InsertThenOtherTable();
        Setup.Get('S');
        Setup.Watermark := Reader.ReadFirstRowVersion();
        Setup.Modify();
        Setup.SetRange(Watermark, Reader.ReadFirstRowVersion());
        if Setup.IsEmpty() then
            Error('empty');
    end;
}
