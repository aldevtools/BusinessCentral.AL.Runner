// The login writes one row per OnAfterLogin into "DLW Login Row" and reads table User Login, which the login code of
// System Application writes, so the runner raises the login through System Application's code. The test codeunits run
// in id order: Disabled A, Disabled B, Codeunit C, Disabled D (deletes the login rows), Codeunit E, then a codeunit
// runner 130450 runs (F), a Function codeunit on the hub (H) and a codeunit with handlers (G).
table 52600 "DLW Login Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(2; "User Logins"; Integer) { }
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
        Memo: Codeunit "DLW Memo";
    begin
        if CurrentClientType() <> ClientType::Web then
            exit;
        Row.Init();
        Row."User Logins" := UserLogin.Count();
        Row.Insert();
        Memo.Fill();
    end;
}

codeunit 52603 "DLW Memo"
{
    SingleInstance = true;

    var
        DataAtLogin: Integer;

    procedure Fill()
    var
        Data: Record "DLW Data";
    begin
        DataAtLogin := Data.Count();
    end;

    procedure Get(): Integer
    begin
        exit(DataAtLogin);
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
