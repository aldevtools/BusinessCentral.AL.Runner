//! Test-suite boundaries as Business Central session boundaries (ADR-066 amendment 6, al-runner issue 6924).
//!
//! A pipeline such as ALOps runs one test-tool run per BC test suite, each in a client session of its own, so the
//! SingleInstance state one suite's test codeunits leave never reaches another suite. `--test-suites FILE` (`serve`:
//! the `runTests` field `testSuites`) tells al-runner where those boundaries are: each suite is its own BC session for
//! order dependence (`runtime/single_lane.rs`). This module only parses the file's text; the front end reads the file
//! (ADR-006).
//!
//! The format (`docs/manual/cli-reference.md`, "Test suites file"):
//!
//! - One suite per line, its test codeunit ids in the order BC runs them, separated by spaces, tabs or commas.
//! - Optionally a name first, ended by the first `:` (`DATACHECK: 81659 81660`). The name may hold spaces
//!   (`suite  1: 93001 93002`). A line without a name, or with an empty one, is named `line N`.
//! - Blank lines and lines whose first non-blank character is `#` are skipped. A UTF-8 byte order mark and Windows
//!   line ends are accepted.
//! - A token that is not a whole number from 1 to 4294967295 is an error, and so is a codeunit listed twice in one
//!   suite. A suite without ids is kept and holds nothing. A codeunit may be listed in more than one suite.

/// One suite: its name, the line it is on (from 1) and its test codeunit ids in BC's run order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestSuite {
    pub name: String,
    pub line: usize,
    pub codeunits: Vec<u32>,
}

/// The suites of a `--test-suites` file, in the file's order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TestSuites {
    pub suites: Vec<TestSuite>,
}

impl TestSuites {
    /// Parses the text of a suites file. `Err`: the first bad line, as `line N: ...`.
    pub fn parse(text: &str) -> Result<TestSuites, String> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut suites = Vec::new();
        for (i, raw) in text.lines().enumerate() {
            let line = i + 1;
            let body = raw.trim();
            if body.is_empty() || body.starts_with('#') {
                continue;
            }
            let (name, ids) = match body.split_once(':') {
                Some((n, rest)) => (n.trim(), rest),
                None => ("", body),
            };
            let name = if name.is_empty() {
                format!("line {line}")
            } else {
                name.to_string()
            };
            let mut codeunits: Vec<u32> = Vec::new();
            for token in ids
                .split(|c: char| c.is_whitespace() || c == ',')
                .filter(|t| !t.is_empty())
            {
                let id =
                    token.parse::<u32>().ok().filter(|&id| id > 0).ok_or_else(|| {
                        format!("line {line}: `{token}` is not a codeunit id (a whole number from 1)")
                    })?;
                if codeunits.contains(&id) {
                    return Err(format!(
                        "line {line}: codeunit {id} is listed twice in suite `{name}` (a BC test suite holds a codeunit once)"
                    ));
                }
                codeunits.push(id);
            }
            suites.push(TestSuite { name, line, codeunits });
        }
        Ok(TestSuites { suites })
    }

    /// Every place codeunit `id` is listed: (suite index, position in that suite).
    pub fn places(&self, id: u32) -> impl Iterator<Item = (usize, usize)> + '_ {
        self.suites
            .iter()
            .enumerate()
            .filter_map(move |(s, suite)| suite.codeunits.iter().position(|&c| c == id).map(|p| (s, p)))
    }

    /// Per codeunit id, every place it is listed (see [`TestSuites::places`]).
    pub fn index(&self) -> std::collections::HashMap<u32, Vec<(usize, usize)>> {
        let mut out: std::collections::HashMap<u32, Vec<(usize, usize)>> = std::collections::HashMap::new();
        for (s, suite) in self.suites.iter().enumerate() {
            for (p, &id) in suite.codeunits.iter().enumerate() {
                out.entry(id).or_default().push((s, p));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn named_and_unnamed_lines_give_suites_in_file_order() {
        let s = TestSuites::parse("DATACHECK: 81659 81660 81673\n80603, 80605,80608\n").unwrap();
        assert_eq!(
            s.suites,
            vec![
                TestSuite {
                    name: "DATACHECK".into(),
                    line: 1,
                    codeunits: vec![81659, 81660, 81673]
                },
                TestSuite {
                    name: "line 2".into(),
                    line: 2,
                    codeunits: vec![80603, 80605, 80608]
                },
            ]
        );
    }

    /// The shape al-runner issue 6924 posted: `suite  1: ids`, a name with spaces.
    #[test]
    fn a_name_may_hold_spaces_and_the_order_is_kept() {
        let s = TestSuites::parse("suite  1: 93003 93001 93002\n").unwrap();
        assert_eq!(s.suites[0].name, "suite  1");
        assert_eq!(s.suites[0].codeunits, vec![93003, 93001, 93002]);
    }

    #[test]
    fn blank_lines_comments_bom_and_crlf_are_accepted() {
        let s =
            TestSuites::parse("\u{feff}# from the pipeline\r\n\r\n  A:\t1\t2 \r\n   # indented comment\r\nB: 3\r\n")
                .unwrap();
        let got: Vec<(&str, usize, &[u32])> = s
            .suites
            .iter()
            .map(|x| (x.name.as_str(), x.line, x.codeunits.as_slice()))
            .collect();
        assert_eq!(got, vec![("A", 3, &[1, 2][..]), ("B", 5, &[3][..])]);
    }

    #[test]
    fn an_empty_name_is_named_by_its_line_and_an_empty_suite_is_kept() {
        let s = TestSuites::parse(": 5 6\nENV:\n").unwrap();
        assert_eq!(s.suites[0].name, "line 1");
        assert_eq!(s.suites[1].name, "ENV");
        assert!(s.suites[1].codeunits.is_empty());
    }

    #[test]
    fn a_token_that_is_no_codeunit_id_is_an_error_naming_the_line() {
        for (text, token) in [
            ("A: 1 x2\n", "x2"),
            ("\n1 0\n", "0"),
            ("1 -3", "-3"),
            ("A: 4294967296", "4294967296"),
        ] {
            let e = TestSuites::parse(text).unwrap_err();
            assert!(e.contains(&format!("`{token}` is not a codeunit id")), "{e}");
            assert!(e.starts_with("line "), "{e}");
        }
        assert!(TestSuites::parse("\n\nB: 1 2 y").unwrap_err().starts_with("line 3:"));
    }

    #[test]
    fn a_codeunit_twice_in_one_suite_is_an_error() {
        let e = TestSuites::parse("A: 7 8 7\n").unwrap_err();
        assert!(e.contains("codeunit 7 is listed twice in suite `A`"), "{e}");
    }

    /// Al-runner issue 6924: suites 5 and 6 (and 2 and 25) list the same codeunits.
    #[test]
    fn a_codeunit_may_be_listed_in_more_than_one_suite() {
        let s = TestSuites::parse("A: 1 2\nB: 3\nC: 2 1\n").unwrap();
        assert_eq!(s.places(1).collect::<Vec<_>>(), vec![(0, 0), (2, 1)]);
        assert_eq!(s.places(2).collect::<Vec<_>>(), vec![(0, 1), (2, 0)]);
        assert_eq!(s.places(9).count(), 0);
        assert_eq!(s.index()[&1], vec![(0, 0), (2, 1)]);
    }
}
