//! The preprocessor symbols the corpus CI defines for a BC version (ADR-015 3.9, ADR-078 point 3).
//!
//! The preprocessor symbols the corpus CI defines for a BC version (ADR-015 3.9, ADR-078 point 3):
//! `BC<n>PLUS` per major from 27, and `BC<n>_<m>PLUS` per released minor at or below the version once
//! `oracle.toml` records `released_minors`. Its callers are al-test-cli's corpus paths (`--print-
//! symbols`, the corpus run, the coverage maps); a must-not-compile case takes the major symbols only
//! (`mnc::case_symbols`). Entry points: `ReleasedMinors`, `corpus_symbols`.

use crate::{BcVersion, MIN_BC_MAJOR, VersionError};
use std::collections::BTreeSet;

/// The released minors the corpus lists (ADR-078 point 3), as `oracle/oracle.toml` `released_minors` records them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ReleasedMinors {
    /// The pinned corpus records no list: its CI defines `BC<n>PLUS` only (the corpus before ADR-078).
    #[default]
    NotRecorded,
    /// Every released `(major, minor)` from 27.0, in version order.
    List(Vec<(u16, u16)>),
}

impl ReleasedMinors {
    /// The list from its entries (`27.0`, `29.1`). An entry must name exactly a major and a minor; an empty list is
    /// an error, as in the corpus (`.github/scripts/ci-legs.py:61` on the corpus branch ci-bc-29-1).
    pub fn parse<S: AsRef<str>>(entries: &[S]) -> Result<ReleasedMinors, VersionError> {
        if entries.is_empty() {
            return Err(VersionError::Empty);
        }
        let mut list = Vec::with_capacity(entries.len());
        for e in entries {
            let e = e.as_ref();
            let v = BcVersion::parse(e)?;
            if v.build.is_some() || !e.contains('.') {
                return Err(VersionError::BadPart(e.to_string()));
            }
            list.push((v.major, v.minor));
        }
        list.sort_unstable();
        list.dedup();
        Ok(ReleasedMinors::List(list))
    }
}

/// The preprocessor symbols the corpus CI defines for `version`, cumulative in version order: `BC<n>PLUS` for every
/// major from 27 to the version's, and, once the corpus lists its released minors, `BC<n>_<m>PLUS` for every listed
/// minor `n.m` (m at least 1) at or below the version, plus the version's own when the list lacks it (an insider build).
/// There is no `_0` form: `BC29PLUS` is 29.0. Only the corpus uses this convention; a third-party app gets only its
/// own `preprocessorSymbols` (ADR-052 3.4).
pub fn corpus_symbols(version: &BcVersion, released: &ReleasedMinors) -> BTreeSet<String> {
    let at = (version.major, version.minor);
    let mut out: BTreeSet<String> = (MIN_BC_MAJOR..=version.major.max(MIN_BC_MAJOR))
        .map(|n| format!("BC{n}PLUS"))
        .collect();
    if let ReleasedMinors::List(list) = released {
        let own = (version.minor >= 1).then_some(at);
        for (n, m) in list.iter().copied().filter(|&v| v <= at).chain(own) {
            if m >= 1 && n >= MIN_BC_MAJOR {
                out.insert(format!("BC{n}_{m}PLUS"));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> BTreeSet<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    fn at(v: &str, released: &ReleasedMinors) -> BTreeSet<String> {
        corpus_symbols(&BcVersion::parse(v).unwrap(), released)
    }

    /// The list of ADR-078 point 3 ("today": 27.0 to 27.5, 28.0 to 28.5, 29.0 and 29.1).
    fn today() -> ReleasedMinors {
        ReleasedMinors::parse(&[
            "27.0", "27.1", "27.2", "27.3", "27.4", "27.5", "28.0", "28.1", "28.2", "28.3", "28.4", "28.5", "29.0",
            "29.1",
        ])
        .unwrap()
    }

    const MINORS_27: [&str; 5] = ["BC27_1PLUS", "BC27_2PLUS", "BC27_3PLUS", "BC27_4PLUS", "BC27_5PLUS"];
    const MINORS_28: [&str; 5] = ["BC28_1PLUS", "BC28_2PLUS", "BC28_3PLUS", "BC28_4PLUS", "BC28_5PLUS"];

    fn with(base: &[&str], more: &[&[&str]]) -> BTreeSet<String> {
        let mut out = s(base);
        for m in more {
            out.extend(s(m));
        }
        out
    }

    #[test]
    fn without_a_recorded_list_a_version_gets_the_major_symbols_only() {
        let none = ReleasedMinors::NotRecorded;
        assert_eq!(at("27.5", &none), s(&["BC27PLUS"]));
        assert_eq!(at("28.5", &none), s(&["BC27PLUS", "BC28PLUS"]));
        assert_eq!(at("29.0", &none), s(&["BC27PLUS", "BC28PLUS", "BC29PLUS"]));
        assert_eq!(at("29.1", &none), s(&["BC27PLUS", "BC28PLUS", "BC29PLUS"]));
        assert!(at("30.0", &none).contains("BC30PLUS"));
    }

    /// An empty list is refused (`ReleasedMinors::parse`); no list is `NotRecorded`.
    #[test]
    fn an_empty_list_is_refused() {
        assert_eq!(ReleasedMinors::parse::<&str>(&[]), Err(VersionError::Empty));
    }

    #[test]
    fn below_the_floor_a_version_still_gets_the_floor_symbol() {
        assert_eq!(at("26.0", &today()), s(&["BC27PLUS"]));
    }

    /// The table of ADR-078 point 3.
    #[test]
    fn the_symbols_of_each_version_are_those_of_every_released_minor_at_or_below_it() {
        let l = today();
        assert_eq!(at("27.0", &l), s(&["BC27PLUS"]));
        assert_eq!(at("27.5", &l), with(&["BC27PLUS"], &[&MINORS_27]));
        let v29 = with(&["BC27PLUS", "BC28PLUS", "BC29PLUS"], &[&MINORS_27, &MINORS_28]);
        assert_eq!(at("29.0", &l), v29);
        let v291 = with(
            &["BC27PLUS", "BC28PLUS", "BC29PLUS", "BC29_1PLUS"],
            &[&MINORS_27, &MINORS_28],
        );
        assert_eq!(at("29.1", &l), v291);
        assert_eq!(v291.len(), 14);
        // An insider build: not in the list, so its own minor is added.
        let mut v292 = v291.clone();
        v292.insert("BC29_2PLUS".into());
        assert_eq!(at("29.2", &l), v292);
        let mut v30 = v291.clone();
        v30.insert("BC30PLUS".into());
        assert_eq!(at("30.0", &l), v30);
    }

    #[test]
    fn a_later_minor_of_an_older_major_holds_on_every_later_version() {
        let ReleasedMinors::List(mut list) = today() else {
            unreachable!()
        };
        list.push((28, 6));
        let l = ReleasedMinors::List(list);
        assert!(at("29.0", &l).contains("BC28_6PLUS"));
        assert!(at("30.0", &l).contains("BC28_6PLUS"));
        assert!(!at("28.5", &l).contains("BC28_6PLUS"));
    }

    #[test]
    fn a_list_entry_names_a_major_and_a_minor_only() {
        assert!(ReleasedMinors::parse(&["29"]).is_err());
        assert!(ReleasedMinors::parse(&["29.1.57326.58275"]).is_err());
        assert!(ReleasedMinors::parse(&["29.x"]).is_err());
        assert_eq!(
            ReleasedMinors::parse(&["29.1", "27.0"]),
            Ok(ReleasedMinors::List(vec![(27, 0), (29, 1)]))
        );
    }
}
