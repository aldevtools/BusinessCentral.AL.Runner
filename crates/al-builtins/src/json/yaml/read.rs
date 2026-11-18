//! The YAML that `ReadFromYaml` reads: the shapes the corpus pins, each at the one place below that reads it. Every
//! other shape refuses. Citations are relative to `corpus/tests/al-language/`:
//! - P88: `json/json/P88ObjectPathYaml.al` (`ReadFromYaml_Shapes`, inputs :210-224, results :225-239);
//! - PnA: `json/PnAJsonYaml.al` (`ReadFromYaml_Shapes`, inputs :150-155, results :156-179);
//! - Dc: `json/DcJsonLeads.al` (`Item50_YamlRoundTrip_NestedList`, inputs :189-199, results :204-207);
//! - Probe: `json/TestProbeJsonContracts.al` (`Yaml_Read`, :573-576, an AL `\n` is a backslash and an `n`).

use super::printable;
use crate::json::{error, write_string};
use crate::native::{Signal, unsupported};

/// A YAML node as BC hands it on to JSON: every scalar but `null` is a text (P88:230-236, PnA:159).
#[derive(Debug, PartialEq)]
pub(crate) enum Yaml {
    Null,
    Text(String),
    Map(Vec<(String, Yaml)>),
    Seq(Vec<Yaml>),
}

impl Yaml {
    pub(crate) fn write_json(&self, out: &mut String) {
        match self {
            Yaml::Null => out.push_str("null"),
            Yaml::Text(t) => write_string(out, t),
            Yaml::Map(members) => {
                out.push('{');
                for (i, (k, v)) in members.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    write_string(out, k);
                    out.push(':');
                    v.write_json(out);
                }
                out.push('}');
            }
            Yaml::Seq(items) => {
                out.push('[');
                for (i, v) in items.iter().enumerate() {
                    if i > 0 {
                        out.push(',');
                    }
                    v.write_json(out);
                }
                out.push(']');
            }
        }
    }
}

/// The YAML error (P88:238, PnA:177-178).
fn bad_yaml() -> Signal {
    error("The data does not represent valid YAML.", "YamlBadFormat")
}

#[derive(Clone, Copy)]
struct Line<'a> {
    indent: usize,
    text: &'a str,
}

/// A block list item: `- ` and its content.
fn item(text: &str) -> Option<&str> {
    text.strip_prefix("- ")
}

/// The lines main read: each at indent 0, `key: value`, no list item and no flow mapping. Only they may hold a carriage
/// return before a line feed or a blank line (main's result); every other shape is pinned with LF only and at most one
/// trailing LF (P88:216; P88 `NL()`, PnA and Dc `Lf()` are character 10).
fn flat(lines: &[Line]) -> bool {
    lines
        .iter()
        .all(|l| l.indent == 0 && l.text.contains(": ") && !l.text.starts_with(['-', '{']))
}

/// The whole text.
pub(crate) fn document(text: &str) -> Result<Yaml, Signal> {
    // The empty text is null (P88:210,225; PnA:154,173).
    if text.is_empty() {
        return Ok(Yaml::Null);
    }
    if text.contains('\t') {
        return unsupported("ReadFromYaml of a text with a tab (not pinned)");
    }
    let lone_cr = text.split("\r\n").any(|part| part.contains('\r'));
    if lone_cr || !text.chars().all(|c| matches!(c, '\n' | '\r') || printable(c)) {
        return unsupported(
            "ReadFromYaml of a text with a control character, a lone carriage return, U+0085, U+2028, U+2029 or a byte order mark (not pinned)",
        );
    }
    let raw: Vec<&str> = text.lines().collect();
    let lines: Vec<Line> = raw
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let body = l.trim_start_matches(' ');
            Line {
                indent: l.len() - body.len(),
                text: body,
            }
        })
        .collect();
    let Some(first) = lines.first().copied() else {
        return unsupported("ReadFromYaml of a text of blank lines (not pinned)");
    };
    if !flat(&lines) && (text.contains('\r') || raw.len() != lines.len()) {
        return unsupported(
            "ReadFromYaml of a carriage return or a blank line in a nested, list or flow form (not pinned)",
        );
    }
    if first.indent != 0 {
        return unsupported("ReadFromYaml of a text whose first line is indented (not pinned)");
    }
    // A trailing line feed is pinned after the flat line `a: 1` only (P88:216).
    if text.ends_with('\n') && !flat(&lines) {
        return unsupported("ReadFromYaml of a line feed after a block, a flow mapping, a list or a word (not pinned)");
    }
    // A one-line flow mapping (P88:211-212).
    if lines.len() == 1 && first.text.starts_with('{') {
        return flow_mapping(first.text);
    }
    // A list at the top (P88:213,228).
    if item(first.text).is_some() {
        let (seq, end) = sequence(&lines, 0, 0)?;
        if end != lines.len() {
            return unsupported("ReadFromYaml of a top list followed by other lines (not pinned)");
        }
        return Ok(seq);
    }
    if lines.len() == 1 && first.text.split_once(": ").is_none() && !first.text.ends_with(':') {
        return top_word(first.text);
    }
    let (map, end) = mapping(&lines, 0, 0)?;
    if end != lines.len() {
        return unsupported("ReadFromYaml of a line after the top mapping (not pinned)");
    }
    Ok(map)
}

/// A whole text of one word of ASCII letters (P88:214,229 `hello`). Several words, a digit, another letter, `null` and a
/// Boolean-like word are not pinned.
fn top_word(t: &str) -> Result<Yaml, Signal> {
    if t.chars().all(|c| c.is_ascii_alphabetic()) && !keyword(t) && !["Null", "NULL"].contains(&t) {
        return Ok(Yaml::Text(t.to_string()));
    }
    unsupported(
        "ReadFromYaml of a whole text that is not one word of ASCII letters, or is null or a Boolean-like word (not pinned)",
    )
}

/// `key: value` and `key:` lines at `indent` (0, or 2 below a top key), from `at`. Returns the mapping and the index of
/// the first line after it.
fn mapping(lines: &[Line], at: usize, indent: usize) -> Result<(Yaml, usize), Signal> {
    let mut members = Vec::new();
    let mut i = at;
    while let Some(line) = lines.get(i).copied().filter(|l| l.indent == indent) {
        if let Some((k, v)) = line.text.split_once(": ") {
            if indent == 0 && lines.len() == 1 && pinned_bad_format(k, v) {
                return Err(bad_yaml());
            }
            let k = key(k)?;
            let value = if indent == 0 { top_value(v)? } else { inner_value(v)? };
            members.push((k, value));
            i += 1;
        } else if let Some(k) = line.text.strip_suffix(':') {
            let k = key(k)?;
            let (value, end) = block_under(lines, i + 1, indent, i == 0)?;
            members.push((k, value));
            i = end;
        } else {
            return unsupported("ReadFromYaml of a line that is not `key: value` or `key:` (not pinned)");
        }
    }
    if indent == 2 && members.len() > 1 {
        return unsupported("ReadFromYaml of a mapping indented by two with several members (not pinned)");
    }
    if lines.get(i).is_some_and(|l| l.indent > indent) {
        return unsupported("ReadFromYaml of a line indented other than by two spaces (not pinned)");
    }
    Ok((Yaml::Map(members), i))
}

/// What `key:` at `indent` holds, read from the line `at`. Every pinned block under a key ends the text (P88:218-219,
/// PnA:151-152, Dc:189-199), holds one member, and its key is the first and only top line (`first`); only a null key is
/// followed by a line. A key after another top line, or any block under it, is not pinned.
fn block_under(lines: &[Line], at: usize, indent: usize, first: bool) -> Result<(Yaml, usize), Signal> {
    if indent == 0 && !first {
        return unsupported("ReadFromYaml of `key:` below another top line (not pinned)");
    }
    let block = match lines.get(at).copied() {
        // A list indented by two, below a top key (PnA:152,167; Dc:189,204) or below a key at indent 2 (Dc:199,207).
        Some(n) if item(n.text).is_some() && n.indent == indent + 2 => sequence(lines, at, indent + 2)?,
        // A list at its top key's own indent (P88:219,234; Dc:195,206).
        Some(n) if item(n.text).is_some() && n.indent == 0 && indent == 0 => sequence(lines, at, 0)?,
        // A mapping indented by two below a top key (P88:218,233; PnA:151,163; Dc:199,207).
        Some(n) if n.indent == 2 && indent == 0 => mapping(lines, at, 2)?,
        // A top key followed by a top `key: value` line is null (P88:224,239).
        Some(n) if n.indent == 0 && indent == 0 && item(n.text).is_none() && n.text.contains(": ") => {
            return Ok((Yaml::Null, at));
        }
        _ => return unsupported("ReadFromYaml of `key:` with this line or nothing below it (not pinned)"),
    };
    if block.1 != lines.len() {
        return unsupported("ReadFromYaml of a line after a nested mapping or a list (not pinned)");
    }
    Ok(block)
}

/// `- ` lines at `indent`, from `at`. An item is a plain word or number; in a list at indent 4 also a mapping whose
/// first pair follows the `- ` and whose other pairs are indented by two more (Dc:199,207).
fn sequence(lines: &[Line], at: usize, indent: usize) -> Result<(Yaml, usize), Signal> {
    let mut items = Vec::new();
    let mut i = at;
    while let Some(rest) = lines.get(i).filter(|l| l.indent == indent).and_then(|l| item(l.text)) {
        i += 1;
        let Some((k, v)) = rest.split_once(": ") else {
            items.push(inner_value(rest)?);
            continue;
        };
        if indent != 4 {
            return unsupported("ReadFromYaml of a mapping item in a list at another indent than 4 (not pinned)");
        }
        let mut members = vec![(key(k)?, inner_value(v)?)];
        while let Some(line) = lines.get(i).copied().filter(|l| l.indent == indent + 2) {
            let Some((k, v)) = line.text.split_once(": ") else {
                return unsupported(
                    "ReadFromYaml of a list item mapping with a line that is not `key: value` (not pinned)",
                );
            };
            members.push((key(k)?, inner_value(v)?));
            i += 1;
        }
        items.push(Yaml::Map(members));
    }
    if lines.get(i).is_some_and(|l| l.indent > indent) {
        return unsupported("ReadFromYaml of a line indented other than by two spaces (not pinned)");
    }
    Ok((Yaml::Seq(items), i))
}

/// The lines that are no YAML, each pinned as the whole text: the key `a` and a value that starts with a digit and holds
/// `: ` again (Probe:573,575, `a: 1\nb: text` with a backslash), the empty key with the value `: :` (PnA:155,177), and the
/// key `a` with an unclosed `[` of plain items (P88:223,238, `a: [1, 2`). The key is checked first and the value holds no
/// ` #` (a comment); every other value with `: ` refuses in `plain`.
fn pinned_bad_format(k: &str, v: &str) -> bool {
    if k.is_empty() {
        return v == ": :";
    }
    if key(k).is_err() || v.contains(" #") {
        return false;
    }
    let digit_then_colon = v.starts_with(|c: char| c.is_ascii_digit()) && v.contains(": ");
    let unclosed_list = v
        .strip_prefix('[')
        .is_some_and(|rest| !rest.is_empty() && !rest.contains(['[', ']', '{', '}', '\'', '"']));
    digit_then_colon || unclosed_list
}

/// A value on a line of the top mapping: a quoted text, `null` or a plain value `plain` takes (as main did; a flat line
/// keeps main's result).
fn top_value(v: &str) -> Result<Yaml, Signal> {
    match quoted(v)? {
        Some(text) => Ok(text),
        None => plain(v),
    }
}

/// A value below the top mapping or in a list: one word of ASCII letters and digits (P88:213,218-219; PnA:151-152;
/// Dc:189-199). `null`, quoted texts and any other character are pinned as top values only, if at all.
fn inner_value(v: &str) -> Result<Yaml, Signal> {
    match plain(v)? {
        text @ Yaml::Text(_) if v.chars().all(|c| c.is_ascii_alphanumeric()) => Ok(text),
        _ => {
            unsupported("ReadFromYaml of a value below the top mapping or in a list that is not one word (not pinned)")
        }
    }
}

/// A word YAML may read as null or a Boolean: pinned only as a plain top value (P88:221-222, PnA:150).
pub(super) fn keyword(w: &str) -> bool {
    ["null", "true", "false", "yes", "no", "on", "off"]
        .iter()
        .any(|k| w.eq_ignore_ascii_case(k))
}

/// `'1'` (P88:220,235) and `"x: y"` (PnA:153,171) are the texts inside: letters, digits, `:` and inner spaces. Escapes,
/// an empty quote, other characters and a word YAML reads as null or a Boolean are not pinned.
fn quoted(v: &str) -> Result<Option<Yaml>, Signal> {
    let Some(q) = v.chars().next().filter(|c| matches!(c, '\'' | '"')) else {
        return Ok(None);
    };
    let inner = v[1..].strip_suffix(q).unwrap_or_default();
    let pinned_chars = inner.chars().all(|c| c.is_ascii_alphanumeric() || c == ':' || c == ' ');
    if inner.is_empty() || !pinned_chars || inner.starts_with(' ') || inner.ends_with(' ') || keyword(inner) {
        return unsupported("ReadFromYaml of a quoted text other than letters, digits, `:` and spaces (not pinned)");
    }
    Ok(Some(Yaml::Text(inner.to_string())))
}

/// YAML's indicator characters: a plain scalar does not start with one.
const INDICATORS: &[char] = &[
    '-', '?', ':', ',', '[', ']', '{', '}', '#', '&', '*', '!', '|', '>', '\'', '"', '%', '@', '`',
];

/// The part a plain key or value shares: no indicator or space first, no space or `:` last, no comment, no `: `.
fn plain_text(v: &str) -> Option<&str> {
    let first = v.chars().next()?;
    let ends_open = v.ends_with([' ', ':']);
    (!INDICATORS.contains(&first) && first != ' ' && !ends_open && !v.contains(" #") && !v.contains(": ")).then_some(v)
}

/// A plain value: `null` is null (P88:222,237); a word or a whole number reads as its text (P88:215,221,230,236;
/// PnA:150,159). Other null forms, other numbers (a sign, a fraction, a leading zero, a date or a time) and the rest are
/// not pinned.
fn plain(v: &str) -> Result<Yaml, Signal> {
    if v == "null" {
        return Ok(Yaml::Null);
    }
    let Some(text) = plain_text(v) else {
        return unsupported("ReadFromYaml of a value that is not a plain word or number (not pinned)");
    };
    let starts_number = text.starts_with(|c: char| c.is_ascii_digit() || c == '+' || c == '.');
    let whole_number = text.chars().all(|c| c.is_ascii_digit()) && (text.len() == 1 || !text.starts_with('0'));
    if ["~", "Null", "NULL"].contains(&text) || (starts_number && !whole_number) {
        return unsupported(
            "ReadFromYaml of a null form or a number other than `null` and a whole number (not pinned)",
        );
    }
    Ok(Yaml::Text(text.to_string()))
}

/// A plain key (every pin; a key may hold `:` and `\`, Probe:574,576, or `<`, P88:224,239).
fn key(k: &str) -> Result<String, Signal> {
    match plain_text(k) {
        Some(k) if !["null", "~", "Null", "NULL"].contains(&k) => Ok(k.to_string()),
        _ => unsupported("ReadFromYaml of a key that is not plain (not pinned)"),
    }
}

/// `{a: 1, b: x}` (P88:211,226): plain keys, `: ` and entries split by `, `; `{"a":1}` (P88:212,227): one double-quoted
/// key and `:`. No space inside the braces; each value a word or a whole number, not one YAML may read as null or a
/// Boolean.
fn flow_mapping(t: &str) -> Result<Yaml, Signal> {
    let Some(inner) = t.strip_prefix('{').and_then(|s| s.strip_suffix('}')) else {
        return unsupported("ReadFromYaml of a flow mapping with text after its `}` (not pinned)");
    };
    let entries: Vec<(String, &str)> = match inner.strip_prefix('"') {
        Some(rest) => match rest.split_once("\":") {
            Some((k, v)) if !k.contains(['"', '\\', ',']) && v.chars().all(|c| c.is_ascii_digit()) => {
                vec![(key(k)?, v)]
            }
            _ => return unsupported("ReadFromYaml of a flow mapping entry with this quoted key (not pinned)"),
        },
        None => {
            let mut entries = Vec::new();
            for entry in inner.split(", ") {
                let Some((k, v)) = entry.split_once(": ") else {
                    return unsupported("ReadFromYaml of a flow mapping entry that is not `key: value` (not pinned)");
                };
                entries.push((key(k)?, v));
            }
            if entries.len() != 2 {
                return unsupported("ReadFromYaml of a flow mapping with other than two plain entries (not pinned)");
            }
            entries
        }
    };
    let mut members = Vec::new();
    for (k, v) in entries {
        let word = !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric()) && !keyword(v);
        match plain(v)? {
            text @ Yaml::Text(_) if word => members.push((k, text)),
            _ => {
                return unsupported(
                    "ReadFromYaml of a flow mapping value that is not a word or a whole number (not pinned)",
                );
            }
        }
    }
    Ok(Yaml::Map(members))
}

#[cfg(test)]
mod tests;
