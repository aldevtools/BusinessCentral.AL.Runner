//! The characters outside printable ASCII that corpus PR 1515 measured in `ReadFromYaml` (`tests/al-language/json/
//! JsonYamlUtf8.al`, codeunit 75621, merged on corpus master at b7fa8dd7, not yet pinned; al-runner issue 8580), and the punctuation its dataset document holds in a
//! `>-` block scalar. A read that takes one of them is marked `read-from-yaml-non-ascii-text`. Citations are lines of
//! that file.

/// The measured characters (`EachChar`, :195-205; the cells :218-608, :678-705): U+00A7, U+00E9, U+00FC, U+00F8, U+20AC
/// and U+1F600 come back unchanged, as their own UTF-16 code units. U+0301 is measured only right after `e` (`{C}`,
/// :84-89), see `combining_after_e`.
pub(super) fn measured(c: char) -> bool {
    matches!(
        c,
        '\u{00A7}' | '\u{00E9}' | '\u{00FC}' | '\u{00F8}' | '\u{20AC}' | '\u{1F600}'
    )
}

/// The combining acute accent, measured as `e` followed by it (:84-89, every `EachChar` cell).
pub(super) const COMBINING_ACUTE: char = '\u{0301}';

/// `text` holds U+0301 only right after an `e`.
pub(super) fn combining_after_e(text: &str) -> bool {
    let mut prev = None;
    for c in text.chars() {
        if c == COMBINING_ACUTE && prev != Some('e') {
            return false;
        }
        prev = Some(c);
    }
    true
}

/// A character a letter of a measured token may be: an ASCII letter or digit, or a measured character (U+0301 is checked
/// for the whole text by `combining_after_e`).
pub(super) fn letter(c: char) -> bool {
    c.is_ascii_alphanumeric() || measured(c) || c == COMBINING_ACUTE
}

/// The line breaks YAML does not take raw: U+0085, U+2028 and U+2029 make `ReadFromYaml` answer false with the YAML
/// error (`ReadFromYaml_Utf8SpecialPlain_Read` :735-755, `..SpecialFolded_Read` :757-777, `..SpecialDoubleQuoted_Read`
/// :779-798), each measured between two ASCII letters of a top plain value, a top `>-` line and a top double-quoted
/// value, with the result used.
pub(super) fn bad_break(c: char) -> bool {
    matches!(c, '\u{0085}' | '\u{2028}' | '\u{2029}')
}

/// `text` with every bad break between two ASCII letters replaced by `x`, or `None` when a bad break stands elsewhere.
pub(super) fn breaks_between_letters(text: &str) -> Option<String> {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    for (i, &c) in chars.iter().enumerate() {
        if bad_break(c) {
            let before = i.checked_sub(1).and_then(|j| chars.get(j));
            let after = chars.get(i + 1);
            if !(before.is_some_and(char::is_ascii_alphabetic) && after.is_some_and(char::is_ascii_alphabetic)) {
                return None;
            }
            out.push('x');
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// The punctuation the dataset document holds inside its `>-` lines, never first on a line (`ReadFromYaml_Utf8Issue
/// Document_Read` :707-723, `..IssueDescriptionLen_Read` :725-733: `.`, `,`, `:`, `-`, `/`).
pub(super) fn folded_punctuation(c: char) -> bool {
    matches!(c, '.' | ',' | ':' | '-' | '/')
}
