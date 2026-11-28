// Runner issue 9393, corpus PR 1657 (LgnRunnerSessions.al): the codeunits runner 130450 runs after a Disabled hub
// codeunit share one session. The login writes one row per OnAfterLogin into "SRS Login Row", with the session's
// SessionId(), and reads table User Login, which System Application's login code writes, so the runner raises the
// login through System Application's code (the probe's own subscriber is in the app under test: refused by main's rule).
// Codeunits in id order: hub H (Disabled), runner N (NonRestrictive), R (Restrictive), M (Disabled + MessageHandler),
// U (unfiltered session tables), W (OnRun changes the login rows), Y (after W), S (StopSession of the hub id).
table 53900 "SRS Login Row"
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

table 53901 "SRS Hub Mark"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Code; Code[10]) { }
        field(2; "Hub Session Id"; Integer) { }
        field(3; "Max Login Entry"; Integer) { }
    }

    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

table 53902 "SRS Other"
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

codeunit 53900 "SRS Login Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin()
    var
        Row: Record "SRS Login Row";
        UserLogin: Record "User Login";
    begin
        if CurrentClientType() <> ClientType::Web then
            exit;
        Row.Init();
        Row."User Logins" := UserLogin.Count();
        Row."Session Id" := SessionId();
        Row.Insert();
        Memo.NoteLogin(SessionId());
        // Session state the login sets: the later runner codeunits of the session see it (one login, :424).
        WorkDate(DMY2Date(15, 6, 2031));
        GlobalLanguage(1031);
    end;

    var
        Memo: Codeunit "SRS Memo";
}

codeunit 53901 "SRS Memo"
{
    SingleInstance = true;

    var
        LoginId: Integer;
        LoginCalls: Integer;
        RunIds: List of [Integer];

    procedure NoteLogin(NewId: Integer)
    begin
        LoginId := NewId;
        LoginCalls += 1;
    end;

    procedure NoteRun(NewId: Integer)
    begin
        RunIds.Add(NewId);
    end;

    procedure SeenLoginId(): Integer
    begin
        exit(LoginId);
    end;

    procedure SeenLoginCalls(): Integer
    begin
        exit(LoginCalls);
    end;

    // How many runner codeunits noted an id in OnRun that is not Id.
    procedure RunsWithOtherId(Id: Integer): Integer
    var
        RunId: Integer;
        Other: Integer;
    begin
        foreach RunId in RunIds do
            if RunId <> Id then
                Other += 1;
        exit(Other);
    end;

    procedure LastRunId(): Integer
    begin
        if RunIds.Count() = 0 then
            exit(-1);
        exit(RunIds.Get(RunIds.Count()));
    end;
}

codeunit 53902 "SRS Assert"
{
    procedure Equal(Expected: Integer; Actual: Integer; What: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected %2, got %3', What, Expected, Actual);
    end;

    procedure IsTrue(Condition: Boolean; What: Text)
    begin
        if not Condition then
            Error('%1: expected true', What);
    end;

    procedure HubId(): Integer
    var
        Mark: Record "SRS Hub Mark";
    begin
        Mark.Get('HUB');
        exit(Mark."Hub Session Id");
    end;
}
