// The reporter's repro of al-runner issue 8344, green on real BC 29.1 (as posted on the issue).
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

page 50208 "ALR Probe Split Worksheet"
{
    PageType = Worksheet;
    SourceTable = "ALR Probe Line";
    AutoSplitKey = true;
    DelayedInsert = true;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            repeater(Lines)
            {
                field("Line No."; Rec."Line No.") { }
                field(Name; Rec.Name) { }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(LoadLines)
            {
                trigger OnAction()
                var
                    NewLine: Record "ALR Probe Line";
                begin
                    NewLine."Line No." := 10000;
                    NewLine.Name := 'Loaded 1';
                    NewLine.Insert();
                    NewLine."Line No." := 20000;
                    NewLine.Name := 'Loaded 2';
                    NewLine.Insert();
                    CurrPage.Update(false);
                    if not Rec.IsEmpty() then
                        Rec.FindSet();
                end;
            }
        }
    }
}

codeunit 50223 "ALR P AutoSplitKeyAfterFindSet"
{
    Subtype = Test;

    [Test]
    procedure NewLineAfterActionPositionedOnFirstRow()
    var
        ProbeLine: Record "ALR Probe Line";
        Worksheet: TestPage "ALR Probe Split Worksheet";
    begin
        ProbeLine.DeleteAll();
        Worksheet.OpenEdit();
        Worksheet.LoadLines.Invoke();
        Worksheet.New();
        Worksheet.Name.SetValue('Manual');
        Worksheet.Close();

        Assert.AreEqual(3, ProbeLine.Count(), 'two loaded lines and one manual line');
        ProbeLine.SetRange(Name, 'Manual');
        ProbeLine.FindFirst();
        Assert.AreEqual(15000, ProbeLine."Line No.", 'AutoSplitKey line no of the manual line');
    end;

    var
        Assert: Codeunit "Library Assert";
}
