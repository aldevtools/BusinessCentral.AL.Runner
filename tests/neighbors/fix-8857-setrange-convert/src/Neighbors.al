// Shapes next to the cells of corpus PR 1552 (SrcSetRangeConvert.al, SrcFilterConvertKinds.al) that keep the conversion they
// had, or that the change refuses. Each test raises what it sees, so a change of outcome shows in the neighbor diff.
table 57410 "SRN Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Short; Code[2]) { }
        field(3; Cod; Code[20]) { }
        field(4; Int; Integer) { }
        field(5; Bool; Boolean) { }
        field(6; D; Date) { }
        field(7; G; Guid) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 57411 "SRN Neighbors"
{
    Subtype = Test;

    local procedure Show(var F: FieldRef)
    begin
        Error('filter [%1]', F.GetFilter());
    end;

    [Test]
    procedure NumberLongerThanTheCodeField()
    var
        R: RecordRef;
        F: FieldRef;
        I: Integer;
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(2);
        I := 123;
        F.SetRange(I);
        Show(F);
    end;

    [Test]
    procedure NumberOverAMillionIntoCode()
    var
        R: RecordRef;
        F: FieldRef;
        I: Integer;
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(3);
        I := 1000000;
        F.SetRange(I);
        Show(F);
    end;

    [Test]
    procedure RecordSetRangeTypedCodeIntoInteger()
    var
        Rec: Record "SRN Row";
        C: Code[20];
    begin
        C := 'ABC';
        Rec.SetRange(Int, C);
        Error('filter [%1]', Rec.GetFilter(Int));
    end;

    [Test]
    procedure RecordSetRangeTwoVariantsIntoInteger()
    var
        Rec: Record "SRN Row";
        V: Variant;
    begin
        V := 'ABC';
        Rec.SetRange(Int, V, V);
        Error('filter [%1]', Rec.GetFilter(Int));
    end;

    [Test]
    procedure MixedVariantAndIntegerOnCode()
    var
        R: RecordRef;
        F: FieldRef;
        V: Variant;
        I: Integer;
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(3);
        V := 'A';
        I := 5;
        F.SetRange(V, I);
        Show(F);
    end;

    [Test]
    procedure TemporaryRecordRef()
    var
        R: RecordRef;
        F: FieldRef;
        C: Code[20];
    begin
        R.Open(Database::"SRN Row", true);
        F := R.Field(4);
        C := 'ABC';
        F.SetRange(C);
        Show(F);
    end;

    [Test]
    procedure FilterGroupTwo()
    var
        R: RecordRef;
        F: FieldRef;
        C: Code[20];
    begin
        R.Open(Database::"SRN Row");
        R.FilterGroup(2);
        F := R.Field(4);
        C := 'ABC';
        F.SetRange(C);
        Show(F);
    end;

    [Test]
    procedure SetFilterOtherText()
    var
        R: RecordRef;
        F: FieldRef;
        C: Code[20];
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(4);
        C := 'ABC';
        F.SetFilter('<>%1', C);
        Show(F);
    end;

    [Test]
    procedure FieldRefValueArgument()
    var
        R: RecordRef;
        F: FieldRef;
        F2: FieldRef;
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(4);
        F2 := R.Field(3);
        F2.Value := 'ABC';
        F.SetRange(F2.Value());
        Show(F);
    end;

    [Test]
    procedure IntegerTwoIntoBoolean()
    var
        R: RecordRef;
        F: FieldRef;
        I: Integer;
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(5);
        I := 2;
        F.SetRange(I);
        Show(F);
    end;

    [Test]
    procedure OneLetterIntoInteger()
    var
        R: RecordRef;
        F: FieldRef;
        C: Code[20];
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(4);
        C := 'A';
        F.SetRange(C);
        Show(F);
    end;

    [Test]
    procedure NegativeIntegerIntoDate()
    var
        R: RecordRef;
        F: FieldRef;
        I: Integer;
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(6);
        I := -3;
        F.SetRange(I);
        Show(F);
    end;

    [Test]
    procedure OtherLanguage()
    var
        R: RecordRef;
        F: FieldRef;
        C: Code[20];
    begin
        GlobalLanguage(1031);
        R.Open(Database::"SRN Row");
        F := R.Field(4);
        C := 'ABC';
        F.SetRange(C);
        Show(F);
    end;

    [Test]
    procedure MixedTextIntoGuid()
    var
        R: RecordRef;
        F: FieldRef;
        V: Variant;
        T: Text;
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(7);
        V := 'ABC';
        T := 'XY';
        F.SetRange(V, T);
        Show(F);
    end;

    [Test]
    procedure RecordSetFilterVariantIntoInteger()
    var
        Rec: Record "SRN Row";
        V: Variant;
    begin
        V := 'ABC';
        Rec.SetFilter(Int, '%1', V);
        Error('filter [%1]', Rec.GetFilter(Int));
    end;

    [Test]
    procedure RecordSetFilterTypedIntegerIntoCode()
    var
        Rec: Record "SRN Row";
        I: Integer;
    begin
        I := 5;
        Rec.SetFilter(Cod, '%1', I);
        Error('filter [%1]', Rec.GetFilter(Cod));
    end;

    [Test]
    procedure KeyRefFieldRef()
    var
        R: RecordRef;
        K: KeyRef;
        F: FieldRef;
        C: Code[20];
    begin
        R.Open(Database::"SRN Row");
        K := R.KeyIndex(1);
        F := K.FieldIndex(1);
        C := 'ABC';
        F.SetRange(C);
        Show(F);
    end;

    [Test]
    procedure FalseIntoCode()
    var
        R: RecordRef;
        F: FieldRef;
        B: Boolean;
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(3);
        B := false;
        F.SetRange(B);
        Show(F);
    end;

    [Test]
    procedure DecimalThreePlacesIntoCode()
    var
        R: RecordRef;
        F: FieldRef;
        D: Decimal;
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(3);
        D := 1.125;
        F.SetRange(D);
        Show(F);
    end;

    [Test]
    procedure DigitsIntoIntegerOnTemporary()
    var
        R: RecordRef;
        F: FieldRef;
        C: Code[20];
    begin
        R.Open(Database::"SRN Row", true);
        F := R.Field(4);
        C := '123';
        F.SetRange(C);
        Show(F);
    end;

    [Test]
    procedure TextIntoCodeOnRecord()
    var
        Rec: Record "SRN Row";
        T: Text;
    begin
        T := 'abc';
        Rec.SetRange(Cod, T);
        Error('filter [%1]', Rec.GetFilter(Cod));
    end;

    [Test]
    procedure RecordSetRangeTypedIntegerIntoBoolean()
    var
        Rec: Record "SRN Row";
        I: Integer;
    begin
        I := 1;
        Rec.SetRange(Bool, I);
        Error('filter [%1]', Rec.GetFilter(Bool));
    end;

    [Test]
    procedure EmptyTextIntoGuidTyped()
    var
        R: RecordRef;
        F: FieldRef;
        T: Text;
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(7);
        T := '';
        F.SetRange(T);
        Show(F);
    end;

    [Test]
    procedure LongDurationIntoDate()
    var
        R: RecordRef;
        F: FieldRef;
        X: Duration;
    begin
        R.Open(Database::"SRN Row");
        F := R.Field(6);
        X := 120000;
        F.SetRange(X);
        Show(F);
    end;
}
