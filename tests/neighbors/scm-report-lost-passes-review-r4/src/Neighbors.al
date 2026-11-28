// Neighbors of the shapes corpus PR 1652 measures (part 2): field facts the gates do not read, and shapes they do not
// admit. Each test has its own report and codeunit, so a refused run leaves nothing to the next test. An expected text is the
// undotted rule, not a pin.

enum 50983 "NB DQ2 Open Enum"
{
    Extensible = true;
    value(0; Alpha) { }
    value(1; Beta) { }
}

table 50984 "NB DQ2 Extra"
{
    fields
    {
        field(1; Id; Integer) { Caption = 'X Id cap'; }
        field(2; TextNoCap; Text[30]) { }
        field(3; DateNoCap; Date) { }
        field(4; EnumNoCap; Enum "NB DQ2 Status") { }
        field(5; OptNoCap; Option) { OptionMembers = A,B; }
        field(6; Flag; Boolean) { Caption = 'X Flag cap'; }
        field(7; ZeroPlaces3; Decimal) { AutoFormatType = 0; Caption = 'X ZeroPlaces3 cap'; DecimalPlaces = 0 : 3; }
        field(8; ZeroExpr; Decimal) { AutoFormatType = 0; AutoFormatExpression = ''; Caption = 'X ZeroExpr cap'; }
        field(9; UnitNoCap; Decimal) { AutoFormatType = 2; }
        field(10; UnitPlaces; Decimal) { AutoFormatType = 2; Caption = 'X UnitPlaces cap'; DecimalPlaces = 0 : 3; }
        field(11; OpenEnum; Enum "NB DQ2 Open Enum") { Caption = 'X OpenEnum cap'; }
        field(12; OptNoOptionCaption; Option) { Caption = 'X Opt cap'; OptionMembers = " ",Open; }
        field(13; "Unit Cost"; Decimal) { AutoFormatType = 2; AutoFormatExpression = ''; Caption = 'X Unit Cost cap'; }
        field(14; AmtPlacesExpr; Decimal) { AutoFormatType = 1; AutoFormatExpression = ''; Caption = 'X AmtPlacesExpr cap'; DecimalPlaces = 0 : 3; }
    }
    keys
    {
        key(PK; Id) { Clustered = true; }
    }
}

codeunit 50985 "NB DQ2 Lib"
{
    procedure Seed()
    var
        X: Record "NB DQ2 Extra";
    begin
        X.DeleteAll();
        X.Id := 1;
        X.TextNoCap := 'T1';
        X.DateNoCap := DMY2Date(15, 1, 2026);
        X.Flag := true;
        X.ZeroPlaces3 := 2.5;
        X.ZeroExpr := 2.5;
        X.UnitNoCap := 1.23456;
        X.UnitPlaces := 1.23456;
        X.OpenEnum := X.OpenEnum::Beta;
        X.OptNoOptionCaption := X.OptNoOptionCaption::Open;
        X."Unit Cost" := 1.23456;
        X.AmtPlacesExpr := 1234.5;
        X.Insert();
    end;

    procedure Lines(ReportId: Integer; Keep: Text): Text
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
            if Line.Trim().StartsWith(Keep) then
                Result += '[' + Line.Trim() + ']';
        end;
        exit(Result);
    end;

    procedure Check(Expected: Text; Actual: Text)
    begin
        if Expected <> Actual then
            Error('expected %1, got %2', Expected, Actual);
    end;
}

// Text field without Caption: not measured (dotted_qualifier_measured reads Caption for Text).
report 50986 "NB DQ2 R50986"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".TextNoCap) { IncludeCaption = true; }
        }
    }
}

codeunit 50986 "NB DQ2 R50986 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_TextWithoutCaption()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">TextNoCap</Label>]', Lib.Lines(Report::"NB DQ2 R50986", '<Label '));
    end;
}

// Date field without Caption: not measured.
report 50987 "NB DQ2 R50987"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".DateNoCap) { IncludeCaption = true; }
        }
    }
}

codeunit 50987 "NB DQ2 R50987 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_DateWithoutCaption()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">DateNoCap</Label>]', Lib.Lines(Report::"NB DQ2 R50987", '<Label '));
    end;
}

// Enum field without Caption: not measured.
report 50988 "NB DQ2 R50988"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".EnumNoCap) { IncludeCaption = true; }
        }
    }
}

codeunit 50988 "NB DQ2 R50988 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_EnumWithoutCaption()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">EnumNoCap</Label>]', Lib.Lines(Report::"NB DQ2 R50988", '<Label '));
    end;
}

// Option field without Caption: not measured.
report 50989 "NB DQ2 R50989"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".OptNoCap) { IncludeCaption = true; }
        }
    }
}

codeunit 50989 "NB DQ2 R50989 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_OptionWithoutCaption()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">OptNoCap</Label>]', Lib.Lines(Report::"NB DQ2 R50989", '<Label '));
    end;
}

// Boolean field: a type the probe does not measure.
report 50990 "NB DQ2 R50990"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".Flag) { IncludeCaption = true; }
        }
    }
}

codeunit 50990 "NB DQ2 R50990 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_BooleanField()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">X Flag cap</Label>]', Lib.Lines(Report::"NB DQ2 R50990", '<Label '));
    end;
}

// AutoFormatType 0 with DecimalPlaces 0:3: only 0:5 is measured.
report 50991 "NB DQ2 R50991"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".ZeroPlaces3) { IncludeCaption = true; }
        }
    }
}

codeunit 50991 "NB DQ2 R50991 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_ZeroTypeOtherPlaces()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X" decimalformatter="#,##0.###">2.5</Column>]', Lib.Lines(Report::"NB DQ2 R50991", '<Column '));
    end;
}

// AutoFormatType 0 with AutoFormatExpression '': not measured.
report 50992 "NB DQ2 R50992"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".ZeroExpr) { IncludeCaption = true; }
        }
    }
}

codeunit 50992 "NB DQ2 R50992 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_ZeroTypeWithExpression()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">X ZeroExpr cap</Label>]', Lib.Lines(Report::"NB DQ2 R50992", '<Label '));
    end;
}

// AutoFormatType 2 on a field without Caption: field_unit_amount_measured reads Caption.
report 50993 "NB DQ2 R50993"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".UnitNoCap) { IncludeCaption = true; }
        }
    }
}

codeunit 50993 "NB DQ2 R50993 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_UnitTypeWithoutCaption()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X" decimalformatter="#,##0.00###">1.23456</Column>]', Lib.Lines(Report::"NB DQ2 R50993", '<Column '));
    end;
}

// AutoFormatType 2 with DecimalPlaces: not measured.
report 50994 "NB DQ2 R50994"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".UnitPlaces) { IncludeCaption = true; }
        }
    }
}

codeunit 50994 "NB DQ2 R50994 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_UnitTypeWithPlaces()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">X UnitPlaces cap</Label>]', Lib.Lines(Report::"NB DQ2 R50994", '<Label '));
    end;
}

// AutoFormatType 1, AutoFormatExpression '' and DecimalPlaces: not measured.
report 50995 "NB DQ2 R50995"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".AmtPlacesExpr) { IncludeCaption = true; }
        }
    }
}

codeunit 50995 "NB DQ2 R50995 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_AmountTypeExpressionWithPlaces()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">X AmtPlacesExpr cap</Label>]', Lib.Lines(Report::"NB DQ2 R50995", '<Label '));
    end;
}

// AutoFormatType 2 + AutoFormatExpression '' under a column without property: not measured; main and head write the formatter of DecimalPlaces.
report 50996 "NB DQ2 R50996"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X"."Unit Cost") {  }
        }
    }
}

codeunit 50996 "NB DQ2 R50996 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_UnitTypeNoProperty()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X" decimalformatter="#,##0.00###">1.23456</Column>]', Lib.Lines(Report::"NB DQ2 R50996", '<Column '));
    end;
}

// A Code field under a column without property: not measured (no label and no formatter reads the field).
report 50997 "NB DQ2 R50997"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".TextNoCap) {  }
        }
    }
}

codeunit 50997 "NB DQ2 R50997 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_CodeNoProperty()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X">T1</Column>]', Lib.Lines(Report::"NB DQ2 R50997", '<Column '));
    end;
}

// AutoFormatType 2 + AutoFormatExpression '' under a bare column: field_unit_amount_measured needs Item.Field.
report 50998 "NB DQ2 R50998"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(DqX; "NB DQ2 Extra")
        {
            column(X; "Unit Cost") { IncludeCaption = true; }
        }
    }
}

codeunit 50998 "NB DQ2 R50998 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Undotted_UnitTypeBareColumn()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X" decimalformatter="#,##0.00###">1.23456</Column>]', Lib.Lines(Report::"NB DQ2 R50998", '<Column '));
    end;
}

// AutoFormatType 2 + AutoFormatExpression '' with the qualifier in another case: not measured.
report 50999 "NB DQ2 R50999"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(DqX; "NB DQ2 Extra")
        {
            column(X; dqx."Unit Cost") { IncludeCaption = true; }
        }
    }
}

codeunit 50999 "NB DQ2 R50999 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Undotted_UnitTypeQualifierOtherCase()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X" decimalformatter="#,##0.00###">1.23456</Column>]', Lib.Lines(Report::"NB DQ2 R50999", '<Column '));
    end;
}

// AutoFormatType 2 + AutoFormatExpression '' with the field name in another case: not measured.
report 51000 "NB DQ2 R51000"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(DqX; "NB DQ2 Extra")
        {
            column(X; DqX."unit cost") { IncludeCaption = true; }
        }
    }
}

codeunit 51000 "NB DQ2 R51000 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Undotted_UnitTypeFieldOtherCase()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X" decimalformatter="#,##0.00###">1.23456</Column>]', Lib.Lines(Report::"NB DQ2 R51000", '<Column '));
    end;
}

// Enum field with a Caption over an extensible enum without value captions: the gate does not read the enum (the label is the field Caption; the value comes from the column routine, which reads no item field). Head serves it.
report 51001 "NB DQ2 R51001"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".OpenEnum) { IncludeCaption = true; }
        }
    }
}

codeunit 51001 "NB DQ2 R51001 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_EnumOpenWithoutValueCaptions()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">X OpenEnum cap</Label>]', Lib.Lines(Report::"NB DQ2 R51001", '<Label '));
        Lib.Check('[<Column name="X">Beta</Column>]', Lib.Lines(Report::"NB DQ2 R51001", '<Column '));
    end;
}

// Option field with a Caption and no OptionCaption: the gate does not read OptionCaption (as for the enum). Head serves it.
report 51002 "NB DQ2 R51002"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(X; "Dq. X".OptNoOptionCaption) { IncludeCaption = true; }
        }
    }
}

codeunit 51002 "NB DQ2 R51002 T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure Dotted_OptionWithoutOptionCaption()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Label name="XCaption">X Opt cap</Label>]', Lib.Lines(Report::"NB DQ2 R51002", '<Label '));
        Lib.Check('[<Column name="X">Open</Column>]', Lib.Lines(Report::"NB DQ2 R51002", '<Column '));
    end;
}

// AutoFormatType 2 + AutoFormatExpression '' in a column a reportextension adds, written with the item's undotted name: the
// field formatter is measured in the report itself only.
report 51003 "NB DQ2 Ext Base"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem(DqX; "NB DQ2 Extra")
        {
            column(BaseId; Id) { }
        }
    }
}

reportextension 51003 "NB DQ2 Ext" extends "NB DQ2 Ext Base"
{
    dataset
    {
        add(DqX)
        {
            column(X; DqX."Unit Cost") { IncludeCaption = true; }
        }
    }
}

codeunit 51003 "NB DQ2 Ext T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ReportExtension_UnitTypeColumn()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X" decimalformatter="#,##0.00###">1.23456</Column>]', Lib.Lines(Report::"NB DQ2 Ext Base", '<Column name="X"'));
    end;
}

// A child item reads the AutoFormatType 2 field of its dotted parent: main's rule for a field of another data item (the
// default formatter), which neither gate touches.
report 51004 "NB DQ2 Child Reads Parent"
{
    ProcessingOnly = false;
    DefaultLayout = RDLC;
    UseRequestPage = false;

    dataset
    {
        dataitem("Dq. X"; "NB DQ2 Extra")
        {
            column(PId; Id) { }
            dataitem(Kid; "NB DQ2 Extra")
            {
                DataItemLink = Id = field(Id);
                column(X; "Dq. X"."Unit Cost") { IncludeCaption = true; }
            }
        }
    }
}

codeunit 51004 "NB DQ2 Child T"
{
    Subtype = Test;
    TestPermissions = Disabled;

    [Test]
    procedure ChildReadsDottedParentUnitType()
    var
        Lib: Codeunit "NB DQ2 Lib";
    begin
        Lib.Seed();
        Lib.Check('[<Column name="X" decimalformatter="#,##0.00###">1.23456</Column>]', Lib.Lines(Report::"NB DQ2 Child Reads Parent", '<Column name="X"'));
    end;
}
