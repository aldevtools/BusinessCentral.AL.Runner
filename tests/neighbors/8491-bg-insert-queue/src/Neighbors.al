// Neighbors of al-runner issue 8491. Not measured, refused: a worker's read of a table whose duplicate-key Insert is deferred in the test session, and a worker's
// TableRelation check of a table whose queue holds rows. (Rows queued under another company value need two companies: al-testrun
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
        Host.RunPageBackgroundTask(Codeunit::"IQ Reader", Parameters, false);
    end;

    [Test]
    procedure Queued_TaskTableRelation()
    var
        Host: TestPage "IQ Host";
        Row: Record "IQ Row";
        Parameters: Dictionary of [Text, Text];
    begin
        Host.OpenView();
        Row."No." := 'IQNB3';
        Row.Insert();
        Parameters.Add('No', 'IQNB3');
        Host.RunPageBackgroundTask(Codeunit::"IQ Relation Reader", Parameters, false);
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
