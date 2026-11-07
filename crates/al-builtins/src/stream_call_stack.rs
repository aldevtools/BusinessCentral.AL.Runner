//! The call stack text of an error in a stream (al-runner issue 6853). The runtime's call stack text is the tracked frames, one
//! per line, and the mark `native::CALL_STACK_MARK` for the untracked frames below them: their text, their length and their line
//! breaks are not known. The corpus pins, for the call stack of a collected error and of the last error on the default, UTF-8,
//! UTF-16, Windows and MS-DOS encodings (tests/al-language/errors/WdCallStackWriteText.al, BC 27, 28, 29):
//! - `OutStream.WriteText` and `OutStream.Write` are never refused (:126-133); the stream then holds StrLen bytes of the text,
//!   2 x StrLen on UTF-16, and Write adds a line break (:136);
//! - `InStream.ReadText` returns the first line and the stream is not at its end after it (:138-141, :541);
//! - `InStream.Read` into a Text returns the whole text (:148-149);
//! - a ReadText loop to EOS returns the lines of the text, how many depends on the untracked frames (:151-159: 3 on Windows,
//!   9 on Linux);
//! - a JSON text of the call stack has no line break of its own and ReadText reads it back equal (:518-530).
//!
//! What depends on the untracked frames is refused here: a stream length, a byte count, and EOS, Position or a read once the
//! read position may be inside them.

use crate::native::{CALL_STACK_MARK, Signal};
use crate::stream::{Encoding, InState};

/// Whether `t` holds the mark of untracked call stack frames.
pub(crate) fn holds_mark(t: &str) -> bool {
    t.contains(CALL_STACK_MARK)
}

/// Whether stream bytes hold the mark: its UTF-8 bytes (what a UTF-8, Windows and MS-DOS stream write for it, see
/// `Encoding::encode`), or the bytes are UTF-16 text that holds it. Arbitrary binary content is almost never valid UTF-16, so
/// the length of an image is still answered.
pub(crate) fn bytes_hold_mark(b: &[u8]) -> bool {
    let utf8 = Encoding::Utf8.encode(&CALL_STACK_MARK.to_string());
    if b.windows(utf8.len()).any(|w| w == utf8.as_slice()) {
        return true;
    }
    b.len() % 2 == 0 && {
        let units = b.chunks_exact(2).map(|p| u16::from_le_bytes([p[0], p[1]]));
        let mut has = false;
        char::decode_utf16(units).all(|c| {
            has |= c == Ok(CALL_STACK_MARK);
            c.is_ok()
        }) && has
    }
}

/// `Blob.Length` and `InStream.Length` of content that holds the mark: StrLen of the untracked frames is not known.
pub(crate) fn refuse_length(b: &[u8], what: &str) -> Result<(), Signal> {
    if bytes_hold_mark(b) {
        return Err(Signal::Unsupported(format!(
            "{what} of a stream that holds a call stack text (the length of its untracked frames is not known)"
        )));
    }
    Ok(())
}

/// The byte count a stream function returns for a text that holds the mark.
pub(crate) fn refuse_count(what: &str) -> Result<(), Signal> {
    Err(Signal::Unsupported(format!(
        "the byte count {what} returns for a call stack text (the length of its untracked frames is not known)"
    )))
}

/// Whether the read position of `s` may be inside the untracked frames of a call stack text: a ReadText already returned
/// the mark, or the rest of the stream is the mark alone (the untracked frames may be empty, so EOS is not known).
pub(crate) fn position_unknown(s: &InState) -> bool {
    s.past_call_stack_mark || s.rest() == s.enc.encode(&CALL_STACK_MARK.to_string())
}

/// EOS, Position, ReadText and Read where `position_unknown`.
pub(crate) fn refuse_position(s: &InState, what: &str) -> Result<(), Signal> {
    if position_unknown(s) {
        return Err(Signal::Unsupported(format!(
            "InStream.{what} at the untracked frames of a call stack text (how many lines they hold is not known)"
        )));
    }
    Ok(())
}
