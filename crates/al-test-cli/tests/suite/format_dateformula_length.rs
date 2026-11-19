//! al-runner issue 8198: `Format(DateFormula, Length)`. The reporter measured on BC 28.4 that `Format(DF, 4)` of
//! `1Y+1M+1D` is `1Y+1`; the runner gave the whole text. No corpus test pins a length on a DateFormula, so the runner
//! refuses it until a probe does. Without a length the text is pinned (types/TestDateRoundFormatStringContracts.al,
//! DateFormula_Evaluate_ThenFormat_PrintsTheNormalizedFormula).

use al_test_cli::report::Outcome;

fn one(test: &str) -> Outcome {
    al_testkit::outcomes_in(&format!("codeunit 63121 T {{ Subtype = Test; {test} }}"), "T")
        .remove(0)
        .1
}

// The reporter's FormatLengthTruncatesDateFormula (issue 8198), unchanged but for the ids.
#[test]
fn format_of_a_date_formula_with_a_length_is_refused() {
    let o = one(
        "[Test] procedure FormatLengthTruncatesDateFormula() var DF: DateFormula; begin
            Evaluate(DF, '<1Y+1M+1D>');
            if Format(DF, 4) <> '1Y+1' then Error('BC gives 1Y+1 for Format(DF, 4); got %1', Format(DF, 4)); end;",
    );
    assert!(
        matches!(&o, Outcome::NotRun(why) if why.contains("DateFormula")),
        "{o:?}"
    );
}

#[test]
fn format_of_a_date_formula_with_a_negative_or_longer_length_is_refused() {
    for len in ["-3", "20"] {
        let o = one(&format!(
            "[Test] procedure T() var DF: DateFormula; begin Evaluate(DF, '1Y+1M+1D'); if Format(DF, {len}) = '' then; end;"
        ));
        assert!(
            matches!(&o, Outcome::NotRun(why) if why.contains("DateFormula")),
            "{len}: {o:?}"
        );
    }
}

// The format-string overload applies the length itself; it is refused the same way (lengths 4, 12, -3).
#[test]
fn format_of_a_date_formula_with_a_length_and_a_format_string_is_refused() {
    for (len, n) in [("4", "0"), ("12", "2"), ("-3", "9")] {
        let o = one(&format!(
            "[Test] procedure T() var DF: DateFormula; begin Evaluate(DF, '1Y+1M+1D');
                if Format(DF, {len}, '<Standard Format,{n}>') = '' then; end;"
        ));
        assert!(
            matches!(&o, Outcome::NotRun(why) if why.contains("DateFormula")),
            "{len}, {n}: {o:?}"
        );
    }
}

// Main's result kept: a format string with length 0 (not pinned either, listed under needs probe).
#[test]
fn format_of_a_date_formula_with_length_0_and_a_format_string_keeps_main() {
    let o = one(
        "[Test] procedure T() var DF: DateFormula; begin Evaluate(DF, '1Y+1M+1D');
            if Format(DF, 0, '<Standard Format,0>') <> '1Y+1M+1D' then Error('got %1', Format(DF, 0, '<Standard Format,0>')); end;",
    );
    assert_eq!(o, Outcome::Pass);
}

// Main's result kept: no length.
#[test]
fn format_of_a_date_formula_without_a_length_is_its_text() {
    let o = one(
        "[Test] procedure T() var DF: DateFormula; begin Evaluate(DF, '<1Y+1M+1D>');
            if Format(DF) <> '1Y+1M+1D' then Error('got %1', Format(DF)); end;",
    );
    assert_eq!(o, Outcome::Pass);
}
