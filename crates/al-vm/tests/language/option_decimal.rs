//! An Option against a Decimal: `=`, `<>`, `<`, `>`, `<=`, `>=` compare the ordinal with the Decimal, in both orders
//! (corpus types/PROptionInteger.al:175-218 QS_OptionVariable_VersusDecimal, :220-251 QS_OptionMember_VersusDecimalConstant,
//! :253-271 QS_OptionField_VersusDecimalField); `case D of O::X:` matches by ordinal, a fraction matches no member
//! (:273-305 QS_Case_DecimalSelector_OptionLabel); `case O of D:` with D = 1 matches (:308-322
//! QS_Case_OptionSelector_DecimalVariableLabel). A Decimal label with a fraction against an Option selector is not pinned
//! (the corpus notes an overflow error on the Linux container) and stays refused.

use al_testkit::text;

const SRC: &str = r#"
codeunit 69790 C
{
    local procedure Cmp(O: Option None,Quantity,Amount; D: Decimal): Text
    begin
        exit(StrSubstNo('=%1 <>%2 <%3 >%4 <=%5 >=%6 rev=%7 rev<%8', O = D, O <> D, O < D, O > D, O <= D, O >= D, D = O, D < O));
    end;

    procedure Vars(): Text
    var O: Option None,Quantity,Amount;
    begin
        O := O::Quantity;
        exit(Cmp(O, 1) + '|' + Cmp(O, 1.5) + '|' + Cmp(O, 0.99999));
    end;

    procedure Members(): Text
    var O: Option None,Quantity,Amount; D: Decimal;
    begin
        D := 1.25;
        exit(Format(O::Quantity = 1.0) + Format(O::Quantity < 1.5) + Format(O::None <= -0.1) + Format(D > O::Amount));
    end;

    procedure CaseDecimal(): Text
    var O: Option None,Quantity,Amount; D: Decimal; Values: List of [Decimal]; T: Text;
    begin
        Values.Add(0); Values.Add(1); Values.Add(1.5); Values.Add(2);
        foreach D in Values do
            case D of
                O::None: T += 'None;';
                O::Quantity: T += 'Quantity;';
                O::Amount: T += 'Amount;';
                else T += 'else;';
            end;
        exit(T);
    end;

    procedure CaseOption(D: Decimal): Text
    var O: Option None,Quantity,Amount;
    begin
        O := O::Quantity;
        case O of
            D: exit('D');
            else exit('else');
        end;
    end;
}
"#;

#[test]
fn option_and_decimal_compare_by_ordinal() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "Vars")),
        "=Yes <>No <No >No <=Yes >=Yes rev=Yes rev<No|=No <>Yes <Yes >No <=Yes >=No rev=No rev<No|=No <>Yes <No >Yes <=No >=Yes rev=No rev<Yes"
    );
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Members")), "YesYesNoNo");
}

#[test]
fn case_of_a_decimal_with_option_labels() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "CaseDecimal")),
        "None;Quantity;else;Amount;"
    );
}

#[test]
fn case_of_an_option_with_a_decimal_label() {
    use al_builtins::value::Value;
    use rust_decimal::Decimal;
    let call = |d: Decimal| {
        let mut b = al_testkit::Built::from_source(SRC);
        let (oi, ri) = b.routine_in("C", "CaseOption");
        b.call(oi, ri, vec![Value::Decimal(d)]).map(|(v, _)| v)
    };
    assert_eq!(text(call(Decimal::new(1, 0))), "D");
    assert_eq!(text(call(Decimal::new(0, 0))), "else");
    match call(Decimal::new(15, 1)) {
        Err(al_builtins::native::Signal::Unsupported(why)) => assert!(why.contains("fraction"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
