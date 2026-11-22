// Neighbors of al-runner issue 8652: Format(Value, Length, FormatString) of an Integer or BigInteger in the cells corpus
// PR 1505 measured (Cell_*, Reader_*), the shapes main keeps (MainKept_*, MainRefused_*: main refuses them too), and the shapes no
// cell measures (Refused_*: refused on main and head).
table 60930 "FLF Row"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Qty; Integer) { }
    }
    keys
    {
        key(PK; "No.") { }
    }
}

codeunit 60930 "FLF Format Tests"
{
    Subtype = Test;

    local procedure Check(Got: Text; Want: Text)
    begin
        if Got <> Want then
            Error('got [%1] want [%2]', Got, Want);
    end;

    [Test]
    procedure Cell_IntegerShorter()
    var
        I: Integer;
    begin
        I := 8;
        Check(Format(I, 2, '<Integer>'), ' 8');
    end;

    [Test]
    procedure Cell_IntegerLongerCut()
    var
        I: Integer;
    begin
        I := 123;
        Check(Format(I, 2, '<Integer>'), '12');
    end;

    [Test]
    procedure Cell_IntegerNegativeLengthCut()
    var
        I: Integer;
    begin
        I := 123;
        Check(Format(I, -2, '<Integer>'), '12');
    end;

    [Test]
    procedure Cell_BigIntegerThousandCut()
    var
        B: BigInteger;
    begin
        B := 1234567890123L;
        Check(Format(B, 16, '<Integer Thousand>'), '1,234,567,890,12');
    end;

    [Test]
    procedure Cell_EmptyString()
    begin
        Check(Format(8, 5, ''), '     ');
    end;

    [Test]
    procedure Reader_Variant()
    var
        V: Variant;
        I: Integer;
    begin
        I := 8;
        V := I;
        Check(Format(V, 2, '<Integer>'), ' 8');
    end;

    [Test]
    procedure Reader_RecordField()
    var
        R: Record "FLF Row";
    begin
        R.Qty := 8;
        Check(Format(R.Qty, 2, '<Integer>'), ' 8');
    end;

    [Test]
    procedure Reader_FieldRefValue()
    var
        R: Record "FLF Row";
        RR: RecordRef;
    begin
        R.Qty := 8;
        RR.GetTable(R);
        Check(Format(RR.Field(2).Value, 2, '<Integer>'), ' 8');
    end;

    [Test]
    procedure MainRefused_FieldRef()
    var
        R: Record "FLF Row";
        RR: RecordRef;
        FR: FieldRef;
    begin
        R.Qty := 8;
        RR.GetTable(R);
        FR := RR.Field(2);
        Check(Format(FR, 2, '<Integer>'), ' 8');
    end;

    [Test]
    procedure MainKept_IntegerFillerEqual()
    begin
        Check(Format(1, 2, '<Integer,2><Filler Character,0>'), '01');
    end;

    [Test]
    procedure MainKept_IntegerLengthZero()
    begin
        Check(Format(-5, 0, '<Integer>'), '5');
    end;

    [Test]
    procedure MainKept_DecimalCut()
    var
        D: Decimal;
    begin
        D := 12.5;
        Check(Format(D, 3, '<Integer><Decimals>'), '12.');
    end;

    [Test]
    procedure MainKept_DateCut()
    begin
        Check(Format(DMY2Date(5, 3, 2024), 5, '<Year4>-<Month,2>'), '2024-');
    end;

    [Test]
    procedure MainRefused_LoneStandardFormat()
    begin
        Check(Format(42, 5, '<Standard Format,0>'), '   42');
    end;

    [Test]
    procedure Refused_IntegerNegativeEqual()
    var
        I: Integer;
    begin
        I := 12;
        Check(Format(I, -2, '<Integer>'), '12');
    end;

    [Test]
    procedure Refused_IntegerUnmeasuredString()
    begin
        Check(Format(8, 2, '<Integer,3>'), '');
    end;

    [Test]
    procedure Refused_SignIntegerLonger()
    begin
        Check(Format(-1234, 3, '<Sign><Integer>'), '');
    end;

    [Test]
    procedure Refused_NegativeLengthOtherString()
    begin
        Check(Format(1234, -3, '<Integer Thousand>'), '');
    end;

    [Test]
    procedure Refused_BigIntegerNegativeLength()
    var
        B: BigInteger;
    begin
        B := 123;
        Check(Format(B, -2, '<Integer>'), '');
    end;

    [Test]
    procedure Refused_BigIntegerUnmeasuredString()
    var
        B: BigInteger;
    begin
        B := 8;
        Check(Format(B, 5, '<Integer,4>'), '');
    end;

    [Test]
    procedure MainRefused_Byte()
    var
        B: Byte;
    begin
        B := 8;
        Check(Format(B, 2, '<Integer>'), '');
    end;

    [Test]
    procedure Refused_DecimalNegativeLength()
    var
        D: Decimal;
    begin
        D := 123;
        Check(Format(D, -2, '<Integer>'), '');
    end;

    [Test]
    procedure MainRefused_IntegerCommaString()
    begin
        Check(Format(1234, 8, '<Integer Thousand><Comma,.>'), '');
    end;
}
