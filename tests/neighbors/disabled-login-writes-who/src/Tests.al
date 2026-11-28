codeunit 52610 "DLW A Disabled"
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

codeunit 52611 "DLW B Disabled After Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure B_NoSecondLoginRow()
    var
        Row: Record "DLW Login Row";
        Data: Record "DLW Data";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Data.Count(), 'rows A left');
        Assert.Equal(1, Row.Count(), 'login rows');
    end;
}


codeunit 52619 "DLW P Untouched"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure P_ReadsNothingOfTheLogin()
    var
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(2, 1 + 1, 'sum');
    end;
}

codeunit 52620 "DLW Q Codeunit Untouched"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Codeunit;

    [Test]
    procedure Q_CodeunitReadsNothingOfTheLogin()
    var
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(2, 1 + 1, 'sum');
    end;
}
