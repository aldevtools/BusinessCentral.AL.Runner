//! `Evaluate` into a Duration (types/date-time/Evaluate.al:379-447, Duration_Evaluate; types/lt-filter-enum-relation-
//! duration-guid/D_Duration.al:420-444, Duration_Evaluate; format-evaluate/EvalLastError.al:257-264).

use al_builtins::native::Signal;

fn run(src: &str) -> Result<String, Signal> {
    al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| al_testkit::text(Ok(v)))
}

fn eval(text: &str, number: &str) -> String {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var D: Duration; begin D := 7; exit(Format(Evaluate(D, '{text}'{number})) + '|' + Format(D, 0, 9)); end; }}"
    );
    run(&src).unwrap()
}

#[test]
fn a_duration_reads_units_and_a_number_of_hours() {
    assert_eq!(eval("1 hour 30 minutes", ""), "Yes|P0DT1H30M0.0S");
    assert_eq!(eval("2 days", ""), "Yes|P2DT0H0M0.0S");
    assert_eq!(eval("1 seconds 5 milliseconds", ", 0"), "Yes|P0DT0H0M1.005S");
    assert_eq!(eval("1 minute 1 hour", ", 0"), "Yes|P0DT1H1M0.0S");
    assert_eq!(eval("1h", ", 0"), "Yes|P0DT1H0M0.0S");
    assert_eq!(eval("1.5 hours", ", 0"), "Yes|P0DT1H30M0.0S");
    assert_eq!(eval("-1 hour", ", 0"), "Yes|-P0DT1H0M0.0S");
    assert_eq!(eval("", ", 0"), "Yes|P0DT0H0M0.0S");
    assert_eq!(eval("1500", ""), "Yes|P62DT12H0M0.0S");
    assert_eq!(eval("1500", ", 3"), "Yes|P62DT12H0M0.0S");
    assert_eq!(eval("-1500", ""), "Yes|-P62DT12H0M0.0S");
    assert_eq!(eval("1.5", ""), "Yes|P0DT1H30M0.0S");
    assert_eq!(eval("5000000000", ""), "Yes|P208333333DT8H0M0.0S");
    assert_eq!(eval("P0DT1H30M0.0S", ", 9"), "Yes|P0DT1H30M0.0S");
}

#[test]
fn a_duration_rejects_what_is_no_duration_and_keeps_its_value() {
    assert_eq!(eval("abc", ""), "No|P0DT0H0M0.007S");
    assert_eq!(eval("1 hour 1 hour", ""), "No|P0DT0H0M0.007S");
    assert_eq!(eval("5000", ", 9"), "No|P0DT0H0M0.007S");
    assert_eq!(eval("1 hour", ", 9"), "No|P0DT0H0M0.007S");
    assert_eq!(eval("", ", 9"), "No|P0DT0H0M0.007S");
    // types/P131EvalScalars.al:144-200 (P131_EvaluateDuration_PlainNumbers).
    for text in ["+5", "12abc", "0 ms", "1,5", "1e3"] {
        assert_eq!(eval(text, ""), "No|P0DT0H0M0.007S", "{text}");
    }
}

#[test]
fn the_unpinned_duration_texts_are_refused() {
    for (text, number) in [
        ("1.5 days", ""),
        ("-1 day 2 hours", ""),
        ("P1D", ", 9"),
        ("1 hour", ", 2"),
        ("1500", ", 4"),
    ] {
        let src = format!(
            "codeunit 50100 C {{ procedure F(): Boolean var D: Duration; begin exit(Evaluate(D, '{text}'{number})); end; }}"
        );
        match run(&src) {
            Err(Signal::Unsupported(_)) => {}
            other => panic!("{text}: {other:?}"),
        }
    }
}

#[test]
fn a_time_plus_a_duration_beyond_integer_raises_a_conversion_error() {
    // types/duration/D_Duration.al:445-478 (TimePlusDuration `af373`).
    let src = "codeunit 50100 C { procedure F(): Text var T: Time; D: Duration; B: BigInteger; begin T := 100000T; \
               B := 50000; B := B * 100000; D := B; asserterror T := T + D; exit(GetLastErrorText() + '|' + GetLastErrorCode() + '|' + Format(T, 0, 9)); end; }";
    assert_eq!(
        run(src).unwrap(),
        "Overflow under conversion of System.Int64 value 5000000000 to System.Int32.|ConversionCast|10:00:00"
    );
}
