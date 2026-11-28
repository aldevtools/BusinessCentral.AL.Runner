// Runner issue 9393: the OnRun shapes next to the one corpus PR 1657 (1d9af7cd) measured, in the session runner 130450
// runs its codeunits in after the held login. Measured: OnRun inserts rows into the login's table, with or without
// Commit(), and writes nothing else; they are gone in the later runner codeunits (LgnRunnerSessions.al:839, :878).
// Not measured, main's answer: OnRun modifies a login row, inserts into another table (alone or next to the login
// table), into two tables the login writes, or writes isolated storage next to its insert. Codeunits in id order: hub
// H, then C, M, O, B, S, T, then reader R.
table 54000 "FRS Login Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        // 0 for a login row; the letter of the runner codeunit that inserted the row otherwise.
        field(2; Writer; Code[10]) { }
        field(3; "Writer Session"; Integer) { }
        // The login sets it to the User Login rows it saw (it reads that table, which System Application's login
        // code writes, so the runner raises the login through that code, as tests/neighbors/serve-runner-session-9393).
        field(4; Note; Integer) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

table 54001 "FRS Other"
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

// A second table the login writes (one row per login), for the OnRun that inserts into both login tables.
table 54002 "FRS Login Second"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        // '' for a login row; the letter of the runner codeunit that inserted the row otherwise.
        field(2; Writer; Code[10]) { }
    }

    keys
    {
        key(PK; "Entry No.") { Clustered = true; }
    }
}

codeunit 54000 "FRS Login Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin()
    var
        Row: Record "FRS Login Row";
        Second: Record "FRS Login Second";
        UserLogin: Record "User Login";
    begin
        if CurrentClientType() <> ClientType::Web then
            exit;
        Row.Init();
        Row.Writer := '';
        Row.Note := UserLogin.Count();
        Row.Insert();
        Second.Init();
        Second.Writer := '';
        Second.Insert();
    end;
}

codeunit 54002 "FRS Assert"
{
    procedure Equal(Expected: Integer; Actual: Integer; What: Text)
    begin
        if Expected <> Actual then
            Error('%1: expected %2, got %3', What, Expected, Actual);
    end;
}

codeunit 54003 "FRS Rows"
{
    // Rows the runner codeunit Writer of this session inserted into the login table.
    procedure Written(Writer: Code[10]): Integer
    var
        Row: Record "FRS Login Row";
    begin
        Row.SetRange(Writer, Writer);
        Row.SetRange("Writer Session", SessionId());
        exit(Row.Count());
    end;

    procedure Insert(Writer: Code[10])
    var
        Row: Record "FRS Login Row";
    begin
        Row.Init();
        Row.Writer := Writer;
        Row."Writer Session" := SessionId();
        Row.Insert();
    end;

    // Login rows whose Note a runner codeunit's OnRun set to 999.
    procedure Modified(): Integer
    var
        Row: Record "FRS Login Row";
    begin
        Row.SetRange(Writer, '');
        Row.SetRange(Note, 999);
        exit(Row.Count());
    end;

    // Rows the runner codeunit Writer inserted into the second login table.
    procedure Second(Writer: Code[10]): Integer
    var
        Second: Record "FRS Login Second";
    begin
        Second.SetRange(Writer, Writer);
        exit(Second.Count());
    end;

    procedure Other(Code: Code[10]): Integer
    var
        Other: Record "FRS Other";
    begin
        Other.SetRange(Code, Code);
        exit(Other.Count());
    end;
}
