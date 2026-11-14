//! A page control whose AutoFormatType is none of the types the runtime models (0, 1, 2, 4, 5, 10, 11) is refused when its text is
//! read or set: BC resolves such a type through the AutoFormatTranslate event of UI Helper Triggers, which the runtime does not
//! raise (al-runner issue 8011). Before, it showed the value with the default two decimals.

/// The repro of al-runner issue 8011, unchanged. BC shows `3,456.7` (Microsoft's Auto Format Test.TestAutoFormatExtensibility
/// asserts it for the same expression); main showed `3,456.68`.
const REPRO: &str = r#"
table 50731 "R969 Amount"
{
    TableType = Temporary;
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Amount; Decimal) { }
    }
    keys
    {
        key(PK; "No.") { Clustered = true; }
    }
}

page 50732 "R969 Amount Card"
{
    PageType = Card;
    SourceTable = "R969 Amount";
    SourceTableTemporary = true;
    layout
    {
        area(Content)
        {
            field(Amount; Rec.Amount)
            {
                AutoFormatType = 50733;
            }
        }
    }
}

codeunit 50733 "R969 Auto Format"
{
    EventSubscriberInstance = StaticAutomatic;

    [EventSubscriber(ObjectType::Codeunit, Codeunit::"UI Helper Triggers", AutoFormatTranslate, '', false, false)]
    local procedure OneDecimal(AutoFormatType: Integer; AutoFormatExpr: Text[80]; var Translation: Text[80])
    begin
        if AutoFormatType = 50733 then
            Translation := '<Precision,1:1><Standard Format,0>';
    end;
}

codeunit 50734 "R969 Auto Format Test"
{
    Subtype = Test;

    [Test]
    procedure CustomAutoFormatTypeShowsOneDecimal() // BC: passes. al-runner: "shown as 3,456.68"
    var
        Card: TestPage "R969 Amount Card";
    begin
        Card.OpenNew();
        Card.Amount.SetValue(3456.67843);
        if Card.Amount.Value() <> '3,456.7' then
            Error('shown as %1', Card.Amount.Value());
    end;
}
"#;

fn refused(r: Result<al_builtins::value::Value, al_builtins::native::Signal>, kind: &str) {
    let e = format!("{r:?}");
    assert!(
        e.contains("Unsupported") && e.contains(&format!("AutoFormatType {kind}")),
        "{e}"
    );
}

#[test]
fn the_issue_8011_repro_is_refused_not_shown_with_two_decimals() {
    refused(
        al_testkit::run_in(REPRO, "R969 Auto Format Test", "CustomAutoFormatTypeShowsOneDecimal"),
        "50733",
    );
}

/// A page over a table field; `CTL` and `FIELD` are the AutoFormatType properties of the control and of the field.
fn page(ctl: &str, field: &str) -> String {
    format!(
        r#"
table 69460 "AfU Row"
{{
    fields {{ field(1; Id; Integer) {{ }} field(2; Amt; Decimal) {{ {field} }} }}
    keys {{ key(PK; Id) {{ Clustered = true; }} }}
}}
page 69461 "AfU Card"
{{
    PageType = Card; SourceTable = "AfU Row"; ApplicationArea = All; UsageCategory = None;
    layout {{ area(Content) {{ field(AmtCtl; Rec.Amt) {{ {ctl} }} }} }}
}}
codeunit 69462 C
{{
    procedure Run(): Text
    var
        R: Record "AfU Row";
        P: TestPage "AfU Card";
    begin
        R.Id := 1;
        R.Amt := 3456.67843;
        R.Insert();
        P.OpenView();
        exit(P.AmtCtl.Value());
    end;
}}
"#
    )
}

#[test]
fn an_unknown_type_on_the_table_field_is_refused_through_a_control_without_one() {
    refused(al_testkit::run_in(&page("", "AutoFormatType = 3;"), "C", "Run"), "3");
}

#[test]
fn an_unknown_type_on_the_control_is_refused_over_a_field_of_a_modelled_type() {
    refused(
        al_testkit::run_in(&page("AutoFormatType = 50733;", "AutoFormatType = 1;"), "C", "Run"),
        "50733",
    );
}

/// A precision or DecimalPlaces beside the unknown type does not decide the text either: refused.
#[test]
fn an_unknown_type_with_a_precision_expression_is_refused() {
    refused(
        al_testkit::run_in(
            &page(
                "AutoFormatType = 50733; AutoFormatExpression = '<Precision,1:1><Standard Format,0>';",
                "DecimalPlaces = 0 : 5;",
            ),
            "C",
            "Run",
        ),
        "50733",
    );
}

/// Neighbors of a modelled type keep main's text: the control's type 1 over a field of an unknown type (the control's own type
/// decides, pages/run-modal/P113AutoFormat.al PovT0), type 11 with its format string (format-evaluate/autoformat/
/// TestAutoFormat.al:78-90), type 0 and no type.
#[test]
fn modelled_types_keep_their_text() {
    let run = |ctl: &str, field: &str| al_testkit::text(al_testkit::run_in(&page(ctl, field), "C", "Run"));
    let got = [
        run("AutoFormatType = 1;", "AutoFormatType = 50733;"),
        run(
            "AutoFormatType = 11; AutoFormatExpression = '<Precision,3:3><Standard Format,0>';",
            "",
        ),
        run("AutoFormatType = 0;", ""),
        run("", ""),
    ];
    assert_eq!(got, ["3,456.68", "3,456.678", "3,456.68", "3,456.68"]);
}

/// `Format` in code of a Decimal field with an unknown AutoFormatType is refused before the test runs, as StrSubstNo was.
#[test]
fn format_in_code_of_a_field_of_an_unknown_type_is_refused() {
    let src = r#"
table 69465 "AfU Code Row"
{
    fields { field(1; Id; Integer) { } field(2; Amt; Decimal) { AutoFormatType = 50733; } field(3; Lit; Decimal) { AutoFormatType = 7; AutoFormatExpression = '<Precision,1:1>'; } }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69466 C
{
    procedure Plain(): Text
    var R: Record "AfU Code Row";
    begin
        R.Amt := 3456.67843;
        exit(Format(R.Amt));
    end;

    procedure WithLiteral(): Text
    var R: Record "AfU Code Row";
    begin
        R.Lit := 3456.67843;
        exit(Format(R.Lit, 0, 1));
    end;

    procedure Subst(): Text
    var R: Record "AfU Code Row";
    begin
        R.Amt := 3456.67843;
        exit(StrSubstNo('%1', R.Amt));
    end;
}
"#;
    refused(al_testkit::run_in(src, "C", "Plain"), "50733");
    refused(al_testkit::run_in(src, "C", "WithLiteral"), "7");
    refused(al_testkit::run_in(src, "C", "Subst"), "50733");
}
