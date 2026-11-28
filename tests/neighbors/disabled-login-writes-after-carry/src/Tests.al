codeunit 52605 "R4 Z Disabled First"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure Z_WritesData()
    var
        Data: Record "DLW Data";
    begin
        Data.Code := 'Z';
        Data.Insert();
        Commit();
    end;
}

codeunit 52610 "R4 A Disabled After Z"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure A_LoginRowsAfterZ()
    var
        Row: Record "DLW Login Row";
        Data: Record "DLW Data";
        UserLogin: Record "User Login";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Data.Count(), 'rows Z left');
        if UserLogin.IsEmpty() then
            Error('no user login row');
        Assert.Equal(1, Row.Count(), 'login rows');
        Row.FindFirst();
        Assert.Equal(0, Row."Data Seen", 'data rows the one login saw');
    end;
}
