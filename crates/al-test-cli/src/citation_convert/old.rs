//! The old corpus citation form that `--convert-citations` replaces: `[path/]File.al`, optionally with lines
//! (`:12`, `:12-14`, `:273,312`, `:29, :59`, `:12 and :20`), and the bare `:line` continuations that cite more lines of
//! a nearby `File.al` in the same text (`... (Foo.al:12) ... it is not 1 (:168)`). In a Rust file only comments are
//! read: a path in a string literal is code (`include_str!`, gate evidence) or a registry text, and stays.
//! Entry point: `find`.

use std::ops::Range;

/// How far a bare `:line` may be from the `File.al` it continues: up to this many lines below it, or on the same line
/// or up to [`BARE_BEFORE`] lines above it when no file citation comes before.
const BARE_AFTER: usize = 6;
const BARE_BEFORE: usize = 2;

/// One old citation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OldCite {
    /// The bytes it takes in the text.
    pub at: Range<usize>,
    /// The path as written (`record/X/TFoo.al`); empty for a bare continuation.
    pub path: String,
    /// The cited line ranges, inclusive; empty when only the file is named.
    pub lines: Vec<(u32, u32)>,
    /// A bare continuation: the index (in `find`'s result) of the file citation it continues, `None` when no file
    /// citation is near enough.
    pub parent: Option<usize>,
    /// A bare continuation is one; a file citation is not.
    pub bare: bool,
    /// In a comment of a Rust file, or anywhere in another file.
    pub in_comment: bool,
}

/// Every old citation of `text` in order; `rust` reads only its comments as prose.
pub fn find(text: &str, rust: bool) -> Vec<OldCite> {
    let comments = if rust { rust_comments(text) } else { vec![0..text.len()] };
    let in_comment = |at: usize| comments.iter().any(|r| r.contains(&at));
    let b = text.as_bytes();
    let mut out: Vec<OldCite> = Vec::new();
    let mut from = 0;
    while let Some(off) = text[from..].find(".al") {
        let dot = from + off;
        from = dot + 3;
        if b.get(dot + 3).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_') {
            continue;
        }
        let mut start = dot;
        while start > 0 && path_byte(b[start - 1]) {
            start -= 1;
        }
        if start == dot || !matches!(b[dot - 1], b'0'..=b'9' | b'a'..=b'z' | b'A'..=b'Z' | b'_' | b'-') {
            continue;
        }
        let (lines, end) = line_list(text, dot + 3);
        out.push(OldCite {
            at: start..end,
            path: text[start..dot + 3].to_string(),
            lines,
            parent: None,
            bare: false,
            in_comment: in_comment(start),
        });
        from = end;
    }
    let files = out.len();
    for at in bare_starts(text, &out) {
        if !in_comment(at) {
            continue;
        }
        let (lines, end) = line_list(text, at);
        if !lines.is_empty() {
            out.push(OldCite {
                at: at..end,
                path: String::new(),
                lines,
                parent: None,
                bare: true,
                in_comment: true,
            });
        }
    }
    attach(text, &mut out, files);
    // Sort by place; a parent index moves with its file citation.
    let mut order: Vec<usize> = (0..out.len()).collect();
    order.sort_by_key(|&i| out[i].at.start);
    let mut new_at = vec![0; out.len()];
    for (to, &from) in order.iter().enumerate() {
        new_at[from] = to;
    }
    order
        .into_iter()
        .map(|i| {
            let mut c = out[i].clone();
            c.parent = c.parent.map(|p| new_at[p]);
            c
        })
        .collect()
}

/// A byte of a path in the old form.
fn path_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'.' | b'/' | b'-')
}

/// The line list at `at` (`:12-14, :20` or `,20` after a first line): the ranges and the byte after the last one.
/// The list starts with its `:` (after a path, or a bare continuation's own).
fn line_list(text: &str, at: usize) -> (Vec<(u32, u32)>, usize) {
    let b = text.as_bytes();
    let mut lines = Vec::new();
    let mut i = at;
    if b.get(i) != Some(&b':') {
        return (lines, i);
    }
    let Some((r, n)) = range_at(text, i + 1) else {
        return (lines, i);
    };
    lines.push(r);
    i += 1 + n;
    loop {
        // `,20` (no space), `, :20`, `,:20`, ` and :20`.
        let next = if b.get(i) == Some(&b',') && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
            Some(i + 1)
        } else if text[i..].starts_with(", :") {
            Some(i + 3)
        } else if text[i..].starts_with(",:") {
            Some(i + 2)
        } else if text[i..].starts_with(" and :") {
            Some(i + 6)
        } else {
            None
        };
        let Some(j) = next else { break };
        let Some((r, n)) = range_at(text, j) else { break };
        lines.push(r);
        i = j + n;
    }
    (lines, i)
}

/// `12` or `12-14` at `at`, not followed by a letter, digit or `:`: the range and its length in bytes.
fn range_at(text: &str, at: usize) -> Option<((u32, u32), usize)> {
    let b = text.as_bytes();
    let digits = |from: usize| b[from.min(b.len())..].iter().take_while(|c| c.is_ascii_digit()).count();
    let n = digits(at);
    let first: u32 = text.get(at..at + n)?.parse().ok()?;
    let mut len = n;
    let mut last = first;
    if b.get(at + n) == Some(&b'-') {
        let m = digits(at + n + 1);
        if m > 0 {
            last = text[at + n + 1..at + n + 1 + m].parse().ok()?;
            len = n + 1 + m;
        }
    }
    if b.get(at + len).is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b':' | b'_')) || last < first {
        return None;
    }
    Some(((first, last), len))
}

/// Where a bare `:line` starts: a `:` and a digit after a space, tab, `(`, `[` or `,`, outside every file citation.
fn bare_starts(text: &str, files: &[OldCite]) -> Vec<usize> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut taken = files.iter().map(|c| c.at.clone()).peekable();
    let mut i = 1;
    while i + 1 < b.len() {
        while taken.peek().is_some_and(|r| r.end <= i) {
            taken.next();
        }
        if let Some(r) = taken.peek()
            && r.contains(&i)
        {
            i = r.end;
            continue;
        }
        if b[i] == b':' && b[i + 1].is_ascii_digit() && matches!(b[i - 1], b' ' | b'\t' | b'(' | b'[' | b',') {
            let (lines, end) = line_list(text, i);
            if !lines.is_empty() {
                out.push(i);
                i = end;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Gives each bare continuation (at `files..`) the nearest file citation: the last one up to [`BARE_AFTER`] lines
/// above it, else the first one on its line or up to [`BARE_BEFORE`] lines below it.
fn attach(text: &str, cites: &mut [OldCite], files: usize) {
    let line = |at: usize| text[..at].matches('\n').count();
    let file_lines: Vec<usize> = cites[..files].iter().map(|c| line(c.at.start)).collect();
    for k in files..cites.len() {
        let (at, l) = (cites[k].at.start, line(cites[k].at.start));
        let before = (0..files)
            .rev()
            .find(|&f| cites[f].at.start < at && file_lines[f] + BARE_AFTER >= l);
        let after = || (0..files).find(|&f| cites[f].at.start > at && file_lines[f] <= l + BARE_BEFORE);
        cites[k].parent = before.or_else(after);
    }
}

/// The byte ranges of the comments of Rust source `text` (`//`, `/* */`); strings, raw strings and char literals are
/// skipped so a `//` inside them is no comment.
fn rust_comments(text: &str) -> Vec<Range<usize>> {
    let b = text.as_bytes();
    let ident = |i: usize| i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'_');
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'/' if b.get(i + 1) == Some(&b'/') => {
                let start = i;
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                out.push(start..i);
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let (start, mut depth) = (i, 0);
                while i < b.len() {
                    if b[i] == b'/' && b.get(i + 1) == Some(&b'*') {
                        depth += 1;
                        i += 2;
                    } else if b[i] == b'*' && b.get(i + 1) == Some(&b'/') {
                        depth -= 1;
                        i += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        i += 1;
                    }
                }
                out.push(start..i);
            }
            b'r' if !ident(i) || (i > 0 && b[i - 1] == b'b' && !ident(i - 1)) => {
                let hashes = b[i + 1..].iter().take_while(|c| **c == b'#').count();
                if b.get(i + 1 + hashes) == Some(&b'"') {
                    let close = format!("\"{}", "#".repeat(hashes));
                    let body = i + 2 + hashes;
                    i = text[body..].find(&close).map_or(b.len(), |k| body + k + close.len());
                } else {
                    i += 1;
                }
            }
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
            }
            b'\'' => {
                // A char literal (`'x'`, `'\n'`, `'é'`) or a lifetime (`'a`).
                if b.get(i + 1) == Some(&b'\\') {
                    i += 2;
                    while i < b.len() && b[i] != b'\'' {
                        i += 1;
                    }
                    i += 1;
                } else if let Some(c) = text.get(i + 1..).and_then(|r| r.chars().next())
                    && b.get(i + 1 + c.len_utf8()) == Some(&b'\'')
                {
                    i += 2 + c.len_utf8();
                } else {
                    i += 1;
                }
            }
            _ => i += 1,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shapes(text: &str, rust: bool) -> Vec<(String, Vec<(u32, u32)>, Option<usize>, bool)> {
        find(text, rust)
            .into_iter()
            .map(|c| (text[c.at.clone()].to_string(), c.lines, c.parent, c.in_comment))
            .collect()
    }

    #[test]
    fn the_line_lists_of_the_old_form_parse() {
        let t = "a (record/X/TFoo.al:12-14), B.al:273,312,351; C.al:29, :59; D.al:12 and :20, E.al, F.al:7.";
        let got = shapes(t, false);
        let want = [
            ("record/X/TFoo.al:12-14", vec![(12, 14)]),
            ("B.al:273,312,351", vec![(273, 273), (312, 312), (351, 351)]),
            ("C.al:29, :59", vec![(29, 29), (59, 59)]),
            ("D.al:12 and :20", vec![(12, 12), (20, 20)]),
            ("E.al", vec![]),
            ("F.al:7", vec![(7, 7)]),
        ];
        assert_eq!(got.len(), want.len(), "{got:?}");
        for (g, (text, lines)) in got.iter().zip(want) {
            assert_eq!((g.0.as_str(), &g.1), (text, &lines));
        }
    }

    #[test]
    fn a_word_that_only_holds_al_is_no_citation() {
        assert!(find("the .al files, x.alpha, Foo.al_x, 12:30, a:5", false).is_empty());
    }

    #[test]
    fn a_bare_continuation_takes_the_nearest_file_citation() {
        let t = "// pinned (A.al:12), not 1 (:168)\n// next\n// and :20-22\n// B.al:5\n";
        let got = shapes(t, true);
        let texts: Vec<&str> = got.iter().map(|g| g.0.as_str()).collect();
        assert_eq!(texts, ["A.al:12", ":168", ":20-22", "B.al:5"]);
        // Both continuations belong to A.al (index 0).
        assert_eq!((got[1].2, got[2].2), (Some(0), Some(0)));
        // A continuation before its file citation, with none above: the file citation on the same line.
        let got = shapes("// (Run :267-296) and x/Edges.al.\n", true);
        assert_eq!(got[0].0, ":267-296");
        assert_eq!(got[0].2, Some(1));
    }

    #[test]
    fn a_rust_file_is_read_in_its_comments_only() {
        let t = "// A.al:1\nlet s = \"B.al:2 // C.al:3\";\nlet c = '\"'; /* D.al:4 */ include_str!(\"E.al\");\nfn f<'a>() {} // F.al:5\n";
        let got: Vec<(String, bool)> = shapes(t, true).into_iter().map(|g| (g.0, g.3)).collect();
        let want = [
            ("A.al:1", true),
            ("B.al:2", false),
            ("C.al:3", false),
            ("D.al:4", true),
            ("E.al", false),
            ("F.al:5", true),
        ];
        assert_eq!(got.iter().map(|(s, b)| (s.as_str(), *b)).collect::<Vec<_>>(), want);
    }
}
