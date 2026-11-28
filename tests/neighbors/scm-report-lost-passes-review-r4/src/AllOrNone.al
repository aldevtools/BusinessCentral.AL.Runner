// All or none: a report serves its dotted columns only when the gate admits each column qualified with its own item's dotted
// name, and no reportextension that changes the run declares a dotted column. An expected text is the probe's (corpus PR 1652
// part 2 at cf6884b9) or the undotted rule, not a pin.

// The typed report of the probe without its Option column (:383-398 without :391): every dotted column is admitted.
report 51005 "NB DQ2 Typed No Option"
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
            column(LStartingDate; "Dq. Line"."Starting Date") { IncludeCaption = true; }
            column(LQuantity; "Dq. Line".Quantity) { IncludeCaption = true; }
            column(LUnitCost; "Dq. Line"."Unit Cost") { IncludeCaption = true; }
            column(LCostAmount; "Dq. Line"."Cost Amount") { IncludeCaption = true; }
            column(LSeq; "Dq. Line".Seq) { IncludeCaption = true; }
            column(LStatusNoProp; "Dq. Line".Status) { }
            column(LQuantityNoProp; "Dq. Line".Quantity) { }
            column(LStartingDateNoProp; "Dq. Line"."Starting Date") { }
        }
    }
}

codeunit 51005 "NB DQ2 Typed No Option T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    local procedure Seed()
    var
        Row: Record "NB DQ2 Row";
        Line: Record "NB DQ2 Line";
    begin
        Row.DeleteAll();
        Line.DeleteAll();
        Row.Id := 1;
        Row.Name := 'Head one';
        Row.Insert();
        Line."Order No." := 'PO-1';
        Line."Line No." := 10000;
        Line.Status := Line.Status::Released;
        Line."Starting Date" := DMY2Date(15, 1, 2026);
        Line.Quantity := 12.34567;
        Line."Unit Cost" := 1.23456;
        Line."Cost Amount" := 1234.5;
        Line.Seq := 7;
        Line.Insert();
    end;

    [Test]
    procedure AllAdmitted_Labels()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Seed();
        Lib.Check('[<Label name="HNameCaption">DQ Name cap</Label>][<Label name="LStatusCaption">DQ Status cap</Label>][<Label name="LOrderNoCaption">DQ Order No. cap</Label>][<Label name="LStartingDateCaption">DQ Starting Date cap</Label>][<Label name="LQuantityCaption">DQ Quantity cap</Label>][<Label name="LUnitCostCaption">DQ Unit Cost cap</Label>][<Label name="LCostAmountCaption">DQ Cost Amount cap</Label>][<Label name="LSeqCaption">Seq</Label>]', Lib.Lines(Report::"NB DQ2 Typed No Option", '<Label '));
    end;

    [Test]
    procedure AllAdmitted_EnumAndDateWithoutProperty()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Seed();
        Lib.Check('[<Column name="LStatusNoProp">DQ Released cap</Column>]', Lib.Lines(Report::"NB DQ2 Typed No Option", '<Column name="LStatusNoProp"'));
        Lib.Check('[<Column name="LStartingDateNoProp">1/15/2026</Column>]', Lib.Lines(Report::"NB DQ2 Typed No Option", '<Column name="LStartingDateNoProp"'));
    end;

    [Test]
    procedure AllAdmitted_Decimals()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Seed();
        Lib.Check('[<Column name="LQuantity" decimalformatter="#,##0.#####">12.34567</Column>]', Lib.Lines(Report::"NB DQ2 Typed No Option", '<Column name="LQuantity"'));
        Lib.Check('[<Column name="LQuantityNoProp" decimalformatter="#,##0.#####">12.34567</Column>]', Lib.Lines(Report::"NB DQ2 Typed No Option", '<Column name="LQuantityNoProp"'));
        Lib.Check('[<Column name="LUnitCost" decimalformatter="#,##0.00###">1.23456</Column>]', Lib.Lines(Report::"NB DQ2 Typed No Option", '<Column name="LUnitCost"'));
        Lib.Check('[<Column name="LCostAmount" decimalformatter="#,##0.00">1234.5</Column>]', Lib.Lines(Report::"NB DQ2 Typed No Option", '<Column name="LCostAmount"'));
    end;
}

// An admitted column and a rejected one (AutoFormatType 2 under a column without property) of the same item: both keep main's
// answer, so the IncludeCaption column is refused as on main.
report 51006 "NB DQ2 Mixed Same Item"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(Admitted; "Dq. X".Id) { IncludeCaption = true; }
            column(Rejected; "Dq. X"."Unit Cost") { }
        }
    }
}

codeunit 51006 "NB DQ2 Mixed Same Item T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure AllOrNone_RejectedColumnSameItem()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="AdmittedCaption">X Id cap</Label>]', Lib.Lines(Report::"NB DQ2 Mixed Same Item", '<Label '));
    end;
}

// An admitted column in one dotted item, a rejected one (an Option field) in another dotted item.
report 51007 "NB DQ2 Mixed Two Items"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(Admitted; "Dq. X".Id) { IncludeCaption = true; }
        }
        dataitem("Dq. Y"; "NB DQ2 Extra")
        {
            column(Rejected; "Dq. Y".OptNoOptionCaption) { IncludeCaption = true; }
        }
    }
}

codeunit 51007 "NB DQ2 Mixed Two Items T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure AllOrNone_RejectedColumnOtherItem()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="AdmittedCaption">X Id cap</Label>][<Label name="RejectedCaption">X Opt cap</Label>]', Lib.Lines(Report::"NB DQ2 Mixed Two Items", '<Label '));
    end;
}

// An admitted column in the report, a dotted column in a reportextension that adds to the dotted item.
report 51008 "NB DQ2 Ext Dotted Base"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(Admitted; "Dq. X".Id) { IncludeCaption = true; }
        }
    }
}

reportextension 51008 "NB DQ2 Ext Dotted" extends "NB DQ2 Ext Dotted Base"
{
    dataset
    {
        add("Dq. X")
        {
            column(Added; "Dq. X".Flag) { }
        }
    }
}

codeunit 51008 "NB DQ2 Ext Dotted T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure AllOrNone_ExtensionDottedColumn()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="AdmittedCaption">X Id cap</Label>]', Lib.Lines(Report::"NB DQ2 Ext Dotted Base", '<Label '));
    end;
}

// An admitted column of the dotted parent and a child column that reads the parent's Integer field through its dotted name: not a
// probe cell (the probe measures a Text field with a Caption, :90), so the report's dotted columns keep main's answer.
report 51009 "NB DQ2 Child Reads Admitted"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(Admitted; "Dq. X".Id) { IncludeCaption = true; }
            dataitem(Kid; "NB DQ2 Extra")
            {
                DataItemLinkReference = "Dq. X";
                DataItemLink = Id = field(Id);
                column(FromParent; "Dq. X".Id) { IncludeCaption = true; }
            }
        }
    }
}

codeunit 51009 "NB DQ2 Child Reads Admitted T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure AllOrNone_ChildReadsDottedParent()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="AdmittedCaption">X Id cap</Label>][<Label name="FromParentCaption">X Id cap</Label>]', Lib.Lines(Report::"NB DQ2 Child Reads Admitted", '<Label '));
        Lib.Check('[<Column name="Admitted">1</Column>][<Column name="FromParent">1</Column>]', Lib.Lines(Report::"NB DQ2 Child Reads Admitted", '<Column '));
    end;
}

// Probe part 1 at cf6884b9, :86-95: an admitted column of the dotted parent and child columns that read the parent's Decimal
// fields through its dotted name (:91-92). BC writes the parent field's formatter (:250); main's rule for another item's field
// writes the default. So the report's dotted columns keep main's answer (not run, as on main).
report 51010 "NB DQ2 Child Reads Decimal"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ2 Row")
        {
            column(DName; "Dq. Row".Name) { IncludeCaption = true; }

            dataitem("Dq. Child"; "NB DQ2 Child")
            {
                DataItemLink = RowId = field(Id);

                column(CParentQty; "Dq. Row".Qty) { IncludeCaption = true; }
                column(CParentAmt; "Dq. Row".Amt) { IncludeCaption = true; }
            }
        }
    }
}

codeunit 51010 "NB DQ2 Child Reads Decimal T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure AllOrNone_ChildReadsDottedParentDecimal()
    var
        Row: Record "NB DQ2 Row";
        Child: Record "NB DQ2 Child";
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Row.DeleteAll();
        Child.DeleteAll();
        Row.Id := 1;
        Row.Name := 'Row one';
        Row.Qty := 2.5;
        Row.Amt := 1234.5;
        Row.Insert();
        Child.Id := 10;
        Child.RowId := 1;
        Child.Insert();
        Lib.Check('[<Column name="CParentQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="CParentAmt" decimalformatter="#,##0.####">1234.5</Column>]', Lib.Lines(Report::"NB DQ2 Child Reads Decimal", '<Column name="CParent'));
    end;
}
