// Corpus PR 1652 (merged at cf6884b9, not yet pinned), tests/al-language/reports/dataset/DataItemNamesDottedQualifier.al
// :24-698, copied with new ids (751xx -> 511xx), a local observer for "Probe Observer" and a temporary Blob table for
// "Temp Blob". The expected values are the probe's, measured on BC 27.5, 28.5 and 29.0; where the probe splits by version (`#if BC28PLUS`,
// :235-239, :285-289) only the BC 28 and 29 value is kept: a neighbor app has no version symbol, and on BC 27 main and head
// write the same `#,##0.###`, so the test fails the same way on both.

codeunit 51111 "NB RCS Observer"
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

table 51112 "NB RCS Buffer"
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

table 51100 "NB RCS Row"
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

table 51101 "NB RCS Child"
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

// The data item names hold a dot and a space. Columns D* read fields of the own item through its name, C* of the child item.
report 51102 "NB RCS Dotted"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB RCS Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }
            column(DQty; "Dq. Row".Qty) { IncludeCaption = true; }
            column(DAmt; "Dq. Row".Amt) { IncludeCaption = true; }
            column(DUnit; "Dq. Row".Unit) { IncludeCaption = true; }
            column(DAmtPlaces; "Dq. Row".AmtPlaces) { IncludeCaption = true; }
            column(DUnitAmt; "Dq. Row"."Unit Amt.") { IncludeCaption = true; }
            column(DPlain; "Dq. Row".Plain) { IncludeCaption = true; }
            column(DQtyOwnPlaces; "Dq. Row".Qty) { DecimalPlaces = 4 : 4; }
            column(DBareQty; Qty) { IncludeCaption = true; }
            column(DUpper; "Dq. Row".Name.ToUpper()) { }

            dataitem("Dq. Child"; "NB RCS Child")
            {
                DataItemLink = RowId = field(Id);

                column(CParentName; "Dq. Row".Name) { IncludeCaption = true; }
                column(CParentQty; "Dq. Row".Qty) { IncludeCaption = true; }
                column(CParentAmt; "Dq. Row".Amt) { IncludeCaption = true; }
                column(CNote; "Dq. Child".Note) { IncludeCaption = true; }
                column(CPrice; "Dq. Child".Price) { IncludeCaption = true; }
            }
        }
    }
}

// Control: the same columns with undotted data item names.
report 51103 "NB RCS Undotted"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(DqRow; "NB RCS Row")
        {
            column(DName; DqRow.Name) { IncludeCaption = true; }
            column(DQty; DqRow.Qty) { IncludeCaption = true; }
            column(DAmt; DqRow.Amt) { IncludeCaption = true; }
            column(DUnit; DqRow.Unit) { IncludeCaption = true; }
            column(DAmtPlaces; DqRow.AmtPlaces) { IncludeCaption = true; }
            column(DUnitAmt; DqRow."Unit Amt.") { IncludeCaption = true; }
            column(DPlain; DqRow.Plain) { IncludeCaption = true; }
            column(DQtyOwnPlaces; DqRow.Qty) { DecimalPlaces = 4 : 4; }
            column(DBareQty; Qty) { IncludeCaption = true; }
            column(DUpper; DqRow.Name.ToUpper()) { }

            dataitem(DqChild; "NB RCS Child")
            {
                DataItemLink = RowId = field(Id);

                column(CParentName; DqRow.Name) { IncludeCaption = true; }
                column(CParentQty; DqRow.Qty) { IncludeCaption = true; }
                column(CParentAmt; DqRow.Amt) { IncludeCaption = true; }
                column(CNote; DqChild.Note) { IncludeCaption = true; }
                column(CPrice; DqChild.Price) { IncludeCaption = true; }
            }
        }
    }
}

// Other dotted names: a dot without a space, and two dots with spaces.
report 51104 "NB RCS Dot Variants"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq.Row"; "NB RCS Row")
        {
            column(VName; "Dq.Row".Name) { IncludeCaption = true; }
            column(VQty; "Dq.Row".Qty) { IncludeCaption = true; }
            column(VAmt; "Dq.Row".Amt) { IncludeCaption = true; }
        }
        dataitem("Dq. R. Two"; "NB RCS Row")
        {
            column(WName; "Dq. R. Two".Name) { IncludeCaption = true; }
            column(WQty; "Dq. R. Two".Qty) { IncludeCaption = true; }
            column(WUnitAmt; "Dq. R. Two"."Unit Amt.") { IncludeCaption = true; }
        }
    }
}

codeunit 51105 "NB RCS Dotted Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Seed()
    var
        Row: Record "NB RCS Row";
        Child: Record "NB RCS Child";
    begin
        Row.DeleteAll();
        Child.DeleteAll();
        Row.Id := 1;
        Row.Name := 'Row one';
        Row.Qty := 2.5;
        Row.Amt := 1234.5;
        Row.Unit := 1.23456;
        Row.AmtPlaces := 1234.5;
        Row."Unit Amt." := 3.25;
        Row.Plain := 7;
        Row.Insert();
        Child.Id := 10;
        Child.RowId := 1;
        Child.Note := 'C1';
        Child.Price := 0.5;
        Child.Insert();
    end;

    local procedure Clear()
    var
        Row: Record "NB RCS Row";
        Child: Record "NB RCS Child";
    begin
        Row.DeleteAll();
        Child.DeleteAll();
    end;

    // The trimmed lines of the SaveAs Xml dataset that start with Keep, each in brackets.
    local procedure Lines(ReportId: Integer; Keep: Text): Text
    var
        Buffer: Record "NB RCS Buffer";
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

    [Test]
    procedure Dotted_Labels_OwnAndChild()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        Observer.Observe('labels', Lines(Report::"NB RCS Dotted", '<Label '));
        Observer.Expect('labels', '[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="DQtyCaption">DQ Qty cap</Label>][<Label name="DAmtCaption">DQ Amt cap</Label>][<Label name="DUnitCaption">DQ Unit cap</Label>][<Label name="DAmtPlacesCaption">DQ AmtPlaces cap</Label>][<Label name="DUnitAmtCaption">Unit Amt.</Label>][<Label name="DPlainCaption">Plain</Label>][<Label name="DBareQtyCaption">DQ Qty cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>][<Label name="CParentQtyCaption">DQ Qty cap</Label>][<Label name="CParentAmtCaption">DQ Amt cap</Label>][<Label name="CNoteCaption">DQ Child Note cap</Label>][<Label name="CPriceCaption">DQ Child Price cap</Label>]');
        Observer.Verify();
    end;

    [Test]
    procedure Dotted_Columns_OwnItem()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        Observer.Observe('columns', Lines(Report::"NB RCS Dotted", '<Column name="D'));
        Observer.Expect('columns', '[<Column name="DName">Row one</Column>][<Column name="DQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="DAmt" decimalformatter="#,##0.00">1234.5</Column>][<Column name="DUnit" decimalformatter="#,##0.00###">1.23456</Column>][<Column name="DAmtPlaces" decimalformatter="#,##0.00">1234.5</Column>][<Column name="DUnitAmt" decimalformatter="#,##0.#">3.25</Column>][<Column name="DPlain" decimalformatter="#,##0.00">7</Column>][<Column name="DQtyOwnPlaces" decimalformatter="#,##0.0000">2.5</Column>][<Column name="DBareQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="DUpper">ROW ONE</Column>]');
        Observer.Verify();
    end;

    [Test]
    procedure Dotted_Columns_ChildItem()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        Observer.Observe('columns', Lines(Report::"NB RCS Dotted", '<Column name="C'));
        Observer.Expect('columns', '[<Column name="CParentName">Row one</Column>][<Column name="CParentQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="CParentAmt" decimalformatter="#,##0.####">1234.5</Column>][<Column name="CNote">C1</Column>][<Column name="CPrice" decimalformatter="#,##0.####">0.5</Column>]');
        Observer.Verify();
    end;

    [Test]
    procedure Dotted_Labels_WhenNoRows()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Clear();
        Observer.Observe('labels', Lines(Report::"NB RCS Dotted", '<Label '));
        Observer.Observe('columns', Lines(Report::"NB RCS Dotted", '<Column '));
        Observer.Expect('labels', '[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="DQtyCaption">DQ Qty cap</Label>][<Label name="DAmtCaption">DQ Amt cap</Label>][<Label name="DUnitCaption">DQ Unit cap</Label>][<Label name="DAmtPlacesCaption">DQ AmtPlaces cap</Label>][<Label name="DUnitAmtCaption">Unit Amt.</Label>][<Label name="DPlainCaption">Plain</Label>][<Label name="DBareQtyCaption">DQ Qty cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>][<Label name="CParentQtyCaption">DQ Qty cap</Label>][<Label name="CParentAmtCaption">DQ Amt cap</Label>][<Label name="CNoteCaption">DQ Child Note cap</Label>][<Label name="CPriceCaption">DQ Child Price cap</Label>]');
        Observer.Expect('columns', '');
        Observer.Verify();
    end;

}

codeunit 51113 "NB RCS Undotted Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Seed()
    var
        Row: Record "NB RCS Row";
        Child: Record "NB RCS Child";
    begin
        Row.DeleteAll();
        Child.DeleteAll();
        Row.Id := 1;
        Row.Name := 'Row one';
        Row.Qty := 2.5;
        Row.Amt := 1234.5;
        Row.Unit := 1.23456;
        Row.AmtPlaces := 1234.5;
        Row."Unit Amt." := 3.25;
        Row.Plain := 7;
        Row.Insert();
        Child.Id := 10;
        Child.RowId := 1;
        Child.Note := 'C1';
        Child.Price := 0.5;
        Child.Insert();
    end;

    local procedure Clear()
    var
        Row: Record "NB RCS Row";
        Child: Record "NB RCS Child";
    begin
        Row.DeleteAll();
        Child.DeleteAll();
    end;

    // The trimmed lines of the SaveAs Xml dataset that start with Keep, each in brackets.
    local procedure Lines(ReportId: Integer; Keep: Text): Text
    var
        Buffer: Record "NB RCS Buffer";
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

    [Test]
    procedure Undotted_Labels_OwnAndChild()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        Observer.Observe('labels', Lines(Report::"NB RCS Undotted", '<Label '));
        Observer.Expect('labels', '[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="DQtyCaption">DQ Qty cap</Label>][<Label name="DAmtCaption">DQ Amt cap</Label>][<Label name="DUnitCaption">DQ Unit cap</Label>][<Label name="DAmtPlacesCaption">DQ AmtPlaces cap</Label>][<Label name="DUnitAmtCaption">Unit Amt.</Label>][<Label name="DPlainCaption">Plain</Label>][<Label name="DBareQtyCaption">DQ Qty cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>][<Label name="CParentQtyCaption">DQ Qty cap</Label>][<Label name="CParentAmtCaption">DQ Amt cap</Label>][<Label name="CNoteCaption">DQ Child Note cap</Label>][<Label name="CPriceCaption">DQ Child Price cap</Label>]');
        Observer.Verify();
    end;

    [Test]
    procedure Undotted_Columns_OwnItem()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        Observer.Observe('columns', Lines(Report::"NB RCS Undotted", '<Column name="D'));
        Observer.Expect('columns', '[<Column name="DName">Row one</Column>][<Column name="DQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="DAmt" decimalformatter="#,##0.00">1234.5</Column>][<Column name="DUnit" decimalformatter="#,##0.00###">1.23456</Column>][<Column name="DAmtPlaces" decimalformatter="#,##0.00">1234.5</Column>][<Column name="DUnitAmt" decimalformatter="#,##0.#">3.25</Column>][<Column name="DPlain" decimalformatter="#,##0.00">7</Column>][<Column name="DQtyOwnPlaces" decimalformatter="#,##0.0000">2.5</Column>][<Column name="DBareQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="DUpper">ROW ONE</Column>]');
        Observer.Verify();
    end;

    [Test]
    procedure Undotted_Columns_ChildItem()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        Observer.Observe('columns', Lines(Report::"NB RCS Undotted", '<Column name="C'));
        Observer.Expect('columns', '[<Column name="CParentName">Row one</Column>][<Column name="CParentQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="CParentAmt" decimalformatter="#,##0.####">1234.5</Column>][<Column name="CNote">C1</Column>][<Column name="CPrice" decimalformatter="#,##0.####">0.5</Column>]');
        Observer.Verify();
    end;

    [Test]
    procedure DotVariants_LabelsAndColumns()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        Observer.Observe('labels', Lines(Report::"NB RCS Dot Variants", '<Label '));
        Observer.Observe('columns', Lines(Report::"NB RCS Dot Variants", '<Column '));
        Observer.Expect('labels', '[<Label name="VNameCaption">DQ Name cap</Label>][<Label name="VQtyCaption">DQ Qty cap</Label>][<Label name="VAmtCaption">DQ Amt cap</Label>][<Label name="WNameCaption">DQ Name cap</Label>][<Label name="WQtyCaption">DQ Qty cap</Label>][<Label name="WUnitAmtCaption">Unit Amt.</Label>]');
        Observer.Expect('columns', '[<Column name="VName">Row one</Column>][<Column name="VQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="VAmt" decimalformatter="#,##0.00">1234.5</Column>][<Column name="WName">Row one</Column>][<Column name="WQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="WUnitAmt" decimalformatter="#,##0.#">3.25</Column>]');
        Observer.Verify();
    end;
}

// Part 2 (al-runner lost pass 9330, report 99000763 "Prod. Order - List"): the field types and properties that report puts behind
//   a dotted data item name, as the second top-level data item. Objects: enum 51107, table 51106, reports 51108 (dotted), 51109
//   (undotted control), test codeunit 51110. Fields: Enum with value captions, Code with and without Caption, Integer with and
//   without Caption, Date, Text[100], Option with OptionCaption, Decimal AutoFormatType 0 + DecimalPlaces 0:5, Decimal
//   AutoFormatType 0 alone, AutoFormatType 2 + AutoFormatExpression '', AutoFormatType 1 + AutoFormatExpression ''. Columns with
//   IncludeCaption, without any property, and bare (unqualified) beside the dotted ones. Row 2 holds only its key (default values).
// Measured on BC on Linux 27.5, 28.5 and 29.0 (CI setup); all three tiers gave the same lines, so there is no version guard.
//   Every Label and Column line of the dotted report 51108 equals the line of the undotted control 51109.
//   IncludeCaption gives the field Caption for every type, or the field name when the field has none ('Bin Code', 'Seq').
//   Labels are written when the item has no rows. Column values: Enum and Option give the value caption ('DQ Released cap',
//   'DQ Simulated cap' for the default, 'DQ Open cap', ' ' for the blank option member); Code and Text the text, '' when empty;
//   Integer the number, 0 by default; Date '1/15/2026', '' for 0D. None of these has a decimalformatter. Decimal
//   decimalformatter: AutoFormatType 0 + DecimalPlaces 0:5 '#,##0.#####', AutoFormatType 0 alone '#,##0.00', AutoFormatType 2 +
//   AutoFormatExpression '' '#,##0.00###', AutoFormatType 1 + AutoFormatExpression '' '#,##0.00'; a zero value is '0'. A column
//   without IncludeCaption, and a bare column beside the dotted ones, give the same value and decimalformatter.

enum 51107 "NB RCS Status"
{
    Extensible = false;

    value(0; Simulated) { Caption = 'DQ Simulated cap'; }
    value(3; Released) { Caption = 'DQ Released cap'; }
}

table 51106 "NB RCS Line"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Status; Enum "NB RCS Status") { Caption = 'DQ Status cap'; }
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
report 51108 "NB RCS Typed Dotted"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Head"; "NB RCS Row")
        {
            column(HName; "Dq. Head".Name) { IncludeCaption = true; }
        }
        dataitem("Dq. Line"; "NB RCS Line")
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
report 51109 "NB RCS Typed Undotted"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(DqHead; "NB RCS Row")
        {
            column(HName; DqHead.Name) { IncludeCaption = true; }
        }
        dataitem(DqLine; "NB RCS Line")
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

codeunit 51110 "NB RCS Typed Dotted Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Seed()
    var
        Row: Record "NB RCS Row";
        Line: Record "NB RCS Line";
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
        Row: Record "NB RCS Row";
        Child: Record "NB RCS Child";
        Line: Record "NB RCS Line";
    begin
        Row.DeleteAll();
        Child.DeleteAll();
        Line.DeleteAll();
    end;

    // The trimmed lines of the SaveAs Xml dataset, each in brackets.
    local procedure Dataset(ReportId: Integer): Text
    var
        Buffer: Record "NB RCS Buffer";
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

    local procedure ObserveLabels(var Observer: Codeunit "NB RCS Observer"; Xml: Text)
    begin
        Observer.Observe('labels head', Pick(Xml, '<Label name="H'));
        Observer.Observe('labels code int date text', Pick(Xml, '<Label name="LOrderNo') + Pick(Xml, '<Label name="LLineNo') + Pick(Xml, '<Label name="LDescription') + Pick(Xml, '<Label name="LStartingDate') + Pick(Xml, '<Label name="LBinCode') + Pick(Xml, '<Label name="LSeq'));
        Observer.Observe('labels enum option', Pick(Xml, '<Label name="LStatus') + Pick(Xml, '<Label name="LKind') + Pick(Xml, '<Label name="LBareStatus'));
        Observer.Observe('labels decimal', Pick(Xml, '<Label name="LQuantity') + Pick(Xml, '<Label name="LUnitCost') + Pick(Xml, '<Label name="LCostAmount') + Pick(Xml, '<Label name="LZeroFmt') + Pick(Xml, '<Label name="LBareQuantity'));
    end;

    local procedure ObserveCodeIntDate(var Observer: Codeunit "NB RCS Observer"; Xml: Text)
    begin
        Observer.Observe('LOrderNo', Pick(Xml, '<Column name="LOrderNo"'));
        Observer.Observe('LLineNo', Pick(Xml, '<Column name="LLineNo"'));
        Observer.Observe('LDescription', Pick(Xml, '<Column name="LDescription"'));
        Observer.Observe('LStartingDate', Pick(Xml, '<Column name="LStartingDate"'));
        Observer.Observe('LStartingDateNoProp', Pick(Xml, '<Column name="LStartingDateNoProp"'));
        Observer.Observe('LBinCode', Pick(Xml, '<Column name="LBinCode"'));
        Observer.Observe('LSeq', Pick(Xml, '<Column name="LSeq"'));
    end;

    local procedure ObserveEnumOption(var Observer: Codeunit "NB RCS Observer"; Xml: Text)
    begin
        Observer.Observe('LStatus', Pick(Xml, '<Column name="LStatus"'));
        Observer.Observe('LStatusNoProp', Pick(Xml, '<Column name="LStatusNoProp"'));
        Observer.Observe('LBareStatus', Pick(Xml, '<Column name="LBareStatus"'));
        Observer.Observe('LKind', Pick(Xml, '<Column name="LKind"'));
    end;

    local procedure ObserveDecimals(var Observer: Codeunit "NB RCS Observer"; Xml: Text)
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
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        ObserveLabels(Observer, Dataset(Report::"NB RCS Typed Dotted"));
        Observer.Expect('labels head', '[<Label name="HNameCaption">DQ Name cap</Label>]');
        Observer.Expect('labels code int date text', '[<Label name="LOrderNoCaption">DQ Order No. cap</Label>][<Label name="LLineNoCaption">DQ Line No. cap</Label>][<Label name="LDescriptionCaption">DQ Description cap</Label>][<Label name="LStartingDateCaption">DQ Starting Date cap</Label>][<Label name="LBinCodeCaption">Bin Code</Label>][<Label name="LSeqCaption">Seq</Label>]');
        Observer.Expect('labels enum option', '[<Label name="LStatusCaption">DQ Status cap</Label>][<Label name="LKindCaption">DQ Kind cap</Label>][<Label name="LBareStatusCaption">DQ Status cap</Label>]');
        Observer.Expect('labels decimal', '[<Label name="LQuantityCaption">DQ Quantity cap</Label>][<Label name="LUnitCostCaption">DQ Unit Cost cap</Label>][<Label name="LCostAmountCaption">DQ Cost Amount cap</Label>][<Label name="LZeroFmtCaption">DQ ZeroFmt cap</Label>][<Label name="LBareQuantityCaption">DQ Quantity cap</Label>]');
        Observer.Verify();
    end;

    [Test]
    procedure TypedDotted_Labels_WhenNoRows()
    var
        Observer: Codeunit "NB RCS Observer";
        Xml: Text;
    begin
        ClearAll();
        Xml := Dataset(Report::"NB RCS Typed Dotted");
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
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        ObserveCodeIntDate(Observer, Dataset(Report::"NB RCS Typed Dotted"));
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
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        ObserveEnumOption(Observer, Dataset(Report::"NB RCS Typed Dotted"));
        Observer.Expect('LStatus', '[<Column name="LStatus">DQ Released cap</Column>][<Column name="LStatus">DQ Simulated cap</Column>]');
        Observer.Expect('LStatusNoProp', '[<Column name="LStatusNoProp">DQ Released cap</Column>][<Column name="LStatusNoProp">DQ Simulated cap</Column>]');
        Observer.Expect('LBareStatus', '[<Column name="LBareStatus">DQ Released cap</Column>][<Column name="LBareStatus">DQ Simulated cap</Column>]');
        Observer.Expect('LKind', '[<Column name="LKind">DQ Open cap</Column>][<Column name="LKind"> </Column>]');
        Observer.Verify();
    end;

    [Test]
    procedure TypedDotted_Columns_Decimal()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        ObserveDecimals(Observer, Dataset(Report::"NB RCS Typed Dotted"));
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

codeunit 51114 "NB RCS Typed Undotted Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Seed()
    var
        Row: Record "NB RCS Row";
        Line: Record "NB RCS Line";
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
        Row: Record "NB RCS Row";
        Child: Record "NB RCS Child";
        Line: Record "NB RCS Line";
    begin
        Row.DeleteAll();
        Child.DeleteAll();
        Line.DeleteAll();
    end;

    // The trimmed lines of the SaveAs Xml dataset, each in brackets.
    local procedure Dataset(ReportId: Integer): Text
    var
        Buffer: Record "NB RCS Buffer";
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

    local procedure ObserveLabels(var Observer: Codeunit "NB RCS Observer"; Xml: Text)
    begin
        Observer.Observe('labels head', Pick(Xml, '<Label name="H'));
        Observer.Observe('labels code int date text', Pick(Xml, '<Label name="LOrderNo') + Pick(Xml, '<Label name="LLineNo') + Pick(Xml, '<Label name="LDescription') + Pick(Xml, '<Label name="LStartingDate') + Pick(Xml, '<Label name="LBinCode') + Pick(Xml, '<Label name="LSeq'));
        Observer.Observe('labels enum option', Pick(Xml, '<Label name="LStatus') + Pick(Xml, '<Label name="LKind') + Pick(Xml, '<Label name="LBareStatus'));
        Observer.Observe('labels decimal', Pick(Xml, '<Label name="LQuantity') + Pick(Xml, '<Label name="LUnitCost') + Pick(Xml, '<Label name="LCostAmount') + Pick(Xml, '<Label name="LZeroFmt') + Pick(Xml, '<Label name="LBareQuantity'));
    end;

    local procedure ObserveCodeIntDate(var Observer: Codeunit "NB RCS Observer"; Xml: Text)
    begin
        Observer.Observe('LOrderNo', Pick(Xml, '<Column name="LOrderNo"'));
        Observer.Observe('LLineNo', Pick(Xml, '<Column name="LLineNo"'));
        Observer.Observe('LDescription', Pick(Xml, '<Column name="LDescription"'));
        Observer.Observe('LStartingDate', Pick(Xml, '<Column name="LStartingDate"'));
        Observer.Observe('LStartingDateNoProp', Pick(Xml, '<Column name="LStartingDateNoProp"'));
        Observer.Observe('LBinCode', Pick(Xml, '<Column name="LBinCode"'));
        Observer.Observe('LSeq', Pick(Xml, '<Column name="LSeq"'));
    end;

    local procedure ObserveEnumOption(var Observer: Codeunit "NB RCS Observer"; Xml: Text)
    begin
        Observer.Observe('LStatus', Pick(Xml, '<Column name="LStatus"'));
        Observer.Observe('LStatusNoProp', Pick(Xml, '<Column name="LStatusNoProp"'));
        Observer.Observe('LBareStatus', Pick(Xml, '<Column name="LBareStatus"'));
        Observer.Observe('LKind', Pick(Xml, '<Column name="LKind"'));
    end;

    local procedure ObserveDecimals(var Observer: Codeunit "NB RCS Observer"; Xml: Text)
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
    procedure TypedUndotted_Labels()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        ObserveLabels(Observer, Dataset(Report::"NB RCS Typed Undotted"));
        Observer.Expect('labels head', '[<Label name="HNameCaption">DQ Name cap</Label>]');
        Observer.Expect('labels code int date text', '[<Label name="LOrderNoCaption">DQ Order No. cap</Label>][<Label name="LLineNoCaption">DQ Line No. cap</Label>][<Label name="LDescriptionCaption">DQ Description cap</Label>][<Label name="LStartingDateCaption">DQ Starting Date cap</Label>][<Label name="LBinCodeCaption">Bin Code</Label>][<Label name="LSeqCaption">Seq</Label>]');
        Observer.Expect('labels enum option', '[<Label name="LStatusCaption">DQ Status cap</Label>][<Label name="LKindCaption">DQ Kind cap</Label>][<Label name="LBareStatusCaption">DQ Status cap</Label>]');
        Observer.Expect('labels decimal', '[<Label name="LQuantityCaption">DQ Quantity cap</Label>][<Label name="LUnitCostCaption">DQ Unit Cost cap</Label>][<Label name="LCostAmountCaption">DQ Cost Amount cap</Label>][<Label name="LZeroFmtCaption">DQ ZeroFmt cap</Label>][<Label name="LBareQuantityCaption">DQ Quantity cap</Label>]');
        Observer.Verify();
    end;

    [Test]
    procedure TypedUndotted_Columns_CodeIntegerDate()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        ObserveCodeIntDate(Observer, Dataset(Report::"NB RCS Typed Undotted"));
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
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        ObserveEnumOption(Observer, Dataset(Report::"NB RCS Typed Undotted"));
        Observer.Expect('LStatus', '[<Column name="LStatus">DQ Released cap</Column>][<Column name="LStatus">DQ Simulated cap</Column>]');
        Observer.Expect('LStatusNoProp', '[<Column name="LStatusNoProp">DQ Released cap</Column>][<Column name="LStatusNoProp">DQ Simulated cap</Column>]');
        Observer.Expect('LBareStatus', '[<Column name="LBareStatus">DQ Released cap</Column>][<Column name="LBareStatus">DQ Simulated cap</Column>]');
        Observer.Expect('LKind', '[<Column name="LKind">DQ Open cap</Column>][<Column name="LKind"> </Column>]');
        Observer.Verify();
    end;

    [Test]
    procedure TypedUndotted_Columns_Decimal()
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        ObserveDecimals(Observer, Dataset(Report::"NB RCS Typed Undotted"));
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
