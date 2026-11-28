// Neighbors of the three column shapes this change serves (al-sema reports/column_source.rs): each test flips one fact of a
// measured cell of corpus PR 1652 (DataItemNamesDottedQualifier.al at cf6884b9) or of the pinned bare column
// (reports/dataset/ReportColumnFormatsHeaderRest.al:25, :46). The expected value is main's answer where the gate keeps it, and
// head's where the gate admits a fact the cell does not vary (an ASSUMED row in differences.tsv).

table 51120 "NB RCS N Row"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { Caption = 'N Id cap'; }
        field(2; AmtPlaces; Decimal) { Caption = 'N AmtPlaces cap'; AutoFormatType = 1; DecimalPlaces = 0 : 3; }
        field(3; AmtPlaces5; Decimal) { Caption = 'N AmtPlaces5 cap'; AutoFormatType = 1; DecimalPlaces = 0 : 5; }
        field(4; AmtPlacesNoCap; Decimal) { AutoFormatType = 1; DecimalPlaces = 0 : 3; }
        field(5; Qty; Decimal) { Caption = 'N Qty cap'; DecimalPlaces = 0 : 3; }
        field(6; Qty5; Decimal) { Caption = 'N Qty5 cap'; DecimalPlaces = 0 : 5; }
        field(7; Amt; Decimal) { Caption = 'N Amt cap'; AutoFormatType = 1; }
        field(8; Kind; Option) { Caption = 'N Kind cap'; OptionMembers = " ",Open,Closed; OptionCaption = ' ,N Open cap,N Closed cap'; }
        field(9; KindNoCap; Option) { OptionMembers = " ",Open,Closed; OptionCaption = ' ,N Open cap,N Closed cap'; }
        field(10; KindEmptyCap; Option) { Caption = 'N KindEmptyCap cap'; OptionMembers = " ",Open,Closed; OptionCaption = ' ,,N Closed cap'; }
        field(11; KindShortCap; Option) { Caption = 'N KindShortCap cap'; OptionMembers = " ",Open,Closed; OptionCaption = ' ,N Open cap'; }
        field(12; RowId; Integer) { }
        field(13; KindExt; Option) { Caption = 'N KindExt cap'; OptionMembers = " ",Open,Closed; OptionCaption = ' ,N Open cap,N Closed cap'; }
        field(14; KindExtMl; Option) { Caption = 'N KindExtMl cap'; OptionMembers = " ",Open,Closed; OptionCaption = ' ,N Open cap,N Closed cap'; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// A child table without a Decimal field: the probe's child table has one with DecimalPlaces 0:4 (:55).
table 51121 "NB RCS N Kid"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; RowId; Integer) { }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// A child table with a Decimal field with DecimalPlaces 0:4 and a Caption, as the probe's (:55), at another field number (no. 3, not
// 4) and without the Text field Note.
table 51122 "NB RCS N Kid4"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; RowId; Integer) { }
        field(3; Price; Decimal) { Caption = 'N Price cap'; DecimalPlaces = 0 : 4; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// A child table whose only Decimal field has DecimalPlaces 0:5 (the probe's has 0:4).
table 51127 "NB RCS N Kid5"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; Id; Integer) { }
        field(2; RowId; Integer) { }
        field(3; Price; Decimal) { DecimalPlaces = 0 : 5; }
    }

    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

// Own-item columns: the AutoFormatType 1 + DecimalPlaces gates and the Option gate.
report 51123 "NB RCS N Own"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(NRow; "NB RCS N Row")
        {
            column(QualPlaces5; NRow.AmtPlaces5) { IncludeCaption = true; }
            column(QualNoProp; NRow.AmtPlaces) { }
            column(QualNoCap; NRow.AmtPlacesNoCap) { IncludeCaption = true; }
            column(QualOtherCase; nrow.AmtPlaces) { IncludeCaption = true; }
            column(QualFieldOtherCase; NRow.amtplaces) { IncludeCaption = true; }
            column(BareWithCap; AmtPlaces) { }
            column(BareIncludeCaption; AmtPlacesNoCap) { IncludeCaption = true; }
            column(BarePinned; AmtPlacesNoCap) { }
            column(KindNoProp; NRow.Kind) { }
            column(KindNoCap; NRow.KindNoCap) { IncludeCaption = true; }
            column(KindEmptyCap; NRow.KindEmptyCap) { IncludeCaption = true; }
            column(KindShortCap; NRow.KindShortCap) { IncludeCaption = true; }
            column(KindOtherCase; nrow.Kind) { IncludeCaption = true; }
        }
    }
}

// The measured Option cell (:428), alone in its report: a report run is flagged for each measured column it writes.
report 51129 "NB RCS N Kind"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(NRow; "NB RCS N Row")
        {
            column(KindQual; NRow.Kind) { IncludeCaption = true; }
        }
    }
}

// The measured parent columns (:127-128) and the parent's own columns (:112-113), at other field numbers: parent Qty no. 5 and Amt
// no. 7, child Price no. 3 (the cell: 3, 4 and 4, cell P6).
report 51130 "NB RCS N Parent Hit"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(PRow; "NB RCS N Row")
        {
            column(HQty; PRow.Qty) { IncludeCaption = true; }
            column(HAmt; PRow.Amt) { IncludeCaption = true; }

            dataitem(PKid4; "NB RCS N Kid4")
            {
                DataItemLink = RowId = field(Id);

                column(K4Qty; PRow.Qty) { IncludeCaption = true; }
                column(K4Amt; PRow.Amt) { IncludeCaption = true; }
                column(K4Price; PKid4.Price) { IncludeCaption = true; }
            }
        }
    }
}

// Child columns over a parent Decimal field: the probe's shape (`Parent.Field`, a linked child, IncludeCaption) with one fact
// flipped per column, and the child tables the probe does not vary.
report 51124 "NB RCS N Parent"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(PRow; "NB RCS N Row")
        {
            dataitem(PKid4; "NB RCS N Kid4")
            {
                DataItemLink = RowId = field(Id);

                column(K4Qty5; PRow.Qty5) { IncludeCaption = true; }
                column(K4AmtPlaces; PRow.AmtPlaces) { IncludeCaption = true; }
                column(K4QtyNoProp; PRow.Qty) { }
                column(K4QtyOtherCase; prow.Qty) { IncludeCaption = true; }
                column(K4Price; PKid4.Price) { IncludeCaption = true; }
            }
            dataitem(PKid4NoCol; "NB RCS N Kid4")
            {
                DataItemLink = RowId = field(Id);

                column(NCAmt; PRow.Amt) { IncludeCaption = true; }
            }
            dataitem(PKid5; "NB RCS N Kid5")
            {
                DataItemLink = RowId = field(Id);

                column(K5Amt; PRow.Amt) { IncludeCaption = true; }
                column(K5Price; PKid5.Price) { IncludeCaption = true; }
            }
            dataitem(PKid; "NB RCS N Kid")
            {
                DataItemLink = RowId = field(Id);

                column(KAmt; PRow.Amt) { IncludeCaption = true; }
                column(KQty; PRow.Qty) { IncludeCaption = true; }
            }
            dataitem(PSame; "NB RCS N Row")
            {
                DataItemLink = RowId = field(Id);

                column(SAmt; PRow.Amt) { IncludeCaption = true; }
            }
            dataitem(PUnlinked; "NB RCS N Kid4")
            {
                column(UAmt; PRow.Amt) { IncludeCaption = true; }
                column(UPrice; PUnlinked.Price) { IncludeCaption = true; }
            }
        }
    }
}

// A parent that is itself nested, and a grandchild column.
report 51125 "NB RCS N Deep"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(DTop; "NB RCS N Kid")
        {
            dataitem(DRow; "NB RCS N Row")
            {
                DataItemLink = Id = field(RowId);

                dataitem(DKid4; "NB RCS N Kid4")
                {
                    DataItemLink = RowId = field(Id);

                    column(NestedParentAmt; DRow.Amt) { IncludeCaption = true; }
                    column(NestedPrice; DKid4.Price) { IncludeCaption = true; }
                }
            }
        }
    }
}

// A grandchild column over the top item's field.
report 51128 "NB RCS N Grand"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(GRow; "NB RCS N Row")
        {
            dataitem(GKid4; "NB RCS N Kid4")
            {
                DataItemLink = RowId = field(Id);

                column(GKPrice; GKid4.Price) { IncludeCaption = true; }

                dataitem(GGrand; "NB RCS N Kid4")
                {
                    DataItemLink = RowId = field(RowId);

                    column(GrandAmt; GRow.Amt) { IncludeCaption = true; }
                    column(GrandPrice; GGrand.Price) { IncludeCaption = true; }
                }
            }
        }
    }
}

codeunit 51126 "NB RCS N Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Seed()
    var
        Row: Record "NB RCS N Row";
        Kid: Record "NB RCS N Kid";
        Kid4: Record "NB RCS N Kid4";
        Kid5: Record "NB RCS N Kid5";
    begin
        Row.DeleteAll();
        Kid.DeleteAll();
        Kid4.DeleteAll();
        Row.Id := 1;
        Row.AmtPlaces := 1234.5;
        Row.AmtPlaces5 := 1234.5;
        Row.AmtPlacesNoCap := 1234.5;
        Row.Qty := 2.5;
        Row.Qty5 := 2.5;
        Row.Amt := 1234.5;
        Row.Kind := Row.Kind::Open;
        Row.KindNoCap := Row.KindNoCap::Open;
        Row.KindEmptyCap := Row.KindEmptyCap::Open;
        Row.KindShortCap := Row.KindShortCap::Open;
        Row.KindExt := Row.KindExt::Open;
        Row.KindExtMl := Row.KindExtMl::Open;
        Row.RowId := 1;
        Row.Insert();
        Kid.Id := 1;
        Kid.RowId := 1;
        Kid.Insert();
        Kid4.Id := 1;
        Kid4.RowId := 1;
        Kid4.Insert();
        Kid5.DeleteAll();
        Kid5.Id := 1;
        Kid5.RowId := 1;
        Kid5.Insert();
        SeedCell();
    end;

    local procedure SeedCell()
    var
        PRow: Record "NB RCS N PRow";
        KidC: Record "NB RCS N KidC";
        KidT: Record "NB RCS N KidT";
        KidN: Record "NB RCS N KidN";
        Kid6: Record "NB RCS N Kid6";
    begin
        PRow.DeleteAll();
        PRow.Id := 1;
        PRow.Name := 'Row one';
        PRow.Qty := 2.5;
        PRow.Amt := 1234.5;
        PRow.Insert();
        KidC.DeleteAll();
        KidC.Id := 1;
        KidC.RowId := 1;
        KidC.Note := 'C1';
        KidC.Price := 0.5;
        KidC.Insert();
        KidT.DeleteAll();
        KidT.Id := 1;
        KidT.RowId := 1;
        KidT.Insert();
        KidN.DeleteAll();
        KidN.Id := 1;
        KidN.RowId := 1;
        KidN.Insert();
        Kid6.DeleteAll();
        Kid6.Id := 1;
        Kid6.RowId := 1;
        Kid6.Insert();
    end;

    // The trimmed Column line of the SaveAs Xml dataset of report ReportId whose name is Name (the first one).
    local procedure Column(ReportId: Integer; Name: Text): Text
    var
        Buffer: Record "NB RCS Buffer";
        OutStr: OutStream;
        InStr: InStream;
        Line: Text;
    begin
        Buffer.Data.CreateOutStream(OutStr);
        Report.SaveAs(ReportId, '', ReportFormat::Xml, OutStr);
        Buffer.Insert();
        Buffer.CalcFields(Data);
        Buffer.Data.CreateInStream(InStr, TextEncoding::UTF8);
        while not InStr.EOS() do begin
            InStr.ReadText(Line);
            if Line.Trim().StartsWith('<Column name="' + Name + '"') then
                exit(Line.Trim());
        end;
        exit('<none>');
    end;

    local procedure Check(ReportId: Integer; Name: Text; Expected: Text)
    var
        Observer: Codeunit "NB RCS Observer";
    begin
        Seed();
        Observer.Observe(Name, Column(ReportId, Name));
        Observer.Expect(Name, Expected);
        Observer.Verify();
    end;

    [Test]
    procedure AmountPlaces_FieldPlaces05()
    begin
        Check(Report::"NB RCS N Own", 'QualPlaces5', '<Column name="QualPlaces5" decimalformatter="#,##0.#####">1234.5</Column>');
    end;

    [Test]
    procedure AmountPlaces_ColumnWithoutProperty()
    begin
        Check(Report::"NB RCS N Own", 'QualNoProp', '<Column name="QualNoProp" decimalformatter="#,##0.###">1234.5</Column>');
    end;

    [Test]
    procedure AmountPlaces_FieldWithoutCaption()
    begin
        Check(Report::"NB RCS N Own", 'QualNoCap', '<Column name="QualNoCap" decimalformatter="#,##0.###">1234.5</Column>');
    end;

    [Test]
    procedure AmountPlaces_QualifierOtherCase()
    begin
        Check(Report::"NB RCS N Own", 'QualOtherCase', '<Column name="QualOtherCase" decimalformatter="#,##0.###">1234.5</Column>');
    end;

    [Test]
    procedure AmountPlaces_FieldOtherCase()
    begin
        Check(Report::"NB RCS N Own", 'QualFieldOtherCase', '<Column name="QualFieldOtherCase" decimalformatter="#,##0.###">1234.5</Column>');
    end;

    [Test]
    procedure AmountPlaces_BareFieldWithCaption()
    begin
        Check(Report::"NB RCS N Own", 'BareWithCap', '<Column name="BareWithCap" decimalformatter="#,##0.###">1234.5</Column>');
    end;

    [Test]
    procedure AmountPlaces_BareIncludeCaption()
    begin
        Check(Report::"NB RCS N Own", 'BareIncludeCaption', '<Column name="BareIncludeCaption" decimalformatter="#,##0.###">1234.5</Column>');
    end;

    // The pinned bare cell (ReportColumnFormatsHeaderRest.al:25, :46, :123): `#,##0.00` from BC 28.
    [Test]
    procedure AmountPlaces_BarePinned()
    begin
        Check(Report::"NB RCS N Own", 'BarePinned', '<Column name="BarePinned" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    // The measured cell (:428, :678) next to its neighbors.
    [Test]
    procedure Option_Qualified()
    begin
        Check(Report::"NB RCS N Kind", 'KindQual', '<Column name="KindQual">N Open cap</Column>');
    end;

    [Test]
    procedure Option_ColumnWithoutProperty()
    begin
        Check(Report::"NB RCS N Own", 'KindNoProp', '<Column name="KindNoProp">Open</Column>');
    end;

    [Test]
    procedure Option_FieldWithoutCaption()
    begin
        Check(Report::"NB RCS N Own", 'KindNoCap', '<Column name="KindNoCap">Open</Column>');
    end;

    [Test]
    procedure Option_EmptyMemberCaption()
    begin
        Check(Report::"NB RCS N Own", 'KindEmptyCap', '<Column name="KindEmptyCap">Open</Column>');
    end;

    [Test]
    procedure Option_FewerCaptionsThanMembers()
    begin
        Check(Report::"NB RCS N Own", 'KindShortCap', '<Column name="KindShortCap">Open</Column>');
    end;

    [Test]
    procedure Option_QualifierOtherCase()
    begin
        Check(Report::"NB RCS N Own", 'KindOtherCase', '<Column name="KindOtherCase">Open</Column>');
    end;

    // A tableextension sets the OptionCaption or OptionCaptionML of the measured Option cell's field.
    [Test]
    procedure Option_ExtensionOptionCaption()
    begin
        Check(Report::"NB RCS N Kind Ext", 'KindExtQual', '<Column name="KindExtQual">Open</Column>');
    end;

    [Test]
    procedure Option_ExtensionOptionCaptionMl()
    begin
        Check(Report::"NB RCS N Kind Ext", 'KindExtMlQual', '<Column name="KindExtMlQual">Open</Column>');
    end;

    // The measured cells (:127-128, :300) in the probe's layout (NeighborsCell.al).
    [Test]
    procedure Parent_Places03()
    begin
        Check(Report::"NB RCS N Cell", 'CParentQty', '<Column name="CParentQty" decimalformatter="#,##0.###">2.5</Column>');
    end;

    [Test]
    procedure Parent_AutoFormatType1()
    begin
        Check(Report::"NB RCS N Cell", 'CParentAmt', '<Column name="CParentAmt" decimalformatter="#,##0.####">1234.5</Column>');
    end;

    // The same columns where the parent fields' numbers meet no child field (Qty no. 5, Amt no. 7; Price no. 3).
    [Test]
    procedure Parent_Places03_OtherFieldNumbers()
    begin
        Check(Report::"NB RCS N Parent Hit", 'K4Qty', '<Column name="K4Qty" decimalformatter="#,##0.00">2.5</Column>');
    end;

    [Test]
    procedure Parent_AutoFormatType1_OtherFieldNumbers()
    begin
        Check(Report::"NB RCS N Parent Hit", 'K4Amt', '<Column name="K4Amt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    // Cell P7: the child field with Amt's number is the Text field; cell P8's neighbor: the one with Qty's number the Decimal 0:4.
    [Test]
    procedure Parent_AutoFormatType1_ChildTextAtSameNumber()
    begin
        Check(Report::"NB RCS N Cell Swapped", 'TParentAmt', '<Column name="TParentAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    [Test]
    procedure Parent_Places03_ChildDecimalAtSameNumber()
    begin
        Check(Report::"NB RCS N Cell Swapped", 'TParentQty', '<Column name="TParentQty" decimalformatter="#,##0.00">2.5</Column>');
    end;

    // Cell P9: the parent has no column of its own over the field.
    [Test]
    procedure Parent_AutoFormatType1_NoParentColumn()
    begin
        Check(Report::"NB RCS N Cell No Own", 'OParentAmt', '<Column name="OParentAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    [Test]
    procedure Parent_Places03_NoParentColumn()
    begin
        Check(Report::"NB RCS N Cell No Own", 'OParentQty', '<Column name="OParentQty" decimalformatter="#,##0.00">2.5</Column>');
    end;

    // The parent's own columns state no property (the cell's state IncludeCaption = true).
    [Test]
    procedure Parent_AutoFormatType1_ParentColumnWithoutProperty()
    begin
        Check(Report::"NB RCS N Cell Own NoProp", 'PParentAmt', '<Column name="PParentAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    // The child's Note and Price state no Caption.
    [Test]
    procedure Parent_AutoFormatType1_ChildPriceWithoutCaption()
    begin
        Check(Report::"NB RCS N Cell NoCap", 'NParentAmt', '<Column name="NParentAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    [Test]
    procedure Parent_Places03_ChildNoteWithoutCaption()
    begin
        Check(Report::"NB RCS N Cell NoCap", 'NParentQty', '<Column name="NParentQty" decimalformatter="#,##0.00">2.5</Column>');
    end;

    // The child item's name holds a dot, the parent's does not.
    [Test]
    procedure Parent_AutoFormatType1_DottedChildName()
    begin
        Check(Report::"NB RCS N Cell Dotted Kid", 'KParentAmt', '<Column name="KParentAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    // Cell P6: Price at a number no parent field uses. Amt meets no child field and keeps main's rule; Qty still meets the
    // Text field Note at its number and is served (the run is flagged for it).
    [Test]
    procedure Parent_AutoFormatType1_PriceAtOtherNumber()
    begin
        Check(Report::"NB RCS N Cell P6", 'SParentAmt', '<Column name="SParentAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    [Test]
    procedure Parent_Places03_PriceAtOtherNumber()
    begin
        Check(Report::"NB RCS N Cell P6", 'SParentQty', '<Column name="SParentQty" decimalformatter="#,##0.###">2.5</Column>');
    end;

    [Test]
    procedure Parent_Places05()
    begin
        Check(Report::"NB RCS N Parent", 'K4Qty5', '<Column name="K4Qty5" decimalformatter="#,##0.00">2.5</Column>');
    end;

    [Test]
    procedure Parent_AutoFormatType1WithPlaces()
    begin
        Check(Report::"NB RCS N Parent", 'K4AmtPlaces', '<Column name="K4AmtPlaces" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    [Test]
    procedure Parent_ColumnWithoutProperty()
    begin
        Check(Report::"NB RCS N Parent", 'K4QtyNoProp', '<Column name="K4QtyNoProp" decimalformatter="#,##0.00">2.5</Column>');
    end;

    [Test]
    procedure Parent_QualifierOtherCase()
    begin
        Check(Report::"NB RCS N Parent", 'K4QtyOtherCase', '<Column name="K4QtyOtherCase" decimalformatter="#,##0.00">2.5</Column>');
    end;

    // The child table has no Decimal field (the probe's has one with DecimalPlaces 0:4).
    [Test]
    procedure Parent_ChildTableWithoutPlaces04Field()
    begin
        Check(Report::"NB RCS N Parent", 'KAmt', '<Column name="KAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    [Test]
    procedure Parent_Places03ChildTableWithoutPlaces04Field()
    begin
        Check(Report::"NB RCS N Parent", 'KQty', '<Column name="KQty" decimalformatter="#,##0.00">2.5</Column>');
    end;

    // The child item reads the parent's own table.
    [Test]
    procedure Parent_ChildOverTheParentsTable()
    begin
        Check(Report::"NB RCS N Parent", 'SAmt', '<Column name="SAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    // The child item has no column over its DecimalPlaces 0:4 field (the probe's has CPrice :130).
    [Test]
    procedure Parent_ChildWithoutPriceColumn()
    begin
        Check(Report::"NB RCS N Parent", 'NCAmt', '<Column name="NCAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    [Test]
    procedure Parent_ChildPricePlaces05()
    begin
        Check(Report::"NB RCS N Parent", 'K5Amt', '<Column name="K5Amt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    [Test]
    procedure Parent_UnlinkedChild()
    begin
        Check(Report::"NB RCS N Parent", 'UAmt', '<Column name="UAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    [Test]
    procedure Parent_ParentNotTopLevel()
    begin
        Check(Report::"NB RCS N Deep", 'NestedParentAmt', '<Column name="NestedParentAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;

    [Test]
    procedure Parent_Grandchild()
    begin
        Check(Report::"NB RCS N Grand", 'GrandAmt', '<Column name="GrandAmt" decimalformatter="#,##0.00">1234.5</Column>');
    end;
}
