//! A decimal control with `AutoFormatType` 1, 2, 4 or 5 (on the control or on the table field). Pinned by
//! autoformat/PWAutoFormatTypes.al (BC 27.5): types 1 and 4 are amounts, 2 and 5 unit amounts. With no
//! currency (a blank expression) an amount has 2:2 places and a unit amount 2:5, whatever General Ledger
//! Setup says (:98-128, :131-175). A currency code that exists gives the places of the currency, its
//! "Amount Decimal Places" for the amounts and its "Unit-Amount Decimal Places" for the unit amounts
//! (:177-195); a currency with both places empty gives 2:2 for all four (:197-217); a code that is no
//! currency gives 2:5 for all four (:263-290).

use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const OBJECTS: &str = r#"
table 62500 "AF Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Amt; Decimal) { AutoFormatType = 1; AutoFormatExpression = ''; }
        field(3; Unit; Decimal) { AutoFormatType = 2; AutoFormatExpression = ''; }
        field(4; UnitNoCur; Decimal) { AutoFormatType = 5; AutoFormatExpression = ''; }
        field(5; Plain; Decimal) { }
    }
    keys { key(PK; "No.") { } }
}
page 62500 "AF Card"
{
    PageType = Card; SourceTable = "AF Row";
    layout { area(Content) {
        field(AmtCtl; Rec.Amt) { }
        field(UnitCtl; Rec.Unit) { }
        field(UnitNoCurCtl; Rec.UnitNoCur) { }
        field(PlainCtl; Rec.Plain) { }
    } }
}
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62501 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

const SEED: &str = "R: Record \"AF Row\"; P: TestPage \"AF Card\";";

fn open(values: &str) -> String {
    format!("R.\"No.\" := 'X'; {values} R.Insert(); P.OpenView(); P.GoToRecord(R);")
}

// autoformat/TestAutoFormat.al: AutoFormatType = 1 with a blank expression keeps two decimals, and
// 1234.5 reads 1,234.50 whatever the type.
#[test]
fn values_with_at_most_two_decimals_read_as_before() {
    let o = outcomes(&format!(
        "[Test] procedure Reads() var {SEED} begin {} \
         if P.AmtCtl.Value() <> '110.74' then Error('amt %1', P.AmtCtl.Value());
         if P.UnitCtl.Value() <> '1,234.50' then Error('unit %1', P.UnitCtl.Value());
         if P.UnitNoCurCtl.Value() <> '3.00' then Error('unit no cur %1', P.UnitNoCurCtl.Value()); end;",
        open("R.Amt := 110.74; R.Unit := 1234.5; R.UnitNoCur := 3;")
    ));
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// :98-128: a unit amount with a blank expression keeps up to five decimals, an amount two; a plain control
// two (autoformat/TestAutoFormat.al).
#[test]
fn a_unit_amount_without_a_currency_has_two_to_five_places() {
    let o = outcomes(&format!(
        "[Test] procedure Unit() var {SEED} begin {} \
         if P.UnitCtl.Value() <> '110.74398' then Error('unit %1', P.UnitCtl.Value());
         if P.UnitNoCurCtl.Value() <> '682.12312' then Error('unit no cur %1', P.UnitNoCurCtl.Value());
         if P.AmtCtl.Value() <> '110.74' then Error('amt %1', P.AmtCtl.Value());
         if P.PlainCtl.Value() <> '110.74' then Error('plain %1', P.PlainCtl.Value()); end;",
        open("R.Unit := 110.74398; R.UnitNoCur := 682.12312; R.Amt := 110.74398; R.Plain := 110.74398;")
    ));
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

const CURRENCY: &str = r#"
table 98 "General Ledger Setup"
{
    fields
    {
        field(1; "Primary Key"; Code[10]) { }
        field(73; "Amount Decimal Places"; Text[5]) { }
        field(74; "Unit-Amount Decimal Places"; Text[5]) { }
    }
    keys { key(PK; "Primary Key") { } }
}
table 4 Currency
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(15; Symbol; Text[10]) { }
        field(18; "Amount Decimal Places"; Text[5]) { }
        field(19; "Unit-Amount Decimal Places"; Text[5]) { }
    }
    keys { key(PK; Code) { } }
}
table 62510 "AF Cur Row"
{
    fields { field(1; "No."; Code[20]) { } field(2; Amount; Decimal) { } field(3; "Currency Code"; Code[10]) { } }
    keys { key(PK; "No.") { } }
}
page 62510 "AF Cur Card"
{
    PageType = Card; SourceTable = "AF Cur Row";
    layout { area(Content) {
        field(T1; Rec.Amount) { AutoFormatType = 1; AutoFormatExpression = Rec."Currency Code"; }
        field(T2; Rec.Amount) { AutoFormatType = 2; AutoFormatExpression = Rec."Currency Code"; }
        field(T4; Rec.Amount) { AutoFormatType = 4; AutoFormatExpression = Rec."Currency Code"; }
        field(T5; Rec.Amount) { AutoFormatType = 5; AutoFormatExpression = Rec."Currency Code"; }
        field(T1Blank; Rec.Amount) { AutoFormatType = 1; AutoFormatExpression = ''; }
        field(T2Blank; Rec.Amount) { AutoFormatType = 2; AutoFormatExpression = ''; }
    } }
}
"#;

fn currency_outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}{CURRENCY}\ncodeunit 62511 T {{ Subtype = Test; {tests} }}");
    let l = al_testkit::compile(&src);
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    run_codeunit(&engine, &l, oi)
}

/// A test that seeds the currency `Code` with the places, reads the page of a row with value `v`, and checks the
/// six controls (t1, t2, t4, t5, t1 blank, t2 blank).
fn arm(code: &str, amount_places: &str, unit_places: &str, row_currency: &str, v: &str, want: [&str; 6]) -> String {
    arm_with("2:5", "2:5", code, amount_places, unit_places, row_currency, v, want)
}

/// [`arm`] with the places of General Ledger Setup.
#[allow(clippy::too_many_arguments)]
fn arm_with(
    setup_amount: &str,
    setup_unit: &str,
    code: &str,
    amount_places: &str,
    unit_places: &str,
    row_currency: &str,
    v: &str,
    want: [&str; 6],
) -> String {
    format!(
        "[Test] procedure A{code}{row_currency}() var G: Record \"General Ledger Setup\"; C: Record Currency; R: Record \"AF Cur Row\"; P: TestPage \"AF Cur Card\"; begin
           G.\"Amount Decimal Places\" := '{setup_amount}'; G.\"Unit-Amount Decimal Places\" := '{setup_unit}'; if not G.Insert() then G.Modify();
           C.Code := '{code}'; C.\"Amount Decimal Places\" := '{amount_places}'; C.\"Unit-Amount Decimal Places\" := '{unit_places}'; C.Insert();
           R.\"No.\" := 'X{code}'; R.Amount := {v}; R.\"Currency Code\" := '{row_currency}'; R.Insert(); P.OpenView(); P.GoToRecord(R);
           if P.T1.Value() <> '{}' then Error('t1 %1', P.T1.Value());
           if P.T2.Value() <> '{}' then Error('t2 %1', P.T2.Value());
           if P.T4.Value() <> '{}' then Error('t4 %1', P.T4.Value());
           if P.T5.Value() <> '{}' then Error('t5 %1', P.T5.Value());
           if P.T1Blank.Value() <> '{}' then Error('t1 blank %1', P.T1Blank.Value());
           if P.T2Blank.Value() <> '{}' then Error('t2 blank %1', P.T2Blank.Value()); end;",
        want[0], want[1], want[2], want[3], want[4], want[5]
    )
}

// :177-195 (cur_never): the currency PWC has amount places 3:3 and unit places 4:4.
#[test]
fn a_currency_gives_its_own_amount_and_unit_places() {
    let o = currency_outcomes(&format!(
        "{}{}",
        arm(
            "PWC",
            "3:3",
            "4:4",
            "PWC",
            "110.74398",
            ["110.744", "110.7440", "110.744", "110.7440", "110.74", "110.74398"]
        ),
        arm(
            "PWF",
            "3:3",
            "4:4",
            "PWF",
            "1234.5",
            [
                "1,234.500",
                "1,234.5000",
                "1,234.500",
                "1,234.5000",
                "1,234.50",
                "1,234.50"
            ]
        )
    ));
    for (n, r) in &o {
        assert_eq!(r, &Outcome::Pass, "{n}: {o:?}");
    }
}

// :197-217 (cur_empty_fcy_symbol): a currency with both places empty reads 2:2 for all four types.
#[test]
fn a_currency_without_places_reads_two_places() {
    let o = currency_outcomes(&arm(
        "PWD",
        "",
        "",
        "PWD",
        "110.74398",
        ["110.74", "110.74", "110.74", "110.74", "110.74", "110.74398"],
    ));
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// :263-290 (cur_unknown): a code that is no currency reads the places of General Ledger Setup, 2:5 for all four types there.
#[test]
fn a_code_that_is_no_currency_reads_two_to_five_places() {
    let o = currency_outcomes(&arm(
        "PWE",
        "3:3",
        "4:4",
        "NOPE",
        "110.74398",
        [
            "110.74398",
            "110.74398",
            "110.74398",
            "110.74398",
            "110.74",
            "110.74398",
        ],
    ));
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

const FIELD_EXPRESSION: &str = r#"
table 62520 "AF Field Expr Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Amount; Decimal) { AutoFormatType = 4; AutoFormatExpression = Rec."Currency Code"; }
        field(3; UnitAmount; Decimal) { AutoFormatType = 5; AutoFormatExpression = "Currency Code"; }
        field(4; Fmt; Decimal) { AutoFormatType = 10; AutoFormatExpression = Pattern(); }
        field(5; "Currency Code"; Code[10]) { }
    }
    keys { key(PK; "No.") { } }
    procedure Pattern(): Text
    begin
        if "No." = 'BOOM' then Error('field format failed');
        exit('<Precision,1:1><Standard Format,0>');
    end;
}
page 62520 "AF Field Expr Card"
{
    PageType = Card; SourceTable = "AF Field Expr Row";
    layout { area(Content) {
        field(AmountCtl; Rec.Amount) { }
        field(UnitCtl; Rec.UnitAmount) { }
        field(FmtCtl; Rec.Fmt) { }
        field(OwnCtl; Rec.Amount) { AutoFormatType = 4; AutoFormatExpression = ''; }
    } }
}
"#;

// types/lt-auto-format-currency/QiTableFieldAutoFormat.al:272, :300: a table field's AutoFormatExpression that is a field of
// the row (`Rec."Currency Code"`, `"Currency Code"`), read through a control without one of its own, is evaluated with the
// control's row.
#[test]
fn a_table_field_expression_is_evaluated_with_the_row() {
    let src = format!(
        "{CURRENCY}{FIELD_EXPRESSION}\ncodeunit 62521 T {{
         procedure F(): Text var C: Record Currency; R: Record \"AF Field Expr Row\"; P: TestPage \"AF Field Expr Card\"; begin
           C.Code := 'PWC'; C.\"Amount Decimal Places\" := '3:3'; C.\"Unit-Amount Decimal Places\" := '4:4'; C.Insert();
           R.\"No.\" := 'X'; R.Amount := 248.02915; R.UnitAmount := 248.02915; R.Fmt := 2.25; R.\"Currency Code\" := 'PWC'; R.Insert();
           P.OpenView(); P.GoToRecord(R);
           if P.AmountCtl.Value() <> '248.029' then Error('amount %1', P.AmountCtl.Value());
           if P.UnitCtl.Value() <> '248.0292' then Error('unit %1', P.UnitCtl.Value());
           if P.OwnCtl.Value() <> '248.03' then Error('own %1', P.OwnCtl.Value());
           exit('ok'); end; }}"
    );
    assert_eq!(al_testkit::text(al_testkit::run_in(&src, "T", "F")), "ok");
}

// types/lt-auto-format-currency/QiTableFieldAutoFormat.al:272 QI_TableFieldExpression_Card_ShowCurrencyNever (`nocur_t1`
// 1,234.57, `nocur_t2` 1,234.56789 with General Ledger Setup 2:2 and 2:5): a code that is no currency reads General Ledger
// Setup's "Amount Decimal Places" for the amounts and its "Unit-Amount Decimal Places" for the unit amounts.
#[test]
fn a_code_that_is_no_currency_reads_the_places_of_general_ledger_setup() {
    let o = currency_outcomes(&arm_with(
        "2:2",
        "2:5",
        "PWE",
        "3:3",
        "4:4",
        "NOPE",
        "1234.56789",
        [
            "1,234.57",
            "1,234.56789",
            "1,234.57",
            "1,234.56789",
            "1,234.57",
            "1,234.56789",
        ],
    ));
    assert_eq!(o[0].1, Outcome::Pass, "{o:?}");
}

// A code that is no currency without a General Ledger Setup row: not pinned, refused.
#[test]
fn a_code_that_is_no_currency_without_a_setup_row_is_refused() {
    let o = currency_outcomes(
        "[Test] procedure NoSetup() var R: Record \"AF Cur Row\"; P: TestPage \"AF Cur Card\"; begin
           R.\"No.\" := 'X'; R.Amount := 1.5; R.\"Currency Code\" := 'NOPE'; R.Insert(); P.OpenView(); P.GoToRecord(R);
           if P.T1.Value() <> '' then; end;",
    );
    assert!(
        matches!(&o[0].1, Outcome::NotRun(why) if why.contains("General Ledger Setup")),
        "{o:?}"
    );
}

// types/lt-auto-format-currency/QiTableFieldAutoFormat.al:421 QI_Type10Procedure_Card_Calls (`after-open` fmt();fmt(A);,
// `after-t10-read` empty): BC runs a table field's AutoFormatExpression that calls a procedure when the page fetches the row,
// not when the control's text is read. The runtime evaluates it on read, so a procedure there is refused.
#[test]
fn a_table_field_expression_that_calls_a_procedure_is_refused() {
    let src = format!(
        "{CURRENCY}{FIELD_EXPRESSION}\ncodeunit 62521 T {{
         procedure F(): Text var R: Record \"AF Field Expr Row\"; P: TestPage \"AF Field Expr Card\"; begin
           R.\"No.\" := 'X'; R.Fmt := 2.25; R.Insert();
           P.OpenView(); P.GoToRecord(R);
           exit(P.FmtCtl.Value()); end; }}"
    );
    match al_testkit::run_in(&src, "T", "F") {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("calls a procedure"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
