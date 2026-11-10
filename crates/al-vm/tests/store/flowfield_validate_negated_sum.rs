//! Validate on a negated Sum FlowField and on a Decimal FlowField that is no sum (record/flowfields/FfmOperations.al:343-454,
//! FlowFieldValidate_ValueAndTrigger_Group1; al-runner issue 7422).

use al_testkit::{run_in, text};

const SRC: &str = r#"
table 69970 "NS Detail"
{
    fields { field(1; "Entry No."; Integer) { } field(2; Owner; Code[10]) { } field(3; Amt; Decimal) { } }
    keys { key(PK; "Entry No.") { } }
}
table 69971 "NS Header"
{
    fields
    {
        field(1; Code; Code[10]) { }
        field(101; NSumF; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = -sum("NS Detail".Amt where(Owner = field(Code)));
            trigger OnValidate() var L: Codeunit "NS Log"; begin L.Add(Format(NSumF)); end;
        }
        field(102; AvgF; Decimal) { FieldClass = FlowField; CalcFormula = average("NS Detail".Amt where(Owner = field(Code))); }
        field(104; CntF; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = count("NS Detail" where(Owner = field(Code)));
            trigger OnValidate() var L: Codeunit "NS Log"; begin L.Add(Format(CntF)); end;
        }
        field(105; MaxF; Decimal)
        {
            FieldClass = FlowField;
            CalcFormula = max("NS Detail".Amt where(Owner = field(Code)));
            trigger OnValidate() begin end;
        }
        field(103; NLkF; Decimal) { FieldClass = FlowField; CalcFormula = -lookup("NS Detail".Amt where(Owner = field(Code))); }
    }
    keys { key(PK; Code) { } }
}
codeunit 69972 "NS Log"
{
    SingleInstance = true;
    var T: Text;
    procedure Add(V: Text) begin T += V + ';'; end;
    procedure Get(): Text var R: Text; begin R := T; T := ''; exit(R); end;
}
codeunit 69973 T
{
    local procedure Seed()
    var D: Record "NS Detail"; H: Record "NS Header";
    begin
        D."Entry No." := 1; D.Owner := 'H1'; D.Amt := 10.5; D.Insert();
        D."Entry No." := 2; D.Owner := 'H1'; D.Amt := 20; D.Insert();
        D."Entry No." := 3; D.Owner := 'H0'; D.Amt := 4; D.Insert();
        D."Entry No." := 4; D.Owner := 'H0'; D.Amt := -4; D.Insert();
        H.Code := 'H1'; H.Insert();
        H.Code := 'H0'; H.Insert();
    end;

    // The sum of H0 is 0.
    procedure WithValue(): Text
    var H: Record "NS Header"; D: Record "NS Detail"; L: Codeunit "NS Log"; R: Text;
    begin
        Seed();
        H.Get('H0');
        H.CalcFields(NSumF);
        R := Format(H.NSumF) + '|';
        H.Validate(NSumF, 5.5);
        R += Format(H.NSumF) + '|' + L.Get() + '|';
        H.CalcFields(NSumF);
        D.FindLast();
        exit(R + Format(H.NSumF) + '|' + Format(D."Entry No.") + ' ' + D.Owner + ' ' + Format(D.Amt));
    end;

    // H1 holds 30.5: the FlowField is -30.5, and Validate without a value after Get takes 0.
    procedure WithoutValue(): Text
    var H: Record "NS Header"; D: Record "NS Detail"; L: Codeunit "NS Log"; R: Text;
    begin
        Seed();
        H.Get('H1');
        H.Validate(NSumF);
        R := L.Get() + '|';
        H.CalcFields(NSumF);
        D.FindLast();
        exit(R + Format(H.NSumF) + '|' + Format(D.Amt));
    end;

    procedure BothNotZero()
    var H: Record "NS Header";
    begin
        Seed();
        H.Get('H1');
        H.Validate(NSumF, 5.5);
    end;

    [TryFunction]
    local procedure TryAvg(var H: Record "NS Header") begin H.Validate(AvgF, 5.5); end;
    [TryFunction]
    local procedure TryNLk(var H: Record "NS Header") begin H.Validate(NLkF); end;

    procedure NotASum(): Text
    var H: Record "NS Header"; R: Text;
    begin
        Seed();
        H.Get('H1');
        H.CalcFields(AvgF);
        if not TryAvg(H) then R := GetLastErrorText() + '|' + Format(H.AvgF) + '|';
        if not TryNLk(H) then R += GetLastErrorText();
        exit(R);
    end;

    [TryFunction]
    local procedure TryCnt(var H: Record "NS Header") begin H.Validate(CntF, 7); end;
    [TryFunction]
    local procedure TryCntNoValue(var H: Record "NS Header") begin H.Validate(CntF); end;

    procedure CountWithTrigger(): Text
    var H: Record "NS Header"; L: Codeunit "NS Log"; R: Text;
    begin
        Seed();
        H.Get('H1');
        H.CalcFields(CntF);
        if not TryCnt(H) then R := GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(H.CntF) + '|' + L.Get() + '|';
        H.Get('H1');
        if not TryCntNoValue(H) then R += L.Get();
        exit(R);
    end;

    procedure CountValueAfterGet(): Integer
    var H: Record "NS Header";
    begin
        Seed();
        H.Get('H1');
        if not TryCnt(H) then exit(H.CntF);
    end;

    procedure MaxWithTrigger()
    var H: Record "NS Header";
    begin
        Seed();
        H.Get('H1');
        H.Validate(MaxF, 1);
    end;

    procedure NotASumCode(): Text
    var H: Record "NS Header";
    begin
        Seed();
        H.Get('H1');
        if not TryAvg(H) then exit(GetLastErrorCode());
    end;
}
"#;

// FfmOperations.al:381-390, :433-437: with the sum at 0 the value is held, OnValidate sees it, and CalcFields gives it
// afterwards: the new row takes the value with the sign turned.
#[test]
fn validate_with_a_value_writes_the_difference_with_the_sign_turned() {
    assert_eq!(text(run_in(SRC, "T", "WithValue")), "0|5.5|5.5;|5.5|5 H0 -5.5");
}

// A sum and a value that are both not 0 are not pinned.
#[test]
fn a_value_on_a_negated_sum_that_is_not_zero_is_refused() {
    let r = run_in(SRC, "T", "BothNotZero");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}

// FfmOperations.al:391-397, :438-439: Validate without a value takes the value the variable holds, 0 after Get.
#[test]
fn validate_without_a_value_takes_the_held_value() {
    assert_eq!(text(run_in(SRC, "T", "WithoutValue")), "0;|0|-30.5");
}

// FfmOperations.al:445-449 (AvgF), :721-725 (NLkDecF): the error text; the value stays.
#[test]
fn a_decimal_flowfield_that_is_no_sum_raises() {
    assert_eq!(
        text(run_in(SRC, "T", "NotASum")),
        "The Calculation formula for the AvgF FlowField in the NS Header table must start with 'Sum(...'.|15.25|The Calculation formula for the NLkF FlowField in the NS Header table must start with 'Sum(...'."
    );
}

// The code of that error is not pinned: GetLastErrorCode is refused.
#[test]
fn the_code_of_the_formula_error_is_refused() {
    assert!(matches!(
        run_in(SRC, "T", "NotASumCode"),
        Err(al_builtins::native::Signal::Unsupported(_))
    ));
}

// FfmOperations.al:520-566 (CntF): OnValidate runs with the new value, then the error; the variable holds the calculated
// value again. Without a value the trigger sees the held value, 0 after Get.
#[test]
fn a_count_flowfield_runs_onvalidate_before_the_error() {
    assert_eq!(
        text(run_in(SRC, "T", "CountWithTrigger")),
        "The type of the CntF FlowField in the NS Header table must be Decimal.|FlowFieldValidation|2|7;|0;"
    );
}

// The variable holds the value from before the call again, also one that was never calculated (DiFwWriteTests.al:877-878, :908).
#[test]
fn the_value_after_a_failed_validate_is_the_old_one() {
    assert_eq!(al_testkit::int(run_in(SRC, "T", "CountValueAfterGet")), 0);
}

// Whether the OnValidate of a Decimal FlowField that is no sum runs before the error is not pinned.
#[test]
fn a_max_flowfield_with_a_trigger_is_refused() {
    let r = run_in(SRC, "T", "MaxWithTrigger");
    assert!(matches!(r, Err(al_builtins::native::Signal::Unsupported(_))), "{r:?}");
}
