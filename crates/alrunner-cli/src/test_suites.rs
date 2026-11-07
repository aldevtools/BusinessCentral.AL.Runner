//! `--test-suites FILE` (ADR-066 amendment 6, al-runner issue 6924): reads the suites file and warns about listed
//! codeunits that are no test codeunit of the apps under test. The format and what a suite means are in
//! `al_testrun::test_suites`; reading the file is the front end's job (ADR-006).

use al_testrun::test_suites::TestSuites;
use std::path::Path;
use std::sync::Arc;

/// The suites of `path`, or `None` without the option. `Err`: the file cannot be read or has a bad line (exit 2).
/// `test_ids`: the test codeunits found in the apps under test; a listed codeunit that is none of them is ignored with a
/// warning on stderr.
pub fn load(path: Option<&Path>, test_ids: impl IntoIterator<Item = u32>) -> Result<Option<Arc<TestSuites>>, String> {
    let Some(path) = path else { return Ok(None) };
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("--test-suites: cannot read {}: {e}", path.display()))?;
    let suites = TestSuites::parse(&text).map_err(|e| format!("--test-suites {}: {e}", path.display()))?;
    let test_ids: Vec<u32> = test_ids.into_iter().collect();
    for w in unknown(&suites, &test_ids) {
        eprintln!("warning: --test-suites {}: {w}", path.display());
    }
    Ok(Some(Arc::new(suites)))
}

/// One warning per suite that lists codeunits that are no test codeunit of the apps under test.
pub fn unknown(suites: &TestSuites, test_ids: &[u32]) -> Vec<String> {
    suites
        .suites
        .iter()
        .filter_map(|s| {
            let missing: Vec<String> = s
                .codeunits
                .iter()
                .filter(|id| !test_ids.contains(id))
                .map(u32::to_string)
                .collect();
            (!missing.is_empty()).then(|| {
                format!(
                    "suite `{}` (line {}) lists {} that {} no test codeunit of the apps under test; ignored",
                    s.name,
                    s.line,
                    missing.join(", "),
                    if missing.len() == 1 { "is" } else { "are" }
                )
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_listed_codeunit_that_is_no_test_codeunit_is_a_warning_per_suite() {
        let s = TestSuites::parse("A: 1 2 3\nB: 2\nsuite  3: 9 8\n").unwrap();
        assert_eq!(
            unknown(&s, &[1, 2]),
            vec![
                "suite `A` (line 1) lists 3 that is no test codeunit of the apps under test; ignored".to_string(),
                "suite `suite  3` (line 3) lists 9, 8 that are no test codeunit of the apps under test; ignored"
                    .to_string(),
            ]
        );
    }

    #[test]
    fn no_option_is_no_suites_and_a_missing_file_is_an_error() {
        assert_eq!(load(None, []), Ok(None));
        let e = load(Some(Path::new("/nonexistent/suites.txt")), []).unwrap_err();
        assert!(
            e.starts_with("--test-suites: cannot read /nonexistent/suites.txt"),
            "{e}"
        );
    }

    /// ADR-066 amendment 6: `--test-suites FILE` takes a path, is off unless given, and is a per-request setting in
    /// `serve` (the `runTests` field `testSuites`), not a server option.
    #[test]
    fn the_option_takes_a_path_and_serve_refuses_it_as_a_server_option() {
        let parse = |s: &str| crate::args::parse(s.split_whitespace().map(String::from));
        let Ok(crate::args::Command::Test(t)) = parse("test A --test-suites /p/suites.txt") else {
            panic!()
        };
        assert_eq!(t.test_suites.as_deref(), Some(Path::new("/p/suites.txt")));
        let Ok(crate::args::Command::Test(t)) = parse("test A") else {
            panic!()
        };
        assert_eq!(t.test_suites, None);
        assert!(parse("test A --test-suites").unwrap_err().contains("needs a value"));
        assert!(parse("serve --test-suites /p/s.txt").is_err());
    }
}
