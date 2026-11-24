// The reporter's repro of al-runner issue 8638 (app and test in one file): a page background task's ReadCommitted Get of a
// row the test codeunit modified waits on the test session's lock and fails; passes on BC 29.1 W1 (bc-results.xml of the issue).

table 50110 "Repro Lock Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 50111 "Repro Lock Rows"
{
    PageType = List;
    SourceTable = "Repro Lock Row";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
                field(Description; Rec.Description) { }
            }
        }
    }
}

codeunit 50112 "Repro Lock Reader"
{
    trigger OnRun()
    var
        Row: Record "Repro Lock Row";
        Results: Dictionary of [Text, Text];
        No: Text;
    begin
        Page.GetBackgroundParameters().Get('No', No);
        if TryGet(No, Row) then
            Results.Add('Seen', Row.Description)
        else
            Results.Add('Seen', 'error: ' + GetLastErrorText());
        Page.SetBackgroundTaskResult(Results);
    end;

    [TryFunction]
    local procedure TryGet(No: Text; var Row: Record "Repro Lock Row")
    begin
        Row.ReadIsolation := IsolationLevel::ReadCommitted;
        Row.Get(No);
    end;
}

codeunit 50130 "Repro Lock Read Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ReadCommittedGet_RowTheTestChanged_WaitsOnTheLock()
    var
        Rows: TestPage "Repro Lock Rows";
        Row: Record "Repro Lock Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        No: Code[20];
        Seen: Text;
    begin
        No := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20);
        Row."No." := No;
        Row.Description := 'first';
        Row.Insert();
        Commit();
        Row.Get(No);
        Row.Description := 'second';
        Row.Modify();
        Rows.OpenView();
        Parameters.Add('No', No);
        Results := Rows.RunPageBackgroundTask(Codeunit::"Repro Lock Reader", Parameters, false);
        Results.Get('Seen', Seen);
        if StrPos(Seen, 'is being updated in a transaction done by another session') = 0 then
            Error('Expected the lock error, the worker read <%1>', Seen);
    end;
}
