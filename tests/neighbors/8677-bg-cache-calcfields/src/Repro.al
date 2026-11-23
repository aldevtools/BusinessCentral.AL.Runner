// The reporter's repro of al-runner issue 8677: the app and its test in one file.
table 50145 "Repro Calc Parent"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Note Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("Repro Calc Note" where(Parent = field("No.")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 50146 "Repro Calc Note"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Parent; Code[20]) { }
        field(2; "Note No."; Integer) { }
    }

    keys
    {
        key(PK; Parent, "Note No.") { Clustered = true; }
    }
}

page 50147 "Repro Calc Notes"
{
    PageType = List;
    SourceTable = "Repro Calc Note";
    ApplicationArea = All;
    UsageCategory = Lists;

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field(Parent; Rec.Parent) { }
                field("Note No."; Rec."Note No.") { }
            }
        }
    }
}

codeunit 50148 "Repro Calc Reader"
{
    trigger OnRun()
    var
        Parent: Record "Repro Calc Parent";
        Results: Dictionary of [Text, Text];
        No: Text;
    begin
        Page.GetBackgroundParameters().Get('No', No);
        Parent.Get(No);
        Parent.CalcFields("Note Count");
        Results.Add('Seen', Format(Parent."Note Count", 0, 9));
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 50149 "Repro Calc Worker Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure WorkerCalcFieldsAgain_AfterAnUncommittedInsert_AnswersTheFirstValue()
    var
        Notes: TestPage "Repro Calc Notes";
        Parent: Record "Repro Calc Parent";
        Note: Record "Repro Calc Note";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        No: Code[20];
        First: Text;
        Second: Text;
    begin
        No := CopyStr(DelChr(Format(CreateGuid()), '=', '{}-'), 1, 20);
        Parent."No." := No;
        Parent.Insert();
        Commit();
        Notes.OpenView();
        Parameters.Add('No', No);
        Results := Notes.RunPageBackgroundTask(Codeunit::"Repro Calc Reader", Parameters, false);
        Results.Get('Seen', First);
        Note.Parent := No;
        Note."Note No." := 1;
        Note.Insert();
        Parent.Get(No);
        Parent.CalcFields("Note Count");
        Results := Notes.RunPageBackgroundTask(Codeunit::"Repro Calc Reader", Parameters, false);
        Results.Get('Seen', Second);
        Notes.Close();
        if (First <> '0') or (Parent."Note Count" <> 1) or (Second <> '0') then
            Error('The worker counted %1 notes, then %2 after the insert the test session counts as %3', First, Second, Parent."Note Count");
    end;
}
