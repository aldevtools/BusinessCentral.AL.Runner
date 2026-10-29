//! An Option into a Decimal is its ordinal, a Decimal into an Option a whole number (al-runner issue 6060). Pinned by
//! types/lt-implicit-conversions/P79OptionDecimal.al.

use al_builtins::assumptions::take_hits;
use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 89961 "LT TL Rec"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Kind; Option) { OptionMembers = Salary,Bonus,Age; }
        field(3; Amount; Decimal) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50100 C
{
    local procedure TakeDecimal(D: Decimal): Text
    begin
        exit(Format(D));
    end;

    procedure FieldToField(): Text
    var
        Source: Record "LT TL Rec";
        Target: Record "LT TL Rec" temporary;
        R: Text;
    begin
        Source.Kind := Source.Kind::Age;
        Target.Amount := Source.Kind;
        R := Format(Target.Amount);
        Source.Kind := Source.Kind::Bonus;
        Target.Amount := Source.Kind;
        exit(R + '/' + Format(Target.Amount));
    end;

    procedure VarToVar(): Text
    var
        O: Option A,B,C;
        D: Decimal;
    begin
        O := O::C;
        D := O;
        exit(Format(D));
    end;

    procedure DecimalToOption(D: Decimal): Text
    var
        O: Option A,B,C;
    begin
        O := D;
        exit(Format(O) + '/' + Format(O, 0, 2));
    end;

    procedure Arithmetic(): Text
    var
        Source: Record "LT TL Rec";
        D: Decimal;
        O: Option A,B,C;
        R: Text;
    begin
        Source.Kind := Source.Kind::Age;
        O := O::C;
        D := Source.Kind * 2.5;
        R := Format(D);
        D := O / 4;
        R += '|' + Format(D);
        D := 0.5;
        D := D + Source.Kind;
        R += '|' + Format(D);
        D := 10;
        D -= O;
        R += '|' + Format(D);
        exit(R + '|' + Format(Source.Kind = 2));
    end;

    procedure ArgumentAndValidate(): Text
    var
        Source: Record "LT TL Rec";
        Target: Record "LT TL Rec";
        O: Option A,B,C;
    begin
        Source.Kind := Source.Kind::Age;
        O := O::B;
        Target.Validate(Amount, Source.Kind);
        exit(TakeDecimal(Source.Kind) + '|' + TakeDecimal(O) + '|' + Format(Target.Amount));
    end;

    procedure FormatAndEvaluate(): Text
    var
        Source: Record "LT TL Rec";
        D: Decimal;
        Ok: Boolean;
        R: Text;
    begin
        Source.Kind := Source.Kind::Age;
        D := Source.Kind;
        R := Format(D) + '|' + Format(Source.Kind);
        Ok := Evaluate(D, Format(Source.Kind, 0, 2));
        R += '|' + Format(Ok) + '/' + Format(D);
        Ok := Evaluate(D, Format(Source.Kind));
        exit(R + '|' + Format(Ok));
    end;
}
"#;

fn run(routine: &str) -> Result<Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

fn clean(routine: &str) -> String {
    take_hits();
    let r = al_testkit::text(run(routine));
    assert!(take_hits().is_empty(), "{routine} is flagged");
    r
}

#[test]
fn an_option_field_or_variable_into_a_decimal_is_its_ordinal() {
    // P79OptionDecimal.al:40-59 OptionField_ToDecimalField, :62-77 OptionVariable_ToDecimalVariable.
    assert_eq!(clean("FieldToField"), "2/1");
    assert_eq!(clean("VarToVar"), "2");
}

#[test]
fn a_decimal_into_an_option_takes_whole_numbers_and_raises_on_a_fraction() {
    // P79OptionDecimal.al:80-107 DecimalToOption_Assign.
    let call = |d: &str| {
        take_hits();
        let r = al_testkit::run_with(SRC, "C", "DecimalToOption", vec![Value::Decimal(d.parse().unwrap())]);
        assert!(take_hits().is_empty());
        r.map(|(v, _)| v)
    };
    assert_eq!(al_testkit::text(call("2")), "C/2");
    assert_eq!(al_testkit::text(call("0")), "A/0");
    assert_eq!(al_testkit::text(call("7")), "7/7");
    assert_eq!(al_testkit::text(call("-1")), "-1/-1");
    match call("1.5") {
        Err(Signal::Error(e)) => {
            assert_eq!(
                e.message,
                "Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 1.5 to System.Int32."
            );
            assert_eq!(e.code, "ConversionCast");
        }
        other => panic!("{other:?}"),
    }
    // Beyond Integer: not pinned.
    assert!(matches!(call("3000000000"), Err(Signal::Unsupported(_))));
}

#[test]
fn arithmetic_with_an_option_uses_its_ordinal() {
    // P79OptionDecimal.al:110-136 Arithmetic_WithOrdinal. Arithmetic with an Option ran before the issue; the
    // assignments here are Decimal to Decimal, so nothing is flagged.
    assert_eq!(al_testkit::text(run("Arithmetic")), "5|0.5|2.5|8|Yes");
}

#[test]
fn an_option_as_a_decimal_argument_and_in_validate() {
    // P79OptionDecimal.al:139-156 Argument_AndValidate.
    assert_eq!(clean("ArgumentAndValidate"), "2|1|2");
}

#[test]
fn format_and_evaluate_of_the_ordinal() {
    // P79OptionDecimal.al:159-179 FormatAndEvaluate_Ordinal.
    assert_eq!(clean("FormatAndEvaluate"), "2|Age|Yes/2|No");
}
