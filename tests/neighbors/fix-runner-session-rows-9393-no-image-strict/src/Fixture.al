// Runner issue 9393: corpus PR 1657 (1d9af7cd), LgnRunnerSessions.al cell 2 (:562-:927), the no-image shape. The login of
// the session runner 130450 runs its codeunits in after the held login puts a SecretText into a SingleInstance codeunit
// and notes the login rows it saw. Every runner codeunit sees one login call, 1 login row at that login (a login on the
// current data counts 2), and the SecretText not empty (:789-:791); rows a runner codeunit's OnRun inserted are gone in
// the later ones (:839, :877, :917). Stand-ins written from the cells, ids 54100-54199. Codeunits in id order: hub H,
// writers I (insert) and C (insert and Commit), readers N and D.
table 54100 "FRN Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; "Session Id"; Integer) { }
        field(3; Kind; Code[10]) { }
        field(4; "User Logins"; Integer) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

table 54101 "FRN Hub"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Code; Code[10]) { }
        field(2; Value; Integer) { }
    }

    keys
    {
        key(PK; Code) { Clustered = true; }
    }
}

codeunit 54100 "FRN Login Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin()
    var
        Row: Record "FRN Row";
        Hub: Record "FRN Hub";
        UserLogin: Record "User Login";
        Memo: Codeunit "FRN Memo";
    begin
        if CurrentClientType() <> ClientType::Web then
            exit;
        // It reads User Login, which System Application's login code writes, so the runner raises the login through
        // that code (as tests/neighbors/serve-runner-session-9393).
        Row.SetRange(Kind, 'LOGIN');
        Memo.NoteLogin(Row.Count());
        if Hub.Get('HUB') then
            if Hub.Value <> SessionId() then
                Memo.SetSecret('FRN secret');
        Row.Init();
        Row."User Logins" := UserLogin.Count();
        Row."Session Id" := SessionId();
        Row.Kind := 'LOGIN';
        Row.Insert();
    end;
}

codeunit 54101 "FRN Memo"
{
    SingleInstance = true;

    var
        Calls: Integer;
        AtLogin: Integer;
        Secret: SecretText;

    procedure NoteLogin(Rows: Integer)
    begin
        Calls += 1;
        if Calls = 1 then
            AtLogin := Rows;
    end;

    procedure SetSecret(Value: Text)
    begin
        Secret := Value;
    end;

    procedure SecretIsEmpty(): Boolean
    begin
        exit(Secret.IsEmpty());
    end;

    procedure LoginCalls(): Integer
    begin
        exit(Calls);
    end;

    procedure RowsAtLogin(): Integer
    begin
        exit(AtLogin);
    end;
}

codeunit 54102 "FRN Check"
{
    procedure Equal(Expected: Integer; Actual: Integer; What: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected %2, got %3', What, Expected, Actual);
    end;

    procedure Rows(Kind: Code[10]; SessionNo: Integer): Integer
    var
        Row: Record "FRN Row";
    begin
        Row.SetRange(Kind, Kind);
        if SessionNo <> 0 then
            Row.SetRange("Session Id", SessionNo);
        exit(Row.Count());
    end;

    // What every runner codeunit sees (:787-:794 and the same at :831, :869, :909).
    procedure Cell(Own: Code[10]; OwnRows: Integer)
    var
        Memo: Codeunit "FRN Memo";
    begin
        Equal(1, Memo.LoginCalls(), 'login calls');
        Equal(1, Memo.RowsAtLogin(), 'login rows at the first login');
        Equal(0, Rows('LOGIN', 0) - 2, 'login rows beyond 2');
        Equal(1, Rows('LOGIN', SessionId()), 'login rows of this session');
        if Memo.SecretIsEmpty() then
            Error('the SecretText the login set is empty');
        Equal(OwnRows, Rows(Own, SessionId()), 'rows of this codeunit');
    end;
}
