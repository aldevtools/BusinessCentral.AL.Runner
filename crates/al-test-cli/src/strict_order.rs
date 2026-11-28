//! The corpus files that run in BC's order (ADR-066 amendment 12, owner decision of 2026-11-09).
//!
//! The corpus runs in AL Runner's default order: every test codeunit starts with fresh SingleInstance state and the
//! install baseline's NumberSequence values. Some corpus files exist to pin what carries from one test codeunit to the
//! next on BC (for example `al-language/codeunits/TestCuP7BReader.al`). They are listed in
//! `metrics/strict-order-files.txt`, one path per line relative to the corpus `tests` folder (`#` starts a comment).
//! Each pass runs the test codeunits of the listed files once more, alone, with `--strict-order`
//! (`RunOptions::strict_order`), and their cases take the outcome of that run. They must pass there; no not-passing
//! line is added for them.
//!
//! the corpus order probes of `metrics/strict-order-files.txt` (ADR-066 amendment 12, owner
//! 2026-11-09): `parse`, `load`, `codeunits` (the listed files' test codeunits). `main.rs`
//! `StrictOrder::of` picks them per pass and `StrictOrder::adopt` runs them alone with
//! `strict_order`, then `CorpusRuntime::adopt_cases` gives their cases that run's outcomes.

use crate::corpus::CorpusIndex;
use crate::oracle::{CaseId, OracleCase};
use crate::report::Outcome;
use std::collections::BTreeSet;
use std::path::Path;

/// Where the list lives, relative to the repository root.
pub const LIST: &str = "metrics/strict-order-files.txt";

/// What `metrics/strict-order-files.txt` lists.
#[derive(Debug, Default)]
pub struct List {
    /// The listed paths, in list order.
    pub files: Vec<String>,
    /// (writer, reader) per writer line `<writer> writes-for <reader>`: a file listed because it writes the state
    /// that `<reader>` reads, so the strict run holds the writer (al-runner issue 8943).
    writes_for: Vec<(String, String)>,
}

/// The list `text` holds: one path per line, blank lines and `#` comments left out, `\` read as `/`. A line
/// `<path> writes-for <reader path>` lists `<path>` as the writer of `<reader path>`.
pub fn parse(text: &str) -> List {
    let mut list = List::default();
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim().replace('\\', "/");
        let (file, reader) = match line.split_once(" writes-for ") {
            Some((file, reader)) => (file.trim().to_string(), Some(reader.trim().to_string())),
            None => (line, None),
        };
        if file.is_empty() {
            continue;
        }
        if let Some(reader) = reader {
            list.writes_for.push((file.clone(), reader));
        }
        list.files.push(file);
    }
    list
}

/// The list under `root`; empty when the file does not exist.
pub fn load(root: &Path) -> Result<List, String> {
    match std::fs::read_to_string(root.join(LIST)) {
        Ok(text) => Ok(parse(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(List::default()),
        Err(e) => Err(format!("{LIST}: {e}")),
    }
}

/// The test codeunits of `index` whose file is one of `files` (its path ends with the listed path), among `ids`.
pub fn codeunits(files: &[String], index: &CorpusIndex, ids: impl IntoIterator<Item = u32>) -> BTreeSet<u32> {
    ids.into_iter()
        .filter(|id| {
            index.codeunits.get(id).is_some_and(|cu| {
                let path = cu.file.to_string_lossy().replace('\\', "/");
                files.iter().any(|f| path.ends_with(&format!("/{f}")) || path == *f)
            })
        })
        .collect()
}

/// The outcome a run recorded for a case, `None` when it recorded none.
pub type Recorded<'a> = dyn Fn(&CaseId) -> Option<Outcome> + 'a;

/// ADR-070 decision 5, second rule (al-runner issue 8943): the listed files whose every oracle case passes in the
/// default order (`default`, what the run recorded before the strict run's outcome replaced it) and in BC's order
/// (`strict`). A file with a case either run did not record (not selected, or not adopted) is not decided. A writer
/// (`writes-for`) is left out while its reader is listed and not found itself: the strict run needs the writer.
pub fn passing_in_both(
    list: &List,
    index: &CorpusIndex,
    all: &[OracleCase],
    default: &Recorded<'_>,
    strict: &Recorded<'_>,
) -> Vec<String> {
    let found: Vec<&String> = list
        .files
        .iter()
        .filter(|f| {
            let ids = codeunits(std::slice::from_ref(*f), index, all.iter().map(|c| c.id.codeunit));
            let mut cases = all.iter().filter(|c| ids.contains(&c.id.codeunit)).peekable();
            cases.peek().is_some()
                && cases.all(|c| {
                    matches!(default(&c.id), Some(Outcome::Pass)) && matches!(strict(&c.id), Some(Outcome::Pass))
                })
        })
        .collect();
    let held = |f: &String| {
        list.writes_for
            .iter()
            .any(|(writer, reader)| writer == f && list.files.contains(reader) && !found.contains(&reader))
    };
    found.iter().filter(|f| !held(f)).map(|f| f.to_string()).collect()
}

/// The error line of a file [`passing_in_both`] found.
pub fn passing_in_both_line(file: &str) -> String {
    format!(
        "{LIST}: every case of {file} passes in the default order and in BC's order; delete its line (ADR-070 decision 5)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::TestCodeunit;
    use std::path::PathBuf;

    #[test]
    fn the_list_leaves_out_comments_and_blank_lines() {
        let text = "# order probes\n\nal-language/codeunits/TestCuP7BReader.al  # P7\nal-language\\codeunits\\run-onrun\\SeqCarry.al\n";
        assert_eq!(
            parse(text).files,
            [
                "al-language/codeunits/TestCuP7BReader.al",
                "al-language/codeunits/run-onrun/SeqCarry.al"
            ]
        );
    }

    #[test]
    fn a_codeunit_is_picked_by_the_whole_file_name_only() {
        let mut ix = CorpusIndex::default();
        for (id, file) in [
            (1, "/c/tests/al-language/codeunits/TestCuP7BReader.al"),
            (2, "/c/tests/al-language/codeunits/XTestCuP7BReader.al"),
            (3, "/c/tests/al-language/codeunits/Other.al"),
        ] {
            ix.codeunits.insert(
                id,
                TestCodeunit {
                    id,
                    area: "codeunits".into(),
                    file: PathBuf::from(file),
                    tests: vec![],
                },
            );
        }
        let files = parse("codeunits/TestCuP7BReader.al").files;
        assert_eq!(codeunits(&files, &ix, [1, 2, 3, 4]), BTreeSet::from([1]));
    }

    fn index_of(files: &[(u32, &str)]) -> CorpusIndex {
        let mut ix = CorpusIndex::default();
        for (id, file) in files {
            ix.codeunits.insert(
                *id,
                TestCodeunit {
                    id: *id,
                    area: "codeunits".into(),
                    file: PathBuf::from(format!("/c/tests/{file}")),
                    tests: vec![],
                },
            );
        }
        ix
    }

    fn case(cu: u32, name: &str) -> OracleCase {
        OracleCase {
            id: CaseId {
                codeunit: cu,
                name: name.into(),
            },
            bc_failed: false,
        }
    }

    /// Al-runner issue 8943: a listed file is reported only when every case passes in both orders.
    #[test]
    fn a_listed_file_whose_every_case_passes_in_both_orders_is_reported() {
        let ix = index_of(&[(1, "a/Both.al"), (2, "a/OnlyStrict.al"), (3, "a/Partly.al"), (4, "a/Unrun.al")]);
        let all = [
            case(1, "P"),
            case(1, "Q"),
            case(2, "P"),
            case(3, "P"),
            case(3, "Q"),
            case(4, "P"),
        ];
        let files = parse("a/Both.al\na/OnlyStrict.al\na/Partly.al\na/Unrun.al\na/NoCases.al");
        // Default order: 2 fails, 3.Q fails, 4 was not run (not adopted); BC's order: all pass.
        let default = |c: &CaseId| match (c.codeunit, c.name.as_str()) {
            (2, _) | (3, "Q") => Some(Outcome::Fail("fresh state".into())),
            (4, _) => None,
            _ => Some(Outcome::Pass),
        };
        let strict = |_: &CaseId| Some(Outcome::Pass);
        assert_eq!(passing_in_both(&files, &ix, &all, &default, &strict), ["a/Both.al"]);
        // A case that fails in BC's order keeps its file off the report too.
        let strict_q = |c: &CaseId| Some(if c.name == "Q" { Outcome::Fail("x".into()) } else { Outcome::Pass });
        assert!(passing_in_both(&files, &ix, &all, &default, &strict_q).is_empty());
    }

    /// Al-runner issue 8943: a writer line (`<path> writes-for <reader>`) keeps the writer listed while its reader is
    /// listed and not itself reported; a writer whose reader is off the list or passes in both orders is reported.
    #[test]
    fn a_writer_is_reported_only_when_its_reader_does_not_hold_it() {
        let ix = index_of(&[(1, "a/Setter.al"), (2, "a/Reader.al"), (3, "a/Lone.al")]);
        let all = [case(1, "W"), case(2, "R"), case(3, "W")];
        let list = parse("a/Setter.al writes-for a/Reader.al\na/Reader.al\na/Lone.al writes-for a/Unlisted.al\n");
        assert_eq!(list.files, ["a/Setter.al", "a/Reader.al", "a/Lone.al"]);
        let strict = |_: &CaseId| Some(Outcome::Pass);
        // The reader fails in the default order: it holds its writer; the writer of an unlisted reader is reported.
        let default = |c: &CaseId| Some(if c.codeunit == 2 { Outcome::Fail("fresh".into()) } else { Outcome::Pass });
        assert_eq!(passing_in_both(&list, &ix, &all, &default, &strict), ["a/Lone.al"]);
        // The reader passes in both orders too: it and its writer are reported.
        let default_all = |_: &CaseId| Some(Outcome::Pass);
        assert_eq!(passing_in_both(&list, &ix, &all, &default_all, &strict), ["a/Setter.al", "a/Reader.al", "a/Lone.al"]);
    }
}
