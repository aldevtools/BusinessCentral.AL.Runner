// Neighbors of al-runner issue 9026: a Boolean value into a Code or Text field by FieldRef.SetRange, FieldRef.SetFilter and
// Record.SetRange, and the shapes next to it. Each test raises an error that carries the filter, so both binaries' answers compare.
table 51001 "BTN Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Cod; Code[20]) { }
        field(3; Txt; Text[50]) { }
        field(4; Cod2; Code[2]) { }
        field(5; Cod1; Code[1]) { }
        field(6; Txt1; Text[1]) { }
        field(7; Cod10; Code[10]) { }
        field(8; Int; Integer) { }
        field(9; Bool; Boolean) { }
        field(10; D; Date) { }
        field(11; Dec; Decimal) { }
        field(12; G; Guid) { }
        field(13; Opt; Option) { OptionMembers = Zero,One; }
        field(14; CodFilter; Code[20]) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 51101 "BTN Neighbors"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure FalseRange(FieldNo: Integer)
    var
        R: RecordRef;
        F: FieldRef;
        X: Boolean;
    begin
        R.Open(Database::"BTN Row");
        F := R.Field(FieldNo);
        X := false;
        F.SetRange(X);
        Error('F:[%1]', F.GetFilter());
    end;

    local procedure TrueRange(FieldNo: Integer)
    var
        R: RecordRef;
        F: FieldRef;
        X: Boolean;
    begin
        R.Open(Database::"BTN Row");
        F := R.Field(FieldNo);
        X := true;
        F.SetRange(X);
        Error('F:[%1]', F.GetFilter());
    end;

    // Routes of the probe on Code[20] and Text[50].
    [Test]
    procedure FalseVariantSetRangeCode()
    var
        R: RecordRef;
        F: FieldRef;
        V: Variant;
    begin
        R.Open(Database::"BTN Row");
        F := R.Field(2);
        V := false;
        F.SetRange(V);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure FalseTwoValuesCode()
    var
        R: RecordRef;
        F: FieldRef;
        X: Boolean;
    begin
        R.Open(Database::"BTN Row");
        F := R.Field(2);
        X := false;
        F.SetRange(X, X);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure FalseVariantSetFilterText()
    var
        R: RecordRef;
        F: FieldRef;
        V: Variant;
    begin
        R.Open(Database::"BTN Row");
        F := R.Field(3);
        V := false;
        F.SetFilter('%1', V);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure FalseRecordSetRangeVariantCode()
    var
        Row: Record "BTN Row";
        V: Variant;
    begin
        V := false;
        Row.SetRange(Cod, V);
        Error('F:[%1]', Row.GetFilter(Cod));
    end;

    [Test]
    procedure FalseRecordSetRangeVariantText()
    var
        Row: Record "BTN Row";
        V: Variant;
    begin
        V := false;
        Row.SetRange(Txt, V);
        Error('F:[%1]', Row.GetFilter(Txt));
    end;

    [Test]
    procedure FalseLiteralSetRangeCode()
    var
        R: RecordRef;
        F: FieldRef;
    begin
        R.Open(Database::"BTN Row");
        F := R.Field(2);
        F.SetRange(false);
        Error('F:[%1]', F.GetFilter());
    end;

    // Field lengths.
    [Test]
    procedure FalseIntoCode2()
    begin
        FalseRange(4);
    end;

    [Test]
    procedure FalseIntoCode1()
    begin
        FalseRange(5);
    end;

    [Test]
    procedure FalseIntoText1()
    begin
        FalseRange(6);
    end;

    [Test]
    procedure FalseIntoCode10()
    begin
        FalseRange(7);
    end;

    [Test]
    procedure TrueIntoCode2()
    begin
        TrueRange(4);
    end;

    [Test]
    procedure TrueIntoCode1()
    begin
        TrueRange(5);
    end;

    [Test]
    procedure TrueIntoText1()
    begin
        TrueRange(6);
    end;

    [Test]
    procedure TrueIntoCode()
    begin
        TrueRange(2);
    end;

    // Shapes the probe does not measure.
    [Test]
    procedure FalseOnTemporary()
    var
        R: RecordRef;
        F: FieldRef;
        X: Boolean;
    begin
        R.Open(Database::"BTN Row", true);
        F := R.Field(2);
        X := false;
        F.SetRange(X);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure FalseFilterGroupTwo()
    var
        R: RecordRef;
        F: FieldRef;
        X: Boolean;
    begin
        R.Open(Database::"BTN Row");
        R.FilterGroup(2);
        F := R.Field(2);
        X := false;
        F.SetRange(X);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure FalseOtherFilterText()
    var
        R: RecordRef;
        F: FieldRef;
        X: Boolean;
    begin
        R.Open(Database::"BTN Row");
        F := R.Field(2);
        X := false;
        F.SetFilter('%1|%2', X, X);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure FalseRecordSetRangeTyped()
    var
        Row: Record "BTN Row";
        X: Boolean;
    begin
        X := false;
        Row.SetRange(Cod, X);
        Error('F:[%1]', Row.GetFilter(Cod));
    end;

    [Test]
    procedure FalseRecordSetFilter()
    var
        Row: Record "BTN Row";
        X: Boolean;
    begin
        X := false;
        Row.SetFilter(Cod, '%1', X);
        Error('F:[%1]', Row.GetFilter(Cod));
    end;

    [Test]
    procedure FalseKeyRefField()
    var
        R: RecordRef;
        K: KeyRef;
        F: FieldRef;
        X: Boolean;
    begin
        R.Open(Database::"BTN Row");
        K := R.KeyIndex(1);
        F := K.FieldIndex(1);
        X := false;
        F.SetRange(X);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure FalseIntoFlowFilter()
    begin
        FalseRange(14);
    end;

    // Other pairs of types.
    [Test]
    procedure FalseIntoDate()
    begin
        FalseRange(10);
    end;

    [Test]
    procedure FalseIntoInteger()
    begin
        FalseRange(8);
    end;

    [Test]
    procedure FalseIntoDecimal()
    begin
        FalseRange(11);
    end;

    [Test]
    procedure FalseIntoGuid()
    begin
        FalseRange(12);
    end;

    [Test]
    procedure FalseIntoOption()
    begin
        FalseRange(13);
    end;

    [Test]
    procedure FalseIntoBoolean()
    begin
        FalseRange(9);
    end;

    [Test]
    procedure IntegerFiveIntoBoolean()
    var
        R: RecordRef;
        F: FieldRef;
        X: Integer;
    begin
        R.Open(Database::"BTN Row");
        F := R.Field(9);
        X := 5;
        F.SetRange(X);
        Error('F:[%1]', F.GetFilter());
    end;

    [Test]
    procedure OptionIntoCode()
    var
        R: RecordRef;
        F: FieldRef;
        X: Option Zero,One;
    begin
        R.Open(Database::"BTN Row");
        F := R.Field(2);
        X := X::One;
        F.SetRange(X);
        Error('F:[%1]', F.GetFilter());
    end;
}

// A test that changes the session language runs in a codeunit of its own.
codeunit 51102 "BTN Language Neighbor"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure FalseOtherLanguage()
    var
        R: RecordRef;
        F: FieldRef;
        X: Boolean;
        Lang: Integer;
    begin
        Lang := GlobalLanguage();
        GlobalLanguage(1031);
        R.Open(Database::"BTN Row");
        F := R.Field(2);
        X := false;
        F.SetRange(X);
        GlobalLanguage(Lang);
        Error('F:[%1]', F.GetFilter());
    end;
}
