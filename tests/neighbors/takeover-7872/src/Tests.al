// The shapes around a Char for-loop with one-character literal bounds (reporter's PR: `for c := '0' to '4'`).
codeunit 50100 "NB Char Loop"
{
    Subtype = Test;

    var
        GlobalChar: Char;

    local procedure Expect(Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('Expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure CharVar_LiteralBounds_To()
    var
        c: Char;
        s: Text;
    begin
        for c := '0' to '4' do
            s += Format(c);
        Expect(s, '01234');
    end;

    [Test]
    procedure CharVar_LiteralBounds_DownTo()
    var
        c: Char;
        s: Text;
    begin
        for c := '4' downto '0' do
            s += Format(c);
        Expect(s, '43210');
    end;

    [Test]
    procedure CharVar_LiteralFrom_CharVarTo()
    var
        c: Char;
        t: Char;
        s: Text;
    begin
        t := 'c';
        for c := 'a' to t do
            s += Format(c);
        Expect(s, 'abc');
    end;

    [Test]
    procedure CharVar_CharVarBounds()
    var
        c: Char;
        a: Char;
        t: Char;
        s: Text;
    begin
        a := 'a';
        t := 'c';
        for c := a to t do
            s += Format(c);
        Expect(s, 'abc');
    end;

    [Test]
    procedure CharVar_IntegerBounds()
    var
        c: Char;
        n: Integer;
    begin
        for c := 48 to 50 do
            n += 1;
        Expect(Format(n), '3');
    end;

    [Test]
    procedure CharVar_MultiCharLiteralBound()
    var
        c: Char;
        n: Integer;
    begin
        for c := '0' to '45' do
            n += 1;
        Expect(Format(n), '1');
    end;

    [Test]
    procedure IntegerVar_OneCharLiteralBounds()
    var
        i: Integer;
        n: Integer;
    begin
        for i := '0' to '4' do
            n += 1;
        Expect(Format(n), '5');
    end;

    [Test]
    procedure TextVar_OneCharLiteralBounds()
    var
        t: Text;
        n: Integer;
    begin
        for t := 'a' to 'b' do
            n += 1;
        Expect(Format(n), '2');
    end;

    [Test]
    procedure CharVar_LiteralFrom_IntegerLiteralTo()
    var
        c: Char;
        n: Integer;
    begin
        for c := '0' to 52 do
            n += 1;
        Expect(Format(n), '5');
    end;

    [Test]
    procedure CharVar_IntegerLiteralFrom_LiteralTo()
    var
        c: Char;
        n: Integer;
    begin
        for c := 48 to '4' do
            n += 1;
        Expect(Format(n), '5');
    end;

    [Test]
    procedure CharVar_CharVarFrom_LiteralTo()
    var
        c: Char;
        a: Char;
        n: Integer;
    begin
        a := 'c';
        for c := a to 'e' do
            n += 1;
        Expect(Format(n), '3');
    end;

    [Test]
    procedure CharVar_LiteralFrom_IntegerVarTo()
    var
        c: Char;
        i: Integer;
        n: Integer;
    begin
        i := 52;
        for c := '0' to i do
            n += 1;
        Expect(Format(n), '5');
    end;

    [Test]
    procedure CharVar_LiteralFrom_TextVarTo()
    var
        c: Char;
        t: Text;
        n: Integer;
    begin
        t := '4';
        for c := '0' to t do
            n += 1;
        Expect(Format(n), '5');
    end;

    [Test]
    procedure CharVar_LiteralFrom_ExpressionTo()
    var
        c: Char;
        n: Integer;
    begin
        for c := '0' to 'a' + 1 do
            n += 1;
        Expect(Format(n), '50');
    end;

    [Test]
    procedure CharVar_Global_LiteralBounds()
    var
        n: Integer;
    begin
        for GlobalChar := '0' to '4' do
            n += 1;
        Expect(Format(n), '5');
    end;

    [Test]
    procedure IntegerVar_LiteralFrom_IntegerLiteralTo()
    var
        i: Integer;
        n: Integer;
    begin
        for i := '0' to 52 do
            n += 1;
        Expect(Format(n), '5');
    end;

    [Test]
    procedure BigIntegerVar_OneCharLiteralBounds()
    var
        b: BigInteger;
        n: Integer;
    begin
        for b := '0' to '4' do
            n += 1;
        Expect(Format(n), '5');
    end;

    [Test]
    procedure DecimalVar_OneCharLiteralBounds()
    var
        d: Decimal;
        n: Integer;
    begin
        for d := '0' to '4' do
            n += 1;
        Expect(Format(n), '5');
    end;

    [Test]
    procedure CharVar_EmptyRange()
    var
        c: Char;
        n: Integer;
    begin
        for c := '4' to '0' do
            n += 1;
        Expect(Format(n), '0');
    end;

    [Test]
    procedure CharVar_NonAscii()
    var
        c: Char;
        n: Integer;
    begin
        for c := 'ä' to 'å' do
            n += 1;
        Expect(Format(n), '2');
    end;

    [Test]
    procedure CharVar_NonBmp_LiteralBounds()
    var
        c: Char;
        n: Integer;
    begin
        for c := '😀' to '😁' do
            n += 1;
        Expect(Format(n), '0');
    end;

    [Test]
    procedure CharVar_IntegerLiteral_OutOfCharRange()
    var
        c: Char;
        n: Integer;
    begin
        for c := '0' to 70000 do
            n += 1;
        Expect(Format(n), '0');
    end;

    [Test]
    procedure IntegerVar_OneCharLiteralBounds_DownTo()
    var
        i: Integer;
        n: Integer;
    begin
        for i := '4' downto '0' do
            n += 1;
        Expect(Format(n), '5');
    end;

    local procedure CountVarParam(var c: Char): Integer
    var
        n: Integer;
    begin
        for c := '0' to '4' do
            n += 1;
        exit(n);
    end;

    [Test]
    procedure CharVarParameter_LiteralBounds()
    var
        c: Char;
    begin
        Expect(Format(CountVarParam(c)), '5');
    end;

    [Test]
    procedure CharVar_CharLiteral_To_65535()
    var
        c: Char;
        n: Integer;
    begin
        for c := 'a' to 65535 do
            n += 1;
        Expect(Format(n), '0');
    end;

    [Test]
    procedure CharVar_Zero_To_CharLiteral()
    var
        c: Char;
        n: Integer;
    begin
        for c := 0 to 'a' do
            n += 1;
        Expect(Format(n), '0');
    end;

    [Test]
    procedure CharVar_CharLiteral_DownTo_One()
    var
        c: Char;
        n: Integer;
    begin
        for c := '4' downto 1 do
            n += 1;
        Expect(Format(n), '0');
    end;

    [Test]
    procedure CharVar_CharLiteral_To_48()
    var
        c: Char;
        n: Integer;
    begin
        for c := '0' to 48 do
            n += 1;
        Expect(Format(n), '0');
    end;

    [Test]
    procedure CharVar_52_DownTo_CharLiteral()
    var
        c: Char;
        n: Integer;
    begin
        for c := 52 downto '0' do
            n += 1;
        Expect(Format(n), '0');
    end;

    [Test]
    procedure CharVar_CharLiteral_DownTo_52()
    var
        c: Char;
        n: Integer;
    begin
        for c := '4' downto 52 do
            n += 1;
        Expect(Format(n), '0');
    end;

    [Test]
    procedure CharVar_48_DownTo_CharLiteral()
    var
        c: Char;
        n: Integer;
    begin
        for c := 48 downto '0' do
            n += 1;
        Expect(Format(n), '0');
    end;
}
