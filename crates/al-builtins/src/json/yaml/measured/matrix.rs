//! The punctuation matrix of corpus PR 1519 (`tests/al-language/json/JsonYamlPunctMatrix.al`, codeunit 75623, merged on
//! corpus master at b3dd6fc9, not yet pinned; al-runner issue 8580): each printable ASCII punctuation mark, and `...`,
//! `: `, ` #`, `- `, `? `, in four slots of a value (S = mark + `ab`, M = `a` + mark + `b`, E = `ab` + mark, W =
//! `a ` + mark + ` b`). Citations are lines of that file. A read that takes one of these rules is marked
//! `read-from-yaml-non-ascii-text`. `Parser` decides the place; this module only reads the value.

/// What a plain or quoted value of the matrix reads as.
#[derive(Debug, PartialEq)]
pub(super) enum Cell {
    Text(String),
    Null,
    /// `ReadFromYaml` answers false with the YAML error (`F1`).
    BadYaml,
    /// A list item YAML reads as a mapping or a nested list (`- ab:`, `- a : b`, `- - ab`, `- ? ab`, :299-301), not read.
    Changes,
}

/// Where a plain value stands: the value of a mapping (`PlainTop`, `PlainNested`, `PlainListMapFirst`,
/// `PlainListMapSecond`, :173-397) or a list item (`PlainListItem`, `PlainNestedList`, :263-307, :398-442).
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Place {
    MapValue,
    ListItem,
}

/// Marks that make the S slot answer false with the YAML error in every plain place (:176-200 `S F1`).
const BAD_START: &str = "!\"%'*,>@[]`{|}";

/// Marks the S slot reads as written before a letter (:179-206 `S ok`); `...` too (:211).
const OK_START: &str = "$()+-./:;<=?\\^_~";

fn letters(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_ascii_alphabetic())
}

fn letters_spaces(s: &str) -> bool {
    s.chars().all(|c| c.is_ascii_alphabetic() || c == ' ')
}

/// The text of a comment the cells hold after `#`: letters and spaces (`#ab`, `a #b`, `a # b`, `ab #`, :178, :210).
fn comment(c: &str) -> bool {
    letters_spaces(c)
}

/// The value `body` (the text after `key: ` or `- `) of a plain scalar at `place`, as the matrix reads it, or `None`
/// for a shape it does not measure. `doc` is the whole text: a quote, bracket or brace the value opens must not be
/// closed later, and `*` must have no anchor to name.
pub(super) fn plain(body: &str, place: Place, doc: &str) -> Option<Cell> {
    if !body.is_ascii() || body.chars().any(|c| c.is_ascii_control()) {
        return None;
    }
    // `#ab` and ` #ab` start a comment: the value is null (S of `#` and ` #`, :178, :210).
    if let Some(c) = body.strip_prefix('#').or_else(|| body.strip_prefix(" #")) {
        return (comment(c) && c.contains(|c: char| c.is_ascii_alphabetic())).then_some(Cell::Null);
    }
    // `&ab` is an anchor without a value: null (:181).
    if let Some(name) = body.strip_prefix('&') {
        return letters(name).then_some(Cell::Null);
    }
    // A tail comment ` #` and letters and spaces is cut, with one space before it (W of ` #`: `a  # b` is `a`, :210).
    let (value, cut) = match body.split_once(" #") {
        Some((v, c)) if comment(c) => (v.strip_suffix(' ').unwrap_or(v), true),
        Some(_) => return None,
        None => (body, false),
    };
    // One space after a closing `-` or `?` is dropped (E of `- ` and `? `: `ab-`, `ab?`, :211-212); after `:` it is
    // E of `: ` (:209).
    let value = match value.strip_suffix(' ') {
        Some(v) if !cut && v.ends_with(['-', '?', ':']) => v,
        _ => value,
    };
    let first = value.chars().next()?;
    if value.ends_with(' ') {
        return None;
    }
    // S of `! " % ' * , > @ [ ] ` { | }`: false and the YAML error, measured with letters after the mark only.
    if BAD_START.contains(first) {
        return (!cut && letters(&value[1..]) && unclosed(first, doc)).then_some(Cell::BadYaml);
    }
    // S of `: `: false in both places; S of `- ` and `? `: false in a mapping value, a nested list or mapping in a list item.
    if let Some(rest) = value.strip_prefix([':', '-', '?']).and_then(|r| r.strip_prefix(' ')) {
        if cut || !letters(rest) {
            return None;
        }
        return Some(match (first, place) {
            (':', _) | (_, Place::MapValue) => Cell::BadYaml,
            _ => Cell::Changes,
        });
    }
    let after_start = value
        .strip_prefix("...")
        .or_else(|| value.strip_prefix(|c: char| OK_START.contains(c)))
        .unwrap_or(value);
    if !after_start.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    // Two spaces only after a lone `-`, `?` or `:` (W of `- `, `? `, `: `: `a -  b`, `a :  b`, :209-212).
    // Never three spaces in a row: no cell measures a run of three (the allowlist is exactly two).
    if value.contains("   ")
        || value
            .match_indices("  ")
            .any(|(i, _)| !matches!(value.get(i.saturating_sub(2)..i), Some(" -" | " ?" | " :")))
    {
        return None;
    }
    // `: ` inside or `:` at the end (M, E and W of `: `, E and W of `:`): false in a mapping value, a mapping in a list.
    if value.contains(": ") || value.ends_with(':') {
        return Some(match place {
            Place::MapValue => Cell::BadYaml,
            Place::ListItem => Cell::Changes,
        });
    }
    // A word YAML reads as null, a Boolean or a special number is no text the cells read.
    let special = super::keyword(value) || [".inf", ".nan"].iter().any(|s| value.eq_ignore_ascii_case(s));
    if special || !(cut || value.contains(|c: char| c.is_ascii_punctuation())) {
        return None;
    }
    Some(Cell::Text(value.to_string()))
}

/// A value opened by `first` is not closed anywhere in the text (the quote, bracket or brace stands once), and `*` has
/// no anchor to name: the cells hold one such mark in the whole text.
fn unclosed(first: char, doc: &str) -> bool {
    let count = |c: char| doc.matches(c).count();
    match first {
        '"' | '\'' => count(first) == 1,
        '[' => count('[') == 1 && count(']') == 0,
        '{' => count('{') == 1 && count('}') == 0,
        '*' => count('&') == 0,
        _ => true,
    }
}

/// A whole quoted top value `v` (`DoubleQuoted`, `SingleQuoted`, :668-756): every printable ASCII mark is kept; in double
/// quotes `\a` is U+0007 and `\ ` a space (:694 S, W; `\b`, M, is not read). A lone quote of its kind makes the value
/// end early or not at all: false and the YAML error (:673, :719 `F1`), measured with letters and spaces around it.
pub(super) fn quoted(v: &str, doc: &str) -> Option<Cell> {
    let q = v.chars().next().filter(|c| matches!(c, '"' | '\''))?;
    if !v.is_ascii() || v.chars().any(|c| c.is_ascii_control()) {
        return None;
    }
    let mut chars = v[1..].char_indices();
    let mut out = String::new();
    let mut end = None;
    while let Some((i, c)) = chars.next() {
        match c {
            '\\' if q == '"' => match chars.next().map(|(_, e)| e) {
                Some('a') => out.push('\u{7}'),
                Some(' ') => out.push(' '),
                Some('"') => out.push('"'),
                Some('n') => out.push('\n'),
                // `"ab\"` (E): the quote is escaped and nothing closes the value.
                None => break,
                _ => return None,
            },
            '\'' if q == '\'' && v[1 + i + 1..].starts_with('\'') => {
                chars.next();
                out.push('\'');
            }
            c if c == q => {
                end = Some(1 + i + 1);
                break;
            }
            c => out.push(c),
        }
    }
    let raw_ok = || {
        let inner = &v[1..];
        inner
            .chars()
            .all(|c| c.is_ascii_alphabetic() || c == ' ' || c == q || c == '\\')
            && inner.matches('\\').count() <= usize::from(inner.ends_with("\\\""))
            && doc.matches(q).count() == v.matches(q).count()
    };
    match end {
        Some(e) if e == v.len() => (!out.is_empty() && !super::keyword(&out)).then_some(Cell::Text(out)),
        // Text after the closing quote (`""ab"`, `"a"b"`, `"ab""`, `"a " b"`): one more quote ends the line.
        Some(_) => (v.matches(q).count() == 3 && v.ends_with(q) && raw_ok()).then_some(Cell::BadYaml),
        // Not closed (`"ab\"`, `'ab''`, and `"ab`, `'ab` of the plain S slot).
        None => raw_ok().then_some(Cell::BadYaml),
    }
}
