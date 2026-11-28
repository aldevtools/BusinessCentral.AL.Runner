table 52600 "DLW Login Row"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "Entry No."; Integer) { AutoIncrement = true; }
        field(3; "Data Seen"; Integer) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 52601 "DLW Data"
{
    DataClassification = SystemMetadata;
    fields { field(1; Code; Code[10]) { } }
    keys { key(PK; Code) { Clustered = true; } }
}

codeunit 52600 "DLW Login Subs"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"System Initialization", 'OnAfterLogin', '', false, false)]
    local procedure AfterLogin()
    var
        Row: Record "DLW Login Row";
        Data: Record "DLW Data";
    begin
        if CurrentClientType() <> ClientType::Web then
            exit;
        Row.Init();
        Row."Data Seen" := Data.Count();
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
