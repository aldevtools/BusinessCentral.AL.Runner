// Control: DataItemNamesDottedQualifier.al:100-136 (report 75803) with the same columns, undotted names (:274, :286, :300):
// main and head give the same answer.
report 50961 "NB DQ Undotted"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(DqRow; "NB DQ Row")
        {
            column(DName; DqRow.Name) { IncludeCaption = true; }
            column(DQty; DqRow.Qty) { IncludeCaption = true; }
            column(DAmt; DqRow.Amt) { IncludeCaption = true; }
            column(DUnitAmt; DqRow."Unit Amt.") { IncludeCaption = true; }
            column(DPlain; DqRow.Plain) { IncludeCaption = true; }
            column(DQtyOwnPlaces; DqRow.Qty) { DecimalPlaces = 4 : 4; }
            column(DBareQty; Qty) { IncludeCaption = true; }
            column(DUpper; DqRow.Name.ToUpper()) { }

            dataitem(DqChild; "NB DQ Child")
            {
                DataItemLink = RowId = field(Id);

                column(CParentName; DqRow.Name) { IncludeCaption = true; }
                column(CNote; DqChild.Note) { IncludeCaption = true; }
                column(CPrice; DqChild.Price) { IncludeCaption = true; }
            }
        }
    }
}

codeunit 50961 "NB DQ Control Tests"
{
    Subtype = Test;
    TestPermissions = Disabled;

    var
        Lib: Codeunit "NB DQ Lib";

    // :224
    [Test]
    procedure Undotted_Labels_OwnAndChild()
    begin
        Lib.Seed();
        Lib.Check('[<Label name="DNameCaption">DQ Name cap</Label>][<Label name="DQtyCaption">DQ Qty cap</Label>][<Label name="DAmtCaption">DQ Amt cap</Label>][<Label name="DUnitAmtCaption">Unit Amt.</Label>][<Label name="DPlainCaption">Plain</Label>][<Label name="DBareQtyCaption">DQ Qty cap</Label>][<Label name="CParentNameCaption">DQ Name cap</Label>][<Label name="CNoteCaption">DQ Child Note cap</Label>][<Label name="CPriceCaption">DQ Child Price cap</Label>]',
            Lib.Lines(Report::"NB DQ Undotted", '<Label '));
    end;

    // :236 and :238 (the same on every version for these columns).
    [Test]
    procedure Undotted_Columns_OwnItem()
    begin
        Lib.Seed();
        Lib.Check('[<Column name="DName">Row one</Column>][<Column name="DQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="DAmt" decimalformatter="#,##0.00">1234.5</Column>][<Column name="DUnitAmt" decimalformatter="#,##0.#">3.25</Column>][<Column name="DPlain" decimalformatter="#,##0.00">7</Column>][<Column name="DQtyOwnPlaces" decimalformatter="#,##0.0000">2.5</Column>][<Column name="DBareQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="DUpper">ROW ONE</Column>]',
            Lib.Lines(Report::"NB DQ Undotted", '<Column name="D'));
    end;

    // :250
    [Test]
    procedure Undotted_Columns_ChildItem()
    begin
        Lib.Seed();
        Lib.Check('[<Column name="CParentName">Row one</Column>][<Column name="CNote">C1</Column>][<Column name="CPrice" decimalformatter="#,##0.####">0.5</Column>]',
            Lib.Lines(Report::"NB DQ Undotted", '<Column name="C'));
    end;

}
