// The login writes one row per OnAfterLogin into "RS Login Row", with the session's id, and reads table User Login,
// which the login code of System Application writes, so the runner raises the login through System Application's
// code. The hub Disabled codeunit A holds the run's one login, notes the hub session's id and leaves the login rows
// alone; the codeunits runner 130450 runs after it (P1, P2) count the login rows of their own session.
table 52600 "RS Login Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; "Session Id"; Integer) { }
        field(3; "User Logins"; Integer) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

table 52601 "RS Mark"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Key"; Code[20]) { }
        field(2; Value; Integer) { }
    }

    keys
    {
        key(PK; "Key") { Clustered = true; }
    }
}

table 52602 "RS Own"
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

codeunit 52600 "RS Login Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin()
    var
        Row: Record "RS Login Row";
        UserLogin: Record "User Login";
    begin
        if CurrentClientType() <> ClientType::Web then
            exit;
        Row.Init();
        Row."Session Id" := SessionId();
        Row."User Logins" := UserLogin.Count();
        Row.Insert();
    end;
}

codeunit 52601 "RS Lib"
{
    procedure OwnRows(): Integer
    var
        Row: Record "RS Login Row";
    begin
        Row.SetRange("Session Id", SessionId());
        exit(Row.Count());
    end;

    procedure Mark(KeyCode: Code[20]): Integer
    var
        Mark: Record "RS Mark";
    begin
        if Mark.Get(KeyCode) then
            exit(Mark.Value);
        exit(-1);
    end;

    procedure Note(KeyCode: Code[20])
    var
        Mark: Record "RS Mark";
    begin
        Mark."Key" := KeyCode;
        Mark.Value := SessionId();
        Mark.Insert();
    end;

    procedure Equal(Expected: Integer; Actual: Integer; What: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected %2, got %3', What, Expected, Actual);
    end;
}
