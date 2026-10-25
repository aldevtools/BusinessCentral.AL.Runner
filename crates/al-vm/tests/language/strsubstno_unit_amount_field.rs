//! StrSubstNo, Message and Error `%n` of a Decimal field with an AutoFormat give the text of `Format(Rec.Field)`
//! (misc/lt-sa-unit-amount-subst/SaUnitAmountSubst.al, al-runner issue 5827): AutoFormatType 1 and 4 take the amount places,
//! 2 and 5 the unit-amount places, of the currency the expression names (a field of the row or a constant code), else of
//! General Ledger Setup; AutoFormatType 2 wins over DecimalPlaces. Without an expression the setup holds the places the
//! probe measured (2:2 and 2:5); other setup places are not run.

use al_builtins::assumptions::take_hits;
use al_builtins::native::Signal;
use al_testkit::{run_in, text};

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
table 50100 "Repro Price Line"
{
    fields
    {
        field(1; "Line No."; Integer) { }
        field(2; "Currency Code"; Code[10]) { }
        field(3; "Unit Price"; Decimal) { AutoFormatType = 2; AutoFormatExpression = Rec."Currency Code"; }
        field(4; "Unit Cost"; Decimal) { AutoFormatType = 2; }
        field(5; "Amt4"; Decimal) { AutoFormatType = 4; AutoFormatExpression = Rec."Currency Code"; }
        field(6; "Amt5"; Decimal) { AutoFormatType = 5; AutoFormatExpression = Rec."Currency Code"; }
        field(7; "Amt5No"; Decimal) { AutoFormatType = 5; }
        field(8; "Amt1Lit"; Decimal) { AutoFormatType = 1; AutoFormatExpression = 'RPX'; }
        field(9; "Amt2Lit"; Decimal) { AutoFormatType = 2; AutoFormatExpression = 'RPX'; }
        field(10; "Amt2Dp"; Decimal) { AutoFormatType = 2; AutoFormatExpression = Rec."Currency Code"; DecimalPlaces = 0 : 1; }
    }
    keys { key(PK; "Line No.") { } }
}
codeunit 50101 T
{
    local procedure Setup(UnitPlaces: Text)
    var
        GeneralLedgerSetup: Record "General Ledger Setup";
        Currency: Record Currency;
    begin
        GeneralLedgerSetup."Amount Decimal Places" := '2:2';
        GeneralLedgerSetup."Unit-Amount Decimal Places" := UnitPlaces;
        GeneralLedgerSetup.Insert();
        Currency.Code := 'RPX';
        Currency."Amount Decimal Places" := '2:2';
        Currency."Unit-Amount Decimal Places" := '3:3';
        Currency.Insert();
        AddLine(1, '', 1234.5, 7.2);
        AddLine(2, 'RPX', 1234.5, 7.2);
    end;

    local procedure AddLine(LineNo: Integer; CurrencyCode: Code[10]; UnitPrice: Decimal; UnitCost: Decimal)
    var
        PriceLine: Record "Repro Price Line";
    begin
        PriceLine."Line No." := LineNo;
        PriceLine."Currency Code" := CurrencyCode;
        PriceLine."Unit Price" := UnitPrice;
        PriceLine."Unit Cost" := UnitCost;
        PriceLine.Amt4 := UnitPrice;
        PriceLine.Amt5 := UnitPrice;
        PriceLine.Amt5No := UnitPrice;
        PriceLine.Amt1Lit := UnitPrice;
        PriceLine.Amt2Lit := UnitPrice;
        PriceLine.Amt2Dp := UnitPrice;
        PriceLine.Insert();
    end;

    procedure UnitPriceWithBlankCurrency(): Text
    var PriceLine: Record "Repro Price Line";
    begin
        Setup('2:5');
        PriceLine.Get(1);
        exit(StrSubstNo('%1', PriceLine."Unit Price"));
    end;

    procedure UnitPriceWithCurrency(): Text
    var PriceLine: Record "Repro Price Line";
    begin
        Setup('2:5');
        PriceLine.Get(2);
        exit(StrSubstNo('%1', PriceLine."Unit Price"));
    end;

    procedure UnitCostWithoutExpression(): Text
    var PriceLine: Record "Repro Price Line";
    begin
        Setup('2:5');
        PriceLine.Get(2);
        exit(StrSubstNo('%1', PriceLine."Unit Cost"));
    end;

    procedure ErrorOfUnitPrice(): Text
    var PriceLine: Record "Repro Price Line";
    begin
        Setup('2:5');
        PriceLine.Get(2);
        asserterror Error('x=%1', PriceLine."Unit Price");
        exit(GetLastErrorText());
    end;

    procedure OtherKinds(LineNo: Integer): Text
    var PriceLine: Record "Repro Price Line";
    begin
        Setup('2:5');
        PriceLine.Get(LineNo);
        exit(StrSubstNo('%1|%2|%3|%4|%5|%6', PriceLine.Amt4, PriceLine.Amt5, PriceLine.Amt5No, PriceLine.Amt1Lit, PriceLine.Amt2Lit, PriceLine.Amt2Dp)
            + '/' + Format(PriceLine.Amt4) + '|' + Format(PriceLine.Amt5) + '|' + Format(PriceLine.Amt5No) + '|' + Format(PriceLine.Amt1Lit)
            + '|' + Format(PriceLine.Amt2Lit) + '|' + Format(PriceLine.Amt2Dp) + '|' + Format(PriceLine."Unit Cost"));
    end;

    procedure OtherKinds1(): Text
    begin
        exit(OtherKinds(1));
    end;

    procedure OtherKinds2(): Text
    begin
        exit(OtherKinds(2));
    end;

    procedure UnitCostOtherSetup(): Text
    var PriceLine: Record "Repro Price Line";
    begin
        Setup('3:4');
        PriceLine.Get(2);
        exit(StrSubstNo('%1', PriceLine."Unit Cost"));
    end;

    procedure UnitPriceBlankOtherSetup(): Text
    var PriceLine: Record "Repro Price Line";
    begin
        Setup('3:4');
        PriceLine.Get(1);
        exit(StrSubstNo('%1', PriceLine."Unit Price"));
    end;

    procedure UnitCostNoSetup(): Text
    var PriceLine: Record "Repro Price Line";
    begin
        AddLine(1, '', 1234.5, 7.2);
        PriceLine.Get(1);
        exit(StrSubstNo('%1', PriceLine."Unit Cost"));
    end;
}
"#;

#[test]
fn a_unit_amount_field_is_substituted_with_its_unit_amount_places() {
    take_hits();
    assert_eq!(text(run_in(SRC, "T", "UnitPriceWithBlankCurrency")), "1,234.50");
    assert_eq!(text(run_in(SRC, "T", "UnitPriceWithCurrency")), "1,234.500");
    assert_eq!(text(run_in(SRC, "T", "UnitCostWithoutExpression")), "7.20");
    assert_eq!(text(run_in(SRC, "T", "ErrorOfUnitPrice")), "x=1,234.500");
    assert!(take_hits().is_empty());
}

// SaUnitAmountSubst.al SA_UA_Subst_1234_5 and SA_UA_Format_1234_5 (`s1`/`s2` and `f1`/`f2`, the newest major): types 4 and 5,
// a constant currency code, AutoFormatType 2 beside DecimalPlaces, and Format equal to StrSubstNo.
#[test]
fn amount_kinds_constant_codes_and_format_match_the_probe() {
    assert_eq!(
        text(run_in(SRC, "T", "OtherKinds1")),
        "1,234.50|1,234.50|1,234.50|1,234.50|1,234.500|1,234.50/1,234.50|1,234.50|1,234.50|1,234.50|1,234.500|1,234.50|7.20"
    );
    assert_eq!(
        text(run_in(SRC, "T", "OtherKinds2")),
        "1,234.50|1,234.500|1,234.50|1,234.50|1,234.500|1,234.500/1,234.50|1,234.500|1,234.50|1,234.50|1,234.500|1,234.500|7.20"
    );
}

#[test]
fn other_setup_places_and_no_setup_are_refused() {
    for p in ["UnitCostOtherSetup", "UnitPriceBlankOtherSetup", "UnitCostNoSetup"] {
        let r = run_in(SRC, "T", p);
        assert!(matches!(&r, Err(Signal::Unsupported(_))), "{p}: {r:?}");
    }
}
