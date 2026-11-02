use super::*;

/// The character inside a text that marks a call stack with an untracked frame. A noncharacter (Unicode permanently
/// reserves U+FDD0 to U+FDEF for internal use): real code puts private-use characters in texts (a barcode font's glyphs
/// are U+E000 and up), so the marks may not be private-use characters.
pub const CALL_STACK_MARK: char = '\u{FDD0}';

/// The text Base64 Convert gives for content carried unread in the company build (`stream::is_carried_mark`, owner decision
/// 2026-10-23 "Carry bytes unread"): no text function reads it (`Txt::has_marker`), it moves between Text slots and into .NET,
/// and a file .NET saves with it is marked (`stream::PACKAGE_CARRIED`).
pub const CARRIED_TEXT_MARK: char = '\u{FDD1}';

/// Stands for a call stack that exists in BC but is not tracked here.
pub const UNTRACKED_CALL_STACK: &str = "\u{FDD0}";

/// Stands for the text and code of an error BC raises but whose wording the corpus does not pin.
/// `GetLastErrorText` and `GetLastErrorCode` refuse to answer while it is the last error, so a
/// test that reads it is not run, and one that only needs the call to fail still passes.
pub const UNPINNED_ERROR: &str = "\u{FDD1}";

/// The last error to set when a call fails with an error whose text is not pinned.
pub fn unpinned_error() -> AlError {
    AlError {
        message: UNPINNED_ERROR.into(),
        code: UNPINNED_ERROR.into(),
        call_stack: String::new(),
        ..Default::default()
    }
}

/// An error BC raises whose text and code the corpus does not pin, naming the probe that would pin them (its row in
/// docs/probes-needed.md). It is the same marker as [`unpinned_error`], with the probe after the mark in the text.
pub fn unpinned_error_for(probe: &str) -> AlError {
    AlError {
        message: format!("{UNPINNED_ERROR}{probe}"),
        ..unpinned_error()
    }
}

/// Follows [`UNPINNED_ERROR`] in the text of an error whose code is pinned and whose text is not ([`unpinned_text_error`]):
/// `GetLastErrorCode` answers the code, `GetLastErrorText` refuses.
pub(super) const PINNED_CODE: char = '\u{FDD2}';

/// An error with a pinned `code` whose text the corpus does not pin, naming the probe that would pin the text.
pub fn unpinned_text_error(code: &str, probe: &str) -> AlError {
    AlError {
        message: format!("{UNPINNED_ERROR}{PINNED_CODE}{probe}"),
        code: code.into(),
        ..unpinned_error()
    }
}

/// Whether the text of `e` is not pinned ([`unpinned_error`], [`unpinned_error_for`]). Some errors have a pinned text and an
/// unpinned code (the code alone is the mark).
pub fn text_unpinned(e: &AlError) -> bool {
    // Anywhere in the text: a text built around a value whose text is not pinned (a refused table caption) holds the mark inside.
    e.message.contains(UNPINNED_ERROR)
}

pub(super) fn unpinned_refusal(e: &AlError, what: &str) -> Signal {
    let probe = e
        .message
        .strip_prefix(UNPINNED_ERROR)
        .map(|p| p.trim_start_matches(PINNED_CODE));
    match probe.filter(|p| !p.is_empty()) {
        Some(probe) => Signal::Unsupported(format!(
            "{what} of an error whose text is not pinned (probe wanted: {probe}, docs/probes-needed.md)"
        )),
        None => Signal::Unsupported(format!("{what} of an error whose text is not pinned")),
    }
}

/// The start of the failure message of a test [`unpinned_failure_text`] reports.
pub const UNPINNED_FAILURE_MARK: &str = "[error text not pinned]";

/// The failure message of a test that ended on an error whose text is not pinned, for a run that reports such a test as
/// failed (opt-in `ALRUNNER_REPORT_UNPINNED_ERRORS=1`, al-runner issue 6487): "[error text not pinned]", then what the
/// error says besides its marks (the probe it names, or the pinned text around an unpinned value), if anything.
pub fn unpinned_failure_text(e: &AlError) -> String {
    let rest: String = e
        .message
        .chars()
        .filter(|c| !UNPINNED_ERROR.contains(*c) && *c != PINNED_CODE)
        .collect();
    match rest.trim() {
        "" => UNPINNED_FAILURE_MARK.to_string(),
        rest => format!("{UNPINNED_FAILURE_MARK} {rest}"),
    }
}

/// Refuses a read of the text of an error whose text is not pinned: `what` is the read; the reason names the probe when the
/// error carries one. Every place that hands an error's text to AL code or to the test result calls it.
pub fn refuse_unpinned_text(e: &AlError, what: &str) -> Result<(), Signal> {
    if text_unpinned(e) {
        Err(unpinned_refusal(e, what))
    } else {
        Ok(())
    }
}

/// Refuses a read of the text or the code of an error when either is not pinned.
pub fn refuse_unpinned(e: &AlError, what: &str) -> Result<(), Signal> {
    let code_pinned = e
        .message
        .strip_prefix(UNPINNED_ERROR)
        .is_some_and(|p| p.starts_with(PINNED_CODE));
    if (text_unpinned(e) && !code_pinned) || e.code == UNPINNED_ERROR {
        Err(unpinned_refusal(e, what))
    } else {
        Ok(())
    }
}
