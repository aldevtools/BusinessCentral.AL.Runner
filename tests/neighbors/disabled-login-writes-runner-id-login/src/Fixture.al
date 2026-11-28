// The login writes one row per OnAfterLogin into "DLW Login Row", with the session's SessionId(), and reads table User
// Login, which the login code of System Application writes, so the runner raises the login through System Application's
// code. Codeunits: Disabled A on the hub, then F2 and F3 that runner 130450 runs (F3 is the second one after the held
// login, and its login reads SessionId()), then a Function codeunit on the hub (H2).
table 52600 "DLW Login Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; "User Logins"; Integer) { }
        field(3; "Session Id"; Integer) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

table 52601 "DLW Data"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Code; Code[10]) { }
    }

    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

codeunit 52600 "DLW Login Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin()
    var
        Row: Record "DLW Login Row";
        UserLogin: Record "User Login";
    begin
        if CurrentClientType() <> ClientType::Web then
            exit;
        Row.Init();
        Row."User Logins" := UserLogin.Count();
        Row."Session Id" := SessionId();
        Row.Insert();
    end;
}

codeunit 52602 "DLW Assert"
{
    procedure Equal(Expected: Integer; Actual: Integer; What: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected %2, got %3', What, Expected, Actual);
    end;
}
