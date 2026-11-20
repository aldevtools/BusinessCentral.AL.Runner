// The reporter's repro of al-runner issue 8342, green on real BC 29.1 (as posted on the issue).
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

page 50206 "ALR Probe CurrRec Worksheet"
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
                field(Name; Rec.Name) { }
            }
        }
    }
    actions
    {
        area(Processing)
        {
            action(ReadBack)
            {
                trigger OnAction()
                var
                    StoredLine: Record "ALR Probe Line";
                    Store: Codeunit "ALR Probe Store";
                begin
                    StoredLine.Get(Rec."Line No.");
                    Store.SetText(StoredLine.Name);
                end;
            }
        }
    }
    trigger OnAfterGetCurrRecord()
    begin
    end;
}

codeunit 50202 "ALR Probe Store"
{
    SingleInstance = true;
    var
        StoredText: Text;
    procedure SetText(NewText: Text) begin StoredText := NewText; end;
    procedure GetText(): Text begin exit(StoredText); end;
}

codeunit 50221 "ALR P SaveBeforeActionCurrRec"
{
    Subtype = Test;

    [Test]
    procedure PageWithOnAfterGetCurrRecordSavesRowBeforeAction()
    var
        ProbeLine: Record "ALR Probe Line";
        Store: Codeunit "ALR Probe Store";
        Worksheet: TestPage "ALR Probe CurrRec Worksheet";
    begin
        ProbeLine.DeleteAll();
        ProbeLine."Line No." := 10000;
        ProbeLine.Name := 'Old';
        ProbeLine.Insert();

        Worksheet.OpenEdit();
        Worksheet.First();
        Worksheet.Name.SetValue('New');
        Worksheet.ReadBack.Invoke();

        Assert.AreEqual('New', Store.GetText(), 'the action read the row from the table');
    end;

    var
        Assert: Codeunit "Library Assert";
}
