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

codeunit 52612 "DLW C Codeunit After Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Codeunit;

    [Test]
    procedure C_NoSecondLoginRow()
    var
        Row: Record "DLW Login Row";
        Data: Record "DLW Data";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Data.Count(), 'rows A left');
        Assert.Equal(1, Row.Count(), 'login rows');
    end;
}

codeunit 52613 "DLW D Disabled Deletes"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Disabled;

    [Test]
    procedure D1_DeleteLoginRows()
    var
        Row: Record "DLW Login Row";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Row.Count(), 'login rows');
        Row.DeleteAll();
        Commit();
    end;

    [Test]
    procedure D2_StayGone()
    var
        Row: Record "DLW Login Row";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(0, Row.Count(), 'login rows');
    end;
}

codeunit 52614 "DLW E Codeunit After Delete"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Codeunit;

    [Test]
    procedure E_LoginRowsStayGone()
    var
        Row: Record "DLW Login Row";
        Data: Record "DLW Data";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Data.Count(), 'rows A left');
        Assert.Equal(0, Row.Count(), 'login rows');
    end;
}

codeunit 52615 "DLW F Runner 130450"
{
    Subtype = Test;

    [Test]
    procedure F_RunnerCodeunitAfterDisabled()
    var
        Row: Record "DLW Login Row";
        Data: Record "DLW Data";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Data.Count(), 'rows A left');
        Assert.Equal(0, Row.Count(), 'login rows');
    end;
}

codeunit 52616 "DLW G Handlers"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    [HandlerFunctions('Yes')]
    procedure G_HandlersAfterDisabled()
    var
        Data: Record "DLW Data";
        Assert: Codeunit "DLW Assert";
    begin
        if Confirm('Go?') then
            Assert.Equal(1, Data.Count(), 'rows A left');
    end;

    [ConfirmHandler]
    procedure Yes(Question: Text[1024]; var Reply: Boolean)
    begin
        Reply := true;
    end;
}

codeunit 52617 "DLW H Function After Disabled"
{
    Subtype = Test;
    TestPermissions = Disabled;
    RequiredTestIsolation = Function;

    [Test]
    procedure H_FunctionAfterDisabled()
    var
        Row: Record "DLW Login Row";
        Data: Record "DLW Data";
        Assert: Codeunit "DLW Assert";
    begin
        Assert.Equal(1, Data.Count(), 'rows A left');
        Assert.Equal(0, Row.Count(), 'login rows');
    end;
}
