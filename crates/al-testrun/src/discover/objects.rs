//! Every AL object a source text declares, with its procedures and triggers and their spans: what a corpus citation
//! names (`al-test-cli` `citation.rs`, ADR-065 amendment 2). The same shallow text scan as the test codeunit scan of
//! `discover.rs` (comments and string texts blanked, no syntax tree), plus brace depth: an object header is a kind word
//! at depth 0. Entry points: `objects`, `string_literals`.

use super::{attribute_is, name_at, strip_comments, words};
use std::ops::Range;

/// The object kinds whose header is `<kind> <id> <name>`.
const WITH_ID: &[&str] = &[
    "codeunit",
    "table",
    "tableextension",
    "page",
    "pageextension",
    "report",
    "reportextension",
    "query",
    "xmlport",
    "enum",
    "enumextension",
    "permissionset",
    "permissionsetextension",
];
/// The object kinds whose header is `<kind> <name>`.
const WITHOUT_ID: &[&str] = &["interface", "controladdin", "profile", "entitlement"];

/// Whether `kind` (lower case) is an object kind [`objects`] reads.
pub fn is_object_kind(kind: &str) -> bool {
    WITH_ID.contains(&kind) || WITHOUT_ID.contains(&kind)
}

/// One object of [`objects`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectDecl {
    /// The kind in lower case (`codeunit`, `table`, ...).
    pub kind: String,
    /// `None` for the kinds without an id (`interface`, ...).
    pub id: Option<u32>,
    /// The name as written, without its quotes.
    pub name: String,
    /// A codeunit that says `Subtype = Test`.
    pub test_codeunit: bool,
    /// Its `procedure` and `trigger` declarations, in order.
    pub members: Vec<MemberDecl>,
}

/// One `procedure` or `trigger` of an object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberDecl {
    /// The name as written, without its quotes.
    pub name: String,
    /// A procedure after `[Test]` or `[TestDataSource(..)]`, as `discover.rs` reads a test.
    pub test: bool,
    /// From its keyword to the next member's keyword or the end of the object, in bytes of the source text.
    pub span: Range<usize>,
}

/// Every object `src` declares, in order (comments and directives are no code).
pub fn objects(src: &str) -> Vec<ObjectDecl> {
    let stripped = strip_comments(src);
    let code = stripped.as_str();
    let spans = words(code);
    let mut out: Vec<ObjectDecl> = Vec::new();
    // The byte where the open object ends, set when its closing brace is passed.
    let mut ends: Vec<usize> = Vec::new();
    let (mut depth, mut pos, mut test_attr) = (0i32, 0usize, false);
    let mut i = 0;
    while i < spans.len() {
        let (at, w) = spans[i];
        braces(code, pos..at, &mut depth, &mut ends, out.len());
        pos = at;
        let lower = w.to_ascii_lowercase();
        if depth == 0 && is_object_kind(&lower) {
            let id = WITH_ID
                .contains(&lower.as_str())
                .then(|| spans.get(i + 1).and_then(|n| n.1.parse::<u32>().ok()))
                .flatten();
            let name_from = match id {
                Some(_) => spans[i + 1].0 + spans[i + 1].1.len(),
                None if WITHOUT_ID.contains(&lower.as_str()) => at + w.len(),
                None => {
                    i += 1;
                    continue;
                }
            };
            if let Some(name) = name_at(src, name_from) {
                if ends.len() < out.len() {
                    ends.push(at);
                }
                out.push(ObjectDecl {
                    kind: lower,
                    id,
                    name: name.to_string(),
                    test_codeunit: false,
                    members: Vec::new(),
                });
                test_attr = false;
            }
            i += 1;
            continue;
        }
        let open = ends.len() < out.len();
        if let Some(cur) = out.last_mut().filter(|_| open) {
            if lower == "subtype" && spans.get(i + 1).is_some_and(|v| v.1.eq_ignore_ascii_case("test")) {
                cur.test_codeunit = cur.kind == "codeunit";
            } else if attribute_is(w, "[test]") || attribute_is(w, "[testdatasource") {
                test_attr = true;
            } else if (lower == "procedure" || lower == "trigger")
                && let Some(name) = name_at(src, at + w.len())
            {
                cur.members.push(MemberDecl {
                    name: name.to_string(),
                    test: test_attr && lower == "procedure",
                    span: at..at,
                });
                test_attr = false;
            }
        }
        i += 1;
    }
    braces(code, pos..code.len(), &mut depth, &mut ends, out.len());
    ends.resize(out.len(), src.len());
    for (o, end) in out.iter_mut().zip(ends) {
        let starts: Vec<usize> = o.members.iter().map(|m| m.span.start).skip(1).chain([end]).collect();
        for (m, next) in o.members.iter_mut().zip(starts) {
            m.span.end = next;
        }
    }
    out
}

/// Counts the braces of `code` in `range` into `depth`; the brace that brings it back to 0 ends the open object (one
/// of `objects` objects has no end in `ends` yet).
fn braces(code: &str, range: Range<usize>, depth: &mut i32, ends: &mut Vec<usize>, objects: usize) {
    for (k, c) in code.as_bytes()[range.clone()].iter().enumerate() {
        match c {
            b'{' => *depth += 1,
            b'}' => {
                *depth -= 1;
                if *depth == 0 && ends.len() < objects {
                    ends.push(range.start + k + 1);
                }
            }
            _ => {}
        }
    }
}

/// The string literals of `src` inside `range`, as written with their quotes (`'it''s'`); a literal in a comment is no
/// literal.
pub fn string_literals(src: &str, range: Range<usize>) -> Vec<&str> {
    let stripped = strip_comments(src);
    let code = stripped.as_bytes();
    let mut out = Vec::new();
    let mut i = range.start;
    while i < range.end.min(code.len()) {
        if code[i] != b'\'' {
            i += 1;
            continue;
        }
        let open = i;
        // A string is blanked between its kept quotes; `''` inside it reads as two adjacent strings.
        let mut close = open + 1;
        loop {
            while close < code.len() && code[close] != b'\'' && code[close] != b'\n' {
                close += 1;
            }
            if close + 1 < code.len() && code[close] == b'\'' && code[close + 1] == b'\'' {
                close += 2;
                continue;
            }
            break;
        }
        if close >= code.len() || code[close] != b'\'' {
            break;
        }
        out.push(&src[open..=close]);
        i = close + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "// codeunit 1 \"In A Comment\"\ncodeunit 50100 \"LT Cites\"\n{\n    Subtype = Test;\n    var\n        I: Interface \"Not A Header\";\n\n    [Test]\n    procedure \"First Test\"()\n    begin\n        Observer.Expect('it''s', 'x'); // 'commented'\n    end;\n\n    local procedure Helper()\n    begin\n    end;\n}\n\ntable 50101 Plain\n{\n    fields { field(1; A; Integer) { trigger OnValidate() begin end; } }\n}\ninterface \"LT Face\"\n{\n    procedure Do();\n}\n";

    #[test]
    fn objects_are_read_at_depth_zero_with_their_members() {
        let o = objects(SRC);
        let heads: Vec<(&str, Option<u32>, &str, bool)> = o
            .iter()
            .map(|o| (o.kind.as_str(), o.id, o.name.as_str(), o.test_codeunit))
            .collect();
        assert_eq!(
            heads,
            vec![
                ("codeunit", Some(50100), "LT Cites", true),
                ("table", Some(50101), "Plain", false),
                ("interface", None, "LT Face", false),
            ]
        );
        let members: Vec<(&str, bool)> = o[0].members.iter().map(|m| (m.name.as_str(), m.test)).collect();
        assert_eq!(members, vec![("First Test", true), ("Helper", false)]);
        assert_eq!(o[1].members[0].name, "OnValidate");
        // The test's span ends where the helper starts; the helper's at the codeunit's closing brace.
        let test = &o[0].members[0];
        assert!(SRC[test.span.clone()].starts_with("procedure \"First Test\""));
        assert!(SRC[test.span.clone()].ends_with("    local "));
        assert!(SRC[o[0].members[1].span.clone()].ends_with("end;\n}"));
    }

    #[test]
    fn string_literals_keep_their_doubled_quotes_and_skip_comments() {
        let o = objects(SRC);
        assert_eq!(
            string_literals(SRC, o[0].members[0].span.clone()),
            vec!["'it''s'", "'x'"]
        );
    }
}
