//! The error a TestPage SetValue raises for an entry the control cannot take: its frame and the language of the platform's words.

use super::controls::note_refusal;
use super::*;

#[derive(Clone, Copy)]
pub(super) enum EntryKind {
    Boolean,
    Enum,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum ValuesAllowedFact {
    Absent,
    Present,
}

/// Facts from the SetValue parse-failure branch that owns the pinned entry text.
#[derive(Clone, Copy)]
pub(super) struct EntryFacts {
    pub parse_failure: bool,
    pub kind: EntryKind,
    pub table_field: bool,
    pub list: bool,
    pub values_allowed: ValuesAllowedFact,
}

#[derive(Clone, Copy)]
pub(super) enum ValidationErrorKind {
    Ordinary,
    Entry(EntryFacts),
    DutchTableRelation,
}

#[derive(Clone, Copy)]
pub(super) struct ValidationFrame<'c> {
    pub language: i32,
    pub draft_key: bool,
    pub caption: &'c str,
    pub error_kind: ValidationErrorKind,
}

/// `draft_key`: the control shows an Integer key field of a draft row (`draft_integer_key`); its Integer-entry error adds no
/// Refresh hint. `caption`: the control's caption, as the client names it in a refused entry.
///
/// The error of a refused entry of control `idx`: `msg` is the sentence the client builds, `suffix` the Refresh hint.
///
/// Under 1043 the platform's own words are Dutch: the frame of the error and the reason an Integer entry could not be read
/// (localization/app-xliff/lt-r7144-ext-caption/R7187PageCaption.al:107-122, :152-166, al-runner issue 7187); the sentence in
/// front, built from the caption, stays English, as it does alone (localization/app-xliff/BaseAppPageExtCaption.al:274-275,
/// :288-290, a Boolean and an Option entry, al-runner issue 7327). Those cases show no Refresh hint; any other text under
/// 1043, or one with the hint, is not pinned and is refused.
pub(super) fn validation_signal(
    engine: &Engine,
    st: &St,
    idx: usize,
    msg: String,
    suffix: &str,
    code: &str,
    frame: ValidationFrame<'_>,
) -> Signal {
    let integer_entry = msg.ends_with(" can't be evaluated into type Integer.");
    let suffix = if frame.draft_key && integer_entry { "" } else { suffix };
    let name = &page_of(engine, st).controls[idx].name;
    let (stored, text) = if frame.language == 1043 {
        let Some(stored) = dutch_text(&msg, suffix, frame.caption, frame.error_kind) else {
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
    pub(super) fn validation_frame<'c>(&self, st: &St, idx: usize, caption: &'c str) -> ValidationFrame<'c> {
        ValidationFrame {
            language: al_builtins::format::FormatCtx::language(self),
            draft_key: self.draft_integer_key(st, idx),
            caption,
            error_kind: ValidationErrorKind::Ordinary,
        }
    }
}

/// The text a page validation error under 1043 stores for `msg` (`validation_signal`), if it is pinned.
fn dutch_text(msg: &str, suffix: &str, caption: &str, error_kind: ValidationErrorKind) -> Option<String> {
    if !suffix.is_empty() {
        return None;
    }
    if matches!(error_kind, ValidationErrorKind::DutchTableRelation) {
        return Some(msg.to_string());
    }
    let integer = msg.split_once(" The value \"").and_then(|(head, tail)| {
        tail.strip_suffix("\" can't be evaluated into type Integer.")
            .map(|entry| format!("{head} De waarde \"{entry}\" kan niet worden geëvalueerd tot type Integer."))
    });
    let bare = || {
        let ValidationErrorKind::Entry(facts) = error_kind else {
            return None;
        };
        let pinned = facts.parse_failure
            && matches!(facts.kind, EntryKind::Boolean | EntryKind::Enum)
            && facts.table_field
            && facts.list
            && facts.values_allowed == ValuesAllowedFact::Absent;
        if pinned {
            msg.strip_prefix("Your entry of '")
                .and_then(|rest| rest.strip_suffix(&format!("' is not an acceptable value for '{caption}'.")))
                .map(|_| msg.to_string())
        } else {
            None
        }
    };
    integer.or_else(bare)
}
