// FlowFields that read more tables than their CalcFormula's own (code review of al-runner PR 8661): "Busy Lines" counts the
// lines whose "Has Notes" (a FlowField over table 50202) is true, and "Amount At Note Count" sums the line whose number is
// "Note Count" (a FlowField of the row over table 50202). A page background task's worker reads the row with a filter on one
// of them next to its key, the test session inserts a note and commits, and the worker reads again: no cell measures whether
// that commit ends the query cache's entry, so the runner reads the rows as they are, as main does.

table 50200 "IX Parent"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Description; Text[50]) { }
        field(3; "Busy Lines"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("IX Line" where(Parent = field("No."), "Has Notes" = const(true)));
        }
        field(4; "Note Count"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("IX Note" where(Parent = field("No.")));
        }
        field(5; "Amount At Note Count"; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = sum("IX Line".Amount where(Parent = field("No."), "Line No." = field("Note Count")));
        }
    }

    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

table 50201 "IX Line"
{
    fields
    {
        field(1; Parent; Code[20]) { }
        field(2; "Line No."; Integer) { }
        field(3; Amount; Decimal) { }
        field(4; "Has Notes"; Boolean)
        {
            FieldClass = FlowField;
            CalcFormula = exist("IX Note" where(Parent = field(Parent), "Line No." = field("Line No.")));
        }
    }

    keys
    {
        key(PK; Parent, "Line No.") { Clustered = true; }
    }
}

table 50202 "IX Note"
{
    fields
    {
        field(1; Parent; Code[20]) { }
        field(2; "Line No."; Integer) { }
    }

    keys
    {
        key(PK; Parent, "Line No.") { Clustered = true; }
    }
}

page 50203 "IX Parents"
{
    PageType = List;
    SourceTable = "IX Parent";

    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("No."; Rec."No.") { }
            }
        }
    }
}

codeunit 50204 "IX Reader"
{
    trigger OnRun()
    var
        Parent: Record "IX Parent";
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        No: Text;
        Seen: Text;
    begin
        Parameters := Page.GetBackgroundParameters();
        Parameters.Get('No', No);
        Parent.SetRange("No.", No);
        if Parameters.Get('Mode') = 'busy' then
            Parent.SetRange("Busy Lines", 1)
        else
            Parent.SetRange("Amount At Note Count", 5);
        Seen := '-';
        if Parent.FindFirst() then
            Seen := Parent.Description;
        Results.Add('Seen', Seen);
        Page.SetBackgroundTaskResult(Results);
    end;
}

codeunit 50205 "IX Tests"
{
    Subtype = Test;

    var
        Assert: Codeunit Assert;

    local procedure Run(var Parents: TestPage "IX Parents"; Mode: Text): Text
    var
        Parameters: Dictionary of [Text, Text];
        Results: Dictionary of [Text, Text];
        Seen: Text;
    begin
        Parameters.Add('No', UpperCase(Mode));
        Parameters.Add('Mode', Mode);
        Results := Parents.RunPageBackgroundTask(Codeunit::"IX Reader", Parameters, false);
        Results.Get('Seen', Seen);
        exit(Seen);
    end;

    local procedure Reads(Mode: Text): Text
    var
        Parents: TestPage "IX Parents";
        Parent: Record "IX Parent";
        Line: Record "IX Line";
        Note: Record "IX Note";
        Seen: Text;
    begin
        Parent."No." := UpperCase(Mode);
        Parent.Description := 'a';
        Parent.Insert();
        Line.Parent := UpperCase(Mode);
        Line."Line No." := 1;
        Line.Amount := 5;
        Line.Insert();
        Commit();
        Parents.OpenView();
        Seen := Run(Parents, Mode);
        Note.Parent := UpperCase(Mode);
        Note."Line No." := 1;
        Note.Insert();
        Commit();
        Seen += ' / ' + Run(Parents, Mode);
        Parents.Close();
        exit(Seen);
    end;

    [Test]
    procedure ConditionOnAFlowFieldOfTheLines_NoteCommitted()
    begin
        Assert.AreEqual('- / a', Reads('busy'), 'Busy Lines 1');
    end;

    [Test]
    procedure FieldOfAFlowFieldOfTheRow_NoteCommitted()
    begin
        Assert.AreEqual('- / a', Reads('amount'), 'Amount At Note Count 5');
    end;
}
