//! `Evaluate` of a blank text into an Option field that declares an OptionCaption or OptionCaptionML: which member it names.

use super::unsupported;
use crate::assumptions::{Assumption, assume};
use crate::native::Signal;

/// The facts of the call that the blank-text shapes read.
pub(super) struct BlankCall<'a> {
    /// The text as given, before blanks were trimmed.
    pub text: &'a str,
    /// The field is named directly (not through a FieldRef) and declares a plain OptionCaption (not OptionCaptionML).
    pub direct_plain_caption: bool,
    /// The result of `Evaluate` is used.
    pub ret_used: bool,
    /// GlobalLanguage.
    pub language: i32,
}

/// A blank text (its trimmed form is empty) into an Option field with captions `captions` and members `names`: the position of
/// the member it names, `None` when it names none (the text is rejected).
/// The members with a blank name or a blank caption decide:
/// - none: rejected (format-evaluate/P131EvalOption.al:146, a field without a blank member rejects `''`);
/// - the first of them named `" "` with the caption `' '`: that member (P131EvalOption.al:107-108, `''` and `' '`;
///   OptionEvaluateCaptions.al:192 BlanksAndPrefixes, a TAB alone);
/// - on the assumption [`Assumption::OptionBlankTextBlankCaptions`] (al-runner issue 8354; corpus PR 1577,
///   format-evaluate/EvalOptionBlankCaptions.al OBC_Option_*), for a field named directly with a plain OptionCaption, under
///   GlobalLanguage 1033, the texts `''`, `' '`, `'  '` and a TAB with the result used and `''` as a statement: a member named
///   `""` with the caption `''` or a caption that is not blank, and a member with a name that is not blank and the caption `''`,
///   are not named; a member named `""`, `" "` or with a name that is not blank, with the caption `' '`, and a member named `" "`
///   with the caption `''`, is named. Every such member not named: rejected. One member, named: that member.
///
/// Every other shape refuses.
pub(super) fn blank_text(names: &[String], captions: &[&str], call: &BlankCall) -> Result<Option<usize>, Signal> {
    if captions.len() != names.len() {
        return unsupported(
            "Evaluate of a blank text into an Option field whose captions are not one per member (not pinned)",
        );
    }
    let blank: Vec<usize> = (0..names.len())
        .filter(|&i| names[i].trim().is_empty() || captions[i].trim().is_empty())
        .collect();
    let Some(&first) = blank.first() else {
        return Ok(None);
    };
    if names[first] == " " && captions[first] == " " {
        return Ok(Some(first));
    }
    let observed_call = call.direct_plain_caption
        && call.language == 1033
        && matches!(call.text, "" | " " | "  " | "\t")
        && (call.ret_used || call.text.is_empty());
    let named: Option<Vec<bool>> = blank.iter().map(|&i| names_it(&names[i], captions[i])).collect();
    if let (true, Some(named)) = (observed_call, named) {
        match named.as_slice() {
            [true] => {
                assume(Assumption::OptionBlankTextBlankCaptions);
                return Ok(Some(first));
            }
            all if all.iter().all(|n| !n) => {
                assume(Assumption::OptionBlankTextBlankCaptions);
                return Ok(None);
            }
            _ => {}
        }
    }
    unsupported(
        "Evaluate of a blank text into an Option field with this mix of blank member names and captions (not pinned)",
    )
}

/// Whether the blank text names a member with this name and caption, for the pairs corpus PR 1577 measured (see
/// [`blank_text`]); `None` for every other pair.
fn names_it(name: &str, caption: &str) -> Option<bool> {
    let named = !name.trim().is_empty();
    let shown = !caption.trim().is_empty();
    match (name, caption) {
        ("", "") => Some(false),
        ("", _) if shown => Some(false),
        (_, "") if named => Some(false),
        ("" | " ", " ") => Some(true),
        (_, " ") if named => Some(true),
        (" ", "") => Some(true),
        _ => None,
    }
}
