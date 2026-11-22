// The reporter's repro of al-runner issue 8491.
table 50100 "Repro Buf Item"
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

page 50101 "Repro Buf Items"
{
    PageType = List;
    SourceTable = "Repro Buf Item";
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

codeunit 50102 "Repro Buf Reader"
{
    trigger OnRun()
    var
        Item: Record "Repro Buf Item";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
    begin
        Parameters := Page.GetBackgroundParameters();
        if Item.Get(Parameters.Get('No')) then
            Results.Add('Seen', Item.Description)
        else
            Results.Add('Seen', '-');
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 50110 "Repro Buf Tests"
{
    Subtype = Test;

    // The test inserts a row and runs a page background task at once: on BC the task does not find the row.
    [Test]
    procedure InsertThenTask_TaskDoesNotFindTheRow()
    var
        Items: TestPage "Repro Buf Items";
        No: Code[20];
    begin
        No := NewNo();
        Items.OpenView();
        AddItem(No, 'new');
        Check('task after the insert', RunTask(Items, No), '-');
        Items.Close();
    end;

    // The control: the test reads the row after its insert, then runs the task: the task finds it.
    [Test]
    procedure InsertReadThenTask_TaskFindsTheRow()
    var
        Item: Record "Repro Buf Item";
        Items: TestPage "Repro Buf Items";
        No: Code[20];
    begin
        No := NewNo();
        Items.OpenView();
        AddItem(No, 'new');
        Item.Get(No);
        Check('task after the insert and a read', RunTask(Items, No), 'new');
        Items.Close();
    end;

    local procedure RunTask(var Items: TestPage "Repro Buf Items"; No: Code[20]): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('No', No);
        Results := Items.RunPageBackgroundTask(Codeunit::"Repro Buf Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure NewNo(): Code[20]
    begin
        exit(CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20));
    end;

    local procedure AddItem(No: Code[20]; Description: Text[50])
    var
        Item: Record "Repro Buf Item";
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
