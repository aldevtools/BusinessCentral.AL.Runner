// The reporter's repro of al-runner issue 8467.
table 50100 "Repro Bg Item"
{
    DataClassification = CustomerContent;

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

page 50101 "Repro Bg Items"
{
    PageType = List;
    SourceTable = "Repro Bg Item";
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
                field(Seen; Seen)
                {
                    Caption = 'Seen by the background task';
                    Editable = false;
                }
            }
        }
    }

    trigger OnAfterGetCurrRecord()
    var
        Parameters: Dictionary of [Text, Text];
        TaskId: Integer;
    begin
        Seen := '';
        Parameters.Add('No', Rec."No.");
        CurrPage.EnqueueBackgroundTask(TaskId, Codeunit::"Repro Bg Reader", Parameters);
    end;

    trigger OnPageBackgroundTaskCompleted(TaskId: Integer; Results: Dictionary of [Text, Text])
    begin
        Seen := CopyStr(Results.Get('Description'), 1, MaxStrLen(Seen));
    end;

    var
        Seen: Text[50];
}

codeunit 50102 "Repro Bg Reader"
{
    trigger OnRun()
    var
        Item: Record "Repro Bg Item";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters := Page.GetBackgroundParameters();
        Item.SetRange("No.", Parameters.Get('No'));
        if Item.FindFirst() then
            Results.Add('Description', Item.Description)
        else
            Results.Add('Description', '-');
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 50110 "Repro Bg Read Tests"
{
    Subtype = Test;

    // A row's background task runs again after the test changed the row: on BC it gets the answer of its first run.
    [Test]
    procedure SecondRead_AfterAWrite_GetsTheFirstAnswer()
    var
        Item: Record "Repro Bg Item";
        Items: TestPage "Repro Bg Items";
        Prefix: Code[10];
    begin
        Prefix := NewPrefix();
        AddItem(Prefix + 'A', 'first');
        AddItem(Prefix + 'B', 'other');

        Items.OpenView();
        Items.Filter.SetFilter("No.", Prefix + '*');
        Items.First();
        Check('first visit of A', Items.Seen.Value(), 'first');

        Item.Get(Prefix + 'A');
        Item.Description := 'second';
        Item.Modify();

        Items.Next();
        Check('visit of B', Items.Seen.Value(), 'other');
        Items.Previous();
        Check('row A on the page', Items.Description.Value(), 'second');
        Check('second visit of A', Items.Seen.Value(), 'first');
        Items.Close();
    end;

    // The control: a row whose background task first runs after the write sees the write.
    [Test]
    procedure FirstRead_AfterAWrite_SeesTheWrite()
    var
        Item: Record "Repro Bg Item";
        Items: TestPage "Repro Bg Items";
        Prefix: Code[10];
    begin
        Prefix := NewPrefix();
        AddItem(Prefix + 'A', 'first');
        AddItem(Prefix + 'B', 'other');

        Items.OpenView();
        Items.Filter.SetFilter("No.", Prefix + '*');
        Items.First();
        Check('first visit of A', Items.Seen.Value(), 'first');

        Item.Get(Prefix + 'B');
        Item.Description := 'other2';
        Item.Modify();

        Items.Next();
        Check('first visit of B', Items.Seen.Value(), 'other2');
        Items.Close();
    end;

    local procedure NewPrefix(): Code[10]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 10));
    end;

    local procedure AddItem(No: Code[20]; Description: Text[50])
    var
        Item: Record "Repro Bg Item";
    begin
        Item."No." := No;
        Item.Description := Description;
        Item.Insert();
    end;

    local procedure Check(What: Text; Actual: Text; Expected: Text)
    begin
        if Actual <> Expected then
            Error('%1: expected <%2>, got <%3>', What, Expected, Actual);
    end;
}
