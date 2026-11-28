// The login writes one row per OnAfterLogin into "DLW Login Row". A test codeunit that reads that table can see what the
// login leaves, so the runner raises the login for it through System Application's code; P reads nothing the login or a
// Company Triggers subscriber writes.
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
    begin
        if CurrentClientType() <> ClientType::Web then
            exit;
        Row.Init();
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

// A Company Triggers subscriber of the app under test that writes when the test session opens the company (main's
// behavior: a Disabled codeunit is refused).
codeunit 52601 "DLW Company Open"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"Company Triggers", 'OnCompanyOpenCompleted', '', false, false)]
    local procedure OnCompanyOpenCompleted()
    var
        Data: Record "DLW Data";
    begin
        if Data.Get('OPEN') then
            exit;
        Data.Code := 'OPEN';
        Data.Insert();
    end;
}
