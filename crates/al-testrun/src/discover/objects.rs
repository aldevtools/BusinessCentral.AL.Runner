//! Every AL object a source text declares, with its procedures and triggers and their spans: what a corpus citation
//! names (`al-test-cli` `citation.rs`, ADR-065 amendment 2). The same shallow text scan as the test codeunit scan of
//! `discover.rs` (comments and string texts blanked, no syntax tree; the same readers of `Subtype = Test` and `[Test]`),
//! plus brace depth: an object header is a kind word at depth 0, a member a `procedure` or `trigger` at depth 1 (a
//! trigger of a field, control or action is none). A member's span runs from its lead-in (the attribute and comment
//! lines right above it) to the line of its body's last `end`. Entry points: `objects`, `cells`, `code_text`.

use super::{is_test_attribute, name_at, says_test_subtype, strip_comments, words};
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
/// The observation calls whose first argument is a cell's name, as the corpus writes them (`Observer.Expect('cell', ..)`,
/// `O.Observe(..)`, `Obs.ObserveError(..)`, `Observer.Ignore(..)`).
const CELL_CALLS: &[&str] = &["Expect", "Observe", "ObserveError", "Ignore"];

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
    /// Its `procedure` and `trigger` declarations at depth 1, in order.
    pub members: Vec<MemberDecl>,
    /// From its kind word to its closing brace (included), in bytes of the source text.
    pub span: Range<usize>,
}

/// One `procedure` or `trigger` of an object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberDecl {
    /// The name as written, without its quotes.
    pub name: String,
    /// A procedure after `[Test]` or `[TestDataSource(..)]`, as `discover.rs` reads a test.
    pub test: bool,
    /// From the start of its first lead-in line (or of its keyword's line) to the end of the line of its body's last
    /// `end` (of its declaration line when it has no body), in bytes of the source text.
    pub span: Range<usize>,
}

/// `src` with comments, directives and the text of string literals blanked, every byte at its offset (a literal keeps
/// its quotes): the code of a source line.
pub fn code_text(src: &str) -> String {
    strip_comments(src)
}

/// Every object `src` declares, in order (comments and directives are no code).
pub fn objects(src: &str) -> Vec<ObjectDecl> {
    let stripped = strip_comments(src);
    let code = stripped.as_str();
    let spans = words(code);
    let mut out: Vec<ObjectDecl> = Vec::new();
    // Per object, the word index of each member's keyword.
    let mut keywords: Vec<Vec<usize>> = Vec::new();
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
                    span: at..at,
                });
                keywords.push(Vec::new());
                test_attr = false;
            }
            i += 1;
            continue;
        }
        let open = ends.len() < out.len();
        if let Some(cur) = out.last_mut().filter(|_| open) {
            if says_test_subtype(w, spans.get(i + 1).map(|n| n.1)) {
                cur.test_codeunit = cur.kind == "codeunit";
            } else if is_test_attribute(w) {
                test_attr = true;
            } else if depth == 1
                && (lower == "procedure" || lower == "trigger")
                && let Some(name) = name_at(src, at + w.len())
            {
                cur.members.push(MemberDecl {
                    name: name.to_string(),
                    test: test_attr && lower == "procedure",
                    span: at..at,
                });
                if let Some(k) = keywords.last_mut() {
                    k.push(i);
                }
                test_attr = false;
            }
        }
        i += 1;
    }
    braces(code, pos..code.len(), &mut depth, &mut ends, out.len());
    ends.resize(out.len(), src.len());
    let lines = Lines::new(src, code);
    for ((o, end), keys) in out.iter_mut().zip(ends).zip(keywords) {
        o.span.end = end;
        // A lead-in never reaches above the object's header line or into the member before.
        let mut floor = lines.line_of(o.span.start) + 1;
        for (k, &kw) in keys.iter().enumerate() {
            let limit = keys.get(k + 1).map_or(end, |&n| spans[n].0);
            let last = body_end(&spans, kw, limit);
            let first = lines.lead_in(lines.line_of(spans[kw].0), floor);
            o.members[k].span = lines.start(first)..lines.end_of_line(last);
            floor = lines.line_of(last) + 1;
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

/// The byte in the last word of the body of the member whose keyword is word `keyword`, before byte `limit`: the `end`
/// that closes its first `begin` (`begin` and `case` open, `end` closes), or the keyword itself when it has no body.
fn body_end(spans: &[(usize, &str)], keyword: usize, limit: usize) -> usize {
    let mut depth = 0usize;
    for &(at, w) in spans[keyword + 1..].iter().take_while(|(at, _)| *at < limit) {
        if w.eq_ignore_ascii_case("begin") || (depth > 0 && w.eq_ignore_ascii_case("case")) {
            depth += 1;
        } else if depth > 0 && w.eq_ignore_ascii_case("end") {
            depth -= 1;
            if depth == 0 {
                return at;
            }
        }
    }
    spans[keyword].0
}

/// The lines of a source text and its code (comments and string texts blanked).
struct Lines<'a> {
    src: &'a str,
    code: &'a str,
    /// The byte where each line starts.
    starts: Vec<usize>,
}

impl<'a> Lines<'a> {
    fn new(src: &'a str, code: &'a str) -> Lines<'a> {
        let starts = std::iter::once(0)
            .chain(src.match_indices('\n').map(|(i, _)| i + 1))
            .collect();
        Lines { src, code, starts }
    }

    /// The 0-based line of byte `at`.
    fn line_of(&self, at: usize) -> usize {
        self.starts.partition_point(|&s| s <= at) - 1
    }

    fn start(&self, line: usize) -> usize {
        self.starts[line]
    }

    /// The byte after the newline of the line of byte `at` (the end of the text on the last line).
    fn end_of_line(&self, at: usize) -> usize {
        self.starts.get(self.line_of(at) + 1).copied().unwrap_or(self.src.len())
    }

    /// The first line of the lead-in above line `keyword`: the attribute lines (`[Test]`) and comment lines right
    /// above it, not above line `floor`.
    fn lead_in(&self, keyword: usize, floor: usize) -> usize {
        let mut first = keyword;
        while first > floor {
            let (from, to) = (self.starts[first - 1], self.starts[first]);
            let code = self.code[from..to].trim();
            let text = self.src[from..to].trim();
            if code.starts_with('[') || (code.is_empty() && !text.is_empty()) {
                first -= 1;
            } else {
                break;
            }
        }
        first
    }
}

/// The cells of `src` inside `range`: the first argument of each observation call ([`CELL_CALLS`]) that is a string
/// literal, as written with its quotes (`'it''s'`). A call in a comment or a string is none.
pub fn cells(src: &str, range: Range<usize>) -> Vec<&str> {
    let stripped = strip_comments(src);
    let code = stripped.as_str();
    let end = range.end.min(code.len());
    let mut out: Vec<(usize, &str)> = Vec::new();
    for call in CELL_CALLS {
        let pattern = format!(".{call}(");
        for (k, _) in code[range.start..end].match_indices(pattern.as_str()) {
            let open = range.start + k + pattern.len();
            let arg = open + code[open..].len() - code[open..].trim_start_matches([' ', '\t']).len();
            if let Some(n) = literal_len(&src[arg..]) {
                out.push((arg, &src[arg..arg + n]));
            }
        }
    }
    out.sort();
    out.into_iter().map(|(_, l)| l).collect()
}

/// The length of the AL string literal at the start of `s` (`''` inside it is a quote), on one line.
pub fn literal_len(s: &str) -> Option<usize> {
    let b = s.as_bytes();
    (b.first() == Some(&b'\'')).then_some(())?;
    let mut i = 1;
    loop {
        match b.get(i)? {
            b'\n' => return None,
            b'\'' if b.get(i + 1) == Some(&b'\'') => i += 2,
            b'\'' => return Some(i + 1),
            _ => i += 1,
        }
    }
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
        // A member's span runs from its lead-in (attributes, `local`) to the line of its body's `end;`.
        let test = &o[0].members[0];
        assert!(SRC[test.span.clone()].starts_with("    [Test]\n    procedure \"First Test\""));
        assert!(SRC[test.span.clone()].ends_with("    end;\n"));
        assert!(SRC[o[0].members[1].span.clone()].starts_with("    local procedure Helper()"));
        assert!(SRC[o[0].members[1].span.clone()].ends_with("    end;\n"));
        // An interface procedure has no body: its declaration line.
        assert_eq!(&SRC[o[2].members[0].span.clone()], "    procedure Do();\n");
        // An object's span runs from its kind word to its closing brace.
        assert!(SRC[o[0].span.clone()].starts_with("codeunit 50100") && SRC[o[0].span.clone()].ends_with("end;\n}"));
    }

    // Review r1 of PR 9432, M2: a trigger of a field, control or action is no member of its object.
    #[test]
    fn a_nested_trigger_is_no_member() {
        let src = "table 50101 Plain\n{\n    fields\n    {\n        field(1; A; Integer) { trigger OnValidate() begin end; }\n        field(2; B; Integer) { trigger OnValidate() begin end; }\n    }\n    trigger OnInsert()\n    begin\n    end;\n}\n";
        let names: Vec<String> = objects(src)[0].members.iter().map(|m| m.name.clone()).collect();
        assert_eq!(names, ["OnInsert"]);
    }

    // Review r1 of PR 9432, M3: the comment and attributes above a test belong to that test, not to the one before.
    #[test]
    fn the_lead_in_of_a_member_belongs_to_it() {
        let src = "codeunit 50100 \"LT Lead\"\n{\n    Subtype = Test;\n\n    [Test]\n    procedure A()\n    begin\n        if true then begin\n        end;\n        case 1 of\n            1: ;\n        end;\n    end;\n\n    // B's comment.\n    [Test]\n    [HandlerFunctions('ConfirmYes')]\n    procedure B()\n    begin\n        Observer.Expect('b cell', 1);\n    end;\n}\n";
        let o = &objects(src)[0];
        let (a, b) = (&src[o.members[0].span.clone()], &src[o.members[1].span.clone()]);
        assert!(a.ends_with("        end;\n    end;\n"), "{a:?}");
        assert!(
            b.starts_with("    // B's comment.\n    [Test]\n    [HandlerFunctions"),
            "{b:?}"
        );
        // L3: a cell is the first argument of an observation; the handler name is none, of either test.
        assert_eq!(cells(src, o.members[0].span.clone()), Vec::<&str>::new());
        assert_eq!(cells(src, o.members[1].span.clone()), vec!["'b cell'"]);
    }

    #[test]
    fn cells_are_the_first_arguments_of_observations() {
        let o = objects(SRC);
        assert_eq!(cells(SRC, o[0].members[0].span.clone()), vec!["'it''s'"]);
        let src = "O.Observe( 'a', X); Obs.ObserveError('b'); Observer.Ignore('c'); Assert.ExpectedError('d'); H.ObservePair('e', 1); O.Expect(Name, 'f');";
        assert_eq!(cells(src, 0..src.len()), vec!["'a'", "'b'", "'c'"]);
    }

    #[test]
    fn code_text_blanks_comments_and_string_texts_at_their_offsets() {
        let src = "x := 'a // b'; // c\n";
        assert_eq!(code_text(src), "x := '      ';     \n");
    }
}
