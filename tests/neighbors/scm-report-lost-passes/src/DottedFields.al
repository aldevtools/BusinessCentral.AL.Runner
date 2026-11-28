// Fields whose readers refuse or change once a qualified column of a dotted data item name resolves to the field
// (review of the pull request, round 1). Each test has its own codeunit, so a refused run leaves nothing to the next test.
table 50933 "NB Dot Fmt"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; AmountAf; Decimal) { AutoFormatType = 1; }
        field(3; AmountUnm; Decimal) { AutoFormatType = 50733; }
        field(4; Pic; Media) { }
        field(5; Data; Blob) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

report 50934 "NB Dot AutoFormat"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dot. Fmt"; "NB Dot Fmt")
        {
            column(QualAf; "Dot. Fmt".AmountAf) { }
        }
    }
}

report 50935 "NB Dot Unmodelled"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dot. Fmt"; "NB Dot Fmt")
        {
            column(QualUnm; "Dot. Fmt".AmountUnm) { }
        }
    }
}

report 50936 "NB Dot Media"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dot. Fmt"; "NB Dot Fmt")
        {
            column(QualPic; "Dot. Fmt".Pic) { }
        }
    }
}

report 50937 "NB Dot Blob"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dot. Fmt"; "NB Dot Fmt")
        {
            column(QualData; "Dot. Fmt".Data) { }
        }
    }
}

// A subscriber outside Microsoft's apps that writes the AutoFormat result.
codeunit 50932 "NB Dot AutoFormat Writer"
{
    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", 'AutoFormatTranslate', '', false, false)]
    local procedure AutoFormatTranslate(AutoFormatType: Integer; AutoFormatExpr: Text[80]; var Translation: Text[80])
    begin
        if AutoFormatType = 1 then
            Translation := '<Precision,3:3><Standard Format,0>';
    end;
}

codeunit 50933 "NB Dot Lib"
{
    procedure Lines(ReportId: Integer; Keep: Text): Text
    var
        Buffer: Record "NB Dot Buffer";
        OutStr: OutStream;
        InStr: InStream;
        Line: Text;
        Result: Text;
    begin
        Buffer.Data.CreateOutStream(OutStr);
        Report.SaveAs(ReportId, '', ReportFormat::Xml, OutStr);
        Buffer.Insert();
        Buffer.CalcFields(Data);
        Buffer.Data.CreateInStream(InStr, TextEncoding::UTF8);
        while not InStr.EOS() do begin
            InStr.ReadText(Line);
            if Line.Trim().StartsWith(Keep) then
                Result += '[' + Line.Trim() + ']';
        end;
        exit(Result);
    end;

    procedure Seed(): Record "NB Dot Fmt"
    var
        Row: Record "NB Dot Fmt";
    begin
        Row.DeleteAll();
        Row.Id := 1;
        Row.AmountAf := 2.5;
        Row.AmountUnm := 2.5;
        exit(Row);
    end;
}

codeunit 50934 "NB Dot AutoFormat Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OwnItemDotted_AutoFormatSubscriber()
    var
        Lib: Codeunit "NB Dot Lib";
        Row: Record "NB Dot Fmt";
        Got: Text;
    begin
        Row := Lib.Seed();
        Row.Insert();
        Got := Lib.Lines(Report::"NB Dot AutoFormat", '<Column ');
        if not Got.StartsWith('[<Column name="QualAf"') then
            Error('got %1', Got);
    end;
}

codeunit 50935 "NB Dot Unmodelled Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OwnItemDotted_AutoFormatUnmodelled()
    var
        Lib: Codeunit "NB Dot Lib";
        Row: Record "NB Dot Fmt";
        Got: Text;
    begin
        Row := Lib.Seed();
        Row.Insert();
        Got := Lib.Lines(Report::"NB Dot Unmodelled", '<Column ');
        if not Got.StartsWith('[<Column name="QualUnm"') then
            Error('got %1', Got);
    end;
}

codeunit 50936 "NB Dot Media Content Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure OwnItemDotted_MediaWithContent()
    var
        Lib: Codeunit "NB Dot Lib";
        Row: Record "NB Dot Fmt";
        Buffer: Record "NB Dot Buffer";
        OutStr: OutStream;
        InStr: InStream;
        Got: Text;
    begin
        Row := Lib.Seed();
        Buffer.Data.CreateOutStream(OutStr);
        OutStr.WriteText('not a picture');
        Buffer.Insert();
        Buffer.CalcFields(Data);
        Buffer.Data.CreateInStream(InStr);
        Row.Pic.ImportStream(InStr, 'pic');
        Row.Insert();
        Got := Lib.Lines(Report::"NB Dot Media", '<Column ');
        if not Got.StartsWith('[<Column name="QualPic"') then
            Error('got %1', Got);
    end;
}

codeunit 50937 "NB Dot Media Empty Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // An empty Media field is an empty element (reports/layouts-saveas/BcXmlBlob.al:102).
    [Test]
    procedure OwnItemDotted_MediaEmpty()
    var
        Lib: Codeunit "NB Dot Lib";
        Row: Record "NB Dot Fmt";
        Got: Text;
    begin
        Row := Lib.Seed();
        Row.Insert();
        Got := Lib.Lines(Report::"NB Dot Media", '<Column ');
        if Got <> '[<Column name="QualPic"></Column>]' then
            Error('got %1', Got);
    end;
}

codeunit 50938 "NB Dot Blob Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    // A Blob the report did not calculate holds the stored bytes (reports/layouts-saveas/BcXmlBlobObjects.al:34 `Row.Data`,
    // reports/layouts-saveas/BcXmlBlob.al:102 `SGVsbG8gQkM=`).
    [Test]
    procedure OwnItemDotted_BlobNotCalculated()
    var
        Lib: Codeunit "NB Dot Lib";
        Row: Record "NB Dot Fmt";
        OutStr: OutStream;
        Got: Text;
    begin
        Row := Lib.Seed();
        Row.Data.CreateOutStream(OutStr);
        OutStr.WriteText('Hello BC');
        Row.Insert();
        Got := Lib.Lines(Report::"NB Dot Blob", '<Column ');
        if Got <> '[<Column name="QualData">SGVsbG8gQkM=</Column>]' then
            Error('got %1', Got);
    end;
}
