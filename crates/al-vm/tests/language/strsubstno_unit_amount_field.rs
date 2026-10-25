//! StrSubstNo, Message and Error `%n` of a unit-amount Decimal field (AutoFormatType 2) without an AutoFormatExpression or
//! with `Rec."Currency Code"` (al-runner issue 5827, the reporter's real-BC repro): the value shows the
//! unit-amount places a page control shows, the currency's, else the 2:5 of General Ledger Setup. Not pinned by the corpus
//! yet (assumption strsubstno-unit-amount-field, awaiting misc/lt-sa-unit-amount-subst/SaUnitAmountSubst.al); a setup of other
//! places is not run.

use al_builtins::assumptions::{Assumption, take_hits};
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
    assert!(take_hits().contains(&Assumption::StrSubstNoUnitAmountField));
}

#[test]
fn other_setup_places_and_no_setup_are_refused() {
    for p in ["UnitCostOtherSetup", "UnitPriceBlankOtherSetup", "UnitCostNoSetup"] {
        let r = run_in(SRC, "T", p);
        assert!(matches!(&r, Err(Signal::Unsupported(_))), "{p}: {r:?}");
    }
}
