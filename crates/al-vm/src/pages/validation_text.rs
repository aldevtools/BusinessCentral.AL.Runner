//! The error a TestPage SetValue raises for an entry the control cannot take: its frame and the language of the platform's words.

use super::controls::note_refusal;
use super::*;

/// `draft_key`: the control shows an Integer key field of a draft row (`draft_integer_key`); its Integer-entry error adds no
/// Refresh hint.
///
/// The error of a refused entry of control `idx`: `msg` is the sentence the client builds, `suffix` the Refresh hint.
///
/// Under 1043 the platform's own words are Dutch: the frame of the error and the reason an Integer entry could not be read
/// (localization/app-xliff/lt-r7144-ext-caption/R7187PageCaption.al:107-122, :152-166, al-runner issue 7187); the sentence in
/// front, built from the caption, stays English. Any other text under 1043 is not pinned and is refused.
pub(super) fn validation_signal(
    engine: &Engine,
    st: &St,
    idx: usize,
    msg: String,
    suffix: &str,
    code: &str,
    (language, draft_key): (i32, bool),
) -> Signal {
    let integer_entry = msg.ends_with(" can't be evaluated into type Integer.");
    let suffix = if draft_key && integer_entry { "" } else { suffix };
    let name = &page_of(engine, st).controls[idx].name;
    let (stored, text) = if language == 1043 {
        let reason = msg.split_once(" The value \"").and_then(|(head, tail)| {
            tail.strip_suffix("\" can't be evaluated into type Integer.")
                .map(|entry| (head.to_string(), entry.to_string()))
        });
        let Some((head, entry)) = reason else {
            return Signal::Unsupported(format!(
                "a page validation error under GlobalLanguage 1043 (the platform's Dutch text for '{msg}' is not pinned)"
            ));
        };
        let stored = format!("{head} De waarde \"{entry}\" kan niet worden geëvalueerd tot type Integer.{suffix}");
        let text = format!("Validatiefout voor veld: {name}, bericht = '{stored}'");
        (stored, text)
    } else {
        let stored = format!("{msg}{suffix}");
        let text = format!("Validation error for Field: {name},  Message = '{stored}'");
        (stored, text)
    };
    if let Err(why) = note_refusal(st, idx, stored) {
        return why;
    }
    error_code(text, code)
}
