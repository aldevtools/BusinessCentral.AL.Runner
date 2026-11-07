//! Format of a TextBuilder (moved out of `format_value`, file size and function length ratchets).

use crate::format::{FormatFail, no_such_format};
use crate::value::LibraryValue;

/// A TextBuilder formats as the text it holds in the standard formats 0, 1, 2 and 9; the formats 3 and 10 do not exist
/// for it (text/textbuilder-bigtext/TxCTextBuilderEdit.al:700-714 Format_TextBuilder_StandardFormats). The other numbers
/// are not pinned.
pub(crate) fn text(l: &LibraryValue, standard: i32) -> Result<String, FormatFail> {
    match standard {
        0 | 1 | 2 | 9 => {}
        3 | 10 => return Err(no_such_format(standard, "TextBuilder")),
        _ => return Err(format!("Format of a TextBuilder (standard format {standard}) is not pinned yet").into()),
    }
    match l {
        LibraryValue::Empty => Ok(String::new()),
        LibraryValue::Text(s) | LibraryValue::TextCap(s, _) => Ok(s.clone()),
        _ => Err("Format of a TextBuilder of this kind is not pinned".to_string().into()),
    }
}

/// A TextBuilder cuts at a positive length and pads it on the right (TxCTextBuilderEdit.al:716-733 `length2` `[ab]`,
/// `length8` `[abc def ]`). A negative length is not pinned.
pub(crate) fn with_length(s: &str, length: i32) -> Result<String, FormatFail> {
    if length < 0 {
        return Err("Format of a TextBuilder with a negative length (not pinned yet)"
            .to_string()
            .into());
    }
    let cut: String = s.chars().take(length as usize).collect();
    let pad = (length as usize).saturating_sub(cut.chars().count());
    Ok(format!("{cut}{}", " ".repeat(pad)))
}
