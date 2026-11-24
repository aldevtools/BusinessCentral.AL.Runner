table 50100 "N8458 Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Txt; Text[50]) { }
    }
    keys
    {
        key(PK; Id) { }
    }
}

table 50101 "N8458 Store"
{
    TableType = Temporary;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys
    {
        key(PK; Id) { }
    }
}

report 50102 "N8458 Word"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = WordOne;
    dataset
    {
        dataitem(Row; "N8458 Row")
        {
            column(Id; Id) { }
            column(Txt; Txt) { }
        }
    }
    rendering
    {
        layout(WordOne) { Type = Word; LayoutFile = 'N8458One.docx'; }
        layout(WordTwo) { Type = Word; LayoutFile = 'N8458Two.docx'; }
    }
}

report 50103 "N8458 Other Word"
{
    UsageCategory = None;
    UseRequestPage = false;
    DefaultRenderingLayout = WordTwo;
    dataset
    {
        dataitem(Row; "N8458 Row")
        {
            column(Id; Id) { }
            column(Txt; Txt) { }
        }
    }
    rendering
    {
        layout(WordOne) { Type = Word; LayoutFile = 'N8458One.docx'; }
        layout(WordTwo) { Type = Word; LayoutFile = 'N8458Two.docx'; }
    }
}

codeunit 50104 "N8458 Saves"
{
    // Saves a report as Html into a temporary Blob and reads the output back as one whole text.
    procedure Body(ReportId: Integer): Text
    var
        Store: Record "N8458 Store" temporary;
        OutS: OutStream;
        InS: InStream;
        T: Text;
    begin
        Store.Id := 1;
        Store.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        Report.SaveAs(ReportId, '', ReportFormat::Html, OutS);
        Store.Insert();
        Store.Data.CreateInStream(InS, TextEncoding::UTF8);
        InS.Read(T);
        exit(T);
    end;

    procedure Rows(Txt: Text; Count: Integer)
    var
        R: Record "N8458 Row";
        i: Integer;
    begin
        R.DeleteAll();
        for i := 1 to Count do begin
            R.Id := i;
            R.Txt := Txt + Format(i);
            R.Insert();
        end;
    end;

    procedure Check(Actual: Boolean; What: Text)
    begin
        if not Actual then
            Error('not as expected: %1', What);
    end;
}

codeunit 50105 "N8458 Keep"
{
    SingleInstance = true;

    var
        Kept: Text;

    procedure Put(T: Text) begin Kept := T; end;

    procedure Get(): Text begin exit(Kept); end;
}

codeunit 50106 "N8458 Keeper"
{
    // Saves in a codeunit of its own and keeps the body in the SingleInstance codeunit.
    trigger OnRun()
    var
        Saves: Codeunit "N8458 Saves";
        Keep: Codeunit "N8458 Keep";
    begin
        Saves.Rows('ROW', 1);
        Keep.Put(Saves.Body(Report::"N8458 Word"));
    end;
}
