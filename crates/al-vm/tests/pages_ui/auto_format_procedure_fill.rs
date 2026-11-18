//! A TestPage control over a table field whose AutoFormatExpression calls a procedure (al-runner issue 7850): the pinned
//! fills of the probe's Card run it and a read takes what it gave (pages/controls/AutoFormatProcedureControl.al, the corpus
//! cases); every other page and fill keeps main's answer, a read that refuses.

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

/// A Card or List over a field whose AutoFormatExpression calls a procedure of a global record of its table; `TRIGGER` is
/// replaced by the page's triggers.
const FIXTURE: &str = r#"
table 50100 "AF Setup"
{
    fields
    {
        field(1; Code; Code[10]) { }
    }
    keys { key(PK; Code) { Clustered = true; } }

    procedure GetFormat(): Text
    begin
        exit('<precision, 2:2><standard format, 0>');
    end;
}

table 50101 "AF Thing"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(2; Amount; Decimal)
        {
            AutoFormatType = 11;
            AutoFormatExpression = AFSetup.GetFormat();
        }
    }
    keys { key(PK; Code) { Clustered = true; } }

    var
        AFSetup: Record "AF Setup";
}

page 50102 "AF Page"
{
    PageType = PAGETYPE;
    SourceTable = "AF Thing";

    layout
    {
        area(Content)
        {
            GROUP
            {
                field(Code; Rec.Code) { }
                field(Amount; Rec.Amount) { }
            }
        }
    }
    TRIGGER
}

codeunit 50103 "AF Tests"
{
    Subtype = Test;

    [Test]
    procedure ReadAfterOpen()
    var
        T: Record "AF Thing";
        P: TestPage "AF Page";
    begin
        T.Code := 'A';
        T.Amount := 3.25;
        T.Insert();
        P.OpenView();
        if P.Amount.Value() <> '3.25' then
            Error('shown as %1', P.Amount.Value());
    end;

    [Test]
    procedure ReadWithTwoRows()
    var
        T: Record "AF Thing";
        P: TestPage "AF Page";
    begin
        T.Code := 'A';
        T.Amount := 3.25;
        T.Insert();
        T.Code := 'B';
        T.Insert();
        P.OpenView();
        P.GoToRecord(T);
        if P.Amount.Value() <> '3.25' then
            Error('shown as %1', P.Amount.Value());
    end;
}
"#;

fn page(page_type: &str, group: &str, triggers: &str) -> String {
    FIXTURE
        .replace("PAGETYPE", page_type)
        .replace("GROUP", group)
        .replace("TRIGGER", triggers)
}

fn refused_as_main(r: Result<al_builtins::value::Value, al_builtins::native::Signal>) {
    let e = format!("{r:?}");
    assert!(
        e.contains("Unsupported") && e.contains("whose AutoFormatExpression calls a procedure"),
        "{e}"
    );
}

#[test]
fn the_issue_7850_repro() {
    // main refused it on SetValue: "a control over table field Amount whose AutoFormatExpression calls a procedure".
    let r = al_testkit::run_in(REPRO, "AF Tests", "OpenCard");
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn the_probe_card_shape_reads_what_the_open_fill_gave() {
    let r = al_testkit::run_in(&page("Card", "group(General)", ""), "AF Tests", "ReadAfterOpen");
    assert!(r.is_ok(), "{r:?}");
}

#[test]
fn a_list_keeps_main_s_refusal() {
    refused_as_main(al_testkit::run_in(
        &page("List", "repeater(Rows)", ""),
        "AF Tests",
        "ReadAfterOpen",
    ));
}

#[test]
fn a_card_with_a_row_trigger_keeps_main_s_refusal() {
    let trigger = "trigger OnAfterGetRecord() begin end;";
    refused_as_main(al_testkit::run_in(
        &page("Card", "group(General)", trigger),
        "AF Tests",
        "ReadAfterOpen",
    ));
}

#[test]
fn two_rows_in_the_view_keep_main_s_refusal() {
    refused_as_main(al_testkit::run_in(
        &page("Card", "group(General)", ""),
        "AF Tests",
        "ReadWithTwoRows",
    ));
}
