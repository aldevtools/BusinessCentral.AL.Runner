// Part 2 of corpus PR 1652 (merged at cf6884b9, not yet pinned), tests/al-language/reports/dataset/DataItemNamesDottedQualifier.al
// :334-698, copied with new ids, a local observer for "Probe Observer" and a temporary Blob table for "Temp Blob". Report
// 50978 is the dotted report 75808, 50979 the undotted control 75809. Head serves the dotted columns through
// report-column-dotted-item-qualifier and the AutoFormatType 2 field (`Unit Cost`) through
// report-column-field-unit-amount-format; main refuses the dotted report (IncludeCaption on a column that is not one field)
// and writes #,##0.00 for `Unit Cost` of the undotted one.

codeunit 50981 "NB DQ2 Observer"
{
    var
        Seen: Dictionary of [Text, Text];
        Failures: Text;

    procedure Observe(Name: Text; Value: Text)
    begin
        Seen.Set(Name, Value);
    end;

    procedure Expect(Name: Text; Expected: Text)
    var
        Actual: Text;
    begin
        if not Seen.Get(Name, Actual) then
            Actual := '<not observed>';
        if Actual <> Expected then
            Failures += StrSubstNo('%1: expected %2, got %3; ', Name, Expected, Actual);
    end;

    procedure Verify()
    begin
        if Failures <> '' then
            Error('%1', Failures);
    end;
}

table 50982 "NB DQ2 Buffer"
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

table 50970 "NB DQ2 Row"
{
    DataClassification = SystemMetadata;

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
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

table 50971 "NB DQ2 Child"
{
    DataClassification = SystemMetadata;

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


enum 50977 "NB DQ2 Status"
{
    Extensible = false;

    value(0; Simulated) { Caption = 'DQ Simulated cap'; }
    value(3; Released) { Caption = 'DQ Released cap'; }
}

table 50976 "NB DQ2 Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Status; Enum "NB DQ2 Status") { Caption = 'DQ Status cap'; }
        field(2; "Order No."; Code[20]) { Caption = 'DQ Order No. cap'; }
        field(3; "Line No."; Integer) { Caption = 'DQ Line No. cap'; }
        field(4; Description; Text[100]) { Caption = 'DQ Description cap'; }
        field(5; "Starting Date"; Date) { Caption = 'DQ Starting Date cap'; }
        field(6; Quantity; Decimal) { AutoFormatType = 0; Caption = 'DQ Quantity cap'; DecimalPlaces = 0 : 5; }
        field(7; "Unit Cost"; Decimal) { AutoFormatType = 2; AutoFormatExpression = ''; Caption = 'DQ Unit Cost cap'; }
        field(8; "Cost Amount"; Decimal) { AutoFormatType = 1; AutoFormatExpression = ''; Caption = 'DQ Cost Amount cap'; }
        field(9; Kind; Option) { Caption = 'DQ Kind cap'; OptionMembers = " ",Open,Closed; OptionCaption = ' ,DQ Open cap,DQ Closed cap'; }
        field(10; "Bin Code"; Code[10]) { }
        field(11; Seq; Integer) { }
        field(12; ZeroFmt; Decimal) { AutoFormatType = 0; Caption = 'DQ ZeroFmt cap'; }
    }

    keys
    {
        key(PK; "Order No.", "Line No.") { Clustered = true; }
    }
}

// The dotted item is the second top-level data item, as "Prod. Order Line" in report 99000763.
report 50978 "NB DQ2 Typed Dotted"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Head"; "NB DQ2 Row")
        {
            column(HName; "Dq. Head".Name) { IncludeCaption = true; }
        }
        dataitem("Dq. Line"; "NB DQ2 Line")
        {
            column(LStatus; "Dq. Line".Status) { IncludeCaption = true; }
            column(LOrderNo; "Dq. Line"."Order No.") { IncludeCaption = true; }
            column(LLineNo; "Dq. Line"."Line No.") { IncludeCaption = true; }
            column(LDescription; "Dq. Line".Description) { IncludeCaption = true; }
            column(LStartingDate; "Dq. Line"."Starting Date") { IncludeCaption = true; }
            column(LQuantity; "Dq. Line".Quantity) { IncludeCaption = true; }
            column(LUnitCost; "Dq. Line"."Unit Cost") { IncludeCaption = true; }
            column(LCostAmount; "Dq. Line"."Cost Amount") { IncludeCaption = true; }
            column(LKind; "Dq. Line".Kind) { IncludeCaption = true; }
            column(LBinCode; "Dq. Line"."Bin Code") { IncludeCaption = true; }
            column(LSeq; "Dq. Line".Seq) { IncludeCaption = true; }
            column(LZeroFmt; "Dq. Line".ZeroFmt) { IncludeCaption = true; }
            column(LStatusNoProp; "Dq. Line".Status) { }
            column(LQuantityNoProp; "Dq. Line".Quantity) { }
            column(LCostAmountNoProp; "Dq. Line"."Cost Amount") { }
            column(LStartingDateNoProp; "Dq. Line"."Starting Date") { }
            column(LBareStatus; Status) { IncludeCaption = true; }
            column(LBareQuantity; Quantity) { IncludeCaption = true; }
        }
    }
}

// Control: the same columns with undotted data item names.
report 50979 "NB DQ2 Typed Undotted"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(DqHead; "NB DQ2 Row")
        {
            column(HName; DqHead.Name) { IncludeCaption = true; }
        }
        dataitem(DqLine; "NB DQ2 Line")
        {
            column(LStatus; DqLine.Status) { IncludeCaption = true; }
            column(LOrderNo; DqLine."Order No.") { IncludeCaption = true; }
            column(LLineNo; DqLine."Line No.") { IncludeCaption = true; }
            column(LDescription; DqLine.Description) { IncludeCaption = true; }
            column(LStartingDate; DqLine."Starting Date") { IncludeCaption = true; }
            column(LQuantity; DqLine.Quantity) { IncludeCaption = true; }
            column(LUnitCost; DqLine."Unit Cost") { IncludeCaption = true; }
            column(LCostAmount; DqLine."Cost Amount") { IncludeCaption = true; }
            column(LKind; DqLine.Kind) { IncludeCaption = true; }
            column(LBinCode; DqLine."Bin Code") { IncludeCaption = true; }
            column(LSeq; DqLine.Seq) { IncludeCaption = true; }
            column(LZeroFmt; DqLine.ZeroFmt) { IncludeCaption = true; }
            column(LStatusNoProp; DqLine.Status) { }
            column(LQuantityNoProp; DqLine.Quantity) { }
            column(LCostAmountNoProp; DqLine."Cost Amount") { }
            column(LStartingDateNoProp; DqLine."Starting Date") { }
            column(LBareStatus; Status) { IncludeCaption = true; }
            column(LBareQuantity; Quantity) { IncludeCaption = true; }
        }
    }
}

codeunit 50980 "NB DQ2 Typed Qualifier Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Seed()
    var
        Row: Record "NB DQ2 Row";
        Line: Record "NB DQ2 Line";
    begin
        ClearAll();
        Row.Id := 1;
        Row.Name := 'Head one';
        Row.Insert();
        Line."Order No." := 'PO-1';
        Line."Line No." := 10000;
        Line.Status := Line.Status::Released;
        Line.Description := 'Line one';
        Line."Starting Date" := DMY2Date(15, 1, 2026);
        Line.Quantity := 12.34567;
        Line."Unit Cost" := 1.23456;
        Line."Cost Amount" := 1234.5;
        Line.Kind := Line.Kind::Open;
        Line."Bin Code" := 'B1';
        Line.Seq := 7;
        Line.ZeroFmt := 2.5;
        Line.Insert();
        Clear(Line);
        Line."Order No." := 'PO-1';
        Line."Line No." := 20000;
        Line.Insert();
    end;

    local procedure ClearAll()
    var
        Row: Record "NB DQ2 Row";
        Child: Record "NB DQ2 Child";
        Line: Record "NB DQ2 Line";
    begin
        Row.DeleteAll();
        Child.DeleteAll();
        Line.DeleteAll();
    end;

    // The trimmed lines of the SaveAs Xml dataset, each in brackets.
    local procedure Dataset(ReportId: Integer): Text
    var
        Buffer: Record "NB DQ2 Buffer";
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
            Result += '[' + Line.Trim() + ']';
        end;
        exit(Result);
    end;

    // The bracketed lines of Xml that start with Keep.
    local procedure Pick(Xml: Text; Keep: Text): Text
    var
        Parts: List of [Text];
        Part: Text;
        Result: Text;
    begin
        Parts := Xml.Split(']');
        foreach Part in Parts do
            if Part.StartsWith('[' + Keep) then
                Result += Part + ']';
        exit(Result);
    end;

    local procedure ObserveLabels(var Observer: Codeunit "NB DQ2 Observer"; Xml: Text)
    begin
        Observer.Observe('labels head', Pick(Xml, '<Label name="H'));
        Observer.Observe('labels code int date text', Pick(Xml, '<Label name="LOrderNo') + Pick(Xml, '<Label name="LLineNo') + Pick(Xml, '<Label name="LDescription') + Pick(Xml, '<Label name="LStartingDate') + Pick(Xml, '<Label name="LBinCode') + Pick(Xml, '<Label name="LSeq'));
        Observer.Observe('labels enum option', Pick(Xml, '<Label name="LStatus') + Pick(Xml, '<Label name="LKind') + Pick(Xml, '<Label name="LBareStatus'));
        Observer.Observe('labels decimal', Pick(Xml, '<Label name="LQuantity') + Pick(Xml, '<Label name="LUnitCost') + Pick(Xml, '<Label name="LCostAmount') + Pick(Xml, '<Label name="LZeroFmt') + Pick(Xml, '<Label name="LBareQuantity'));
    end;

    local procedure ObserveCodeIntDate(var Observer: Codeunit "NB DQ2 Observer"; Xml: Text)
    begin
        Observer.Observe('LOrderNo', Pick(Xml, '<Column name="LOrderNo"'));
        Observer.Observe('LLineNo', Pick(Xml, '<Column name="LLineNo"'));
        Observer.Observe('LDescription', Pick(Xml, '<Column name="LDescription"'));
        Observer.Observe('LStartingDate', Pick(Xml, '<Column name="LStartingDate"'));
        Observer.Observe('LStartingDateNoProp', Pick(Xml, '<Column name="LStartingDateNoProp"'));
        Observer.Observe('LBinCode', Pick(Xml, '<Column name="LBinCode"'));
        Observer.Observe('LSeq', Pick(Xml, '<Column name="LSeq"'));
    end;

    local procedure ObserveEnumOption(var Observer: Codeunit "NB DQ2 Observer"; Xml: Text)
    begin
        Observer.Observe('LStatus', Pick(Xml, '<Column name="LStatus"'));
        Observer.Observe('LStatusNoProp', Pick(Xml, '<Column name="LStatusNoProp"'));
        Observer.Observe('LBareStatus', Pick(Xml, '<Column name="LBareStatus"'));
        Observer.Observe('LKind', Pick(Xml, '<Column name="LKind"'));
    end;

    local procedure ObserveDecimals(var Observer: Codeunit "NB DQ2 Observer"; Xml: Text)
    begin
        Observer.Observe('LQuantity', Pick(Xml, '<Column name="LQuantity"'));
        Observer.Observe('LQuantityNoProp', Pick(Xml, '<Column name="LQuantityNoProp"'));
        Observer.Observe('LBareQuantity', Pick(Xml, '<Column name="LBareQuantity"'));
        Observer.Observe('LUnitCost', Pick(Xml, '<Column name="LUnitCost"'));
        Observer.Observe('LCostAmount', Pick(Xml, '<Column name="LCostAmount"'));
        Observer.Observe('LCostAmountNoProp', Pick(Xml, '<Column name="LCostAmountNoProp"'));
        Observer.Observe('LZeroFmt', Pick(Xml, '<Column name="LZeroFmt"'));
    end;

    [Test]
    procedure TypedDotted_Labels()
    var
        Observer: Codeunit "NB DQ2 Observer";
    begin
        Seed();
        ObserveLabels(Observer, Dataset(Report::"NB DQ2 Typed Dotted"));
        Observer.Expect('labels head', '[<Label name="HNameCaption">DQ Name cap</Label>]');
        Observer.Expect('labels code int date text', '[<Label name="LOrderNoCaption">DQ Order No. cap</Label>][<Label name="LLineNoCaption">DQ Line No. cap</Label>][<Label name="LDescriptionCaption">DQ Description cap</Label>][<Label name="LStartingDateCaption">DQ Starting Date cap</Label>][<Label name="LBinCodeCaption">Bin Code</Label>][<Label name="LSeqCaption">Seq</Label>]');
        Observer.Expect('labels enum option', '[<Label name="LStatusCaption">DQ Status cap</Label>][<Label name="LKindCaption">DQ Kind cap</Label>][<Label name="LBareStatusCaption">DQ Status cap</Label>]');
        Observer.Expect('labels decimal', '[<Label name="LQuantityCaption">DQ Quantity cap</Label>][<Label name="LUnitCostCaption">DQ Unit Cost cap</Label>][<Label name="LCostAmountCaption">DQ Cost Amount cap</Label>][<Label name="LZeroFmtCaption">DQ ZeroFmt cap</Label>][<Label name="LBareQuantityCaption">DQ Quantity cap</Label>]');
        Observer.Verify();
    end;

    [Test]
    procedure TypedDotted_Labels_WhenNoRows()
    var
        Observer: Codeunit "NB DQ2 Observer";
        Xml: Text;
    begin
        ClearAll();
        Xml := Dataset(Report::"NB DQ2 Typed Dotted");
        ObserveLabels(Observer, Xml);
        Observer.Observe('columns', Pick(Xml, '<Column '));
        Observer.Expect('labels head', '[<Label name="HNameCaption">DQ Name cap</Label>]');
        Observer.Expect('labels code int date text', '[<Label name="LOrderNoCaption">DQ Order No. cap</Label>][<Label name="LLineNoCaption">DQ Line No. cap</Label>][<Label name="LDescriptionCaption">DQ Description cap</Label>][<Label name="LStartingDateCaption">DQ Starting Date cap</Label>][<Label name="LBinCodeCaption">Bin Code</Label>][<Label name="LSeqCaption">Seq</Label>]');
        Observer.Expect('labels enum option', '[<Label name="LStatusCaption">DQ Status cap</Label>][<Label name="LKindCaption">DQ Kind cap</Label>][<Label name="LBareStatusCaption">DQ Status cap</Label>]');
        Observer.Expect('labels decimal', '[<Label name="LQuantityCaption">DQ Quantity cap</Label>][<Label name="LUnitCostCaption">DQ Unit Cost cap</Label>][<Label name="LCostAmountCaption">DQ Cost Amount cap</Label>][<Label name="LZeroFmtCaption">DQ ZeroFmt cap</Label>][<Label name="LBareQuantityCaption">DQ Quantity cap</Label>]');
        Observer.Expect('columns', '');
        Observer.Verify();
    end;

    [Test]
    procedure TypedDotted_Columns_CodeIntegerDate()
    var
        Observer: Codeunit "NB DQ2 Observer";
    begin
        Seed();
        ObserveCodeIntDate(Observer, Dataset(Report::"NB DQ2 Typed Dotted"));
        Observer.Expect('LOrderNo', '[<Column name="LOrderNo">PO-1</Column>][<Column name="LOrderNo">PO-1</Column>]');
        Observer.Expect('LLineNo', '[<Column name="LLineNo">10000</Column>][<Column name="LLineNo">20000</Column>]');
        Observer.Expect('LDescription', '[<Column name="LDescription">Line one</Column>][<Column name="LDescription"></Column>]');
        Observer.Expect('LStartingDate', '[<Column name="LStartingDate">1/15/2026</Column>][<Column name="LStartingDate"></Column>]');
        Observer.Expect('LStartingDateNoProp', '[<Column name="LStartingDateNoProp">1/15/2026</Column>][<Column name="LStartingDateNoProp"></Column>]');
        Observer.Expect('LBinCode', '[<Column name="LBinCode">B1</Column>][<Column name="LBinCode"></Column>]');
        Observer.Expect('LSeq', '[<Column name="LSeq">7</Column>][<Column name="LSeq">0</Column>]');
        Observer.Verify();
    end;

    [Test]
    procedure TypedDotted_Columns_EnumOption()
    var
        Observer: Codeunit "NB DQ2 Observer";
    begin
        Seed();
        ObserveEnumOption(Observer, Dataset(Report::"NB DQ2 Typed Dotted"));
        Observer.Expect('LStatus', '[<Column name="LStatus">DQ Released cap</Column>][<Column name="LStatus">DQ Simulated cap</Column>]');
        Observer.Expect('LStatusNoProp', '[<Column name="LStatusNoProp">DQ Released cap</Column>][<Column name="LStatusNoProp">DQ Simulated cap</Column>]');
        Observer.Expect('LBareStatus', '[<Column name="LBareStatus">DQ Released cap</Column>][<Column name="LBareStatus">DQ Simulated cap</Column>]');
        Observer.Expect('LKind', '[<Column name="LKind">DQ Open cap</Column>][<Column name="LKind"> </Column>]');
        Observer.Verify();
    end;

    [Test]
    procedure TypedDotted_Columns_Decimal()
    var
        Observer: Codeunit "NB DQ2 Observer";
    begin
        Seed();
        ObserveDecimals(Observer, Dataset(Report::"NB DQ2 Typed Dotted"));
        Observer.Expect('LQuantity', '[<Column name="LQuantity" decimalformatter="#,##0.#####">12.34567</Column>][<Column name="LQuantity" decimalformatter="#,##0.#####">0</Column>]');
        Observer.Expect('LQuantityNoProp', '[<Column name="LQuantityNoProp" decimalformatter="#,##0.#####">12.34567</Column>][<Column name="LQuantityNoProp" decimalformatter="#,##0.#####">0</Column>]');
        Observer.Expect('LBareQuantity', '[<Column name="LBareQuantity" decimalformatter="#,##0.#####">12.34567</Column>][<Column name="LBareQuantity" decimalformatter="#,##0.#####">0</Column>]');
        Observer.Expect('LUnitCost', '[<Column name="LUnitCost" decimalformatter="#,##0.00###">1.23456</Column>][<Column name="LUnitCost" decimalformatter="#,##0.00###">0</Column>]');
        Observer.Expect('LCostAmount', '[<Column name="LCostAmount" decimalformatter="#,##0.00">1234.5</Column>][<Column name="LCostAmount" decimalformatter="#,##0.00">0</Column>]');
        Observer.Expect('LCostAmountNoProp', '[<Column name="LCostAmountNoProp" decimalformatter="#,##0.00">1234.5</Column>][<Column name="LCostAmountNoProp" decimalformatter="#,##0.00">0</Column>]');
        Observer.Expect('LZeroFmt', '[<Column name="LZeroFmt" decimalformatter="#,##0.00">2.5</Column>][<Column name="LZeroFmt" decimalformatter="#,##0.00">0</Column>]');
        Observer.Verify();
    end;

    [Test]
    procedure TypedUndotted_Labels()
    var
        Observer: Codeunit "NB DQ2 Observer";
    begin
        Seed();
        ObserveLabels(Observer, Dataset(Report::"NB DQ2 Typed Undotted"));
        Observer.Expect('labels head', '[<Label name="HNameCaption">DQ Name cap</Label>]');
        Observer.Expect('labels code int date text', '[<Label name="LOrderNoCaption">DQ Order No. cap</Label>][<Label name="LLineNoCaption">DQ Line No. cap</Label>][<Label name="LDescriptionCaption">DQ Description cap</Label>][<Label name="LStartingDateCaption">DQ Starting Date cap</Label>][<Label name="LBinCodeCaption">Bin Code</Label>][<Label name="LSeqCaption">Seq</Label>]');
        Observer.Expect('labels enum option', '[<Label name="LStatusCaption">DQ Status cap</Label>][<Label name="LKindCaption">DQ Kind cap</Label>][<Label name="LBareStatusCaption">DQ Status cap</Label>]');
        Observer.Expect('labels decimal', '[<Label name="LQuantityCaption">DQ Quantity cap</Label>][<Label name="LUnitCostCaption">DQ Unit Cost cap</Label>][<Label name="LCostAmountCaption">DQ Cost Amount cap</Label>][<Label name="LZeroFmtCaption">DQ ZeroFmt cap</Label>][<Label name="LBareQuantityCaption">DQ Quantity cap</Label>]');
        Observer.Verify();
    end;

    [Test]
    procedure TypedUndotted_Columns_CodeIntegerDate()
    var
        Observer: Codeunit "NB DQ2 Observer";
    begin
        Seed();
        ObserveCodeIntDate(Observer, Dataset(Report::"NB DQ2 Typed Undotted"));
        Observer.Expect('LOrderNo', '[<Column name="LOrderNo">PO-1</Column>][<Column name="LOrderNo">PO-1</Column>]');
        Observer.Expect('LLineNo', '[<Column name="LLineNo">10000</Column>][<Column name="LLineNo">20000</Column>]');
        Observer.Expect('LDescription', '[<Column name="LDescription">Line one</Column>][<Column name="LDescription"></Column>]');
        Observer.Expect('LStartingDate', '[<Column name="LStartingDate">1/15/2026</Column>][<Column name="LStartingDate"></Column>]');
        Observer.Expect('LStartingDateNoProp', '[<Column name="LStartingDateNoProp">1/15/2026</Column>][<Column name="LStartingDateNoProp"></Column>]');
        Observer.Expect('LBinCode', '[<Column name="LBinCode">B1</Column>][<Column name="LBinCode"></Column>]');
        Observer.Expect('LSeq', '[<Column name="LSeq">7</Column>][<Column name="LSeq">0</Column>]');
        Observer.Verify();
    end;

    [Test]
    procedure TypedUndotted_Columns_EnumOption()
    var
        Observer: Codeunit "NB DQ2 Observer";
    begin
        Seed();
        ObserveEnumOption(Observer, Dataset(Report::"NB DQ2 Typed Undotted"));
        Observer.Expect('LStatus', '[<Column name="LStatus">DQ Released cap</Column>][<Column name="LStatus">DQ Simulated cap</Column>]');
        Observer.Expect('LStatusNoProp', '[<Column name="LStatusNoProp">DQ Released cap</Column>][<Column name="LStatusNoProp">DQ Simulated cap</Column>]');
        Observer.Expect('LBareStatus', '[<Column name="LBareStatus">DQ Released cap</Column>][<Column name="LBareStatus">DQ Simulated cap</Column>]');
        Observer.Expect('LKind', '[<Column name="LKind">DQ Open cap</Column>][<Column name="LKind"> </Column>]');
        Observer.Verify();
    end;

    [Test]
    procedure TypedUndotted_Columns_Decimal()
    var
        Observer: Codeunit "NB DQ2 Observer";
    begin
        Seed();
        ObserveDecimals(Observer, Dataset(Report::"NB DQ2 Typed Undotted"));
        Observer.Expect('LQuantity', '[<Column name="LQuantity" decimalformatter="#,##0.#####">12.34567</Column>][<Column name="LQuantity" decimalformatter="#,##0.#####">0</Column>]');
        Observer.Expect('LQuantityNoProp', '[<Column name="LQuantityNoProp" decimalformatter="#,##0.#####">12.34567</Column>][<Column name="LQuantityNoProp" decimalformatter="#,##0.#####">0</Column>]');
        Observer.Expect('LBareQuantity', '[<Column name="LBareQuantity" decimalformatter="#,##0.#####">12.34567</Column>][<Column name="LBareQuantity" decimalformatter="#,##0.#####">0</Column>]');
        Observer.Expect('LUnitCost', '[<Column name="LUnitCost" decimalformatter="#,##0.00###">1.23456</Column>][<Column name="LUnitCost" decimalformatter="#,##0.00###">0</Column>]');
        Observer.Expect('LCostAmount', '[<Column name="LCostAmount" decimalformatter="#,##0.00">1234.5</Column>][<Column name="LCostAmount" decimalformatter="#,##0.00">0</Column>]');
        Observer.Expect('LCostAmountNoProp', '[<Column name="LCostAmountNoProp" decimalformatter="#,##0.00">1234.5</Column>][<Column name="LCostAmountNoProp" decimalformatter="#,##0.00">0</Column>]');
        Observer.Expect('LZeroFmt', '[<Column name="LZeroFmt" decimalformatter="#,##0.00">2.5</Column>][<Column name="LZeroFmt" decimalformatter="#,##0.00">0</Column>]');
        Observer.Verify();
    end;
}
