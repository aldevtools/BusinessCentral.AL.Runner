//! The YAML that `ReadFromYaml` reads: the shapes the corpus pins, each at the one place below that reads it. Every
//! other shape refuses. Citations are relative to `corpus/tests/al-language/`:
//! - P88: `json/json/P88ObjectPathYaml.al` (`ReadFromYaml_Shapes`, inputs :210-224, results :225-239);
//! - PnA: `json/PnAJsonYaml.al` (`ReadFromYaml_Shapes`, inputs :150-155, results :156-179);
//! - Dc: `json/DcJsonLeads.al` (`Item50_YamlRoundTrip_NestedList`, inputs :189-199, results :204-207);
//! - Probe: `json/TestProbeJsonContracts.al` (`Yaml_Read`, :573-576, an AL `\n` is a backslash and an `n`).

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

/// The whole text. Blank lines are skipped (P88:216, a text that ends with a line feed).
pub(crate) fn document(text: &str) -> Result<Yaml, Signal> {
    // The empty text is null (P88:210,225; PnA:154,173).
    if text.is_empty() {
        return Ok(Yaml::Null);
    }
    if text.contains('\t') {
        return unsupported("ReadFromYaml of a text with a tab (not pinned)");
    }
    let lines: Vec<Line> = text
        .lines()
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
    if first.indent != 0 {
        return unsupported("ReadFromYaml of a text whose first line is indented (not pinned)");
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
    // A single word (P88:214,229); a single `null` is not pinned.
    if lines.len() == 1 && first.text.split_once(": ").is_none() && !first.text.ends_with(':') {
        return match plain(first.text)? {
            text @ Yaml::Text(_) => Ok(text),
            _ => unsupported("ReadFromYaml of a single null (not pinned)"),
        };
    }
    let (map, end) = mapping(&lines, 0, 0)?;
    if end != lines.len() {
        return unsupported("ReadFromYaml of a line after the top mapping (not pinned)");
    }
    Ok(map)
}

/// `key: value` and `key:` lines at `indent`, from `at`. Returns the mapping and the index of the first line after it.
fn mapping(lines: &[Line], at: usize, indent: usize) -> Result<(Yaml, usize), Signal> {
    let mut members = Vec::new();
    let mut i = at;
    while let Some(line) = lines.get(i).copied().filter(|l| l.indent == indent) {
        if let Some((k, v)) = line.text.split_once(": ") {
            let value = if indent == 0 {
                top_value(v, i + 1 == lines.len())?
            } else {
                value(v)?
            };
            members.push((key(k)?, value));
            i += 1;
        } else if let Some(k) = line.text.strip_suffix(':') {
            let k = key(k)?;
            let (value, end) = block_under(lines, i + 1, indent)?;
            members.push((k, value));
            i = end;
        } else {
            return unsupported("ReadFromYaml of a line that is not `key: value` or `key:` (not pinned)");
        }
    }
    if lines.get(i).is_some_and(|l| l.indent > indent) {
        return unsupported("ReadFromYaml of a line indented other than by two spaces (not pinned)");
    }
    Ok((Yaml::Map(members), i))
}

/// What `key:` at `indent` holds, read from the line `at`.
fn block_under(lines: &[Line], at: usize, indent: usize) -> Result<(Yaml, usize), Signal> {
    match lines.get(at).copied() {
        // A list indented by two (PnA:152,167; Dc:189,204) or at the key's own indent (P88:219,234; Dc:195,206).
        Some(n) if item(n.text).is_some() && n.indent == indent + 2 => sequence(lines, at, indent + 2),
        Some(n) if item(n.text).is_some() && n.indent == indent => sequence(lines, at, indent),
        // A mapping indented by two (P88:218,233; PnA:151,163; Dc:199,207).
        Some(n) if n.indent == indent + 2 => mapping(lines, at, indent + 2),
        // A key with nothing under it before the next key is null (P88:224,239).
        Some(n) if n.indent == indent => Ok((Yaml::Null, at)),
        _ => unsupported("ReadFromYaml of `key:` at the end of a block (not pinned)"),
    }
}

/// `- ` lines at `indent`, from `at`. An item is a scalar, or a mapping whose first pair follows the `- ` and whose
/// other pairs are indented by two more (Dc:199,207).
fn sequence(lines: &[Line], at: usize, indent: usize) -> Result<(Yaml, usize), Signal> {
    let mut items = Vec::new();
    let mut i = at;
    while let Some(rest) = lines.get(i).filter(|l| l.indent == indent).and_then(|l| item(l.text)) {
        i += 1;
        let Some((k, v)) = rest.split_once(": ") else {
            items.push(value(rest)?);
            continue;
        };
        let mut members = vec![(key(k)?, value(v)?)];
        while let Some(line) = lines.get(i).copied().filter(|l| l.indent == indent + 2) {
            let Some((k, v)) = line.text.split_once(": ") else {
                return unsupported(
                    "ReadFromYaml of a list item mapping with a line that is not `key: value` (not pinned)",
                );
            };
            members.push((key(k)?, value(v)?));
            i += 1;
        }
        items.push(Yaml::Map(members));
    }
    if lines.get(i).is_some_and(|l| l.indent > indent) {
        return unsupported("ReadFromYaml of a line indented other than by two spaces (not pinned)");
    }
    Ok((Yaml::Seq(items), i))
}

/// A value on a line of the top mapping. No YAML: a value that starts with a letter or digit and holds `: ` again
/// (Probe:573,575, `1\nb: text`), a value that starts with `: ` (PnA:155,177, `: : :`), and an unclosed `[` on the last
/// line (P88:223,238). Any other value with `: ` refuses in `plain`.
fn top_value(v: &str, last: bool) -> Result<Yaml, Signal> {
    if let Some(text) = quoted(v)? {
        return Ok(text);
    }
    let word_then_colon = v.starts_with(char::is_alphanumeric) && v.contains(": ");
    let unclosed_list = last && v.starts_with('[') && !v.contains(']');
    if word_then_colon || v.starts_with(": ") || unclosed_list {
        return Err(bad_yaml());
    }
    plain(v)
}

/// A value below the top mapping.
fn value(v: &str) -> Result<Yaml, Signal> {
    match quoted(v)? {
        Some(text) => Ok(text),
        None => plain(v),
    }
}

/// `'1'` (P88:220,235) and `"x: y"` (PnA:153,171) are the texts inside. Escapes and an empty quote are not pinned.
fn quoted(v: &str) -> Result<Option<Yaml>, Signal> {
    for q in ['\'', '"'] {
        if !v.starts_with(q) {
            continue;
        }
        return match v[1..].strip_suffix(q) {
            Some(inner) if !inner.is_empty() && !inner.contains([q, '\\']) => Ok(Some(Yaml::Text(inner.to_string()))),
            _ => unsupported("ReadFromYaml of a quoted text with an escape, an inner quote or no end (not pinned)"),
        };
    }
    Ok(None)
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
/// PnA:150,159). Other null forms, other numbers (a sign, a fraction, a leading zero) and the rest are not pinned.
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

/// `{a: 1, b: x}` (P88:211,226) and `{"a":1}` (P88:212,227): plain or double-quoted keys, plain texts as values.
fn flow_mapping(t: &str) -> Result<Yaml, Signal> {
    let inner = t.strip_prefix('{').and_then(|s| s.strip_suffix('}'));
    let Some(inner) = inner.filter(|s| !s.trim().is_empty() && !s.contains(['{', '}', '[', ']', '\''])) else {
        return unsupported(
            "ReadFromYaml of a flow mapping that is empty or holds a collection or a quote (not pinned)",
        );
    };
    let mut members = Vec::new();
    for entry in inner.split(',').map(str::trim) {
        let (k, v) = match entry.strip_prefix('"') {
            Some(rest) => match rest.split_once("\":") {
                Some((k, v)) if !k.is_empty() && !k.contains(['"', '\\']) => (k.to_string(), v),
                _ => return unsupported("ReadFromYaml of a flow mapping entry with this quoted key (not pinned)"),
            },
            None => match entry.split_once(": ") {
                Some((k, v)) => (key(k)?, v),
                None => {
                    return unsupported("ReadFromYaml of a flow mapping entry that is not `key: value` (not pinned)");
                }
            },
        };
        match plain(v)? {
            Yaml::Text(v) if !v.contains('"') => members.push((k, Yaml::Text(v))),
            _ => return unsupported("ReadFromYaml of a flow mapping value that is not a plain text (not pinned)"),
        }
    }
    Ok(Yaml::Map(members))
}

#[cfg(test)]
mod tests;
