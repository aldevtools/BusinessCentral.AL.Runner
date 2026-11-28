// Al-runner issue 8620: what a successful OnRun ran through counts for every test of its codeunit.
// Each codeunit is one neighbor shape of that rule.

// OnRun converts a Variant holding a text of digits into an Integer (assumption text-digits-to-integer), then succeeds.
codeunit 64950 "NB OnRun Assumed"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Got: Integer;

    trigger OnRun()
    var
        V: Variant;
    begin
        V := '12';
        Got := V;
    end;

    [Test]
    procedure FirstTestReadsNothing()
    var
        X: Integer;
    begin
        X := 1;
    end;

    [Test]
    procedure SecondTestReadsNothing()
    var
        X: Integer;
    begin
        X := 2;
    end;
}

// The codeunit after it has no OnRun: the marks of the codeunit before do not cross into it.
codeunit 64951 "NB No OnRun After"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ReadsNothing()
    var
        X: Integer;
    begin
        X := 3;
    end;
}

// OnRun runs no assumed path: its tests stay clean.
codeunit 64952 "NB OnRun Plain"
{
    Subtype = Test;
    TestPermissions = Disabled;

    trigger OnRun()
    var
        X: Integer;
    begin
        X := 12;
    end;

    [Test]
    procedure ReadsNothing()
    var
        X: Integer;
    begin
        X := 4;
    end;

    // The test itself runs the assumed path: its own mark, as on main.
    [Test]
    procedure RunsTheAssumedPathItself()
    var
        V: Variant;
        I: Integer;
    begin
        V := '12';
        I := V;
    end;
}

// OnRun runs the assumed path and then fails: main's refusal of the codeunit stays.
codeunit 64953 "NB OnRun Assumed Fails"
{
    Subtype = Test;
    TestPermissions = Disabled;

    trigger OnRun()
    var
        V: Variant;
        I: Integer;
    begin
        V := '12';
        I := V;
        Error('OnRun fails');
    end;

    [Test]
    procedure ReadsNothing()
    var
        X: Integer;
    begin
        X := 5;
    end;
}
