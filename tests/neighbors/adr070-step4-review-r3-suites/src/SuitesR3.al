// Review round 3 of the ADR-070 step 4 reports: codeunit 50333 is in two BC sessions (`--test-suites`). In the first the
// earlier codeunit took from another sequence; in the second it took from the same one, so the failing test gets the
// numbers hint from the second. Reports only: the outcome is the same on main and on head.

codeunit 50330 "N74S3 Installer"
{
    Subtype = Install;

    trigger OnInstallAppPerCompany()
    begin
        NumberSequence.Insert('N74S3X', 1, 1);
        NumberSequence.Insert('N74S3Y', 1, 1);
    end;
}

codeunit 50331 "N74S3 X Taker"
{
    Subtype = Test;

    [Test]
    procedure TakesX()
    var
        N: BigInteger;
    begin
        N := NumberSequence.Next('N74S3X');
    end;
}

codeunit 50332 "N74S3 Y Taker"
{
    Subtype = Test;

    [Test]
    procedure TakesY()
    var
        N: BigInteger;
    begin
        N := NumberSequence.Next('N74S3Y');
    end;
}

codeunit 50333 "N74S3 Y Late Taker"
{
    Subtype = Test;

    [Test]
    procedure TakesYAndFails()
    var
        N: BigInteger;
    begin
        N := NumberSequence.Next('N74S3Y');
        Error('unrelated %1', N);
    end;
}
