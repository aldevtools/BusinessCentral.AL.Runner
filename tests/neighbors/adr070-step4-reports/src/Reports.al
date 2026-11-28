// ADR-070 step 4 changes what the reports say about a test (order mode, the numbers hint, the reads of a passing test,
// the stability cause), not its outcome. Each codeunit is one shape of those reports; the outcome is the same on main and
// on head. The same app runs in BC's order in ../adr070-step4-reports-strict.

codeunit 50200 "N74 State"
{
    SingleInstance = true;

    var
        Flag: Text;

    procedure SetFlag(V: Text)
    begin
        Flag := V;
    end;

    procedure GetFlag(): Text
    begin
        exit(Flag);
    end;
}

codeunit 50201 "N74 Installer"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
        NumberSequence.Insert('N74SEQ', 1, 1);
    end;
}

// Writes the single instance a later codeunit reads.
codeunit 50202 "N74 Setter"
{
    Subtype = Test;

    [Test]
    procedure SetsTheFlag()
    var
        S: Codeunit "N74 State";
    begin
        S.SetFlag('left');
    end;
}

// A reader that passes with fresh state (decision 4: its reads are kept on the passing test) and fails in BC's order, and
// a test of the same codeunit that reads nothing after it.
codeunit 50203 "N74 Passing Reader"
{
    Subtype = Test;

    [Test]
    procedure ReadsTheEmptyFlag()
    var
        S: Codeunit "N74 State";
    begin
        if S.GetFlag() <> '' then
            Error('flag is set');
    end;

    [Test]
    procedure ReadsNothing()
    begin
    end;
}

// A reader that passes in both orders: it reads the flag and accepts either value.
codeunit 50204 "N74 Tolerant Reader"
{
    Subtype = Test;

    [Test]
    procedure ReadsEitherValue()
    var
        S: Codeunit "N74 State";
    begin
        if not (S.GetFlag() in ['', 'left']) then
            Error('unexpected flag %1', S.GetFlag());
    end;
}

// Takes the first number of the sequence: passes in both orders.
codeunit 50205 "N74 First Taker"
{
    Subtype = Test;

    [Test]
    procedure TakesOne()
    var
        N: BigInteger;
    begin
        N := NumberSequence.Next('N74SEQ');
    end;
}

// Expects the second number: fails in the isolated order (decision 3: the numbers hint names the sequence) and passes in
// BC's order (no numbers hint there).
codeunit 50206 "N74 Second Taker"
{
    Subtype = Test;

    [Test]
    procedure TakesTheSecond()
    begin
        if NumberSequence.Next('N74SEQ') <> 2 then
            Error('took the first number');
    end;
}

// Takes a number and passes whatever it gets: a passing taker has no numbers hint.
codeunit 50207 "N74 Any Taker"
{
    Subtype = Test;

    [Test]
    procedure TakesAny()
    var
        N: BigInteger;
    begin
        N := NumberSequence.Next('N74SEQ');
    end;
}
