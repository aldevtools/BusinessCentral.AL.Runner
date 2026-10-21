//! Finding test codeunits and their `[Test]` procedures in AL source text.
//!
//! A deliberately shallow text scan: it reads no syntax tree, so it is cheap enough to run over a
//! whole bucket before anything is compiled (to choose what to compile for a test filter).

/// `(codeunit id, [Test] procedure names)` for every `Subtype = Test` codeunit in `src`
/// (comments are ignored).
pub fn test_codeunits(src: &str) -> Vec<(u32, Vec<String>)> {
    scan(src).into_iter().map(|c| (c.0, c.1)).collect()
}

/// `(codeunit id, TestType)` for every `Subtype = Test` codeunit in `src`. The type is the value of the codeunit's
/// `TestType = ...;` property as written (`UnitTest`, `IntegrationTest`, `Uncategorized`, `AITest`), or `None` when the
/// codeunit has no such property. Microsoft's recipe picks the company of a run from it (BCApps
/// `build/scripts/AppObjectValidation.psm1`, regex `TestType\s+=\s+(\w+);`).
pub fn test_codeunit_types(src: &str) -> Vec<(u32, Option<String>)> {
    scan(src).into_iter().map(|c| (c.0, c.2)).collect()
}

/// Blanks `//` line comments and `/* */` block comments and the text of string
/// literals, so a `//` inside quotes is not mistaken for a comment and words in a string are not code.
/// Every byte keeps its offset (a blanked byte becomes a space, a newline stays): the scan reads a quoted
/// procedure name from the original text at the same offset.
fn strip_comments(src: &str) -> String {
    let b = src.as_bytes();
    let mut out = String::with_capacity(src.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            q @ (b'\'' | b'"') => {
                let start = i;
                i += 1;
                while i < b.len() && b[i] != q && b[i] != b'\n' {
                    i += 1;
                }
                i = (i + 1).min(b.len());
                // The text of a string is blanked (same length): words inside it (`codeunit 449`) are not code.
                blank(&mut out, &src[start..i], true);
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                let start = i;
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
                blank(&mut out, &src[start..i], false);
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                let start = i;
                i += 2;
                while i < b.len() && !(b[i] == b'*' && b.get(i + 1) == Some(&b'/')) {
                    i += 1;
                }
                i = (i + 2).min(b.len());
                blank(&mut out, &src[start..i], false);
            }
            _ => {
                let start = i;
                while i < b.len() && b[i] != b'\'' && b[i] != b'"' && b[i] != b'/' {
                    i += 1;
                }
                if i == start {
                    i += 1;
                }
                out.push_str(&src[start..i]);
            }
        }
    }
    out
}

/// One space per byte of `text`, except newlines and (`keep_quotes`) the quote characters.
fn blank(out: &mut String, text: &str, keep_quotes: bool) {
    out.extend(text.bytes().map(|c| match c {
        b'\n' => '\n',
        b'\'' | b'"' if keep_quotes => c as char,
        _ => ' ',
    }));
}

/// The procedure name that starts at `at` in `src`: a quoted identifier as written between its quotes
/// (`"Space Step"`), else the plain identifier.
fn name_at(src: &str, at: usize) -> Option<&str> {
    let rest = src.get(at..)?.trim_start();
    if let Some(q) = rest.strip_prefix('"') {
        return q
            .find(['"', '\n'])
            .filter(|&e| q.as_bytes()[e] == b'"')
            .map(|e| &q[..e]);
    }
    let end = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    (end > 0).then(|| &rest[..end])
}

/// Returns `(codeunit id, [Test] procedure names)` for every test codeunit in `src`.
fn scan(original: &str) -> Vec<(u32, Vec<String>, Option<String>)> {
    let stripped = strip_comments(original);
    let src = stripped.as_str();
    debug_assert_eq!(src.len(), original.len());
    // The words of the code with their byte offsets (the same in `original`).
    let mut spans: Vec<(usize, &str)> = Vec::new();
    let mut start = None;
    for (i, c) in src.char_indices() {
        let word = c.is_alphanumeric() || c == '_' || c == '[' || c == ']';
        match (word, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                spans.push((s, &src[s..i]));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(s) = start {
        spans.push((s, &src[s..]));
    }
    let words: Vec<&str> = spans.iter().map(|w| w.1).collect();
    let mut out: Vec<(u32, Vec<String>, bool, Option<String>)> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let w = words[i];
        if w.eq_ignore_ascii_case("codeunit") {
            if let Some(id) = words.get(i + 1).and_then(|n| n.parse::<u32>().ok()) {
                // Only an object header is followed by an id; `Record Codeunit` etc. is not.
                out.push((id, Vec::new(), false, None));
                i += 2;
                continue;
            }
        }
        if let Some(cur) = out.last_mut() {
            if w.eq_ignore_ascii_case("subtype") {
                if words.get(i + 1).is_some_and(|v| v.eq_ignore_ascii_case("test")) {
                    cur.2 = true;
                }
            } else if w.eq_ignore_ascii_case("testtype") {
                if let Some(v) = words.get(i + 1) {
                    cur.3.get_or_insert_with(|| v.to_string());
                }
            } else if w.eq_ignore_ascii_case("[test]") {
                if let Some(pos) = words[i + 1..].iter().position(|w| w.eq_ignore_ascii_case("procedure")) {
                    // The name is read from the original text: a quoted name is blanked in `src` (issue 5572).
                    let (at, kw) = spans[i + 1 + pos];
                    if let Some(name) = name_at(original, at + kw.len()) {
                        cur.1.push(name.to_string());
                    }
                }
            }
        }
        i += 1;
    }
    out.into_iter().filter(|c| c.2).map(|c| (c.0, c.1, c.3)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_quoted_name_that_starts_with_a_number_does_not_start_a_codeunit() {
        // Tests-Misc "6.0SP1 - VAT 1 to 1" (132517): `Codeunit::"6.0SP1 - VAT 1 to 1"` reads as the words `Codeunit 6`; 38 tests were lost.
        let src = "codeunit 132517 \"6.0SP1 - VAT 1 to 1\"\n{\n Subtype = Test;\n [Test]\n procedure A()\n begin X(Codeunit::\"6.0SP1 - VAT 1 to 1\"); end;\n [Test]\n procedure B()\n begin end;\n}\n";
        assert_eq!(
            test_codeunits(src),
            vec![(132517, vec!["A".to_string(), "B".to_string()])]
        );
    }

    #[test]
    fn a_string_that_says_codeunit_and_a_number_does_not_start_a_codeunit() {
        // Tests-Misc "Test Job Queue SNAP" (139020): a Label text names "COD449" and "codeunit 449"; the scan took that for a
        // second codeunit header and lost every [Test] after it, so `--parity` dropped 68 tests that Microsoft does not disable.
        let src = "codeunit 139020 \"Snap\"\n{\n Subtype = Test;\n var T: Label 'run codeunit 449 now';\n [Test]\n procedure A()\n begin end;\n [Test]\n procedure B()\n begin end;\n}\n";
        assert_eq!(
            test_codeunits(src),
            vec![(139020, vec!["A".to_string(), "B".to_string()])]
        );
    }

    #[test]
    fn the_test_type_property_is_read_per_codeunit() {
        let src = "codeunit 1 \"A\" { Subtype = Test; TestType = UnitTest; [Test] procedure X() begin end; }\n\
                   codeunit 2 \"B\" { Subtype = Test; [Test] procedure Y() begin end; }\n\
                   codeunit 3 \"C\" { Subtype = Test; TestType = IntegrationTest; RequiredTestIsolation = Disabled; [Test] procedure Z() begin end; }\n\
                   codeunit 4 \"Lib\" { TestType = UnitTest; }";
        assert_eq!(
            test_codeunit_types(src),
            vec![
                (1, Some("UnitTest".to_string())),
                (2, None),
                (3, Some("IntegrationTest".to_string()))
            ]
        );
    }

    #[test]
    fn two_codeunits_and_comments_are_still_found() {
        let src = "codeunit 1 \"A\" { Subtype = Test; [Test] procedure X() begin end; }\n// codeunit 9 \"no\"\ncodeunit 2 \"B\" { Subtype = Test; [Test] procedure Y() begin end; }";
        assert_eq!(
            test_codeunits(src),
            vec![(1, vec!["X".to_string()]), (2, vec!["Y".to_string()])]
        );
    }

    /// Issue 5572: a test whose name is a quoted identifier is found by its AL name (spaces, dots, a leading digit,
    /// a keyword, letters outside ASCII), also after a comment with letters outside ASCII.
    #[test]
    fn a_quoted_test_name_is_found_as_written() {
        let src = "codeunit 50650 \"Probe Tests\"\n{\n    Subtype = Test;\n    // Prüfung: ä\n    /* ö */\n    [Test] procedure \"Space Step\"() begin end;\n    [Test]\n    procedure \"Dot.Name\"()\n    begin\n    end;\n    [Test] procedure \"1Leading\"() begin end;\n    [Test] procedure \"Default\"() begin end;\n    [Test] procedure \"Blåbær Step\"() begin end;\n    [Test] procedure Passes() begin Message('procedure \"No\"'); end;\n}\n";
        assert_eq!(
            test_codeunits(src),
            vec![(
                50650,
                ["Space Step", "Dot.Name", "1Leading", "Default", "Blåbær Step", "Passes"]
                    .map(String::from)
                    .to_vec()
            )]
        );
    }
}
