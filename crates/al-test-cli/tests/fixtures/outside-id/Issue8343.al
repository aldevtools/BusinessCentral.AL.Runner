// The reporter's repro of al-runner issue 8343, green on real BC 29.1 (as posted on the issue).
table 50200 "ALR Probe Line"
{
    DataClassification = CustomerContent;
    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; Name; Text[50]) { }
        field(3; Processed; Boolean) { }
    }
    keys { key(PK; "Line No.") { Clustered = true; } key(ByProcessed; Processed) { } }
}

page 50209 "ALR Probe Refresh Worksheet"
{
    PageType = Worksheet;
    SourceTable = "ALR Probe Line";
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Line No."; Rec."Line No.") { }
                field(Processed; Rec.Processed) { }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(MarkAllProcessed)
            {
                trigger OnAction()
                var
                    AllLines: Record "ALR Probe Line";
                begin
                    AllLines.ModifyAll(Processed, true);
                    CurrPage.Update(false);
                end;
            }
        }
    }
    trigger OnOpenPage()
    begin
        Rec.SetRange(Processed, false);
    end;

    trigger OnAfterGetRecord()
    begin
    end;
}

codeunit 50224 "ALR P RefreshAfterAction"
{
    Subtype = Test;

    [Test]
    procedure FirstAfterActionOnPageWithOnAfterGetRecord()
    var
        ProbeLine: Record "ALR Probe Line";
        Worksheet: TestPage "ALR Probe Refresh Worksheet";
    begin
        ProbeLine.DeleteAll();
        ProbeLine."Line No." := 10000;
        ProbeLine.Insert();

        Worksheet.OpenEdit();
        Assert.IsTrue(Worksheet.First(), 'the line is listed before the action');
        Worksheet.MarkAllProcessed.Invoke();
        Assert.IsFalse(Worksheet.First(), 'the processed line leaves the filtered view');
    end;

    var
        Assert: Codeunit "Library Assert";
}
