//! Comparing case sets across BC versions (ADR-015 3.6).
//!
//! A case that one version has and another lacks is a difference to look at: in the
//! oracle it is drift between corpus legs, in the runtime it is a behaviour that
//! needs a version gate or is already gated.
//!
//! Entry points: `differences`, `common`, `restrict`, `table`.

use crate::oracle::CaseId;
use std::collections::BTreeSet;

/// For each case that is not in every set: the versions that have it.
pub fn differences(sets: &[(String, BTreeSet<CaseId>)]) -> Vec<(CaseId, Vec<String>)> {
    let all: BTreeSet<&CaseId> = sets.iter().flat_map(|(_, s)| s.iter()).collect();
    all.into_iter()
        .filter_map(|c| {
            let have: Vec<String> = sets
                .iter()
                .filter(|(_, s)| s.contains(c))
                .map(|(v, _)| v.clone())
                .collect();
            (have.len() != sets.len()).then(|| (c.clone(), have))
        })
        .collect()
}

/// The cases every set has.
pub fn common(sets: &[(String, BTreeSet<CaseId>)]) -> BTreeSet<CaseId> {
    let Some((_, first)) = sets.first() else {
        return BTreeSet::new();
    };
    first
        .iter()
        .filter(|c| sets.iter().all(|(_, s)| s.contains(c)))
        .cloned()
        .collect()
}

/// The sets cut down to `keep`. A case that one version's oracle lacks (its oracle is from an
/// older corpus commit, ADR-007) cannot show a version difference in the runtime either.
pub fn restrict(sets: &[(String, BTreeSet<CaseId>)], keep: &BTreeSet<CaseId>) -> Vec<(String, BTreeSet<CaseId>)> {
    sets.iter()
        .map(|(v, s)| (v.clone(), s.intersection(keep).cloned().collect()))
        .collect()
}

/// A short text table: the first lines of the differences, or `none`.
pub fn table(what: &str, sets: &[(String, BTreeSet<CaseId>)]) -> String {
    const SHOWN: usize = 20;
    let diffs = differences(sets);
    if diffs.is_empty() {
        return format!("{what}: same cases in every version\n");
    }
    let mut out = format!(
        "{what}: {} case(s) differ between versions (BC majors that have the case)\n",
        diffs.len()
    );
    for (c, have) in diffs.iter().take(SHOWN) {
        out.push_str(&format!("  {} {}: {}\n", c.codeunit, c.name, have.join(", ")));
    }
    if diffs.len() > SHOWN {
        out.push_str(&format!("  ... and {} more\n", diffs.len() - SHOWN));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(cu: u32, n: &str) -> CaseId {
        CaseId {
            codeunit: cu,
            name: n.into(),
        }
    }
    fn set(v: &[(u32, &str)]) -> BTreeSet<CaseId> {
        v.iter().map(|(c, n)| id(*c, n)).collect()
    }

    #[test]
    fn identical_sets_have_no_differences() {
        let s = [
            ("27".to_string(), set(&[(1, "a")])),
            ("29".to_string(), set(&[(1, "a")])),
        ];
        assert!(differences(&s).is_empty());
        assert!(table("oracle", &s).contains("same cases"));
    }

    #[test]
    fn the_runtime_comparison_ignores_cases_an_oracle_lacks() {
        let oracle = [
            ("27".to_string(), set(&[(1, "a"), (2, "new")])),
            ("29".to_string(), set(&[(1, "a")])),
        ];
        let runtime = [
            ("27".to_string(), set(&[(1, "a"), (2, "new")])),
            ("29".to_string(), set(&[(1, "a")])),
        ];
        assert_eq!(common(&oracle), set(&[(1, "a")]));
        assert!(differences(&restrict(&runtime, &common(&oracle))).is_empty());
        assert_eq!(differences(&runtime).len(), 1, "unrestricted it would be reported");
    }

    #[test]
    fn a_case_missing_in_one_version_is_listed_with_the_versions_that_have_it() {
        let s = [
            ("27".to_string(), set(&[(1, "a"), (2, "b")])),
            ("28".to_string(), set(&[(1, "a"), (2, "b")])),
            ("29".to_string(), set(&[(1, "a")])),
        ];
        assert_eq!(
            differences(&s),
            vec![(id(2, "b"), vec!["27".to_string(), "28".to_string()])]
        );
        let t = table("runtime", &s);
        assert!(t.contains("1 case(s) differ") && t.contains("2 b: 27, 28"), "{t}");
    }
}
