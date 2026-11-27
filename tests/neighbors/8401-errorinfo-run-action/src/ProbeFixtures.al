// Fixtures of corpus PR 1601: EnTErrorInfoRaise.al:9-18 and EnTErrorInfoProps.al:9-18.
codeunit 73027 "LT EnT ErrorInfo Actions"
{
    procedure DoAction(ErrInfo: ErrorInfo)
    begin
    end;

    procedure Other(ErrInfo: ErrorInfo)
    begin
    end;
}

table 73026 "LT EnT Info Rec"
{
    DataClassification = SystemMetadata;
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Name; Text[50]) { }
    }
    keys { key(PK; "No.") { Clustered = true; } }
}
