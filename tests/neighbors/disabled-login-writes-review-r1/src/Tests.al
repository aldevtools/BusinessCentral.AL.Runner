codeunit 52610 "DLW R1 A Disabled"
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

codeunit 52611 "DLW R1 B Codeunit Rows As Left"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Codeunit;

    [Test]
    procedure B_ReadsLoginRowAsLeft()
    var
        Row: Record "DLW Login Row";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Row.Count(), 'login rows');
    end;
}

codeunit 52612 "DLW R1 C Disabled Modifies"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure C_ModifyLoginRow()
    var
        Row: Record "DLW Login Row";
    begin
        Row.FindFirst();
        Row."User Logins" := 99;
        Row.Modify();
        Commit();
    end;
}

codeunit 52613 "DLW R1 D Codeunit After Modify"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Codeunit;

    [Test]
    procedure D1_ReadsOnlyData()
    var
        Data: Record "DLW Data";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Data.Count(), 'rows A left');
    end;

    [Test]
    procedure D2_ReadsLoginRow()
    var
        Row: Record "DLW Login Row";
        Assert: Codeunit "DLW Assert";
    begin
        Row.FindFirst();
        Assert.Equal(99, Row."User Logins", 'modified login row');
    end;

    [Test]
    procedure D3_ReadsLoginRowThroughRecordRef()
    var
        Ref: RecordRef;
        Assert: Codeunit "DLW Assert";
    begin
        Ref.Open(52600);
        Assert.Equal(1, Ref.Count(), 'login rows');
    end;
}

codeunit 52614 "DLW R1 E Function After Modify"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    [Test]
    procedure E1_ReadsOnlyData()
    var
        Data: Record "DLW Data";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Data.Count(), 'rows A left');
    end;

    [Test]
    procedure E2_ReadsLoginRow()
    var
        Row: Record "DLW Login Row";
        Assert: Codeunit "DLW Assert";
    begin
        Row.FindFirst();
        Assert.Equal(99, Row."User Logins", 'modified login row');
    end;
}

codeunit 52615 "DLW R1 F Disabled After Modify"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure F_ReadsModifiedLoginRow()
    var
        Row: Record "DLW Login Row";
        Assert: Codeunit "DLW Assert";
    begin
        Row.FindFirst();
        Assert.Equal(99, Row."User Logins", 'modified login row');
    end;
}
