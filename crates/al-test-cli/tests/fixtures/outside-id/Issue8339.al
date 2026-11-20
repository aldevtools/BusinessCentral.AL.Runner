// The reporter's repro of al-runner issue 8339, green on real BC 29.1 (as posted on the issue).
page 50202 "ALR Probe Company Pick"
{
    PageType = Card;
    UsageCategory = None;
    layout
    {
        area(Content)
        {
            field(CompanyNameField; CompName) { Caption = 'Company Name'; TableRelation = Company; }
        }
    }
    var
        CompName: Text[30];
}

report 50201 "ALR Probe Company Report"
{
    ProcessingOnly = true;
    UsageCategory = None;
    requestpage
    {
        layout
        {
            area(Content)
            {
                field(CompanyNameField; CompName) { Caption = 'Company Name'; TableRelation = Company; }
            }
        }
    }
    trigger OnPreReport()
    var
        Store: Codeunit "ALR Probe Store";
    begin
        Store.SetText(CompName);
    end;
    var
        CompName: Text[30];
}

codeunit 50202 "ALR Probe Store"
{
    SingleInstance = true;
    var
        StoredText: Text;
    procedure SetText(NewText: Text) begin StoredText := NewText; end;
    procedure GetText(): Text begin exit(StoredText); end;
}

codeunit 50218 "ALR P CompanyRelationSetValue"
{
    Subtype = Test;

    [Test]
    procedure PageVariableWithCompanyRelationTakesStoredName()
    var
        CompanyPick: TestPage "ALR Probe Company Pick";
    begin
        CompanyPick.OpenEdit();
        CompanyPick.CompanyNameField.SetValue(UpperCase(CompanyName()));
        Assert.AreEqual(CompanyName(), CompanyPick.CompanyNameField.Value, 'value after SetValue');
        CompanyPick.Close();
    end;

    [Test]
    [HandlerFunctions('CompanyRequestPageHandler')]
    procedure RequestPageVariableWithCompanyRelationTakesStoredName()
    var
        Store: Codeunit "ALR Probe Store";
    begin
        Commit();
        Report.RunModal(Report::"ALR Probe Company Report", true, false);
        Assert.AreEqual(CompanyName(), Store.GetText(), 'value the report received');
    end;

    [RequestPageHandler]
    procedure CompanyRequestPageHandler(var RequestPage: TestRequestPage "ALR Probe Company Report")
    begin
        RequestPage.CompanyNameField.SetValue(UpperCase(CompanyName()));
        RequestPage.OK().Invoke();
    end;

    var
        Assert: Codeunit "Library Assert";
}
