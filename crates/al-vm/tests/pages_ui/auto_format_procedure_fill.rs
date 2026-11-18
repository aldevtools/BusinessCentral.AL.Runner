//! A TestPage control over a table field whose AutoFormatExpression calls a procedure (al-runner issue 7850). The fills and
//! steps of pages/controls/AutoFormatProcedureControl.al run it and a read takes what it gave; a page that would have to run
//! such a procedure anywhere else refuses at the open, and a step outside the pinned ones refuses before it runs.

/// The repro of al-runner issue 7850, unchanged.
const REPRO: &str = r#"
table 50100 "AF Setup"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; Decimals; Integer) { }
    }
    keys { key(PK; Code) { Clustered = true; } }

    procedure GetFormat(FieldNo: Integer): Text
    begin
        exit('<precision, 2:2><standard format, 0>');
    end;
}

table 50103 "AF Thing"
{
    fields
    {
        field(1; Code; Code[10]) { }
    }
    keys { key(PK; Code) { Clustered = true; } }
}

tableextension 50104 "AF Thing Ext" extends "AF Thing"
{
    fields
    {
        field(50100; Amount; Decimal)
        {
            AutoFormatType = 11;
            AutoFormatExpression = AFSetup.GetFormat(AFSetup.FieldNo(Decimals));
        }
    }

    var
        AFSetup: Record "AF Setup";
}

page 50101 "AF Card"
{
    SourceTable = "AF Thing";
    PageType = Card;

    layout
    {
        area(Content)
        {
            field(Code; Rec.Code) { }
        }
    }
}

pageextension 50105 "AF Card Ext" extends "AF Card"
{
    layout
    {
        addlast(Content)
        {
            field(Amount; Rec.Amount) { }
        }
    }
}

codeunit 50102 "AF Tests"
{
    Subtype = Test;

    [Test]
    procedure OpenCard()
    var
        S: Record "AF Thing";
        P: TestPage "AF Card";
    begin
        S.Code := 'A';
        S.Amount := 5;
        S.Insert();
        P.OpenView();
        P.GoToRecord(S);
        P.Amount.SetValue(7);
        P.Amount.AssertEquals(7);
    end;
}
"#;

/// The probe's shape (AutoFormatProcedureControl.al:19-172) written anew; each upper-case word is a hole a test fills, the
/// probe's value by default. `Custom` runs CUSTOM after `Init`, which stores row T1 with 3.25 and sets the text the procedure
/// gives.
const FIXTURE: &str = r#"
table 50100 "AF Setup"
{
    DataClassification = SystemMetadata;

    fields
    {
        field(1; "Code"; Code[10]) { DataClassification = SystemMetadata; }
        field(2; Decimals; Integer) { DataClassification = SystemMetadata; }
    }
    keys { key(PK; "Code") { Clustered = true; } }

    procedure GetFormat(FieldNo: Integer): Text
    var
        Counter: Codeunit "AF Counter";
    begin
        exit(Counter.Next(FieldNo));
    end;

    procedure GetFormatFor(Key: Code[10]): Text
    var
        Counter: Codeunit "AF Counter";
    begin
        exit(Counter.Next(StrLen(Key)));
    end;

    procedure CountedFormat(FieldNo: Integer): Text
    var
        Counter: Codeunit "AF Counter";
    begin
        Decimals += 1;
        exit(Counter.Next(FieldNo));
    end;
}

table 50101 "AF Thing"
{
    DataClassification = SystemMetadata;
    TABLEPROPS

    fields
    {
        field(1; "Code"; Code[10]) { DataClassification = SystemMetadata; FIRSTFIELDPROPS }
        field(2; Amount; AMOUNTTYPE)
        {
            DataClassification = SystemMetadata;
            AutoFormatType = AFTYPE;
            AutoFormatExpression = EXPR;
            FIELDPROPS
        }
        EXTRAFIELDS
    }
    keys { key(PK; "Code") { Clustered = true; } }

    var
        AFSetup: Record "AF Setup";
        NoRecord: Codeunit "AF Counter";
    TABLETRIGGERS
}

page 50102 "AF Card"
{
    PageType = PAGETYPE;
    SourceTable = "AF Thing";
    ApplicationArea = All;
    UsageCategory = None;
    PAGEPROPS

    layout
    {
        area(Content)
        {
            group(General)
            {
                GROUPPROPS
                field("Code"; Rec."Code") { ApplicationArea = All; CODEPROPS }
                AMOUNTCONTROL
                EXTRACONTROLS
            }
        }
    }
    PAGETRIGGERS
}

codeunit 50103 "AF Counter"
{
    SingleInstance = true;

    var
        Calls: Integer;
        FormatText: Text;
        Fail: Boolean;

    procedure Reset(NewFormat: Text)
    begin
        Calls := 0;
        FormatText := NewFormat;
    end;

    procedure FailNext()
    begin
        Fail := true;
    end;

    procedure Next(FieldNo: Integer): Text
    begin
        Calls += 1;
        if Fail then
            Error('format failed');
        exit(FormatText);
    end;

    procedure Take() Result: Integer
    begin
        Result := Calls;
        Calls := 0;
    end;
}

codeunit 50104 "AF Tests"
{
    Subtype = Test;

    var
        Counter: Codeunit "AF Counter";

    local procedure Init(NewFormat: Text)
    var
        T: Record "AF Thing";
    begin
        T.DeleteAll();
        T.Init();
        T."Code" := 'T1';
        T.Amount := AMOUNTVALUE;
        T.Insert();
        Counter.Reset(NewFormat);
    end;

    local procedure Check(Step: Text; Want: Integer)
    var
        Got: Integer;
    begin
        Got := Counter.Take();
        if Got <> Want then
            Error('%1: %2 calls', Step, Got);
    end;

    [Test]
    procedure ViewSteps()
    var
        T: Record "AF Thing";
        P: TestPage "AF Card";
    begin
        Init('<precision, 2:2><standard format, 0>');
        T.Get('T1');
        P.OpenView();
        Check('openview', 2);
        P.GoToRecord(T);
        Check('gotorecord', 1);
        if P.Amount.Value() <> '3.25' then
            Error('shown as %1', P.Amount.Value());
        P.Close();
        Check('close', 0);
    end;

    [Test]
    procedure EditSteps()
    var
        T: Record "AF Thing";
        P: TestPage "AF Card";
    begin
        Init('<precision, 2:2><standard format, 0>');
        T.Get('T1');
        P.OpenEdit();
        P.GoToRecord(T);
        Check('openedit', 3);
        P.Amount.SetValue(7);
        if P.Amount.Value() <> '7.00' then
            Error('shown as %1', P.Amount.Value());
        P.Close();
        Check('close', 1);
    end;

    [Test]
    procedure Custom()
    var
        T: Record "AF Thing";
        Other: Record "AF Thing";
        P: TestPage "AF Card";
        Q: TestPage "AF Card";
    begin
        Init('<precision, 2:2><standard format, 0>');
        T.Get('T1');
        CUSTOM
    end;
}
"#;

/// The fixture with the holes in `fill` replaced and every other hole at the probe's value.
fn fixture(fill: &[(&str, &str)]) -> String {
    let defaults = [
        ("TABLEPROPS", ""),
        ("FIRSTFIELDPROPS", ""),
        ("AMOUNTTYPE", "Decimal"),
        ("AMOUNTVALUE", "3.25"),
        ("AFTYPE", "11"),
        ("EXPR", "AFSetup.GetFormat(AFSetup.FieldNo(Decimals))"),
        ("FIELDPROPS", ""),
        ("EXTRAFIELDS", ""),
        ("TABLETRIGGERS", ""),
        ("PAGETYPE", "Card"),
        ("PAGEPROPS", ""),
        ("GROUPPROPS", ""),
        ("CODEPROPS", ""),
        ("AMOUNTCONTROL", "field(Amount; Rec.Amount) { ApplicationArea = All; CONTROLPROPS }"),
        ("CONTROLPROPS", ""),
        ("EXTRACONTROLS", ""),
        ("PAGETRIGGERS", ""),
        ("CUSTOM", ""),
    ];
    let mut src = FIXTURE.to_string();
    for (hole, default) in defaults {
        let with = fill.iter().find(|(h, _)| *h == hole).map_or(default, |(_, v)| *v);
        src = src.replace(hole, with);
    }
    src
}

type Outcome = Result<al_builtins::value::Value, al_builtins::native::Signal>;

fn run(fill: &[(&str, &str)], test: &str) -> Outcome {
    al_testkit::run_in(&fixture(fill), "AF Tests", test)
}

/// The test refused (`Signal::Unsupported`) with a reason that holds `why`.
fn refused(r: Outcome, why: &str) {
    let e = format!("{r:?}");
    assert!(e.contains("Unsupported") && e.contains(why), "{e}");
}

/// `Custom` with `custom` (and the holes in `fill`) refused at a fetch of the page, for the reason `why`.
fn open_refused(fill: &[(&str, &str)], custom: &str, why: &str) {
    let mut fill = fill.to_vec();
    fill.push(("CUSTOM", custom));
    let e = format!("{:?}", run(&fill, "Custom"));
    assert!(e.contains("Unsupported") && e.contains("fetches") && e.contains(why), "{e}");
}

#[test]
fn the_probe_steps_in_view_and_edit_mode() {
    let r = run(&[], "ViewSteps");
    assert!(r.is_ok(), "{r:?}");
    let r = run(&[], "EditSteps");
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn the_issue_7850_repro_refuses_at_the_open() {
    // The repro's page shows the field directly in the area and declares no ApplicationArea, and its test sets the value in
    // View mode: none of it is pinned. main refused it at SetValue.
    let e = format!("{:?}", al_testkit::run_in(REPRO, "AF Tests", "OpenCard"));
    assert!(e.contains("Unsupported") && e.contains("fetches"), "{e}");
}

// An OpenEdit runs its fill only when the next statement is the GoToRecord of the same TestPage: no AL code can read what the
// OpenEdit alone ran (AutoFormatProcedureControl.al:239-241 counts both together).
#[test]
fn edit_open_with_code_before_the_go_to_record_refuses() {
    open_refused(&[], "P.OpenEdit(); Check('openedit', 2); P.GoToRecord(T);", "GoToRecord");
}

#[test]
fn edit_open_without_a_go_to_record_refuses() {
    open_refused(&[], "P.OpenEdit(); P.Close();", "GoToRecord");
}

#[test]
fn edit_open_then_go_to_record_of_another_test_page_refuses() {
    open_refused(&[], "P.OpenEdit(); Q.GoToRecord(T);", "GoToRecord");
}

#[test]
fn edit_open_then_go_to_record_whose_answer_is_used_refuses() {
    open_refused(&[], "P.OpenEdit(); if P.GoToRecord(T) then;", "GoToRecord");
}

// The expression and the procedure.
#[test]
fn an_expression_that_reads_the_row_refuses_at_the_open() {
    open_refused(&[("EXPR", "AFSetup.GetFormatFor(Rec.\"Code\")")], "P.OpenView();", "expression");
}

#[test]
fn a_procedure_that_writes_its_record_refuses_at_the_open() {
    open_refused(&[("EXPR", "AFSetup.CountedFormat(AFSetup.FieldNo(Decimals))")], "P.OpenView();", "procedure");
}

#[test]
fn a_receiver_that_is_no_record_refuses_at_the_open() {
    open_refused(&[("EXPR", "NoRecord.Next(1)")], "P.OpenView();", "expression");
}

// The open.
#[test]
fn a_trapped_page_run_refuses_at_the_open() {
    open_refused(&[], "P.Trap(); Page.Run(Page::\"AF Card\", T);", "opened");
}

#[test]
fn open_new_refuses_at_the_open() {
    open_refused(&[], "P.OpenNew();", "opened");
}

#[test]
fn an_empty_table_at_the_open_refuses() {
    open_refused(&[], "T.Delete(); P.OpenView();", "row");
}

#[test]
fn two_rows_at_the_open_refuse() {
    open_refused(&[], "Other := T; Other.\"Code\" := 'T2'; Other.Insert(); P.OpenView();", "row");
}

// Controls.
#[test]
fn two_controls_over_the_field_refuse_at_the_open() {
    open_refused(
        &[("EXTRACONTROLS", "field(Amount2; Rec.Amount) { ApplicationArea = All; }")],
        "P.OpenView();",
        "control",
    );
}

#[test]
fn a_control_property_refuses_at_the_open() {
    open_refused(&[("CONTROLPROPS", "Editable = false;")], "P.OpenView();", "control");
}

#[test]
fn a_property_of_another_control_refuses_at_the_open() {
    open_refused(&[("CODEPROPS", "Caption = 'C';")], "P.OpenView();", "control");
}

#[test]
fn a_group_property_refuses_at_the_open() {
    open_refused(&[("GROUPPROPS", "Visible = false;")], "P.OpenView();", "control");
}

#[test]
fn a_control_trigger_refuses_at_the_open() {
    open_refused(&[("CONTROLPROPS", "trigger OnValidate() begin end;")], "P.OpenView();", "control");
}

#[test]
fn a_format_of_the_control_s_own_keeps_main_s_answer() {
    // The control names its own AutoFormatExpression: the table field's procedure is not the control's format; main opened the
    // page and ran nothing.
    let r = run(
        &[
            ("CONTROLPROPS", "AutoFormatType = 11; AutoFormatExpression = '<precision, 2:2><standard format, 0>';"),
            ("CUSTOM", "P.OpenView(); Check('openview', 0);"),
        ],
        "Custom",
    );
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn a_page_without_a_control_over_the_field_keeps_main_s_answer() {
    let r = run(
        &[("AMOUNTCONTROL", ""), ("CUSTOM", "P.OpenView(); Check('openview', 0); P.Next();")],
        "Custom",
    );
    assert!(r.is_ok(), "{r:?}");
}

// Fields and the table.
#[test]
fn a_second_procedure_field_without_a_control_refuses_at_the_open() {
    open_refused(
        &[(
            "EXTRAFIELDS",
            "field(3; Other; Decimal) { DataClassification = SystemMetadata; AutoFormatType = 11; AutoFormatExpression = AFSetup.GetFormat(AFSetup.FieldNo(Decimals)); }",
        )],
        "P.OpenView();",
        "field",
    );
}

#[test]
fn an_integer_field_refuses_at_the_open() {
    open_refused(&[("AMOUNTTYPE", "Integer"), ("AMOUNTVALUE", "3")], "P.OpenView();", "field");
}

#[test]
fn auto_format_type_10_refuses_at_the_open() {
    open_refused(&[("AFTYPE", "10")], "P.OpenView();", "field");
}

#[test]
fn a_property_beside_the_expression_refuses_at_the_open() {
    open_refused(&[("FIELDPROPS", "DecimalPlaces = 0 : 5;")], "P.OpenView();", "field");
}

#[test]
fn a_property_of_another_field_refuses_at_the_open() {
    open_refused(&[("FIRSTFIELDPROPS", "Caption = 'C';")], "P.OpenView();", "field");
}

#[test]
fn a_table_property_refuses_at_the_open() {
    open_refused(&[("TABLEPROPS", "Caption = 'T';")], "P.OpenView();", "table");
}

#[test]
fn a_table_trigger_refuses_at_the_open() {
    open_refused(&[("TABLETRIGGERS", "trigger OnModify() begin end;")], "P.OpenView();", "table");
}

// The page.
#[test]
fn a_page_property_refuses_at_the_open() {
    open_refused(&[("PAGEPROPS", "Caption = 'P';")], "P.OpenView();", "page");
}

#[test]
fn a_page_trigger_refuses_at_the_open() {
    open_refused(&[("PAGETRIGGERS", "trigger OnAfterGetRecord() begin end;")], "P.OpenView();", "page");
}

#[test]
fn a_document_page_refuses_at_the_open() {
    open_refused(&[("PAGETYPE", "Document")], "P.OpenView();", "page");
}

#[test]
fn a_list_refuses_at_the_open() {
    open_refused(&[("PAGETYPE", "List")], "P.OpenView();", "page");
}

// Steps after a pinned open.
#[test]
fn next_after_the_open_refuses() {
    refused(run(&[("CUSTOM", "P.OpenView(); P.Next();")], "Custom"), "TestPage.Next");
}

#[test]
fn a_second_go_to_record_refuses() {
    refused(
        run(&[("CUSTOM", "P.OpenView(); P.GoToRecord(T); P.GoToRecord(T);")], "Custom"),
        "TestPage.GoToRecord",
    );
}

#[test]
fn close_before_the_go_to_record_refuses() {
    refused(run(&[("CUSTOM", "P.OpenView(); P.Close();")], "Custom"), "TestPage.Close");
}

#[test]
fn a_set_value_in_view_mode_refuses() {
    refused(
        run(&[("CUSTOM", "P.OpenView(); P.GoToRecord(T); P.Amount.SetValue(7);")], "Custom"),
        "TestField.SetValue",
    );
}

#[test]
fn a_set_value_of_another_control_refuses() {
    refused(
        run(&[("CUSTOM", "P.OpenEdit(); P.GoToRecord(T); P.\"Code\".SetValue('T9');")], "Custom"),
        "TestField.SetValue",
    );
}

#[test]
fn ok_after_a_set_value_refuses() {
    refused(
        run(&[("CUSTOM", "P.OpenEdit(); P.GoToRecord(T); P.Amount.SetValue(7); P.OK().Invoke();")], "Custom"),
        "TestPage.OK",
    );
}

#[test]
fn close_after_a_set_value_of_the_same_value_refuses() {
    refused(
        run(&[("CUSTOM", "P.OpenEdit(); P.GoToRecord(T); P.Amount.SetValue(3.25); P.Close();")], "Custom"),
        "changes no value",
    );
}

#[test]
fn close_in_edit_mode_without_a_set_value_refuses() {
    refused(run(&[("CUSTOM", "P.OpenEdit(); P.GoToRecord(T); P.Close();")], "Custom"), "TestPage.Close");
}

#[test]
fn an_error_in_the_procedure_at_the_open_refuses() {
    refused(run(&[("CUSTOM", "Counter.FailNext(); P.OpenView();")], "Custom"), "raised an error");
}

#[test]
fn an_unmeasured_format_text_refuses_the_read() {
    refused(
        run(
            &[("CUSTOM", "Counter.Reset('<Precision,2:2><Standard Format,0>'); P.OpenView(); if P.Amount.Value() = '' then;")],
            "Custom",
        ),
        "gave a format text",
    );
}
