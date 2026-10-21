//! Round of a Duration (AL Runner issue 5581), pinned by types/lt-bc-round-duration/BcRoundDuration.al.

const SRC: &str = r#"
codeunit 50100 C
{
    [TryFunction]
    local procedure TryRound(D: Duration; Precision: Decimal; Direction: Text; var R: Decimal)
    begin
        if Direction = '' then
            R := Round(D, Precision)
        else
            R := Round(D, Precision, Direction);
    end;

    local procedure Rnd(D: Duration; Precision: Decimal; Direction: Text): Text
    var
        R: Decimal;
    begin
        if TryRound(D, Precision, Direction, R) then
            exit(Format(R, 0, 9));
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    [TryFunction]
    local procedure TryRoundToDuration(D: Duration; Precision: Decimal; var R: Duration)
    begin
        R := Round(D, Precision);
    end;

    local procedure RndDur(D: Duration; Precision: Decimal): Text
    var
        R: Duration;
    begin
        if TryRoundToDuration(D, Precision, R) then
            exit(Format(R) + ' = ' + Format(R, 0, 9));
        exit('error:' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    local procedure Row(D: Duration): Text
    begin
        exit(Rnd(D, 1, '') + ' ; ' + Rnd(D, 1000, '') + ' ; ' + Rnd(D, 1000, '<') + ' ; ' + Rnd(D, 1000, '>') + ' ; ' + Rnd(D, 1000, '=') + ' ; ' + Rnd(D, 0.5, '') + ' ; ' + Rnd(D, 60000, ''));
    end;

    procedure Repro(): Text
    var
        Value: Duration;
    begin
        if Round(Value, 1) <> 0 then
            exit('differs');
        exit('zero');
    end;

    procedure Rows(): Text
    var
        D: Duration;
        R: Text;
    begin
        R := Row(D);
        D := 1500; R += '\' + Row(D);
        D := 2500; R += '\' + Row(D);
        D := -1500; R += '\' + Row(D);
        D := -2500; R += '\' + Row(D);
        D := 1234567; R += '\' + Row(D);
        D := 1; R += '\' + Row(D);
        exit(R);
    end;

    procedure Errors(): Text
    var
        D: Duration;
        R: Text;
    begin
        D := 1499;
        R := Format(Round(D), 0, 9);
        D := 1500;
        R += '\' + Format(Round(D), 0, 9);
        R += '\' + Rnd(D, 0, '');
        R += '\' + Rnd(D, -1000, '');
        R += '\' + Rnd(D, 1000, 'x');
        exit(R);
    end;

    procedure ResultType(): Text
    var
        D: Duration;
        Dec: Decimal;
        V: Variant;
        R: Text;
    begin
        D := 2500;
        V := Round(D, 1000);
        R := Format(V.IsDecimal()) + Format(V.IsDuration()) + Format(V.IsInteger()) + Format(V.IsBigInteger());
        Dec := Round(D, 1000);
        R += '\' + Format(Dec, 0, 9);
        R += '\' + RndDur(D, 1000);
        R += '\' + RndDur(D, 1);
        D := 1;
        R += '\' + RndDur(D, 0.3);
        D := -3723004;
        R += '\' + RndDur(D, 60000);
        exit(R);
    end;
}
"#;

fn run(routine: &str) -> Vec<String> {
    al_testkit::text(al_testkit::run_in(SRC, "C", routine))
        .split('\\')
        .map(str::to_string)
        .collect()
}

/// BcRoundDuration.al:51 (Round_Zero_Duration_Repro).
#[test]
fn a_zero_duration_rounds_to_zero() {
    assert_eq!(run("Repro"), ["zero"]);
}

/// BcRoundDuration.al:61-88 (Round_Durations_PrecisionsAndDirections).
#[test]
fn durations_round_by_precision_and_direction() {
    assert_eq!(
        run("Rows"),
        [
            "0 ; 0 ; 0 ; 0 ; 0 ; 0 ; 0",
            "1500 ; 2000 ; 1000 ; 2000 ; 2000 ; 1500 ; 0",
            "2500 ; 3000 ; 2000 ; 3000 ; 3000 ; 2500 ; 0",
            "-1500 ; -2000 ; -1000 ; -2000 ; -2000 ; -1500 ; 0",
            "-2500 ; -3000 ; -2000 ; -3000 ; -3000 ; -2500 ; 0",
            "1234567 ; 1235000 ; 1234000 ; 1235000 ; 1235000 ; 1234567 ; 1260000",
            "1 ; 0 ; 0 ; 1000 ; 0 ; 1 ; 0",
        ]
    );
}

/// BcRoundDuration.al:91-109 (Round_Duration_DefaultPrecision_And_Errors).
#[test]
fn round_of_a_duration_default_precision_and_errors() {
    assert_eq!(
        run("Errors"),
        [
            "1499",
            "1500",
            "error:The value of ROUND parameter 2 is outside of the permitted range. The current value is: 0. The permitted range is: from 1 to 2,147,483,647.|OutsidePermittedRange",
            "error:The value of ROUND parameter 2 is outside of the permitted range. The current value is: 1,000. The permitted range is: from 1 to 2,147,483,647.|OutsidePermittedRange",
            "error:The supplied value is not valid.|InvalidValue",
        ]
    );
}

/// BcRoundDuration.al:112-143 (Round_Duration_ResultType): the result is a Decimal.
#[test]
fn round_of_a_duration_is_a_decimal() {
    assert_eq!(
        run("ResultType"),
        [
            "YesNoNoNo",
            "3000",
            "3 seconds = P0DT0H0M3.0S",
            "2 seconds 500 milliseconds = P0DT0H0M2.5S",
            "error:Overflow under conversion of Microsoft.Dynamics.Nav.Runtime.Decimal18 value 0.9 to System.Int64.|ConversionCast",
            "-1 hour 2 minutes = -P0DT1H2M0.0S",
        ]
    );
}
