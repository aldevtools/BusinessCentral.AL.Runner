//! The corpus files that run in BC's order (ADR-066 amendment 12, owner decision of 2026-11-09).
//!
//! The corpus runs in AL Runner's default order: every test codeunit starts with fresh SingleInstance state and the
//! install baseline's NumberSequence values. Some corpus files exist to pin what carries from one test codeunit to the
//! next on BC (for example `al-language/codeunits/TestCuP7BReader.al`). They are listed in
//! `metrics/strict-order-files.txt`, one path per line relative to the corpus `tests` folder (`#` starts a comment).
//! Each pass runs the test codeunits of the listed files once more, alone, with `--strict-order`
//! (`RunOptions::strict_order`), and their cases take the outcome of that run. They must pass there; no not-passing
//! line is added for them.

use crate::corpus::CorpusIndex;
use std::collections::BTreeSet;
use std::path::Path;

/// Where the list lives, relative to the repository root.
pub const LIST: &str = "metrics/strict-order-files.txt";

/// The paths `text` lists: one per line, blank lines and `#` comments left out, `\` read as `/`.
pub fn parse(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.split('#').next().unwrap_or("").trim().replace('\\', "/"))
        .filter(|l| !l.is_empty())
        .collect()
}

/// The list under `root`; empty when the file does not exist.
pub fn load(root: &Path) -> Result<Vec<String>, String> {
    match std::fs::read_to_string(root.join(LIST)) {
        Ok(text) => Ok(parse(&text)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::TestCodeunit;
    use std::path::PathBuf;

    #[test]
    fn the_list_leaves_out_comments_and_blank_lines() {
        let text = "# order probes\n\nal-language/codeunits/TestCuP7BReader.al  # P7\nal-language\\codeunits\\run-onrun\\SeqCarry.al\n";
        assert_eq!(
            parse(text),
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
        let files = parse("codeunits/TestCuP7BReader.al");
        assert_eq!(codeunits(&files, &ix, [1, 2, 3, 4]), BTreeSet::from([1]));
    }
}
