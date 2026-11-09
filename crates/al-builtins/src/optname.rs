//! The exact text of a blank option member.
//!
//! The syntax layer used to keep a member declared as `""` and one declared as `" "` as the same one space, so the runtime
//! could not tell them apart. A member declared `" "` stays one space. A member declared `""` is held as [`QUOTED_EMPTY`]
//! (a private-use character no AL name holds), and an unquoted empty member (`Option ,A`) stays the empty text.
//!
//! What real BC does with each blank form is pinned only in two places:
//! - a first member `" "` reads the empty text, a blank text and NUL as ordinal 0
//!   (format-evaluate/format-matrix/FMEvalOption.al:64-69, :262-267);
//! - a last member `""` reads none of them (types/decimal-integer/ScT70062.al:Option_EvaluateByName `empty`) and `Format`
//!   gives one character (types/decimal-integer/ScT70053.al:Option_Format `blank_len`).
//!
//! Every other place of a blank member is refused (hard rule 4) until the probe of `docs/probes-needed.md` row OPTBLANK
//! pins it.

use crate::native::Signal;

/// How a member declared as `""` is held in a member list.
pub const QUOTED_EMPTY: &str = "\u{E000}";

/// The member list of a declaration, from the text and the quoted flag of each member.
pub fn declared(forms: Vec<(String, bool)>) -> Vec<String> {
    forms
        .into_iter()
        .map(|(text, quoted)| {
            if quoted && text.is_empty() {
                QUOTED_EMPTY.to_string()
            } else {
                text
            }
        })
        .collect()
}

/// Whether the member is blank in any form: `" "`, `""` or an unquoted empty member.
pub fn is_blank(m: &str) -> bool {
    m == QUOTED_EMPTY || m.trim().is_empty()
}

/// Whether any member of the list is a quoted `""`.
pub fn has_quoted_empty(members: &[String]) -> bool {
    members.iter().any(|m| m == QUOTED_EMPTY)
}

/// The text of a member for an error message or a listing; the one place a `""` member is shown as one space (the length
/// of its `Format` is pinned, its character is not: probe OPTBLANK).
pub fn listed(m: &str) -> &str {
    if m == QUOTED_EMPTY { " " } else { m }
}

/// What a blank text (the empty text, blanks, NUL) names in a member list.
#[derive(Debug, PartialEq, Eq)]
pub enum BlankRead {
    /// The member at this position (a lone `" "` first member).
    Member(usize),
    /// No member (no blank member, or a lone `""` last member).
    None,
    /// A blank layout no corpus test pins.
    Unpinned,
}

/// Reads a blank text against the members.
pub fn read_blank(members: &[String]) -> BlankRead {
    let blanks: Vec<usize> = (0..members.len()).filter(|&i| is_blank(&members[i])).collect();
    match blanks.as_slice() {
        [] => BlankRead::None,
        [0] if members[0] == " " => BlankRead::Member(0),
        [i] if *i + 1 == members.len() && members[*i] == QUOTED_EMPTY => BlankRead::None,
        // An unquoted empty member alone (`Option ,A`) reads nothing: a field's `''` does not name a member declared as
        // `""` (format-evaluate/AxFormatOption.al:208-211).
        [i] if members[*i].is_empty() => BlankRead::None,
        _ => BlankRead::Unpinned,
    }
}

/// The refusal of a blank member at a place no corpus test pins.
pub fn refuse<T>(what: &str) -> Result<T, Signal> {
    Err(Signal::Unsupported(format!(
        "{what} with a blank option member (\"\" or \" \") at a place no corpus test pins (probe OPTBLANK)"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_declared_forms_stay_apart() {
        let l = declared(vec![
            ("".into(), false),
            (" ".into(), true),
            ("".into(), true),
            ("A".into(), false),
        ]);
        assert_eq!(l, vec!["".to_string(), " ".into(), QUOTED_EMPTY.into(), "A".into()]);
    }

    #[test]
    fn a_blank_text_reads_only_the_pinned_layouts() {
        assert_eq!(read_blank(&m(&[" ", "A"])), BlankRead::Member(0));
        assert_eq!(read_blank(&m(&["A", "B", QUOTED_EMPTY])), BlankRead::None);
        assert_eq!(read_blank(&m(&["A", "B"])), BlankRead::None);
        assert_eq!(read_blank(&m(&[QUOTED_EMPTY, "A"])), BlankRead::Unpinned);
        assert_eq!(read_blank(&m(&["A", " ", "B"])), BlankRead::Unpinned);
        assert_eq!(read_blank(&m(&["A", "B", " "])), BlankRead::Unpinned);
        assert_eq!(read_blank(&m(&[" ", "A", QUOTED_EMPTY])), BlankRead::Unpinned);
    }
}
