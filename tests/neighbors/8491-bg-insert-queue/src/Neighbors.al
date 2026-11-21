// Neighbors of al-runner issue 8491. A worker's Get while the error of a duplicate-key Insert is deferred in the test session runs as
// BC does (BgTaskInsertQueue.al:800): it does not find the queued row, and the duplicate comes out at the end of the test. Not
// measured, refused: a worker's TableRelation check of a table whose queue holds rows (BC does not find the queued row there,
// measured on a local BC 29.1 container). (Rows queued under another company value need two companies: al-testrun
// `companies_opt_in.rs`.)

codeunit 97590 "IQ Neighbor Tests"
{
    Subtype = Test;

    [Test]
    procedure DeferredDuplicate_TaskRead()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Row."No." := 'IQNB1';
        Row.Insert();
        Row.Get('IQNB1');
        Host.OpenView();
        Row."No." := 'IQNB2';
        Row.Insert();
        Row."No." := 'IQNB1';
        Row.Insert();
        Parameters.Add('No', 'IQNB2');
        Parameters.Add('Mode', 'get');
        Results := Host.RunPageBackgroundTask(Codeunit::"IQ Reader", Parameters, false);
        Results.Get('Seen', Seen);
        // BC: no error at the duplicate Insert, and the worker does not find the queued row (a local BC 29.1 container).
        if Seen <> '-' then
            Error('The worker saw %1.', Seen);
    end;

    [Test]
    procedure Queued_TaskTableRelation()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Host.OpenView();
        Row."No." := 'IQNB3';
        Row.Insert();
        Parameters.Add('No', 'IQNB3');
        // BC: the worker's TableRelation check does not find the queued row (a local BC 29.1 container).
        asserterror Results := Host.RunPageBackgroundTask(Codeunit::"IQ Relation Reader", Parameters, false);
        if StrPos(GetLastErrorText(), 'cannot be found in the related table') = 0 then
            Error('Another error: %1', GetLastErrorText());
    end;
}

codeunit 97591 "IQ Relation Reader"
{
    trigger OnRun()
    var
        Link: Record "IQ Link";
        Results: Dictionary of [Text, Text];
        No: Text;
    begin
        Page.GetBackgroundParameters().Get('No', No);
        Link.Validate("Row No.", No);
        Results.Add('Seen', Link."Row No.");
        Page.SetBackgroundTaskResult(Results);
    end;
}
