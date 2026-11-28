// Review round 1 of the ADR-070 step 4 reports: the numbers cause of a stability flip in both directions (MED-1) and the
// numbers hint next to the carry chain (LOW-1). Reports only: the outcome is the same on main and on head. The same app
// runs in BC's order in ../adr070-step4-review-r1-strict.

codeunit 50301 "N74R Installer"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
        NumberSequence.Insert('N74RSEQ', 1, 1);
    end;
}

table 50300 "N74R Row"
{
    fields
    {
        field(1; Id; Integer) { }
    }
}

// Takes the first number of the sequence: passes in both orders.
codeunit 50302 "N74R First Taker"
{
    Subtype = Test;

    [Test]
    procedure TakesOne()
    var
        N: BigInteger;
    begin
        N := NumberSequence.Next('N74RSEQ');
    end;
}

// Expects the first number after a taker: passes in the isolated order (a passing taker: no hint, no property) and fails
// in BC's order. In a stability run it is a flip whose cause is the numbers (MED-1).
codeunit 50303 "N74R First After A Taker"
{
    Subtype = Test;

    [Test]
    procedure TakesTheFirstAfterATaker()
    begin
        if NumberSequence.Next('N74RSEQ') <> 1 then
            Error('took a later number');
    end;
}

// A carry-chain member (its OnRun writes a row under runner 130450) that takes a number and passes.
codeunit 50304 "N74R Chain Taker"
{
    Subtype = Test;

    trigger OnRun()
    var
        R: Record "N74R Row";
    begin
        R.Id := 1;
        R.Insert();
    end;

    [Test]
    procedure ChainTakesOne()
    var
        N: BigInteger;
    begin
        N := NumberSequence.Next('N74RSEQ');
    end;
}

// A second chain member that starts from the first's carry, takes a number and fails for an unrelated reason: its numbers
// hint names the last free taker (50303), not the chain member before it, whose numbers reached it (LOW-1).
codeunit 50305 "N74R Chain Second"
{
    Subtype = Test;

    trigger OnRun()
    var
        R: Record "N74R Row";
    begin
        R.Id := 2;
        R.Insert();
    end;

    [Test]
    procedure ChainTakesAndFails()
    var
        N: BigInteger;
    begin
        N := NumberSequence.Next('N74RSEQ');
        Error('unrelated');
    end;
}

// A codeunit outside the chain after the chain members: it runs again from their carry, so it saw their numbers (LOW-1).
codeunit 50306 "N74R Free After Chain"
{
    Subtype = Test;

    [Test]
    procedure FreeTakesAndFails()
    var
        N: BigInteger;
    begin
        N := NumberSequence.Next('N74RSEQ');
        Error('unrelated');
    end;
}
