// The cells corpus PR 1652 measures (tests/al-language/reports/dataset/DataItemNamesDottedQualifier.al at 1751a6b0): a
// column of the own data item written with the item's dotted name. Head serves them through the assumption
// report-column-dotted-item-qualifier; main splits the name at its dot and finds no field.
enum 50950 "NB DQ Status"
{
    value(0; Planned) { Caption = 'Planned cap'; }
    value(1; Released) { Caption = 'Released cap'; }
}

table 50950 "NB DQ Row"
{
    fields
    {
        field(1; Id; Integer) { Caption = 'DQ Id cap'; }
        field(2; Name; Text[30]) { Caption = 'DQ Name cap'; }
        field(3; Qty; Decimal) { Caption = 'DQ Qty cap'; DecimalPlaces = 0 : 3; }
        field(4; Amt; Decimal) { Caption = 'DQ Amt cap'; AutoFormatType = 1; }
        field(5; Unit; Decimal) { Caption = 'DQ Unit cap'; AutoFormatType = 2; }
        field(6; AmtPlaces; Decimal) { Caption = 'DQ AmtPlaces cap'; AutoFormatType = 1; DecimalPlaces = 0 : 3; }
        field(7; "Unit Amt."; Decimal) { DecimalPlaces = 0 : 1; }
        field(8; Plain; Decimal) { }
        field(9; "No."; Code[20]) { Caption = 'DQ No cap'; }
        field(10; Status; Enum "NB DQ Status") { Caption = 'DQ Status cap'; }
        field(11; Expr; Decimal) { Caption = 'DQ Expr cap'; AutoFormatType = 1; AutoFormatExpression = ''; }
        field(12; "Due Date"; Date) { Caption = 'DQ Due cap'; }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 50951 "NB DQ Child"
{
    fields
    {
        field(1; Id; Integer) { Caption = 'DQ Child Id cap'; }
        field(2; RowId; Integer) { }
        field(3; Note; Text[30]) { Caption = 'DQ Child Note cap'; }
        field(4; Price; Decimal) { Caption = 'DQ Child Price cap'; DecimalPlaces = 0 : 4; }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 50952 "NB DQ Buffer"
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

// DataItemNamesDottedQualifier.al:66-98 (report 75802), without the columns head does not serve: DUnit (AutoFormatType 2)
// and DAmtPlaces (AutoFormatType 1 with DecimalPlaces), see Unmeasured.al; CParentQty and CParentAmt (main's rule for a field
// of another data item, unchanged), see Unmeasured.al.
report 50950 "NB DQ Dotted"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }
            column(DQty; "Dq. Row".Qty) { IncludeCaption = true; }
            column(DAmt; "Dq. Row".Amt) { IncludeCaption = true; }
            column(DUnitAmt; "Dq. Row"."Unit Amt.") { IncludeCaption = true; }
            column(DPlain; "Dq. Row".Plain) { IncludeCaption = true; }
            column(DQtyOwnPlaces; "Dq. Row".Qty) { DecimalPlaces = 4 : 4; }
            column(DBareQty; Qty) { IncludeCaption = true; }
            column(DUpper; "Dq. Row".Name.ToUpper()) { }

            dataitem("Dq. Child"; "NB DQ Child")
            {
                DataItemLink = RowId = field(Id);

                column(CParentName; "Dq. Row".Name) { IncludeCaption = true; }
                column(CNote; "Dq. Child".Note) { IncludeCaption = true; }
                column(CPrice; "Dq. Child".Price) { IncludeCaption = true; }
            }
        }
    }
}

// DataItemNamesDottedQualifier.al:140-158 (report 75804).
report 50951 "NB DQ Dot Variants"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq.Row"; "NB DQ Row")
        {
            column(VName; "Dq.Row".Name) { IncludeCaption = true; }
            column(VQty; "Dq.Row".Qty) { IncludeCaption = true; }
            column(VAmt; "Dq.Row".Amt) { IncludeCaption = true; }
        }
        dataitem("Dq. R. Two"; "NB DQ Row")
        {
            column(WName; "Dq. R. Two".Name) { IncludeCaption = true; }
            column(WQty; "Dq. R. Two".Qty) { IncludeCaption = true; }
            column(WUnitAmt; "Dq. R. Two"."Unit Amt.") { IncludeCaption = true; }
        }
    }
}

codeunit 50950 "NB DQ Lib"
{
    procedure Lines(ReportId: Integer; Keep: Text): Text
    var
        Buffer: Record "NB DQ Buffer";
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

    procedure Seed()
    var
        Row: Record "NB DQ Row";
        Child: Record "NB DQ Child";
    begin
        Clear();
        Row.Id := 1;
        Row.Name := 'Row one';
        Row.Qty := 2.5;
        Row.Amt := 1234.5;
        Row.Unit := 1.23456;
        Row.AmtPlaces := 1234.5;
        Row."Unit Amt." := 3.25;
        Row.Plain := 7;
        Row."No." := 'N1';
        Row.Status := Row.Status::Released;
        Row.Expr := 1234.5;
        Row."Due Date" := 20260101D;
        Row.Insert();
        Child.Id := 10;
        Child.RowId := 1;
        Child.Note := 'C1';
        Child.Price := 0.5;
        Child.Insert();
    end;

    procedure Clear()
    var
        Row: Record "NB DQ Row";
        Child: Record "NB DQ Child";
    begin
        Row.DeleteAll();
        Child.DeleteAll();
    end;

    procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('expected %1, got %2', Expected, Actual);
    end;
}

codeunit 50951 "NB DQ Measured Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "NB DQ Lib";

    // :224
    [Test]
    procedure Dotted_Labels_OwnAndChild()
    begin
        Lib.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="DQtyCaption">DQ Qty cap</Label>][<Label name="DAmtCaption">DQ Amt cap</Label>][<Label name="DUnitAmtCaption">Unit Amt.</Label>][<Label name="DPlainCaption">Plain</Label>][<Label name="DBareQtyCaption">DQ Qty cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>][<Label name="CNoteCaption">DQ Child Note cap</Label>][<Label name="CPriceCaption">DQ Child Price cap</Label>]',
            Lib.Lines(Report::"NB DQ Dotted", '<Label '));
    end;

    // :236 and :238 (the same on every version for these columns).
    [Test]
    procedure Dotted_Columns_OwnItem()
    begin
        Lib.Seed();
        Lib.Check('[<Column name="DName">Row one</Column>][<Column name="DQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="DAmt" decimalformatter="#,##0.00">1234.5</Column>][<Column name="DUnitAmt" decimalformatter="#,##0.#">3.25</Column>][<Column name="DPlain" decimalformatter="#,##0.00">7</Column>][<Column name="DQtyOwnPlaces" decimalformatter="#,##0.0000">2.5</Column>][<Column name="DBareQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="DUpper">ROW ONE</Column>]',
            Lib.Lines(Report::"NB DQ Dotted", '<Column name="D'));
    end;

    // :250
    [Test]
    procedure Dotted_Columns_ChildItem()
    begin
        Lib.Seed();
        Lib.Check('[<Column name="CParentName">Row one</Column>][<Column name="CNote">C1</Column>][<Column name="CPrice" decimalformatter="#,##0.####">0.5</Column>]',
            Lib.Lines(Report::"NB DQ Dotted", '<Column name="C'));
    end;

    // :262-263
    [Test]
    procedure Dotted_Labels_WhenNoRows()
    begin
        Lib.Clear();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="DQtyCaption">DQ Qty cap</Label>][<Label name="DAmtCaption">DQ Amt cap</Label>][<Label name="DUnitAmtCaption">Unit Amt.</Label>][<Label name="DPlainCaption">Plain</Label>][<Label name="DBareQtyCaption">DQ Qty cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>][<Label name="CNoteCaption">DQ Child Note cap</Label>][<Label name="CPriceCaption">DQ Child Price cap</Label>]',
            Lib.Lines(Report::"NB DQ Dotted", '<Label '));
        Lib.Check('', Lib.Lines(Report::"NB DQ Dotted", '<Column '));
    end;

    // :312-313
    [Test]
    procedure DotVariants_LabelsAndColumns()
    begin
        Lib.Seed();
        Lib.Check('[<Label name="VNameCaption">DQ Name cap</Label>][<Label name="VQtyCaption">DQ Qty cap</Label>][<Label name="VAmtCaption">DQ Amt cap</Label>][<Label name="WNameCaption">DQ Name cap</Label>][<Label name="WQtyCaption">DQ Qty cap</Label>][<Label name="WUnitAmtCaption">Unit Amt.</Label>]',
            Lib.Lines(Report::"NB DQ Dot Variants", '<Label '));
        Lib.Check('[<Column name="VName">Row one</Column>][<Column name="VQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="VAmt" decimalformatter="#,##0.00">1234.5</Column>][<Column name="WName">Row one</Column>][<Column name="WQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="WUnitAmt" decimalformatter="#,##0.#">3.25</Column>]',
            Lib.Lines(Report::"NB DQ Dot Variants", '<Column '));
    end;
}
