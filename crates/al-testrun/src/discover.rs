//! Finding test codeunits and their `[Test]` procedures in AL source text.
//!
//! A deliberately shallow text scan: it reads no syntax tree, so it is cheap enough to run over a
//! whole bucket before anything is compiled (to choose what to compile for a test filter).
//!
//! Finds `Subtype = Test` codeunits and their `[Test]` procedure names in AL text, without a syntax
//! tree.

/// `(codeunit id, [Test] procedure names)` for every `Subtype = Test` codeunit in `src`
/// (comments are ignored).
pub fn test_codeunits(src: &str) -> Vec<(u32, Vec<String>)> {
    scan(src).into_iter().map(|c| (c.0, c.1)).collect()
}

/// The ids of the `Subtype = Test` codeunits in `src` that set `RequiredTestIsolation = Disabled`. Microsoft's recipe
/// runs them only in a second pass of its UnitTest lane (BCApps `build/scripts/RunTestsInBcContainer.ps1`).
pub fn isolation_disabled_codeunits(src: &str) -> Vec<u32> {
    scan(src)
        .into_iter()
        .filter(|c| c.3.as_deref().is_some_and(|v| v.eq_ignore_ascii_case("disabled")))
        .map(|c| c.0)
        .collect()
}

/// `(codeunit id, TestType)` for every `Subtype = Test` codeunit in `src`. The type is the value of the codeunit's
/// `TestType = ...;` property as written (`UnitTest`, `IntegrationTest`, `Uncategorized`, `AITest`), or `None` when the
/// codeunit has no such property. Microsoft's recipe picks the company of a run from it (BCApps
/// `build/scripts/AppObjectValidation.psm1`, regex `TestType\s+=\s+(\w+);`).
pub fn test_codeunit_types(src: &str) -> Vec<(u32, Option<String>)> {
    scan(src).into_iter().map(|c| (c.0, c.2)).collect()
}

/// `(codeunit id, codeunit name, [Test] procedure names)` for every `Subtype = Test` codeunit in `src`: the tests a run
/// stopped at its memory limit before its compile ended reports as not run (ADR-066 amendment 10, decision 6).
pub fn test_codeunits_named(src: &str) -> Vec<(u32, String, Vec<String>)> {
    scan(src).into_iter().map(|c| (c.0, c.4, c.1)).collect()
}

/// One test codeunit of [`scan`]: id, `[Test]` procedures, `TestType`, `RequiredTestIsolation` (as written), name.
type Scanned = (u32, Vec<String>, Option<String>, Option<String>, String);

/// Blanks `//` line comments, `/* */` block comments, directive lines and the text of string
/// literals, so a `//` inside quotes is not mistaken for a comment and words in a string are not code.
/// Every byte keeps its offset (a blanked byte becomes a space, a newline stays): the scan reads a quoted
/// procedure name from the original text at the same offset.
fn strip_comments(src: &str) -> String {
    let directives = directive_starts(src);
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
            // A directive (`#region`, `#endregion`, `#pragma`, `#if`) starts where the lexer reads a `Directive` token
            // and runs to the end of its line. Its text is not code: `#region (former codeunit 50149)` is no object header
            // (al-runner issue 8350).
            b'#' if directives.binary_search(&i).is_ok() => {
                let start = i;
                while i < b.len() && b[i] != b'\n' && b[i] != b'\r' {
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
                while i < b.len() && !matches!(b[i], b'\'' | b'"' | b'/' | b'#') {
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

/// The byte offsets where the `al_syntax` lexer starts a `Directive` token, in order: the lexer owns which white space
/// and comments may come before a directive on its line.
fn directive_starts(src: &str) -> Vec<usize> {
    let mut at = 0;
    let mut out = Vec::new();
    for t in al_syntax::lex(src) {
        if t.kind == al_syntax::SyntaxKind::Directive {
            out.push(at);
        }
        at += t.len as usize;
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

/// Whether bracket word `w` is `name` (lower case) once its spaces are dropped: `[ Test ]` is `[test]`.
fn attribute_is(w: &str, name: &str) -> bool {
    w.chars()
        .filter(|c| !matches!(c, ' ' | '\t'))
        .map(|c| c.to_ascii_lowercase())
        .eq(name.chars())
}

/// Returns `(codeunit id, [Test] procedure names)` for every test codeunit in `src`.
fn scan(original: &str) -> Vec<Scanned> {
    let stripped = strip_comments(original);
    let src = stripped.as_str();
    debug_assert_eq!(src.len(), original.len());
    // The words of the code with their byte offsets (the same in `original`).
    let mut spans: Vec<(usize, &str)> = Vec::new();
    let mut start = None;
    for (i, c) in src.char_indices() {
        // A word that starts with `[` runs on over spaces up to its `]`, so `[ Test ]` is one word: the scan must find
        // every test the full run executes (review of al-runner PR 8237, probe 8237b).
        let in_brackets = start.is_some_and(|s| src.as_bytes()[s] == b'[') && matches!(c, ' ' | '\t');
        let word = c.is_alphanumeric() || c == '_' || c == '[' || c == ']' || in_brackets;
        match (word, start) {
            (true, None) => start = Some(i),
            (false, Some(s)) => {
                spans.push((s, &src[s..i]));
                start = None;
            }
            _ => {}
        }
        // A `]` ends its word, so `[Test][HandlerFunctions(..)]` is `[Test]` and then the next attribute (issue 8207).
        if c == ']'
            && let Some(s) = start.take()
        {
            spans.push((s, &src[s..=i]));
        }
    }
    if let Some(s) = start {
        spans.push((s, &src[s..]));
    }
    let words: Vec<&str> = spans.iter().map(|w| w.1).collect();
    // Each codeunit as scanned, and whether it says `Subtype = Test`.
    let mut out: Vec<(Scanned, bool)> = Vec::new();
    let mut i = 0;
    while i < words.len() {
        let w = words[i];
        if w.eq_ignore_ascii_case("codeunit") {
            if let Some(id) = words.get(i + 1).and_then(|n| n.parse::<u32>().ok()) {
                // Only an object header is followed by an id; `Record Codeunit` etc. is not.
                let (at, kw) = spans[i + 1];
                let name = name_at(original, at + kw.len()).unwrap_or_default().to_string();
                out.push(((id, Vec::new(), None, None, name), false));
                i += 2;
                continue;
            }
        }
        if let Some((cur, is_test)) = out.last_mut() {
            if w.eq_ignore_ascii_case("subtype") {
                if words.get(i + 1).is_some_and(|v| v.eq_ignore_ascii_case("test")) {
                    *is_test = true;
                }
            } else if w.eq_ignore_ascii_case("testtype") {
                if let Some(v) = words.get(i + 1) {
                    cur.2.get_or_insert_with(|| v.to_string());
                }
            } else if w.eq_ignore_ascii_case("requiredtestisolation") {
                if let Some(v) = words.get(i + 1) {
                    cur.3.get_or_insert_with(|| v.to_string());
                }
            } else if attribute_is(w, "[test]") || attribute_is(w, "[testdatasource") {
                // A data-driven procedure carries `[TestDataSource(...)]` and no `[Test]` (al-runner issue 7180; corpus
                // al-language-testlibraries/test-framework/data-driven-tests-and-handlers/BiTestHandlerDefault.al:12, :257-291).
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
    out.into_iter().filter(|c| c.1).map(|c| c.0).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_name_of_a_test_codeunit_is_read_quoted_or_plain() {
        let src = "codeunit 50100 \"My Tests\"\n{\n Subtype = Test;\n [Test]\n procedure A()\n begin end;\n}\ncodeunit 50101 Plain\n{\n Subtype = Test;\n}\n";
        assert_eq!(
            test_codeunits_named(src),
            vec![
                (50100, "My Tests".to_string(), vec!["A".to_string()]),
                (50101, "Plain".to_string(), vec![])
            ]
        );
    }

    // al-runner issue 8207: `[Test]` followed by another attribute on its line still marks the procedure a test.
    #[test]
    fn a_test_attribute_followed_by_another_attribute_on_its_line_is_found() {
        let src = "codeunit 50100 \"Scan Gap Tests\"\n{\n    Subtype = Test;\n    [Test]\n    procedure Alpha()\n    begin end;\n    [Test][HandlerFunctions('MsgHandler')]\n    procedure Beta()\n    begin end;\n    [Test] procedure Gamma()\n    begin end;\n    [MessageHandler]\n    procedure MsgHandler(Msg: Text[1024])\n    begin end;\n}\n";
        let names = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(test_codeunits(src), vec![(50100, names(&["Alpha", "Beta", "Gamma"]))]);
    }

    // Review of PR 8237, probe 8237b: spaces inside the brackets still mark a test; the full run executes these tests.
    #[test]
    fn a_test_attribute_with_spaces_inside_its_brackets_is_found() {
        let src = "codeunit 50150 \"Spaces\"\n{\n    Subtype = Test;\n    [Test]\n    procedure Plain()\n    begin end;\n    [ Test ]\n    procedure SpacesInside()\n    begin end;\n    [Test ]\n    procedure SpaceBeforeClose()\n    begin end;\n    [ Test]\n    procedure SpaceAfterOpen()\n    begin end;\n    [\tTEST\t]\n    procedure Tabs()\n    begin end;\n    [ TestDataSource(Codeunit::\"S\", 'v')]\n    procedure Data(C: Interface ITestContext)\n    begin end;\n}\n";
        let names = [
            "Plain",
            "SpacesInside",
            "SpaceBeforeClose",
            "SpaceAfterOpen",
            "Tabs",
            "Data",
        ]
        .map(String::from)
        .to_vec();
        assert_eq!(test_codeunits(src), vec![(50150, names)]);
    }

    // Neighbors: an index or a length in brackets is not the attribute, with or without spaces.
    #[test]
    fn an_index_or_a_length_in_brackets_is_not_a_test_attribute() {
        let src = "codeunit 50151 \"Not Tests\"\n{\n    Subtype = Test;\n    procedure Helper(T: Text[30])\n    var Arr: array[2] of Integer; Test: Integer;\n    begin Arr[Test] := 1; Arr[ Test ] := 2; end;\n    procedure NotATest()\n    begin end;\n}\n";
        assert_eq!(test_codeunits(src), vec![(50151, vec![])]);
    }

    // The shapes the "]" rule of issue 8207 added: another attribute first, no space before `procedure`, three on a line.
    #[test]
    fn a_test_attribute_among_other_attributes_on_its_line_is_found() {
        let src = "codeunit 50152 \"Shapes\"\n{\n    Subtype = Test;\n    [HandlerFunctions('H')][Test]\n    procedure HandlerFirst()\n    begin end;\n    [Test]procedure Glued()\n    begin end;\n    [Test][HandlerFunctions('H')][TransactionModel(TransactionModel::AutoRollback)]\n    procedure Three()\n    begin end;\n    [Test][HandlerFunctions('H')] procedure OneLine()\n    begin end;\n}\n";
        let names = ["HandlerFirst", "Glued", "Three", "OneLine"].map(String::from).to_vec();
        assert_eq!(test_codeunits(src), vec![(50152, names)]);
    }

    #[test]
    fn a_codeunit_with_only_data_driven_procedures_is_a_test_codeunit() {
        let src = "codeunit 52132 \"R18 DD\"\n{\n    Subtype = Test;\n\n    [TestDataSource(Codeunit::\"R18 Source\", 'values')]\n    procedure Double(Context: Interface ITestContext)\n    begin\n    end;\n}\n";
        assert_eq!(test_codeunits(src), vec![(52132, vec!["Double".to_string()])]);
    }

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
    fn the_required_test_isolation_disabled_property_is_read_per_codeunit() {
        let src = "codeunit 1 \"A\" { Subtype = Test; RequiredTestIsolation = Disabled; [Test] procedure X() begin end; }\n\
                   codeunit 2 \"B\" { Subtype = Test; RequiredTestIsolation = None; [Test] procedure Y() begin end; }\n\
                   codeunit 3 \"C\" { Subtype = Test; TestType = UnitTest; [Test] procedure Z() begin end; }";
        assert_eq!(isolation_disabled_codeunits(src), vec![1]);
    }

    /// al-runner issue 8350, the reporter's repro: `#region` text that names "codeunit <id>" is a directive, not an
    /// object header. The scan took it for one and lost the codeunit's tests from every selection.
    #[test]
    fn region_text_that_names_a_codeunit_does_not_start_a_codeunit() {
        let src = "codeunit 50100 \"Region Plain Test\"\n{\n    Subtype = Test;\n    #region First block\n    [Test]\n    procedure PlainRegionRuns()\n    begin\n    end;\n    #endregion\n}\n\ncodeunit 50101 \"Region Names Missing Test\"\n{\n    Subtype = Test;\n    #region First block (former codeunit 50149)\n    [Test]\n    procedure RegionNamingAMissingCodeunitRuns()\n    begin\n    end;\n    #endregion\n}\n\ncodeunit 50102 \"Region Names Own Id Test\"\n{\n    Subtype = Test;\n    #region First block (former codeunit 50102)\n    [Test]\n    procedure RegionNamingItsOwnIdRuns()\n    begin\n    end;\n    #endregion\n}\n";
        let one = |id, name: &str| (id, vec![name.to_string()]);
        assert_eq!(
            test_codeunits(src),
            vec![
                one(50100, "PlainRegionRuns"),
                one(50101, "RegionNamingAMissingCodeunitRuns"),
                one(50102, "RegionNamingItsOwnIdRuns")
            ]
        );
    }

    /// Neighbors of issue 8350: a header under `#if` is still found (the scan does not preprocess; both branches count, as
    /// before), and a `#` that is not first on its line, or sits in a string, does not blank the line.
    #[test]
    fn lines_after_a_directive_and_a_hash_inside_code_keep_their_meaning() {
        let src = "#if BC28PLUS // codeunit 7 \"No\"\ncodeunit 1 \"A\"\n#else\ncodeunit 2 \"B\"\n#endif\n{ Subtype = Test;\n  #pragma warning disable AA0001\n  [Test] procedure X() begin Message('#region codeunit 9'); end; }\ncodeunit 3 \"C\" { Subtype = Test; [Test] procedure Y() begin end; }";
        assert_eq!(
            test_codeunits(src),
            vec![(2, vec!["X".to_string()]), (3, vec!["Y".to_string()])]
        );
    }

    /// The white space before a directive is the lexer's (`al_syntax` lexer): a non-breaking space (U+00A0) or a form
    /// feed before `#region` makes the line a directive too, so its "codeunit <id>" text starts no codeunit (review of
    /// al-runner issue 8350).
    #[test]
    fn a_directive_after_a_non_breaking_space_or_form_feed_is_no_header() {
        for indent in ["\u{a0}\u{a0} ", "\u{c}"] {
            let src = format!(
                "codeunit 50101 \"Shape Test\"\n{{\n    Subtype = Test;\n{indent}#region First block (former codeunit 50149)\n    [Test]\n    procedure ShapeRuns()\n    begin\n    end;\n    #endregion\n}}\n"
            );
            assert_eq!(
                test_codeunits(&src),
                vec![(50101, vec!["ShapeRuns".to_string()])],
                "{indent:?}"
            );
        }
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
