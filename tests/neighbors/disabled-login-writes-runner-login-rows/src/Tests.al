codeunit 52610 "G1 A Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure A_Write()
    var
        Data: Record "DLW Data";
    begin
        Data.Code := 'A';
        Data.Insert();
        Commit();
    end;
}

// Runner 130450 (TestPermissions NonRestrictive), no OnRun: its test only checks that a login row exists.
// BC: the hub session's row and this session's own row exist, so it passes; main: its own row exists.
codeunit 52630 "G1 Login Row Exists"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    [Test]
    procedure G_LoginRowExists()
    var
        Row: Record "DLW Login Row";
    begin
        if Row.IsEmpty() then
            Error('no login row');
    end;
}

// Same, with an OnRun write (a carry member in the default order).
codeunit 52631 "G1 Login Row Exists OnRun"
{
    Subtype = Test;
    TestPermissions = NonRestrictive;

    trigger OnRun()
    var
        Own: Record "R5 Own";
    begin
        Own.Code := 'G';
        Own.Insert();
    end;

    [Test]
    procedure G2_LoginRowExists()
    var
        Row: Record "DLW Login Row";
    begin
        if Row.IsEmpty() then
            Error('no login row');
    end;
}

table 52604 "R5 Own"
{
    DataClassification = SystemMetadata;
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

// Hub Function codeunit after A: reads a login row exists (no change by A).
codeunit 52640 "G1 Fn Login Row Exists"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    [Test]
    procedure H_LoginRowExists()
    var
        Row: Record "DLW Login Row";
    begin
        if Row.IsEmpty() then
            Error('no login row');
    end;
}
