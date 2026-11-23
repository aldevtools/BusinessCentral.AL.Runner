//! Test selection (`--codeunit`, `--test`, `--area`, `--file`).
//!
//! Each flag may repeat; a case is selected when any criterion matches it (the union). A
//! selection narrows what is compiled (the unselected test codeunits are not roots of the
//! check), what is run, what is reported and what the ratchet compares.
//!
//! The unit of a run is the test codeunit, not the test: under `TestIsolation = Codeunit` the
//! database persists from one test of a codeunit to the next, so a test run alone could see a
//! different state than the oracle's run did. The selected codeunits run whole; the report keeps
//! only the selected cases.
//!
//! Test selection: --codeunit, --test, --area, --file (union). Entry points: `Selection`, `matches`,
//! `filter`.

use crate::corpus::CorpusIndex;
use crate::oracle::{CaseId, OracleCase};

#[derive(Debug, Default, Clone)]
pub struct Selection {
    /// `--codeunit ID`: the test codeunit's object id.
    pub codeunits: Vec<u32>,
    /// `--test TEXT`: a test whose name equals TEXT or contains it (any case).
    pub tests: Vec<String>,
    /// `--area NAME`: the corpus folder a test codeunit lives in (the first folder under the app).
    pub areas: Vec<String>,
    /// `--file SUFFIX`: a test codeunit whose source path ends with SUFFIX.
    pub files: Vec<String>,
}

impl Selection {
    /// Whether any criterion was given. Without one, everything runs.
    pub fn is_active(&self) -> bool {
        !(self.codeunits.is_empty() && self.tests.is_empty() && self.areas.is_empty() && self.files.is_empty())
    }

    /// Whether `case` meets any criterion. `index` says where its codeunit lives.
    pub fn matches(&self, case: &CaseId, index: &CorpusIndex) -> bool {
        if self.codeunits.contains(&case.codeunit) {
            return true;
        }
        let name = case.name.to_lowercase();
        if self.tests.iter().any(|t| name.contains(&t.to_lowercase())) {
            return true;
        }
        let Some(cu) = index.codeunits.get(&case.codeunit) else {
            return false;
        };
        if self
            .areas
            .iter()
            .any(|a| cu.area.eq_ignore_ascii_case(a.trim_matches('/')))
        {
            return true;
        }
        let path = cu.file.to_string_lossy().replace('\\', "/");
        self.files.iter().any(|f| path.ends_with(&f.replace('\\', "/")))
    }

    /// The oracle cases the selection picks; all of them when it is not active.
    pub fn filter(&self, cases: &[OracleCase], index: &CorpusIndex) -> Vec<OracleCase> {
        if !self.is_active() {
            return cases.to_vec();
        }
        cases.iter().filter(|c| self.matches(&c.id, index)).cloned().collect()
    }

    /// A description for messages: `--codeunit 5 --area record`.
    pub fn describe(&self) -> String {
        let mut parts = Vec::new();
        parts.extend(self.codeunits.iter().map(|c| format!("--codeunit {c}")));
        parts.extend(self.tests.iter().map(|t| format!("--test {t}")));
        parts.extend(self.areas.iter().map(|a| format!("--area {a}")));
        parts.extend(self.files.iter().map(|f| format!("--file {f}")));
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::corpus::TestCodeunit;
    use std::path::PathBuf;

    fn index() -> CorpusIndex {
        let mut ix = CorpusIndex::default();
        for (id, area, file) in [
            (1, "record", "/c/record/TestA.al"),
            (2, "record", "/c/record/TestB.al"),
            (3, "json", "/c/json/TestJ.al"),
        ] {
            ix.codeunits.insert(
                id,
                TestCodeunit {
                    id,
                    area: area.into(),
                    file: PathBuf::from(file),
                    tests: vec![],
                },
            );
        }
        ix
    }

    fn case(cu: u32, name: &str) -> CaseId {
        CaseId {
            codeunit: cu,
            name: name.into(),
        }
    }

    #[test]
    fn without_a_criterion_nothing_is_selected_and_the_selection_is_inactive() {
        let s = Selection::default();
        assert!(!s.is_active());
        assert!(!s.matches(&case(1, "X"), &index()));
    }

    #[test]
    fn each_criterion_selects_and_several_make_a_union() {
        let ix = index();
        let by = |s: Selection| {
            [case(1, "Insert_Works"), case(2, "Delete_Works"), case(3, "Parse_Works")]
                .iter()
                .filter(|c| s.matches(c, &ix))
                .map(|c| c.codeunit)
                .collect::<Vec<_>>()
        };
        let one = |f: fn(&mut Selection)| {
            let mut s = Selection::default();
            f(&mut s);
            by(s)
        };
        assert_eq!(one(|s| s.codeunits.push(2)), [2]);
        assert_eq!(one(|s| s.tests.push("PARSE".into())), [3]);
        assert_eq!(one(|s| s.areas.push("record".into())), [1, 2]);
        assert_eq!(one(|s| s.files.push("json/TestJ.al".into())), [3]);
        assert_eq!(
            one(|s| {
                s.codeunits.push(1);
                s.files.push("TestJ.al".into());
            }),
            [1, 3]
        );
    }
}
