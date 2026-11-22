//! The YAML shapes corpus PR 1493 measured on BC 27.5, 28.5 and 29.0 (`tests/al-language/json/JsonYamlBlankLines.al`,
//! codeunit 75620, open; al-runner issue 8580), read only for a text the pinned reader (`read::document`) refuses. The
//! caller marks the assumption `read-from-yaml-blank-lines-and-blocks` on every read this module answers. Every shape no
//! cell measures refuses. A cell is named `Test/cell` after the probe's test and its `Observe` name.
//!
//! Measured: blank lines and lines of spaces only between list items, between keys and after the last line; CRLF line
//! ends throughout; block lists of words under a key at its own indent (top keys only), block lists indented by two whose
//! items are words, flow forms or mappings whose first pair follows the `- `; nested mappings indented by two; the block
//! scalars `|`, `|-`, `>` and `>-`; comment lines at indent 0 between top keys and a tail comment after a plain value;
//! quoted keys and values; flow lists and flow mappings of one line; plain scalars as text; `Null` and `null` as a top
//! value null, and the empty top value and the first top line `key:` as null. Plain keys start with a letter. A word YAML
//! reads as null or a Boolean is neither a key nor a quoted value (the pinned reader refuses them on purpose).

use super::read::{Yaml, bad_yaml, keyword};
use crate::native::{Signal, unsupported};

mod matrix;
mod utf8;

/// What a measured read gave: the tree, and whether it took a character or a punctuation mark only corpus PR 1515
/// measured (`utf8`; the caller then also marks `read-from-yaml-non-ascii-text`).
pub(super) struct Measured {
    pub(super) tree: Yaml,
    pub(super) utf8: bool,
}

/// One line of the text after CRLF became LF.
#[derive(Clone, Copy, PartialEq)]
enum Row<'a> {
    /// The empty line.
    Empty,
    /// A line of spaces only.
    Spaces,
    /// A line whose first character after its indent is `#`.
    Comment { indent: usize, text: &'a str },
    /// Any other line: its indent in spaces and the rest.
    Text { indent: usize, text: &'a str },
}

fn row(line: &str) -> Row<'_> {
    let text = line.trim_start_matches(' ');
    let indent = line.len() - text.len();
    match text {
        "" if indent == 0 => Row::Empty,
        "" => Row::Spaces,
        _ if text.starts_with('#') => Row::Comment { indent, text },
        _ => Row::Text { indent, text },
    }
}

/// `ReadFromYaml` of `text` in a measured shape, or a refusal. A raw U+0085, U+2028 or U+2029 at a place corpus PR 1515
/// measured gives the YAML error (`Signal::Error`, raised nowhere else in this module) once the rest of the text reads.
pub(super) fn document(text: &str) -> Result<Measured, Signal> {
    // Printable ASCII, LF and CR (every cell of PR 1493), the characters PR 1515 measured, and its bad line breaks, which
    // `Parser` takes only at their measured places. A tab, a byte order mark and every other character refuse.
    let known = |c: char| matches!(c, '\n' | '\r' | ' '..='~') || utf8::letter(c) || utf8::bad_break(c);
    if !text.chars().all(known) || !utf8::combining_after_e(text) {
        return unsupported(
            "ReadFromYaml of a character other than printable ASCII that no cell measures (not measured)",
        );
    }
    // Either no CR, or every line end CRLF (the `-crlf` cells).
    let crlf = text.contains('\r');
    if crlf && text.split("\r\n").any(|part| part.contains(['\r', '\n'])) {
        return unsupported("ReadFromYaml of a text that mixes CRLF with LF or holds a lone CR (not measured)");
    }
    let lf = text.replace("\r\n", "\n");
    let rows: Vec<Row> = lf.split('\n').map(row).collect();
    let mut p = Parser {
        rows: &rows,
        doc: &lf,
        at: 0,
        taken: 0,
        punctuation: false,
        matrix: false,
        bad_break: false,
        yaml_error: false,
        in_item_map: 0,
        later_pair: false,
        block_rows: Vec::new(),
    };
    let tree = match p.peek() {
        // The text starts with its first line at indent 0 (every cell), or a comment line (CommentsAndQuotes/comment).
        Some(Row::Text { indent: 0, text }) if matches!(rows[0], Row::Text { .. } | Row::Comment { .. }) => {
            if text.starts_with("- ") {
                p.top_list()?
            } else {
                p.mapping(0, Vec::new())?
            }
        }
        _ => return unsupported("ReadFromYaml of a text that starts with a blank or indented line (not measured)"),
    };
    // After the last line only blank lines and lines of spaces follow (TrailingLines, SpacesOnlyLineMore/trailing).
    if p.rows[p.at..].iter().any(|r| !matches!(r, Row::Empty | Row::Spaces)) {
        return unsupported("ReadFromYaml of a line after the end of the block it belongs to (not measured)");
    }
    check_comments(&rows, &p.block_rows)?;
    // The matrix of PR 1519 holds LF line ends only, and no raw U+0085, U+2028 or U+2029 (JsonYamlPunctMatrix.al :33-34);
    // the cells of PR 1515 with a bad break hold no punctuation.
    if crlf && p.matrix {
        return unsupported(
            "ReadFromYaml of punctuation measured with LF line ends only, in a text with CRLF line ends (not measured)",
        );
    }
    if p.bad_break && (p.punctuation || p.matrix) {
        return unsupported(
            "ReadFromYaml of a raw U+0085, U+2028 or U+2029 with punctuation in one text (not measured)",
        );
    }
    if p.bad_break || p.yaml_error {
        return Err(bad_yaml());
    }
    Ok(Measured {
        tree,
        utf8: p.punctuation || p.matrix || !text.is_ascii(),
    })
}

/// A comment line is measured at indent 0, holding `# ` and words, first or after a top `key: value` line, and followed
/// by a top line (CommentsAndQuotes/comment `# head`, `# between`; DatasetDocument). A line of a block scalar that starts
/// with `#` (`block_rows`) is no comment.
fn check_comments(rows: &[Row], block_rows: &[usize]) -> Result<(), Signal> {
    for (i, r) in rows.iter().enumerate() {
        let Row::Comment { indent, text } = *r else { continue };
        if block_rows.contains(&i) {
            continue;
        }
        let words = text
            .strip_prefix("# ")
            .is_some_and(|t| t.chars().all(|c| utf8::letter(c) || c == ' '));
        let text_row = |r: &&Row| matches!(r, Row::Text { .. });
        let before = rows[..i].iter().rev().find(text_row);
        let after = rows[i + 1..].iter().find(text_row);
        let before_ok = match before {
            None => true,
            Some(Row::Text { indent, text }) => *indent == 0 && text.contains(": ") && !text.starts_with("- "),
            Some(_) => false,
        };
        let after_ok = matches!(after, Some(Row::Text { indent: 0, text }) if !text.starts_with("- "));
        if indent != 0 || !words || !before_ok || !after_ok {
            return unsupported("ReadFromYaml of a comment line at this place (not measured)");
        }
    }
    Ok(())
}

struct Parser<'r, 'a> {
    rows: &'r [Row<'a>],
    /// The whole text, LF line ends (`matrix::plain` looks for a closing quote, bracket or brace).
    doc: &'r str,
    at: usize,
    /// How many `Text` rows `take` moved past.
    taken: usize,
    /// A `>-` line took punctuation (`utf8::folded_punctuation`).
    punctuation: bool,
    /// A value or block line took a rule only the matrix of PR 1519 measured (`matrix`).
    matrix: bool,
    /// A bad line break stood at a measured place (`utf8::bad_break`).
    bad_break: bool,
    /// A value the matrix answers with the YAML error (`matrix::Cell::BadYaml`).
    yaml_error: bool,
    /// How many list item mappings the parser is inside (the matrix measured values in one, of a list under a top key).
    in_item_map: usize,
    /// The value being read is a later pair of a list item mapping (the matrix measured `>-` as the first pair only).
    later_pair: bool,
    /// The rows a block scalar took as its lines.
    block_rows: Vec<usize>,
}

impl<'a> Parser<'_, 'a> {
    /// The next `Text` row, skipping blank, space and comment rows (blank lines are ignored between lines).
    fn peek(&self) -> Option<Row<'a>> {
        self.rows[self.at..]
            .iter()
            .copied()
            .find(|r| matches!(r, Row::Text { .. }))
    }

    /// Moves past the next `Text` row.
    fn take(&mut self) {
        while !matches!(self.rows.get(self.at), Some(Row::Text { .. }) | None) {
            self.at += 1;
        }
        self.at += 1;
        self.taken += 1;
    }

    fn next_indent(&self) -> Option<usize> {
        match self.peek() {
            Some(Row::Text { indent, .. }) => Some(indent),
            _ => None,
        }
    }

    /// The row right after the one just taken, blank lines not skipped.
    fn adjacent(&self) -> Option<Row<'a>> {
        self.rows.get(self.at).copied()
    }

    /// A top list: words only, read as a list (BlankLineTopList/toplist, CrLfNoBlank/toplist).
    fn top_list(&mut self) -> Result<Yaml, Signal> {
        let list = self.sequence(0)?;
        words_only(list)
    }

    /// The `key: value` and `key:` lines at `indent`, added to `members` (the first pair of a list item mapping).
    fn mapping(&mut self, indent: usize, mut members: Vec<(String, Yaml)>) -> Result<Yaml, Signal> {
        let item_map = !members.is_empty();
        while let Some(Row::Text { indent: at, text }) = self.peek() {
            if at < indent {
                break;
            }
            if at > indent || text.starts_with("- ") {
                return unsupported("ReadFromYaml of a line indented other than by two spaces (not measured)");
            }
            self.take();
            let (key, rest) = key_and_rest(text)?;
            self.later_pair = item_map;
            let value = self.value(indent, rest);
            self.later_pair = false;
            let value = value?;
            if members.iter().any(|(k, _)| *k == key) {
                return unsupported("ReadFromYaml of a duplicate key (not measured)");
            }
            members.push((key, value));
        }
        Ok(Yaml::Map(members))
    }

    /// What follows the `:` of a key at column `col`.
    fn value(&mut self, col: usize, rest: &str) -> Result<Yaml, Signal> {
        if rest.chars().any(utf8::bad_break) {
            return self.value_with_bad_break(col, rest);
        }
        match rest {
            "" => self.block_under(col),
            // An empty top value is null (ScalarForms/words `c: `), followed by a top line.
            " " if col == 0 && matches!(self.next_indent(), Some(0)) => Ok(Yaml::Null),
            _ => {
                // `key_and_rest` leaves a space before every value it returns.
                let body = &rest[1..];
                let v = Some(body).filter(|v| !v.is_empty() && !v.starts_with(' '));
                let read = match v {
                    Some(ind @ ("|" | "|-" | ">" | ">-")) => return self.block_scalar(col, ind),
                    Some(v) if v.starts_with(['[', '{']) => flow(v),
                    Some(v) if v.starts_with(['"', '\'']) => quoted(v),
                    Some(v) => plain_value(v, col),
                    None => unsupported("ReadFromYaml of an empty value below the top mapping (not measured)"),
                };
                match read {
                    Err(Signal::Unsupported(_)) => self.matrix_value(col, body, read),
                    read => read,
                }
            }
        }
    }

    /// A mapping value the readers of PR 1493 and 1515 refuse (`refused`), read as the matrix of PR 1519 reads it at
    /// the places it measured: a top value, a value of a mapping under a top key, and a value of a list item mapping
    /// of a list under a top key (`PlainTop`, `PlainNested`, `PlainListMapFirst`, `PlainListMapSecond`, :173-397). A
    /// quoted value is read whole at the top only (`DoubleQuoted`, `SingleQuoted`, :668-756).
    fn matrix_value(&mut self, col: usize, body: &str, refused: Result<Yaml, Signal>) -> Result<Yaml, Signal> {
        let top = col == 0 && self.in_item_map == 0;
        let cell = match body.starts_with(['"', '\'']) && top {
            true => matrix::quoted(body, self.doc),
            false => matrix::plain(body, matrix::Place::MapValue, self.doc),
        };
        let place = matches!((col, self.in_item_map), (0 | 2, 0) | (4, 1));
        self.matrix_cell(cell, place, refused)
    }

    /// A list item the readers of PR 1493 and 1515 refuse, read as the matrix reads it in a list under a top key or
    /// under a key of a mapping under a top key (`PlainListItem`, `PlainNestedList`, :263-307, :398-442).
    fn matrix_item(&mut self, indent: usize, rest: &str, refused: Result<Yaml, Signal>) -> Result<Yaml, Signal> {
        let cell = matrix::plain(rest, matrix::Place::ListItem, self.doc);
        let place = matches!(indent, 2 | 4) && self.in_item_map == 0;
        self.matrix_cell(cell, place, refused)
    }

    fn matrix_cell(
        &mut self,
        cell: Option<matrix::Cell>,
        place: bool,
        refused: Result<Yaml, Signal>,
    ) -> Result<Yaml, Signal> {
        let Some(cell) = cell else { return refused };
        if !place {
            return unsupported("ReadFromYaml of punctuation in a plain value at this place (not measured)");
        }
        self.matrix = true;
        match cell {
            matrix::Cell::Text(t) => Ok(Yaml::Text(t)),
            matrix::Cell::Null => Ok(Yaml::Null),
            matrix::Cell::BadYaml => {
                self.yaml_error = true;
                Ok(Yaml::Null)
            }
            matrix::Cell::Changes => {
                unsupported("ReadFromYaml of a list item that YAML reads as a mapping or a nested list (not measured)")
            }
        }
    }

    /// A value holding a bad line break: measured between two ASCII letters of a top plain value without a tail comment
    /// and of a top double-quoted value (`utf8::bad_break`). The value is read with each break as a letter; every other
    /// place refuses.
    fn value_with_bad_break(&mut self, col: usize, rest: &str) -> Result<Yaml, Signal> {
        let lettered = utf8::breaks_between_letters(rest);
        let measured = |v: &str| {
            let v = v.strip_prefix(' ').unwrap_or_default();
            v.starts_with('"') || (v.starts_with(|c: char| c.is_ascii_alphabetic()) && !v.contains(" #"))
        };
        match lettered {
            Some(v) if col == 0 && measured(&v) => {
                self.bad_break = true;
                self.value(col, &v)
            }
            _ => unsupported("ReadFromYaml of a raw U+0085, U+2028 or U+2029 at this place (not measured)"),
        }
    }

    /// The block under `key:` at column `col`, starting on the very next line.
    fn block_under(&mut self, col: usize) -> Result<Yaml, Signal> {
        let Some(Row::Text { indent, text }) = self.adjacent() else {
            return unsupported("ReadFromYaml of `key:` followed by a blank line or nothing (not measured)");
        };
        let item = text.starts_with("- ");
        match (indent, item) {
            // A list indented by two (BlankLineNestedList, DatasetDocument `tests:` and `lines:`).
            (i, true) if i == col + 2 => self.sequence(i),
            // A list at the key's own indent, top keys only (BlankLineTopList/keylist0); it ends the text.
            (0, true) if col == 0 => {
                let list = words_only(self.sequence(0)?)?;
                if self.peek().is_some() {
                    return unsupported("ReadFromYaml of a line after a list at its key's indent (not measured)");
                }
                Ok(list)
            }
            // A mapping indented by two (BlankLineNestedKeys, SpacesOnlyLineMore/deep, DatasetDocument).
            (i, false) if i == col + 2 => self.mapping(i, Vec::new()),
            // The first top line `key:` followed by a top `key: value` line is null (P88ObjectPathYaml.al:224,239); after
            // another top line the pinned reader refuses it and no cell measures it.
            (0, false) if col == 0 && self.taken == 1 && text.contains(": ") => Ok(Yaml::Null),
            _ => unsupported("ReadFromYaml of `key:` with this line below it (not measured)"),
        }
    }

    /// `- ` lines at `indent`.
    fn sequence(&mut self, indent: usize) -> Result<Yaml, Signal> {
        let mut items = Vec::new();
        while let Some(Row::Text { indent: at, text }) = self.peek() {
            if at < indent || (at == indent && !text.starts_with("- ")) {
                break;
            }
            let Some(rest) = text.strip_prefix("- ").filter(|_| at == indent) else {
                return unsupported("ReadFromYaml of a line indented other than by two spaces (not measured)");
            };
            self.take();
            items.push(self.item(indent, rest)?);
        }
        Ok(Yaml::Seq(items))
    }

    /// A list item: a flow form (FlowInBlockList/flow), a mapping whose first pair follows the `- ` and whose other keys
    /// are indented by two more (BlankLineListItemMapping, BlockScalarInListItem, DatasetDocument), or a word.
    fn item(&mut self, indent: usize, rest: &str) -> Result<Yaml, Signal> {
        if rest.starts_with(['[', '{']) {
            return match flow(rest) {
                refused @ Err(Signal::Unsupported(_)) => self.matrix_item(indent, rest, refused),
                read => read,
            };
        }
        if let Ok((key, after)) = key_and_rest(rest) {
            if after.is_empty() || after == " " {
                return unsupported(
                    "ReadFromYaml of a list item mapping that opens a block on its dash line (not measured)",
                );
            }
            self.in_item_map += 1;
            let map = match self.value(indent + 2, after) {
                Ok(first) => self.mapping(indent + 2, vec![(key, first)]),
                Err(e) => Err(e),
            };
            self.in_item_map -= 1;
            return map;
        }
        match scalar(rest, false) {
            Ok(text @ Yaml::Text(_)) if !rest.starts_with(['"', '\'']) => Ok(text),
            Err(e @ Signal::Error(_)) => Err(e),
            refused @ Err(_) => self.matrix_item(indent, rest, refused),
            Ok(_) => {
                let refused = unsupported(
                    "ReadFromYaml of a list item that is not a word, a flow form or a mapping (not measured)",
                );
                self.matrix_item(indent, rest, refused)
            }
        }
    }

    /// A line of a block scalar: letters, digits, the measured characters and spaces, starting and ending with a letter or
    /// a digit (PR 1493, PR 1515); a `>-` line may also hold `utf8::folded_punctuation` inside. At the places of the
    /// matrix of PR 1519 (`>-`, `>`, `|`, `|-` under a top key, `>-` in a list item mapping of a list under a top key,
    /// :443-667) every printable ASCII mark is kept, anywhere on a line that holds an ASCII letter.
    fn block_line(&mut self, col: usize, ind: &str, text: &str) -> Result<(), Signal> {
        if text.contains(utf8::bad_break) {
            return self.block_line_with_break(col, ind, text);
        }
        let folded = ind == ">-";
        let inner = |c: char| utf8::letter(c) || c == ' ' || (folded && utf8::folded_punctuation(c));
        let edges = text.starts_with(utf8::letter) && text.ends_with(utf8::letter);
        if text.chars().all(inner) && edges {
            self.punctuation |= text.contains(utf8::folded_punctuation);
            return Ok(());
        }
        let printable = text.chars().all(|c| matches!(c, ' '..='~') || utf8::letter(c));
        if printable && text.contains(|c: char| c.is_ascii_alphabetic()) && self.matrix_block(col, ind) {
            self.matrix = true;
            return Ok(());
        }
        let marks = text
            .chars()
            .all(|c| utf8::letter(c) || c == ' ' || utf8::folded_punctuation(c));
        unsupported(match (folded, marks) {
            _ if printable && self.later_pair && folded && col == 4 && self.in_item_map == 1 => {
                "ReadFromYaml of a `>-` block line with punctuation as a later pair of a list item mapping (not measured)"
            }
            (true, true) => "ReadFromYaml of a block scalar line that starts or ends with punctuation (not measured)",
            (false, true) if !self.matrix_block(col, ind) => {
                "ReadFromYaml of punctuation in a `|`, `|-` or `>` line at this place (not measured)"
            }
            _ => "ReadFromYaml of a block scalar line with other characters (not measured)",
        })
    }

    /// A block scalar at a place of the matrix of PR 1519 (:443-667).
    fn matrix_block(&self, col: usize, ind: &str) -> bool {
        (col == 0 && self.in_item_map == 0) || (col == 4 && self.in_item_map == 1 && ind == ">-" && !self.later_pair)
    }

    /// A block line holding a raw U+0085, U+2028 or U+2029: measured in a top `>-` line of ASCII letters only, the break
    /// between two of them (`a{X}b`, JsonYamlUtf8.al :757-777).
    fn block_line_with_break(&mut self, col: usize, ind: &str, text: &str) -> Result<(), Signal> {
        if col != 0 || ind != ">-" {
            return unsupported("ReadFromYaml of a raw U+0085, U+2028 or U+2029 at this place (not measured)");
        }
        let letters = text.chars().all(|c| c.is_ascii_alphabetic() || utf8::bad_break(c));
        if !letters || utf8::breaks_between_letters(text).is_none() {
            return unsupported(
                "ReadFromYaml of a raw U+0085, U+2028 or U+2029 in a block line with other characters (not measured)",
            );
        }
        self.bad_break = true;
        Ok(())
    }

    /// A block scalar under the key at column `col`: its lines indented by two more, from the very next line
    /// (LiteralBlockScalar, FoldedBlockScalar, BlockScalarEnd, BlockScalarInListItem).
    fn block_scalar(&mut self, col: usize, ind: &str) -> Result<Yaml, Signal> {
        let content = col + 2;
        let mut lines: Vec<&str> = Vec::new();
        loop {
            match self.adjacent() {
                Some(Row::Text { indent, text }) if indent == content => {
                    self.block_line(col, ind, text)?;
                    lines.push(text);
                    self.at += 1;
                }
                // A line that starts with `#` is a line of the block (S of `#`, JsonYamlPunctMatrix.al :443-667).
                Some(Row::Comment { indent, text }) if indent == content => {
                    if !self.matrix_block(col, ind) {
                        return unsupported(
                            "ReadFromYaml of a block scalar line that starts with `#` at this place (not measured)",
                        );
                    }
                    self.block_line(col, ind, text)?;
                    self.block_rows.push(self.at);
                    lines.push(text);
                    self.at += 1;
                }
                // The first line indented by more (S of ` #`, :480) sets a deeper indent: not read.
                Some(Row::Text { indent, .. } | Row::Comment { indent, .. })
                    if lines.is_empty() && indent > content =>
                {
                    return unsupported(
                        "ReadFromYaml of a block scalar whose first line is indented by more than two (not measured)",
                    );
                }
                // One empty line between two lines of a `|` or `>` scalar (`literal-blank`, `folded-blank`).
                Some(Row::Empty)
                    if matches!(ind, "|" | ">")
                        && lines.last().is_some_and(|l| !l.is_empty())
                        && matches!(self.rows.get(self.at + 1), Some(Row::Text { indent, .. }) if *indent == content) =>
                {
                    lines.push("");
                    self.at += 1;
                }
                _ => break,
            }
        }
        if lines.is_empty() {
            return unsupported("ReadFromYaml of a block scalar without a line right below it (not measured)");
        }
        // How the scalar ends: a less indented line (every indicator), the end of the text (`|`, BlockScalarEnd/
        // literal-last; `>-`, BlockScalarInListItem), or one line feed and the end (`|`, BlockScalarEnd/literal-last-lf).
        let rest = &self.rows[self.at..];
        let final_lf = match (rest, ind) {
            ([Row::Text { indent, .. }, ..], _) if *indent < content => true,
            ([], "|" | ">-") => false,
            ([Row::Empty], "|") => true,
            _ => return unsupported("ReadFromYaml of a block scalar that ends in this way (not measured)"),
        };
        let mut text = if ind.starts_with('|') {
            lines.join("\n")
        } else {
            // Folded: lines joined by a space, an empty line becomes the line feed (`folded-blank`).
            let mut out = String::new();
            for (i, l) in lines.iter().enumerate() {
                if l.is_empty() {
                    out.push('\n');
                } else {
                    if i > 0 && !lines[i - 1].is_empty() {
                        out.push(' ');
                    }
                    out.push_str(l);
                }
            }
            out
        };
        if final_lf && !ind.ends_with('-') {
            text.push('\n');
        }
        Ok(Yaml::Text(text))
    }
}

/// A key and what follows its `:` (`""` or a space and more). A plain key holds letters, digits and `_`; a quoted key
/// (CommentsAndQuotes/quoted-key, DatasetDocument `"Expected Amount"`, `"Item No."`) letters, digits, spaces and, in
/// double quotes, `.`.
fn key_and_rest(text: &str) -> Result<(String, &str), Signal> {
    let (key, after) = match text.chars().next() {
        Some(q @ ('"' | '\'')) => {
            let Some((inner, after)) = text[1..].split_once(q) else {
                return unsupported("ReadFromYaml of an unclosed quoted key (not measured)");
            };
            let ok = |c: char| c.is_ascii_alphanumeric() || c == ' ' || (q == '"' && c == '.');
            if inner.is_empty() || !inner.chars().all(ok) || keyword(inner) {
                return unsupported("ReadFromYaml of a quoted key with these characters (not measured)");
            }
            (inner, after)
        }
        _ => {
            let end = text
                .find(|c: char| !(utf8::letter(c) || c == '_'))
                .unwrap_or(text.len());
            let key = &text[..end];
            // Every plain key of a cell starts with a letter and is no word YAML reads as null or a Boolean.
            if !key.starts_with(|c: char| c.is_ascii_alphabetic()) || keyword(key) {
                return unsupported("ReadFromYaml of a plain key of this form (not measured)");
            }
            (key, &text[end..])
        }
    };
    match after.strip_prefix(':') {
        Some(rest) if !key.is_empty() && (rest.is_empty() || rest.starts_with(' ')) => Ok((key.to_string(), rest)),
        _ => unsupported("ReadFromYaml of a line that is not `key: value` or `key:` (not measured)"),
    }
}

/// A plain value with an optional tail comment ` # ` and words (CommentsAndQuotes/comment `1 # tail`, DatasetDocument).
fn plain_value(v: &str, col: usize) -> Result<Yaml, Signal> {
    let (value, comment) = match v.split_once(" #") {
        Some((value, c)) => (value, Some(c)),
        None => (v, None),
    };
    let comment_ok = comment.is_none_or(|c| {
        c.strip_prefix(' ')
            .is_some_and(|t| t.chars().all(|c| utf8::letter(c) || c == ' '))
    });
    if !comment_ok {
        return unsupported("ReadFromYaml of a tail comment with these characters (not measured)");
    }
    // `Null` and `null` are null as a top value only (ScalarForms/words `d: Null`); below the top the pinned reader refuses
    // them and no cell measures them.
    if col == 0 && matches!(value, "Null" | "null") {
        return Ok(Yaml::Null);
    }
    scalar(value, false)
}

/// A list of words, as every cell of a list at a key's own indent holds (BlankLineTopList/toplist, keylist0): an item
/// mapping or a flow item refuses.
fn words_only(list: Yaml) -> Result<Yaml, Signal> {
    match &list {
        Yaml::Seq(items) if items.iter().all(|i| matches!(i, Yaml::Text(_))) => Ok(list),
        _ => unsupported("ReadFromYaml of a list at its key's indent of other items than words (not measured)"),
    }
}

/// A plain scalar (ScalarForms, FlowInBlockList, DatasetDocument): `Null` and `null` are null; `yes` and the numbers
/// `1`, `-3`, `0010`, `1.5`, `1e3` are text; else letters, digits, single inner spaces, `-`, `.`, `/` and, outside a
/// flow form, `:` not before a space, starting with a letter. Other words YAML may read otherwise refuse.
fn scalar(v: &str, in_flow: bool) -> Result<Yaml, Signal> {
    if v.starts_with(['"', '\'']) {
        return quoted(v);
    }
    if v == "yes" {
        return Ok(Yaml::Text(v.to_string()));
    }
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let number = digits(v.strip_prefix('-').unwrap_or(v))
        || v.split_once('.').is_some_and(|(a, b)| digits(a) && digits(b))
        || v.split_once('e').is_some_and(|(a, b)| digits(a) && digits(b));
    // PR 1515 measured its characters in plain values of letters and spaces only (`utf8::letter`), never next to `-`,
    // `.`, `/` or `:`.
    let chars_ok = v
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '.' | '/' | ':'))
        || v.chars().all(|c| utf8::letter(c) || c == ' ');
    let word = v.starts_with(|c: char| c.is_ascii_alphabetic())
        && chars_ok
        && !(in_flow && v.contains(':'))
        && !v.contains(": ")
        && !v.contains("  ")
        && !v.ends_with([' ', ':'])
        && !keyword(v)
        && v != "NULL";
    if number || word {
        return Ok(Yaml::Text(v.to_string()));
    }
    unsupported("ReadFromYaml of a plain value of this form (not measured)")
}

/// A whole quoted value: `"..."` of letters, digits, spaces and `$-<>+,.`, with the escapes `\"` and `\n`
/// (CommentsAndQuotes/quoted-escapes, ScalarForms/special, FlowInListItemMapping/flow-quoted); `'...'` of letters,
/// digits, spaces and `''` (quoted-escapes `'it''s'`, DatasetDocument `'2000'`). Nothing may follow the closing quote.
fn quoted(v: &str) -> Result<Yaml, Signal> {
    let refuse = || unsupported("ReadFromYaml of a quoted value of this form (not measured)");
    let mut chars = v.chars();
    let q = chars.next();
    let mut out = String::new();
    let mut closed = false;
    while let Some(c) = chars.next() {
        // PR 1515 measured non-ASCII letters in double quotes of letters and spaces only, never beside `$-<>+,.`.
        if q == Some('"') && !c.is_ascii() && v.contains(['$', '-', '<', '>', '+', ',', '.']) {
            return unsupported(
                "ReadFromYaml of a non-ASCII character next to punctuation in a double-quoted value (not measured)",
            );
        }
        match (q, c) {
            (Some('"'), '\\') => match chars.next() {
                Some('"') => out.push('"'),
                Some('n') => out.push('\n'),
                _ => return refuse(),
            },
            (Some('\''), '\'') if chars.clone().next() == Some('\'') => {
                chars.next();
                out.push('\'');
            }
            (Some(q), c) if c == q => {
                closed = true;
                break;
            }
            (Some('"'), c) if utf8::letter(c) || " $-<>+,.".contains(c) => out.push(c),
            (Some('\''), c) if utf8::letter(c) || c == ' ' => out.push(c),
            _ => return refuse(),
        }
    }
    // A quoted word YAML reads as null or a Boolean is refused by the pinned reader and measured by no cell.
    if !closed || out.is_empty() || chars.next().is_some() || keyword(&out) {
        return refuse();
    }
    Ok(Yaml::Text(out))
}

/// A one-line flow form: `[]`, `[a, b]` of words and quoted texts split by `, ` (FlowInBlockList, FlowInListItemMapping),
/// `{x: 1, y: two words}` of plain keys and words (FlowInBlockList/flow-values, DatasetDocument `options`).
fn flow(v: &str) -> Result<Yaml, Signal> {
    if v == "[]" {
        return Ok(Yaml::Seq(Vec::new()));
    }
    if let Some(inner) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        let items = split_flow(inner)?
            .into_iter()
            .map(|e| scalar(e, true))
            .collect::<Result<Vec<_>, _>>()?;
        if items.iter().all(|i| matches!(i, Yaml::Text(_))) {
            return Ok(Yaml::Seq(items));
        }
    } else if let Some(inner) = v.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        let mut members: Vec<(String, Yaml)> = Vec::new();
        for entry in split_flow(inner)? {
            let (key, rest) = key_and_rest(entry)?;
            let value = rest.strip_prefix(' ').map(|v| scalar(v, true)).transpose()?;
            match value {
                Some(text @ Yaml::Text(_))
                    if !entry.starts_with(['"', '\'']) && !rest[1..].starts_with(['"', '\'']) =>
                {
                    if members.iter().any(|(k, _)| *k == key) {
                        return unsupported("ReadFromYaml of a duplicate key (not measured)");
                    }
                    members.push((key, text));
                }
                _ => return unsupported("ReadFromYaml of a flow mapping entry of this form (not measured)"),
            }
        }
        return Ok(Yaml::Map(members));
    }
    unsupported("ReadFromYaml of a flow form of this shape (not measured)")
}

/// The entries of a flow form split by `, ` outside quotes; an empty entry, a nested flow form or text around an entry
/// refuses.
fn split_flow(inner: &str) -> Result<Vec<&str>, Signal> {
    let mut entries = Vec::new();
    let mut start = 0;
    let mut quote: Option<char> = None;
    let bytes = inner.as_bytes();
    for (i, c) in inner.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(q), c) if c == q => quote = None,
            (None, '[' | ']' | '{' | '}' | '#') => {
                return unsupported("ReadFromYaml of a nested flow form or a comment in one (not measured)");
            }
            (None, ',') => {
                if bytes.get(i + 1) != Some(&b' ') {
                    return unsupported("ReadFromYaml of flow entries split other than by `, ` (not measured)");
                }
                entries.push(&inner[start..i]);
                start = i + 2;
            }
            _ => {}
        }
    }
    entries.push(&inner[start..]);
    if quote.is_some()
        || entries
            .iter()
            .any(|e| e.is_empty() || e.starts_with(' ') || e.ends_with(' '))
    {
        return unsupported("ReadFromYaml of a flow form of this shape (not measured)");
    }
    Ok(entries)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod utf8_tests;

#[cfg(test)]
mod punctuation_tests;

#[cfg(test)]
mod matrix_tests;
