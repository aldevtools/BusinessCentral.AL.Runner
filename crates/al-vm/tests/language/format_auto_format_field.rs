//! `Format` in code of a Decimal table field whose AutoFormatExpression is an expression: BC applies the field's AutoFormat
//! outside a page too (types/lt-auto-format-currency/QiTableFieldAutoFormat.al:400-418 QI_TableFieldExpression_FormatInCode:
//! a currency of 3:3 / 4:4 places gives `1,234.568` for type 1, `1,234.5679` for type 2, and the procedure of type 10 that
//! answers `<Precision,3:3><Standard Format,0>` gives `1,234.568`).

use al_testkit::text;

const SRC: &str = r#"
table 4 Currency
{
    fields { field(1; Code; Code[10]) { } field(2; "Amount Decimal Places"; Text[5]) { } field(3; "Unit-Amount Decimal Places"; Text[5]) { } }
    keys { key(PK; Code) { } }
}
table 98 "General Ledger Setup"
{
    fields { field(1; "Primary Key"; Code[10]) { } field(2; "Amount Decimal Places"; Text[5]) { } field(3; "Unit-Amount Decimal Places"; Text[5]) { } }
    keys { key(PK; "Primary Key") { } }
}
table 86820 "LT QI AF Row"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Currency Code"; Code[10]) { }
        field(3; Amt1; Decimal) { AutoFormatType = 1; AutoFormatExpression = Rec."Currency Code"; }
        field(4; Amt2; Decimal) { AutoFormatType = 2; AutoFormatExpression = Rec."Currency Code"; }
        field(5; Amt4; Decimal) { AutoFormatType = 4; AutoFormatExpression = Rec."Currency Code"; }
        field(7; Amt10; Decimal) { AutoFormatType = 10; AutoFormatExpression = LoggedFormat(); }
    }
    keys { key(PK; "No.") { } }
    procedure LoggedFormat(): Text
    begin
        exit('<Precision,3:3><Standard Format,0>');
    end;
}
codeunit 86822 "QI AF Test"
{
    local procedure Seed()
    var
        GL: Record "General Ledger Setup";
        Cur: Record Currency;
        Row: Record "LT QI AF Row";
    begin
        GL."Amount Decimal Places" := '2:2';
        GL."Unit-Amount Decimal Places" := '2:5';
        GL.Insert();
        Cur.Code := 'QIC';
        Cur."Amount Decimal Places" := '3:3';
        Cur."Unit-Amount Decimal Places" := '4:4';
        Cur.Insert();
        Row."No." := 'A';
        Row."Currency Code" := 'QIC';
        Row.Amt1 := 1234.56789;
        Row.Amt2 := 1234.56789;
        Row.Amt4 := 1234.56789;
        Row.Amt10 := 1234.56789;
        Row.Insert();
    end;

    procedure InCode(): Text
    var
        Row: Record "LT QI AF Row";
    begin
        Seed();
        Row.Get('A');
        exit(Format(Row.Amt1) + '|' + Format(Row.Amt2) + '|' + Format(Row.Amt10));
    end;

    procedure TypeFour(): Text
    var
        Row: Record "LT QI AF Row";
    begin
        Seed();
        Row.Get('A');
        exit(Format(Row.Amt4));
    end;
}
"#;

#[test]
fn format_in_code_applies_the_fields_auto_format_expression() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "QI AF Test", "InCode")),
        "1,234.568|1,234.5679|1,234.568"
    );
}

/// AutoFormatType 4 with a field of the row takes the amount places of the currency, as type 1 does
/// (misc/lt-sa-unit-amount-subst/SaUnitAmountSubst.al `f2A4`).
#[test]
fn format_in_code_of_auto_format_type_four_takes_the_amount_places() {
    assert_eq!(text(al_testkit::run_in(SRC, "QI AF Test", "TypeFour")), "1,234.568");
}
