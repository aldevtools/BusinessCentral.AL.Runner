//! `--dev-endpoint-ui` (issue 3697) in the output: a warning per UI call that got the dev-endpoint runner's answer,
//! and the mark on the test in --junit and --output-json. The pass is also flagged by the assumption `dev-endpoint-ui`,
//! so `Tests:` counts it apart from the clean passes.
use al_testrun::runtime::TestResult;

/// One `warning:` line per answered UI call, naming the test, the call and the answer, in the order of the results.
pub fn warnings(results: &[&TestResult]) -> Vec<String> {
    results
        .iter()
        .flat_map(|r| warnings_of(r.codeunit, &r.test, &r.ui_answered))
        .collect()
}

/// The `warning:` lines of one test.
pub fn warnings_of(codeunit: u32, test: &str, answered: &[String]) -> Vec<String> {
    answered
        .iter()
        .map(|a| {
            format!(
                "warning: dev-endpoint UI: Codeunit{codeunit}.{test} had no handler for {a} (answered as the dev-endpoint test runner does)"
            )
        })
        .collect()
}

/// The JUnit property of a test with answered UI calls: `dev_endpoint_ui`, the calls joined by `; `.
pub fn junit_property(r: &TestResult) -> Option<(String, String)> {
    (!r.ui_answered.is_empty()).then(|| ("dev_endpoint_ui".to_string(), r.ui_answered.join("; ")))
}

/// `uiAnswered` on the --output-json row of a test with answered UI calls.
pub fn add_json(row: &mut serde_json::Map<String, serde_json::Value>, answered: &[String]) {
    if !answered.is_empty() {
        row.insert("uiAnswered".into(), serde_json::json!(answered));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_testrun::Outcome;

    fn answered(test: &str, calls: &[&str]) -> TestResult {
        TestResult {
            suite: 0,
            app: 0,
            codeunit: 50100,
            codeunit_name: "Dev".into(),
            test: test.into(),
            outcome: Outcome::Pass,
            call_stack: String::new(),
            assumptions: if calls.is_empty() {
                Vec::new()
            } else {
                vec![al_testrun::assumptions::Assumption::DevEndpointUi]
            },
            time: std::time::Duration::ZERO,
            coverage: Vec::new(),
            messages: Vec::new(),
            captured: None,
            impact: Vec::new(),
            stubs: Vec::new(),
            ui_answered: calls.iter().map(|c| c.to_string()).collect(),
            slept_ms: 0,
            entered: Vec::new(),
            dotnet: false,
        }
    }

    #[test]
    fn each_answered_call_is_a_warning_and_the_test_is_marked_in_junit_and_json() {
        let a = answered("Asks", &["Confirm 'Go on?': false", "Message Done: no answer"]);
        let clean = answered("Clean", &[]);
        let rs = [&a, &clean];
        assert_eq!(
            warnings(&rs),
            vec![
                "warning: dev-endpoint UI: Codeunit50100.Asks had no handler for Confirm 'Go on?': false (answered as the dev-endpoint test runner does)",
                "warning: dev-endpoint UI: Codeunit50100.Asks had no handler for Message Done: no answer (answered as the dev-endpoint test runner does)",
            ]
        );
        let x = crate::output::junit(&rs, None, 0.0, &|_, _| false, &[]);
        assert!(
            x.contains("<property name=\"assumptions\" value=\"dev-endpoint-ui\" /><property name=\"dev_endpoint_ui\" value=\"Confirm ")
                && x.contains(": false; Message Done: no answer\" />"),
            "{x}"
        );
        assert!(x.contains("name=\"Clean\" time=\"0.000\" />"), "{x}");
        let j: serde_json::Value = serde_json::from_str(&crate::output::json(&rs, 0, 0.0)).unwrap();
        assert_eq!(j["tests"][0]["flagged"], true);
        assert_eq!(j["tests"][0]["uiAnswered"][0], "Confirm 'Go on?': false");
        assert!(j["tests"][1].get("uiAnswered").is_none());
    }
}
