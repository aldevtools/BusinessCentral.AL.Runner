//! `ALRUNNER_REPORT_UNPINNED_ERRORS=1` (al-runner issue 6487, opt-in): a test that fails with an error whose text is not
//! pinned is reported as failed, its message starting with "[error text not pinned]", instead of not run. The default is
//! not run. Every output says the switch is on, because such a run is not a conformance result: the summary line, the
//! `--output-json` document (`reportUnpinnedErrors`, `unpinnedErrorFailures`) and each JUnit `testsuite` (a property).

use al_builtins::native::UNPINNED_FAILURE_MARK;
use al_testrun::Outcome;

/// The environment variable of the switch.
pub const VAR: &str = "ALRUNNER_REPORT_UNPINNED_ERRORS";

/// Whether the switch is on: the variable is `1`.
pub fn enabled() -> bool {
    on(std::env::var(VAR).ok().as_deref())
}

/// Whether a value of [`VAR`] turns the switch on: only `1`.
fn on(value: Option<&str>) -> bool {
    value == Some("1")
}

/// The failures the switch turned from not run into failed.
pub fn count<'a>(outcomes: impl IntoIterator<Item = &'a Outcome>) -> usize {
    outcomes.into_iter().filter(|o| reported(o)).count()
}

/// Appended to the summary line: nothing when the switch is off.
pub fn summary_note(enabled: bool, failures: usize) -> String {
    if !enabled {
        return String::new();
    }
    format!(
        " ({VAR}=1: {failures} failed with an error text that is not pinned, reported instead of not run; not a conformance result)"
    )
}

/// The `--output-json` document with the switch's fields; unchanged when the switch is off.
pub fn json(doc: String, enabled: bool, failures: usize) -> String {
    if !enabled {
        return doc;
    }
    match serde_json::from_str::<serde_json::Value>(&doc) {
        Ok(mut d) if d.is_object() => {
            d["reportUnpinnedErrors"] = true.into();
            d["unpinnedErrorFailures"] = failures.into();
            serde_json::to_string_pretty(&d).expect("a JSON value serializes")
        }
        _ => doc,
    }
}

/// The JUnit file with the property `report_unpinned_errors` on every `testsuite`; unchanged when the switch is off.
pub fn junit(xml: &str, enabled: bool) -> String {
    if !enabled {
        return xml.to_string();
    }
    crate::seed::with_properties(xml, "<property name=\"report_unpinned_errors\" value=\"1\" />")
}

/// Whether `outcome` is a failure the switch reported.
fn reported(outcome: &Outcome) -> bool {
    matches!(outcome, Outcome::Fail(m) if m.starts_with(UNPINNED_FAILURE_MARK))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_turns_it_on() {
        assert!(on(Some("1")));
        for v in [None, Some(""), Some("0"), Some("true"), Some(" 1")] {
            assert!(!on(v), "{v:?}");
        }
    }

    #[test]
    fn the_summary_says_the_switch_is_on_and_off_adds_nothing() {
        assert_eq!(summary_note(false, 3), "");
        let note = summary_note(true, 2);
        assert!(note.contains("ALRUNNER_REPORT_UNPINNED_ERRORS=1"), "{note}");
        assert!(
            note.contains("2 failed with an error text that is not pinned"),
            "{note}"
        );
        assert!(note.contains("not a conformance result"), "{note}");
        assert!(summary_note(true, 0).contains("ALRUNNER_REPORT_UNPINNED_ERRORS=1"));
    }

    #[test]
    fn json_and_junit_say_the_switch_is_on_and_off_change_nothing() {
        let doc = r#"{"passed": 0}"#.to_string();
        assert_eq!(json(doc.clone(), false, 1), doc);
        let d: serde_json::Value = serde_json::from_str(&json(doc, true, 1)).unwrap();
        assert_eq!(d["reportUnpinnedErrors"], true);
        assert_eq!(d["unpinnedErrorFailures"], 1);
        let xml = "<testsuites>\n  <testsuite name=\"T\" tests=\"1\">\n  </testsuite>\n</testsuites>";
        assert_eq!(junit(xml, false), xml);
        assert!(junit(xml, true).contains("<property name=\"report_unpinned_errors\" value=\"1\" />"));
    }

    #[test]
    fn only_marked_failures_count() {
        let results = [
            Outcome::Fail("[error text not pinned] probe".into()),
            Outcome::Fail("Plain failure".into()),
            Outcome::NotRun("The test result of an error whose text is not pinned".into()),
            Outcome::Pass,
        ];
        assert_eq!(count(&results), 1);
    }
}
