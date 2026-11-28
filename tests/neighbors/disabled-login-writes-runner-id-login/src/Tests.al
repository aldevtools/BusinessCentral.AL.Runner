table 52690 "R1 Own"
{
    DataClassification = SystemMetadata;
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

table 52691 "R1 Other"
{
    DataClassification = SystemMetadata;
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

table 52692 "R1 Third"
{
    DataClassification = SystemMetadata;
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

codeunit 52610 "R1 A Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure A_LoginRowThenWrite()
    var
        Row: Record "DLW Login Row";
        Data: Record "DLW Data";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Row.Count(), 'login rows');
        Data.Code := 'A';
        Data.Insert();
        Commit();
    end;
}

codeunit 52620 "R1 F2 OnRun Writer 130450"
{
    Subtype = Test;

    trigger OnRun()
    var
        Own: Record "R1 Own";
    begin
        Own.Code := 'X';
        Own.Insert();
    end;

    [Test]
    procedure F2_OwnTableOnly()
    var
        Own: Record "R1 Own";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Own.Count(), 'own rows');
    end;
}

codeunit 52621 "R1 F3 Unrelated 130450"
{
    Subtype = Test;

    // Runner 130450 carries what OnRun writes, so F3 is in the carry chain after F2 without --strict-order too.
    trigger OnRun()
    var
        Third: Record "R1 Third";
    begin
        Third.Code := 'Y';
        Third.Insert();
    end;


    [Test]
    procedure F3_OtherTableOnly()
    var
        Other: Record "R1 Other";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(0, Other.Count(), 'other rows');
    end;

    // The login rows hold F2's SessionId() and F3's own: how they relate is not measured (runner issue 9393).
    [Test]
    procedure F3_LoginRows()
    var
        Row: Record "DLW Login Row";
        Assert: Codeunit "DLW Assert";
    begin
        Row.SetRange("Session Id", SessionId());
        Assert.Equal(1, Row.Count(), 'own login rows');
    end;

    // F2's OnRun left a row in R1 Own.
    [Test]
    procedure F3_EarlierRunnerTable()
    var
        Own: Record "R1 Own";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Own.Count(), 'own rows of F2');
    end;
}

codeunit 52622 "R1 H2 Function Unrelated"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    [Test]
    procedure H2_OtherTableOnly()
    var
        Other: Record "R1 Other";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(0, Other.Count(), 'other rows');
    end;
}
