//! A TestPage control over a table field whose AutoFormatExpression calls a procedure (al-runner issue 7850). The fills and
//! steps of pages/controls/AutoFormatProcedureControl.al run it and a read takes what it gave; every other page and fetch keeps
//! main's answer (the procedure does not run, a read refuses), and after a pinned open a step outside the pinned ones refuses.

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

    procedure Reset(NewFormat: Text)
    begin
        Calls := 0;
        FormatText := NewFormat;
    end;

    procedure Prime(NewCalls: Integer)
    begin
        Calls := NewCalls;
    end;

    procedure Next(FieldNo: Integer): Text
    begin
        NEXTBODY
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
        (
            "AMOUNTCONTROL",
            "field(Amount; Rec.Amount) { ApplicationArea = All; CONTROLPROPS }",
        ),
        ("CONTROLPROPS", ""),
        ("EXTRACONTROLS", ""),
        ("PAGETRIGGERS", ""),
        ("CUSTOM", ""),
        ("NEXTBODY", "Calls += 1; exit(FormatText);"),
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

/// `Custom` with `custom` (and the holes in `fill`) keeps main's answer: the procedure has not run, and a read of the
/// format control refuses with main's text.
fn main_kept(fill: &[(&str, &str)], custom: &str) {
    let mut fill = fill.to_vec();
    let custom = format!("{custom} Check('main', 0); if P.Amount.Value() = '' then;");
    fill.push(("CUSTOM", &custom));
    refused(run(&fill, "Custom"), MAIN_REFUSAL);
}

/// main's refusal of a read of a control whose field's AutoFormatExpression calls a procedure no fetch ran.
const MAIN_REFUSAL: &str =
    "whose AutoFormatExpression calls a procedure (BC runs it when the row is fetched; not modelled)";

#[test]
fn the_probe_steps_in_view_and_edit_mode() {
    let r = run(&[], "ViewSteps");
    assert!(r.is_ok(), "{r:?}");
    let r = run(&[], "EditSteps");
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn the_issue_7850_repro_keeps_main_s_answer() {
    // The repro's page shows the field directly in the area and declares no ApplicationArea, and its test sets the value in
    // View mode: none of it is pinned. main refused it at SetValue.
    refused(
        al_testkit::run_in(REPRO, "AF Tests", "OpenCard"),
        "AutoFormatExpression calls a procedure",
    );
}

// An OpenEdit runs its fill only when the next statement is the GoToRecord of the same TestPage: no AL code can read what the
// OpenEdit alone ran (AutoFormatProcedureControl.al:239-241 counts both together).
#[test]
fn edit_open_with_code_before_the_go_to_record_keeps_main_s_answer() {
    main_kept(&[], "P.OpenEdit(); Check('openedit', 0); P.GoToRecord(T);");
}

#[test]
fn edit_open_without_a_go_to_record_keeps_main_s_answer() {
    main_kept(&[], "P.OpenEdit();");
}

#[test]
fn edit_open_then_go_to_record_of_another_test_page_keeps_main_s_answer() {
    // The OpenEdit of P runs nothing; main answers the GoToRecord of Q, which is not open.
    let r = run(&[("CUSTOM", "P.OpenEdit(); Q.GoToRecord(T);")], "Custom");
    let e = format!("{r:?}");
    assert!(e.contains("The TestPage is not open."), "{e}");
}

#[test]
fn edit_open_then_go_to_record_whose_answer_is_used_keeps_main_s_answer() {
    main_kept(&[], "P.OpenEdit(); if P.GoToRecord(T) then;");
}

// The expression and the procedure.
#[test]
fn an_expression_that_reads_the_row_keeps_main_s_answer() {
    main_kept(&[("EXPR", "AFSetup.GetFormatFor(Rec.\"Code\")")], "P.OpenView();");
}

#[test]
fn a_procedure_that_writes_its_record_keeps_main_s_answer() {
    main_kept(
        &[("EXPR", "AFSetup.CountedFormat(AFSetup.FieldNo(Decimals))")],
        "P.OpenView();",
    );
}

#[test]
fn a_codeunit_procedure_that_calls_message_keeps_main_s_answer() {
    // The probe's codeunit procedure only counts and exits (AutoFormatProcedureControl.al:161-165); one that also calls
    // Message is not measured.
    let next = [("NEXTBODY", "Calls += 1; Message('call %1', Calls); exit(FormatText);")];
    main_kept(&next, "P.OpenEdit(); P.GoToRecord(T);");
    main_kept(&next, "P.OpenView();");
}

#[test]
fn a_codeunit_procedure_that_exits_a_call_keeps_main_s_answer() {
    main_kept(
        &[("NEXTBODY", "Calls += 1; exit(CopyStr(FormatText, 1));")],
        "P.OpenView();",
    );
}

#[test]
fn a_receiver_that_is_no_record_keeps_main_s_answer() {
    main_kept(&[("EXPR", "NoRecord.Next(1)")], "P.OpenView();");
}

// The open.
#[test]
fn a_trapped_page_run_keeps_main_s_answer() {
    main_kept(&[], "P.Trap(); Page.Run(Page::\"AF Card\", T);");
}

#[test]
fn open_new_keeps_main_s_answer() {
    main_kept(&[], "P.OpenNew();");
}

#[test]
fn an_empty_table_at_the_open_keeps_main_s_answer() {
    // main reads the control of an empty page without running the procedure.
    let r = run(
        &[(
            "CUSTOM",
            "T.Delete(); P.OpenView(); Check('main', 0); if P.Amount.Value() <> '' then Error('read');",
        )],
        "Custom",
    );
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn two_rows_at_the_open_keep_main_s_answer() {
    main_kept(&[], "Other := T; Other.\"Code\" := 'T2'; Other.Insert(); P.OpenView();");
}

// Controls.
#[test]
fn two_controls_over_the_field_keeps_main_s_answer() {
    main_kept(
        &[("EXTRACONTROLS", "field(Amount2; Rec.Amount) { ApplicationArea = All; }")],
        "P.OpenView();",
    );
}

#[test]
fn a_control_property_keeps_main_s_answer() {
    main_kept(&[("CONTROLPROPS", "Editable = false;")], "P.OpenView();");
}

#[test]
fn a_property_of_another_control_keeps_main_s_answer() {
    main_kept(&[("CODEPROPS", "Caption = 'C';")], "P.OpenView();");
}

#[test]
fn a_group_property_keeps_main_s_answer() {
    main_kept(&[("GROUPPROPS", "Caption = 'G';")], "P.OpenView();");
}

#[test]
fn a_control_trigger_keeps_main_s_answer() {
    main_kept(&[("CONTROLPROPS", "trigger OnValidate() begin end;")], "P.OpenView();");
}

#[test]
fn a_format_of_the_control_s_own_keeps_main_s_answer() {
    // The control names its own AutoFormatExpression: the table field's procedure is not the control's format; main opened the
    // page and ran nothing.
    let r = run(
        &[
            (
                "CONTROLPROPS",
                "AutoFormatType = 11; AutoFormatExpression = '<precision, 2:2><standard format, 0>';",
            ),
            ("CUSTOM", "P.OpenView(); Check('openview', 0);"),
        ],
        "Custom",
    );
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn a_page_without_a_control_over_the_field_keeps_main_s_answer() {
    let r = run(
        &[
            ("AMOUNTCONTROL", ""),
            ("CUSTOM", "P.OpenView(); Check('openview', 0); P.Next();"),
        ],
        "Custom",
    );
    assert!(r.is_ok(), "{r:?}");
}

// Fields and the table.
#[test]
fn a_second_procedure_field_without_a_control_keeps_main_s_answer() {
    main_kept(
        &[(
            "EXTRAFIELDS",
            "field(3; Other; Decimal) { DataClassification = SystemMetadata; AutoFormatType = 11; AutoFormatExpression = AFSetup.GetFormat(AFSetup.FieldNo(Decimals)); }",
        )],
        "P.OpenView();",
    );
}

#[test]
fn an_integer_field_keeps_main_s_answer() {
    main_kept(&[("AMOUNTTYPE", "Integer"), ("AMOUNTVALUE", "3")], "P.OpenView();");
}

#[test]
fn auto_format_type_10_keeps_main_s_answer() {
    main_kept(&[("AFTYPE", "10")], "P.OpenView();");
}

#[test]
fn a_property_beside_the_expression_keeps_main_s_answer() {
    main_kept(&[("FIELDPROPS", "DecimalPlaces = 0 : 5;")], "P.OpenView();");
}

#[test]
fn a_property_of_another_field_keeps_main_s_answer() {
    main_kept(&[("FIRSTFIELDPROPS", "Caption = 'C';")], "P.OpenView();");
}

#[test]
fn a_table_property_keeps_main_s_answer() {
    main_kept(&[("TABLEPROPS", "Caption = 'T';")], "P.OpenView();");
}

#[test]
fn a_table_trigger_keeps_main_s_answer() {
    main_kept(&[("TABLETRIGGERS", "trigger OnModify() begin end;")], "P.OpenView();");
}

// The page.
#[test]
fn a_page_property_keeps_main_s_answer() {
    main_kept(&[("PAGEPROPS", "Caption = 'P';")], "P.OpenView();");
}

#[test]
fn a_page_trigger_keeps_main_s_answer() {
    main_kept(
        &[("PAGETRIGGERS", "trigger OnAfterGetRecord() begin end;")],
        "P.OpenView();",
    );
}

#[test]
fn a_document_page_keeps_main_s_answer() {
    main_kept(&[("PAGETYPE", "Document")], "P.OpenView();");
}

#[test]
fn a_list_keeps_main_s_answer() {
    main_kept(&[("PAGETYPE", "List")], "P.OpenView();");
}

// Steps after a pinned open.
#[test]
fn next_after_the_open_refuses() {
    refused(run(&[("CUSTOM", "P.OpenView(); P.Next();")], "Custom"), "TestPage.Next");
}

#[test]
fn a_go_to_record_after_a_second_row_came_refuses() {
    // The view holds two rows when the GoToRecord fetches: only one is pinned, so it runs nothing and refuses.
    refused(
        run(
            &[(
                "CUSTOM",
                "P.OpenView(); Other := T; Other.\"Code\" := 'T2'; Other.Insert(); P.GoToRecord(T);",
            )],
            "Custom",
        ),
        "a GoToRecord that fetched no row",
    );
}

#[test]
fn a_second_go_to_record_refuses() {
    refused(
        run(
            &[("CUSTOM", "P.OpenView(); P.GoToRecord(T); P.GoToRecord(T);")],
            "Custom",
        ),
        "TestPage.GoToRecord",
    );
}

#[test]
fn close_before_the_go_to_record_refuses() {
    refused(
        run(&[("CUSTOM", "P.OpenView(); P.Close();")], "Custom"),
        "TestPage.Close",
    );
}

#[test]
fn a_set_value_in_view_mode_refuses() {
    refused(
        run(
            &[("CUSTOM", "P.OpenView(); P.GoToRecord(T); P.Amount.SetValue(7);")],
            "Custom",
        ),
        "TestField.SetValue",
    );
}

#[test]
fn a_set_value_of_another_control_refuses() {
    refused(
        run(
            &[("CUSTOM", "P.OpenEdit(); P.GoToRecord(T); P.\"Code\".SetValue('T9');")],
            "Custom",
        ),
        "TestField.SetValue",
    );
}

#[test]
fn ok_after_a_set_value_refuses() {
    refused(
        run(
            &[(
                "CUSTOM",
                "P.OpenEdit(); P.GoToRecord(T); P.Amount.SetValue(7); P.OK().Invoke();",
            )],
            "Custom",
        ),
        "TestPage.OK",
    );
}

#[test]
fn close_after_a_set_value_of_the_same_value_refuses() {
    refused(
        run(
            &[(
                "CUSTOM",
                "P.OpenEdit(); P.GoToRecord(T); P.Amount.SetValue(3.25); P.Close();",
            )],
            "Custom",
        ),
        "changes no value",
    );
}

#[test]
fn a_go_to_record_of_a_missing_key_between_open_edit_and_a_read_refuses() {
    // The afp3 H1a shape: the GoToRecord after the OpenEdit fetches no row, so the 2 runs of the open alone could be read.
    let custom = "Other.\"Code\" := 'ZZ'; asserterror begin P.OpenEdit(); P.GoToRecord(Other); end; Check('open', 2);";
    refused(run(&[("CUSTOM", custom)], "Custom"), "not pinned");
}

#[test]
fn close_in_edit_mode_without_a_set_value_refuses() {
    refused(
        run(&[("CUSTOM", "P.OpenEdit(); P.GoToRecord(T); P.Close();")], "Custom"),
        "TestPage.Close",
    );
}

#[test]
fn an_error_in_the_procedure_at_the_open_refuses() {
    // The probe's procedure raises an error here only when its count passes the largest Integer.
    refused(
        run(&[("CUSTOM", "Counter.Prime(2147483647); P.OpenView();")], "Custom"),
        "raised an error",
    );
}

#[test]
fn an_unmeasured_format_text_refuses_the_read() {
    refused(
        run(
            &[(
                "CUSTOM",
                "Counter.Reset('<Precision,2:2><Standard Format,0>'); P.OpenView(); if P.Amount.Value() = '' then;",
            )],
            "Custom",
        ),
        "gave a format text",
    );
}
