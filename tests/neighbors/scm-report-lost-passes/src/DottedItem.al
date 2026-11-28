table 50930 "NB Dot Row"
{
    fields
    {
        field(1; Id; Integer) { Caption = 'Dot Row Id cap'; }
        field(2; Name; Text[30]) { Caption = 'Dot Row Name cap'; }
        field(3; Amount; Decimal) { Caption = 'Dot Row Amount cap'; DecimalPlaces = 0 : 5; }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 50931 "NB Dot Child"
{
    fields
    {
        field(1; Id; Integer) { Caption = 'Dot Child Id cap'; }
        field(2; RowId; Integer) { }
        field(3; Note; Text[30]) { Caption = 'Dot Child Note cap'; }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 50932 "NB Dot Buffer"
{
    TableType = Temporary;
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// Own item, its name holds a dot: IncludeCaption labels and the Decimal formatter of a qualified field.
report 50930 "NB Dot Own"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dot. Row"; "NB Dot Row")
        {
            column(OwnName; "Dot. Row".Name) { IncludeCaption = true; }
            column(OwnId; "Dot. Row".Id) { IncludeCaption = true; }
            column(OwnAmount; "Dot. Row".Amount) { }
            column(BareName; Name) { IncludeCaption = true; }
        }
    }
}

// A child item reads a field of its dotted-name parent (the other-item path, unchanged).
report 50931 "NB Dot Parent"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dot. Row"; "NB Dot Row")
        {
            column(PId; Id) { }
            dataitem("Dot. Child"; "NB Dot Child")
            {
                DataItemLink = RowId = field(Id);
                column(FromParent; "Dot. Row".Name) { IncludeCaption = true; }
                column(OwnNote; "Dot. Child".Note) { IncludeCaption = true; }
            }
        }
    }
}

// The qualifier names another table, not the item (no field of the own item).
report 50932 "NB Dot Other"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dot. Row"; "NB Dot Row")
        {
            column(GlobalName; Other.Note) { IncludeCaption = true; }
        }
    }

    var
        Other: Record "NB Dot Child";
}

// Own item, dotted name, no IncludeCaption: only the Decimal formatter reads the qualified field.
report 50933 "NB Dot Decimal"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dot. Row"; "NB Dot Row")
        {
            column(QualAmount; "Dot. Row".Amount) { }
            column(BareAmount; Amount) { }
        }
    }
}

codeunit 50930 "NB Dot Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Seed()
    var
        Row: Record "NB Dot Row";
        Child: Record "NB Dot Child";
    begin
        Row.DeleteAll();
        Child.DeleteAll();
        Row.Id := 1;
        Row.Name := 'R1';
        Row.Amount := 2.5;
        Row.Insert();
        Child.Id := 10;
        Child.RowId := 1;
        Child.Note := 'C1';
        Child.Insert();
    end;

    local procedure Lines(ReportId: Integer; Keep: Text): Text
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

    local procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('expected %1, got %2', Expected, Actual);
    end;

    [Test]
    procedure OwnItemDotted_Labels()
    begin
        Seed();
        Check('[<Label name="OwnNameCaption">Dot Row Name cap</Label>][<Label name="OwnIdCaption">Dot Row Id cap</Label>][<Label name="BareNameCaption">Dot Row Name cap</Label>]',
            Lines(Report::"NB Dot Own", '<Label '));
    end;

    [Test]
    procedure OwnItemDotted_Columns()
    begin
        Seed();
        Check('[<Column name="OwnName">R1</Column>][<Column name="OwnId">1</Column>][<Column name="OwnAmount" decimalformatter="#,##0.#####">2.5</Column>][<Column name="BareName">R1</Column>]',
            Lines(Report::"NB Dot Own", '<Column '));
    end;

    [Test]
    procedure ParentItemDotted_Labels()
    begin
        Seed();
        Check('[<Label name="FromParentCaption">Dot Row Name cap</Label>][<Label name="OwnNoteCaption">Dot Child Note cap</Label>]',
            Lines(Report::"NB Dot Parent", '<Label '));
    end;

    [Test]
    procedure GlobalRecordQualifier_Labels()
    begin
        Seed();
        Check('[<Label name="GlobalNameCaption">Dot Child Note cap</Label>]',
            Lines(Report::"NB Dot Other", '<Label '));
    end;

    [Test]
    procedure OwnItemDotted_DecimalFormatter()
    begin
        Seed();
        Check('[<Column name="QualAmount" decimalformatter="#,##0.#####">2.5</Column>][<Column name="BareAmount" decimalformatter="#,##0.#####">2.5</Column>]',
            Lines(Report::"NB Dot Decimal", '<Column '));
    end;
}
