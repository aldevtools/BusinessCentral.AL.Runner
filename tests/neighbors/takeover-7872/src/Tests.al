// The shapes around a Char for-loop with one-character literal bounds (reporter's PR: `for c := '0' to '4'`).
codeunit 50100 "NB Char Loop"
{
    Subtype = Test;

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
}
