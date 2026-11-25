//! `Evaluate` of a blank text into an Option field that declares an OptionCaption or OptionCaptionML: which member it names.

use super::unsupported;
use crate::assumptions::{Assumption, assume};
use crate::native::Signal;

/// What a blank text names.
pub(super) enum BlankText {
    /// The member at this position.
    Member(usize),
    /// No member: the text is rejected.
    NoMember,
}

/// The facts of the call that the blank-text shapes read.
pub(super) struct BlankCall<'a> {
    /// The text as given, before blanks were trimmed.
    pub text: &'a str,
    /// The field is named directly (not through a FieldRef) and declares a plain OptionCaption (not OptionCaptionML).
    pub direct_plain_caption: bool,
    /// The result of `Evaluate` is used.
    pub ret_used: bool,
}

/// A blank text (its trimmed form is empty) into an Option field with captions `captions` and members `names`.
/// The members with a blank name or a blank caption decide; each shape is allowed by equality:
/// - none: rejected (format-evaluate/P131EvalOption.al:146, a field without a blank member rejects `''`);
/// - the first of them named `" "` with the caption `' '`: that member (P131EvalOption.al:107-108, `''` and `' '`;
///   OptionEvaluateCaptions.al:192 BlanksAndPrefixes, a TAB alone);
/// - every one of them named `""` with the caption `''`, none the first member: rejected, on the assumption
///   [`Assumption::OptionBlankTextEmptyCaptionMembers`] (al-runner issue 8354, corpus PR 1577).
///
/// Every other shape refuses.
pub(super) fn blank_text(names: &[String], captions: &[&str], call: &BlankCall) -> Result<BlankText, Signal> {
    if captions.len() != names.len() {
        return unsupported(
            "Evaluate of a blank text into an Option field whose captions are not one per member (not pinned)",
        );
    }
    let blank: Vec<usize> = (0..names.len())
        .filter(|&i| names[i].trim().is_empty() || captions[i].trim().is_empty())
        .collect();
    let Some(&first) = blank.first() else {
        return Ok(BlankText::NoMember);
    };
    if names[first] == " " && captions[first] == " " {
        return Ok(BlankText::Member(first));
    }
    let reporter_shape = first > 0 && blank.iter().all(|&i| names[i].is_empty() && captions[i].is_empty());
    if reporter_shape && call.direct_plain_caption && call.ret_used && call.text.bytes().all(|b| b == b' ') {
        assume(Assumption::OptionBlankTextEmptyCaptionMembers);
        return Ok(BlankText::NoMember);
    }
    unsupported(
        "Evaluate of a blank text into an Option field with this mix of blank member names and captions (not pinned)",
    )
}
