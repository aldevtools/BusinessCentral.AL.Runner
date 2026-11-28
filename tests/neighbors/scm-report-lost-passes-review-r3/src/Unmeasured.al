// Dotted shapes part 1 of corpus PR 1652 does not measure: head keeps main's answer (the qualifier names no field), so main
// and head give the same outcome. The `Part2_` tests are shapes part 2 measures (fields :348-359 at cf6884b9): head serves
// them through report-column-dotted-item-qualifier. The expected text is the undotted rule; it is not a pin. Each test has its
// own codeunit and report, so a refused run leaves nothing to the next test.

report 50952 "NB DQ U50952"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(X; "Dq. Row"."No.") { IncludeCaption = true; }
        }
    }
}

codeunit 50952 "NB DQ U50952 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Part2_CodeField_IncludeCaption()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">DQ No cap</Label>]', Lib.Lines(Report::"NB DQ U50952", '<Label '));
    end;
}

report 50953 "NB DQ U50953"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(X; "Dq. Row".Id) { IncludeCaption = true; }
        }
    }
}

codeunit 50953 "NB DQ U50953 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Part2_IntegerField_IncludeCaption()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">DQ Id cap</Label>]', Lib.Lines(Report::"NB DQ U50953", '<Label '));
    end;
}

report 50954 "NB DQ U50954"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(X; "Dq. Row".Status) { IncludeCaption = true; }
        }
    }
}

codeunit 50954 "NB DQ U50954 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Part2_EnumField_IncludeCaption()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">DQ Status cap</Label>]', Lib.Lines(Report::"NB DQ U50954", '<Label '));
    end;
}

report 50955 "NB DQ U50955"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(X; "Dq. Row"."Due Date") { IncludeCaption = true; }
        }
    }
}

codeunit 50955 "NB DQ U50955 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Part2_DateField_IncludeCaption()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">DQ Due cap</Label>]', Lib.Lines(Report::"NB DQ U50955", '<Label '));
    end;
}

report 50956 "NB DQ U50956"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(X; "Dq. Row".Expr) { IncludeCaption = true; }
        }
    }
}

codeunit 50956 "NB DQ U50956 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Part2_AutoFormatExpression_IncludeCaption()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">DQ Expr cap</Label>]', Lib.Lines(Report::"NB DQ U50956", '<Label '));
    end;
}

report 50957 "NB DQ U50957"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(X; "Dq. Row".Qty) {  }
        }
    }
}

codeunit 50957 "NB DQ U50957 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Unmeasured_DecimalNoProperty()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X" decimalformatter="#,##0.###">2.5</Column>]', Lib.Lines(Report::"NB DQ U50957", '<Column '));
    end;
}

report 50958 "NB DQ U50958"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(X; "Dq. Row".Qty) { IncludeCaption = true; DecimalPlaces = 4 : 4; }
        }
    }
}

codeunit 50958 "NB DQ U50958 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Unmeasured_IncludeCaptionAndDecimalPlaces()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">DQ Qty cap</Label>]', Lib.Lines(Report::"NB DQ U50958", '<Label '));
    end;
}

report 50959 "NB DQ U50959"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(X; "dq. row".Name) { IncludeCaption = true; }
        }
    }
}

codeunit 50959 "NB DQ U50959 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Unmeasured_QualifierOtherCase()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">DQ Name cap</Label>]', Lib.Lines(Report::"NB DQ U50959", '<Label '));
    end;
}

// A reportextension adds a column written with the base item's dotted name.
report 50960 "NB DQ Ext Base"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(BaseId; Id) { }
        }
    }
}

reportextension 50950 "NB DQ Ext" extends "NB DQ Ext Base"
{
    dataset
    {
        add("Dq. Row")
        {
            column(ExtName; "Dq. Row".Name) { IncludeCaption = true; }
        }
    }
}

codeunit 50960 "NB DQ Ext T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Unmeasured_ReportExtensionColumn()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="ExtNameCaption">DQ Name cap</Label>]', Lib.Lines(Report::"NB DQ Ext Base", '<Label '));
    end;
}

// Measured (DataItemNamesDottedQualifier.al:236, AutoFormatType 2 gives #,##0.00###), but the field formatter answers it by
// DecimalPlaces (#,##0.00, also undotted), so head keeps main's answer.
report 50962 "NB DQ U50962"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(X; "Dq. Row".Unit) { IncludeCaption = true; }
        }
    }
}

codeunit 50962 "NB DQ U50962 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Part2_UnitAmountType()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X" decimalformatter="#,##0.00###">1.23456</Column>]', Lib.Lines(Report::"NB DQ U50962", '<Column '));
    end;
}

// Measured (DataItemNamesDottedQualifier.al:236, BC 28 and 29: #,##0.00), but the field formatter answers DecimalPlaces
// (#,##0.###, also undotted), so head keeps main's answer.
report 50963 "NB DQ U50963"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(X; "Dq. Row".AmtPlaces) { IncludeCaption = true; }
        }
    }
}

codeunit 50963 "NB DQ U50963 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Measured_AmountTypeWithPlaces_KeepsMain()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X" decimalformatter="#,##0.00">1234.5</Column>]', Lib.Lines(Report::"NB DQ U50963", '<Column '));
    end;
}

// A child column reads Decimal fields of its dotted-name parent (DataItemNamesDottedQualifier.al:250, undotted :300): main's
// rule for a field of another data item, which this change does not touch. The runtime writes #,##0.00 for both, dotted
// and undotted alike, so main and head fail alike.
report 50964 "NB DQ U50964"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. Row"; "NB DQ Row")
        {
            column(PId; Id) { }
            dataitem("Dq. Child"; "NB DQ Child")
            {
                DataItemLink = RowId = field(Id);
                column(CParentQty; "Dq. Row".Qty) { IncludeCaption = true; }
                column(CParentAmt; "Dq. Row".Amt) { IncludeCaption = true; }
            }
        }
    }
}

codeunit 50964 "NB DQ U50964 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure MainRule_ChildReadsDottedParentDecimal()
    var
        Lib: Codeunit "NB DQ Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="CParentQty" decimalformatter="#,##0.###">2.5</Column>][<Column name="CParentAmt" decimalformatter="#,##0.####">1234.5</Column>]',
            Lib.Lines(Report::"NB DQ U50964", '<Column name="C'));
    end;
}
