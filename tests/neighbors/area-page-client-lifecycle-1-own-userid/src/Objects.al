// A codeunit with its own UserId(): Integer, beside the global UserId: which one the bare `UserId()` binds is not
// measured (must-not-compile 3214 measures the name Caption in a page).
codeunit 50300 "NUser Own UserId"
{
    procedure UserId(): Integer
    begin
        exit(5);
    end;

    procedure Read(): Text
    var
        S: Text;
    begin
        S := UserId();
        exit(S);
    end;
}

codeunit 50310 "NUser Tests"
{
    Subtype = Test;

    [Test]
    procedure OwnUserId_IntoText()
    var
        C: Codeunit "NUser Own UserId";
    begin
        if C.Read() = '' then
            Error('empty');
    end;

    [Test]
    procedure OtherTestOfTheApp_Runs()
    begin
    end;
}
