//! RecordRef load-field calls with field number 0 and unknown numbers among known ones
//! (recordref/lt-y71-loadfields/Y71RecRefLoadFields.al:293-525).

use al_testkit::{error_message, run, text};

const SRC: &str = r#"
table 60031 "LF Zero Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } field(3; Qty; Decimal) { } field(5; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count("LF Zero Row"); } }
    keys { key(PK; Id) { } }
}
codeunit 50141 C
{
    local procedure Flags(var RR: RecordRef): Text
    begin
        exit(Format(RR.AreFieldsLoaded(1)) + Format(RR.AreFieldsLoaded(2)) + Format(RR.AreFieldsLoaded(3)));
    end;

    procedure Zero(): Text
    var
        Row: Record "LF Zero Row";
        RR: RecordRef;
        R: Text;
    begin
        Row.Id := 1; Row.Name := 'one'; Row.Insert();
        RR.Open(60031);
        RR.SetLoadFields(0);
        RR.FindFirst();
        R := Flags(RR);
        RR.Close();
        RR.Open(60031);
        RR.SetLoadFields(2, 0, 0);
        RR.FindFirst();
        R += '|' + Flags(RR) + '|' + Format(RR.AreFieldsLoaded(0)) + Format(RR.AreFieldsLoaded(99)) + Format(RR.AreFieldsLoaded(2, 99)) + Format(RR.AreFieldsLoaded(2, 0));
        RR.AddLoadFields(3, 0);
        exit(R);
    end;

    procedure UnknownAmongKnown()
    var
        RR: RecordRef;
    begin
        RR.Open(60031);
        RR.SetLoadFields(2, 98, 99, 0);
    end;

    procedure AutoCalcZero()
    var
        RR: RecordRef;
    begin
        RR.Open(60031);
        RR.SetAutoCalcFields(5, 0);
    end;

    procedure AutoCalcNormal()
    var
        RR: RecordRef;
    begin
        RR.Open(60031);
        RR.SetAutoCalcFields(2, 0);
    end;
}
"#;

#[test]
fn zero_is_skipped_and_unknown_answers_no() {
    assert_eq!(text(run(SRC, "Zero")), "YesNoNo|YesYesNo|YesNoNoYes");
}

#[test]
fn first_unknown_number_is_named() {
    assert_eq!(
        error_message(run(SRC, "UnknownAmongKnown")),
        "The supplied field number '98' cannot be found in the 'LF Zero Row' table."
    );
}

#[test]
fn set_auto_calc_fields_zero_is_the_timestamp() {
    assert_eq!(
        error_message(run(SRC, "AutoCalcZero")),
        "The timestamp field in the LF Zero Row table must be a FlowField."
    );
    assert_eq!(
        error_message(run(SRC, "AutoCalcNormal")),
        "The Name field in the LF Zero Row table must be a FlowField."
    );
}
