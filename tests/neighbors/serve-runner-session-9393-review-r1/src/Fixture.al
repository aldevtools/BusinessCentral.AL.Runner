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

// Review round 1 (runner PR 9426): the worker of a page background task, a session the test session starts, reports its
// SessionId(); a SingleInstance codeunit keeps the ids the runner session's codeunits saw.
codeunit 53903 "SRS Bg Worker"
{
    trigger OnRun()
    var
        Results: Dictionary of [Text, Text];
    begin
        Results.Add('Session', Format(SessionId()));
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 53904 "SRS Bg Memo"
{
    SingleInstance = true;

    var
        Ids: List of [Text];

    procedure Note(Id: Text)
    begin
        Ids.Add(Id);
    end;

    procedure Seen(Id: Text): Boolean
    begin
        exit(Ids.Contains(Id));
    end;

    procedure Count(): Integer
    begin
        exit(Ids.Count());
    end;
}

page 53905 "SRS Bg Card"
{
    PageType = Card;
    SourceTable = "SRS Other";
    ApplicationArea = All;
    UsageCategory = None;

    layout
    {
        area(Content)
        {
            field(CodeCtl; Rec.Code) { ApplicationArea = All; }
            field(TaskSessionCtl; TaskSession) { ApplicationArea = All; Caption = 'Task Session'; }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Args: Dictionary of [Text, Text];
    begin
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"SRS Bg Worker", Args, 5000);
    end;

    trigger OnPageBackgroundTaskCompleted(CompletedTaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        Results.Get('Session', TaskSession);
    end;

    var
        TaskId: Integer;
        TaskSession: Text;
}

codeunit 53906 "SRS Bg Start"
{
    // Opens the card on a row; its background task's worker reports its id, which must be neither the test session's nor
    // the hub session's, and one no earlier worker of this session had. Returns the id.
    procedure RunTask(): Text
    var
        Other: Record "SRS Other";
        Memo: Codeunit "SRS Bg Memo";
        Assert: Codeunit "SRS Assert";
        Card: TestPage "SRS Bg Card";
        Id: Text;
    begin
        Other.Code := 'BG';
        if Other.Insert() then;
        Card.OpenView();
        Id := Card.TaskSessionCtl.Value();
        Card.Close();
        if Id = '' then
            Error('the worker reported no id');
        if Id = Format(SessionId()) then
            Error('the worker had the test session id %1', Id);
        // Reads what the hub codeunit committed, so the codeunit starts from what it left (the runner session).
        if Id = Format(Assert.HubId()) then
            Error('the worker had the hub session id %1', Id);
        if Memo.Seen(Id) then
            Error('an earlier worker of this session had id %1', Id);
        Memo.Note(Id);
        exit(Id);
    end;
}
