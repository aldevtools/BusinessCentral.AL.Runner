//! The error a TestPage SetValue raises for an entry the control cannot take: its frame and the language of the platform's words.

use super::controls::note_refusal;
use super::*;

/// `draft_key`: the control shows an Integer key field of a draft row (`draft_integer_key`); its Integer-entry error adds no
/// Refresh hint. `caption`: the control's caption, as the client names it in a refused entry.
///
/// The error of a refused entry of control `idx`: `msg` is the sentence the client builds, `suffix` the Refresh hint.
///
/// Under 1043 the platform's own words are Dutch: the frame of the error and the reason an Integer entry could not be read
/// (localization/app-xliff/lt-r7144-ext-caption/R7187PageCaption.al:107-122, :152-166, al-runner issue 7187); the sentence in
/// front, built from the caption, stays English, as it does alone (localization/app-xliff/BaseAppPageExtCaption.al:274-275,
/// :288-290, a Boolean and an Option entry, al-runner issue 7327). The text of a TableRelation error is the server's Dutch one
/// (BaseAppPageExtCaption.al:276, `relations::is_dutch_relation_text`). Those cases show no Refresh hint; any other text
/// under 1043, or one with the hint, is not pinned and is refused.
pub(super) fn validation_signal(
    engine: &Engine,
    st: &St,
    idx: usize,
    msg: String,
    suffix: &str,
    code: &str,
    (language, draft_key, caption): (i32, bool, &str),
) -> Signal {
    let integer_entry = msg.ends_with(" can't be evaluated into type Integer.");
    let suffix = if draft_key && integer_entry { "" } else { suffix };
    let name = &page_of(engine, st).controls[idx].name;
    let (stored, text) = if language == 1043 {
        let Some(stored) = dutch_text(&msg, suffix, caption) else {
            return Signal::unsupported(format!(
                "a page validation error under GlobalLanguage 1043 (the platform's Dutch text for '{msg}' is not pinned)"
            ));
        };
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

impl Session {
    /// What [`validation_signal`] needs of the session for control `idx`: the language in force, `draft_key` and `caption`.
    pub(super) fn validation_frame<'c>(&self, st: &St, idx: usize, caption: &'c str) -> (i32, bool, &'c str) {
        (
            al_builtins::format::FormatCtx::language(self),
            self.draft_integer_key(st, idx),
            caption,
        )
    }
}

/// The text a page validation error under 1043 stores for `msg` (`validation_signal`), if it is pinned.
fn dutch_text(msg: &str, suffix: &str, caption: &str) -> Option<String> {
    if !suffix.is_empty() {
        return None;
    }
    let integer = msg.split_once(" The value \"").and_then(|(head, tail)| {
        tail.strip_suffix("\" can't be evaluated into type Integer.")
            .map(|entry| format!("{head} De waarde \"{entry}\" kan niet worden geëvalueerd tot type Integer."))
    });
    let bare = || {
        msg.strip_prefix("Your entry of '")
            .and_then(|rest| rest.strip_suffix(&format!("' is not an acceptable value for '{caption}'.")))
            .map(|_| msg.to_string())
    };
    let relation = || crate::relations::is_dutch_relation_text(msg).then(|| msg.to_string());
    integer.or_else(bare).or_else(relation)
}
