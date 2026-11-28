// Review round 1 of the corpus probe (corpus PR 1642): a Codeunit or Function codeunit on the hub after a Disabled codeunit
// changed the login rows is not measured. The login writes one row per OnAfterLogin into "DLW Login Row" and reads table
// User Login, which the login code of System Application writes. The test codeunits run in id order: Disabled A, Codeunit
// B (rows as the login left them), Disabled C (modifies the login row), Codeunit D and Function E after it, Disabled F.
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
    begin
        if CurrentClientType() <> ClientType::Web then
            exit;
        Row.Init();
        Row."User Logins" := UserLogin.Count();
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
