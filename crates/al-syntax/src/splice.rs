//! Adding routines to a parsed file without parsing the file again.
//!
//! The compiler adds routines to a page, report or table (`al_sema::pages`, ADR-012): each piece of added text is one or
//! more whole routines that go between two members of an object body. Parsing the joined text again costs as much as the
//! first parse. [`splice_routines`] parses only the added text and puts its routines into the tree of the file instead.
//!
//! Adds the routines the compiler generates (page expressions) to a parsed file without parsing the
//! file again.

use crate::{Parse, SyntaxKind, TreeCache};
use rowan::{GreenNode, GreenNodeData, NodeOrToken};
use std::collections::BTreeSet;

/// The parse of `src` with `inserts` added, built from `parse` (the parse of `src`) and a parse of the added text alone.
///
/// Each insert is (byte offset in `src`, text), in offset order. A text is one or more routines, each after a line break
/// (`"\nprocedure P(): Boolean begin exit(true); end;"`), and its offset is a boundary between the children of an object
/// body (before the body's `}`, or after a member). Then the joined text gives the same tree: the added text starts with a
/// line break, which is a token of its own whatever comes before it, and ends with the `;` of a routine, so the tokens on
/// both sides stay as they were; the leading trivia of a routine belongs to the body, and a routine's rule reads nothing
/// after its `;`.
///
/// `None` when that cannot be shown for this input (the file or the added text has a parse error, the added text holds
/// a `#`, an offset is not such a boundary, a lone carriage return comes before it); the caller then parses the joined
/// text.
pub fn splice_routines(
    src: &str,
    parse: &Parse,
    inserts: &[(usize, &str)],
    symbols: &BTreeSet<String>,
    cache: &mut TreeCache,
) -> Option<Parse> {
    if !parse.errors.is_empty() {
        return None;
    }
    let mut green = parse.green.clone();
    // From the last insert to the first, so the offsets of the ones still to do stay those of `src`.
    for &(at, text) in inserts.iter().rev() {
        // A `#` could start a directive, which the preprocessor reads with the rest of the file.
        if !text.starts_with('\n') || text.contains('#') || src[..at].ends_with('\r') {
            return None;
        }
        let routines = routines_of(text, symbols, cache)?;
        green = insert_at(&green, 0, at, &routines)?;
    }
    Some(Parse {
        green,
        errors: Vec::new(),
    })
}

/// The children of an object body that `text` makes: the line breaks and the routines between the body's braces.
fn routines_of(
    text: &str,
    symbols: &BTreeSet<String>,
    cache: &mut TreeCache,
) -> Option<Vec<NodeOrToken<GreenNode, rowan::GreenToken>>> {
    let wrapped = format!("codeunit 0 X {{{text}}}");
    let p = crate::parser::parse_with_cache(&wrapped, symbols, cache);
    if !p.errors.is_empty() {
        return None;
    }
    let object = p.green.children().next()?.into_node()?;
    let body = object.children().last()?.into_node()?;
    if body.kind() != SyntaxKind::ObjectBody.into() {
        return None;
    }
    let mut children: Vec<_> = body.children().map(|c| c.to_owned()).collect();
    // The braces are the wrapper's.
    children.pop();
    children.remove(0);
    // Only routines and the white space between them; the last is a routine, so nothing after it can join a token.
    let ok = children.iter().all(|c| match c {
        NodeOrToken::Node(n) => n.kind() == SyntaxKind::Procedure.into(),
        NodeOrToken::Token(t) => [SyntaxKind::Newline.into(), SyntaxKind::Whitespace.into()].contains(&t.kind()),
    }) && matches!(children.last(), Some(NodeOrToken::Node(_)));
    ok.then_some(children)
}

/// `node` (starting at byte `start`) with `new` added at byte `at`, which must fall between two children of an object
/// body, after its `{`.
fn insert_at(
    node: &GreenNodeData,
    start: usize,
    at: usize,
    new: &[NodeOrToken<GreenNode, rowan::GreenToken>],
) -> Option<GreenNode> {
    let mut off = start;
    for (i, child) in node.children().enumerate() {
        let end = off + usize::from(child.text_len());
        if off < at && at < end {
            // Inside this child: descend into a node; a token cannot be split.
            let inner = insert_at(child.into_node()?, off, at, new)?;
            return Some(node.replace_child(i, NodeOrToken::Node(inner)));
        }
        if off == at {
            if node.kind() != SyntaxKind::ObjectBody.into() || i == 0 {
                return None;
            }
            return Some(node.splice_children(i..i, new.iter().cloned()));
        }
        off = end;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    fn joined(src: &str, inserts: &[(usize, &str)]) -> String {
        let mut out = src.to_string();
        for (at, text) in inserts.iter().rev() {
            out.insert_str(*at, text);
        }
        out
    }

    /// The spliced tree is the tree of the joined text, node for node and token for token.
    fn same_as_full_parse(src: &str, inserts: &[(usize, &str)]) {
        let syms: BTreeSet<String> = ["BC27PLUS".to_string()].into();
        let original = parse(src, &syms);
        let spliced = splice_routines(src, &original, inserts, &syms, &mut TreeCache::default()).expect("spliced");
        let full = parse(&joined(src, inserts), &syms);
        assert!(full.errors.is_empty(), "{:?}", full.errors);
        assert_eq!(spliced.errors, full.errors);
        assert_eq!(spliced.green, full.green);
    }

    fn close(src: &str, nth: usize) -> usize {
        src.match_indices('}').map(|(i, _)| i).nth(nth).expect("a `}`")
    }

    const PAGE: &str = "page 1 P\n{\n    layout\n    {\n        field(A; Rec.A) { Visible = Show; }\n    }\n    var\n        Show: Boolean;\n}\n";

    #[test]
    fn a_routine_before_the_closing_brace_of_an_object() {
        let at = PAGE.rfind('}').expect("close");
        same_as_full_parse(
            PAGE,
            &[(at, "\nprocedure __visible_40(): Boolean begin exit(Show); end;")],
        );
    }

    #[test]
    fn a_closing_brace_right_after_code_and_two_routines_in_one_text() {
        let src = "table 1 T { fields { field(1; A; Text[10]) { CaptionClass = '1,' + A; } } procedure P() begin end;}";
        let at = src.rfind('}').expect("close");
        same_as_full_parse(
            src,
            &[(
                at,
                "\nprocedure __captionclass_1(): Text begin exit('1,' + A); end;\nprocedure __x_2(): Text begin exit('a'); end;",
            )],
        );
    }

    #[test]
    fn routines_after_a_member_of_a_nested_body_and_before_the_object_end() {
        let src = "report 1 R\n{\n    dataset\n    {\n        dataitem(D; Customer)\n        {\n            column(No; D.\"No.\") { }\n            column(N; D.Name) { }\n        }\n    }\n}\n";
        // After the first column section and after the second; then before the object's `}`.
        let first = close(src, 0) + 1;
        let second = close(src, 1) + 1;
        let end = src.rfind('}').expect("close");
        same_as_full_parse(
            src,
            &[
                (first, "\nprocedure __column_0(): Variant begin exit(D.\"No.\"); end;"),
                (second, "\nprocedure __column_1(): Variant begin exit(D.Name); end;"),
                (end, "\nprocedure __visible_9(): Boolean begin exit(true); end;"),
            ],
        );
    }

    #[test]
    fn a_file_with_crlf_line_ends_comments_and_directives() {
        let src = "#if BC27PLUS\r\npage 1 P\r\n{\r\n    // last\r\n    /* block */}\r\n#endif\r\npage 2 Q { }";
        let at = close(src, 0);
        same_as_full_parse(
            src,
            &[(at, "\nprocedure __visible_1(): Boolean begin exit(1 = 2); end;")],
        );
    }

    #[test]
    fn what_cannot_be_shown_is_left_to_a_full_parse() {
        let syms = BTreeSet::new();
        let none = |src: &str, inserts: &[(usize, &str)]| {
            let p = parse(src, &syms);
            splice_routines(src, &p, inserts, &syms, &mut TreeCache::default()).is_none()
        };
        let r = "\nprocedure __visible_1(): Boolean begin exit(true); end;";
        let at = PAGE.rfind('}').expect("close");
        // Inside a token, inside a routine body, before the object's `{`.
        assert!(none(PAGE, &[(PAGE.find("layout").expect("l") + 2, r)]));
        assert!(none("codeunit 1 C { procedure P() begin end; }", &[(29, r)]));
        assert!(none(PAGE, &[(PAGE.find('{').expect("open"), r)]));
        // A lone carriage return before the offset lexes with the added line break as one token.
        let cr = "page 1 P { field(A; B) { } \r}";
        assert!(none(cr, &[(cr.rfind('}').expect("close"), r)]));
        // A text that does not parse, a directive, a text without a leading line break.
        assert!(none(
            PAGE,
            &[(at, "\nprocedure __visible_1(): Boolean begin exit(; end;")]
        ));
        assert!(none(
            PAGE,
            &[(
                at,
                "\nprocedure __visible_1(): Boolean begin\n#if X\nexit(true);\n#endif\nend;"
            )]
        ));
        assert!(none(
            PAGE,
            &[(at, "procedure __visible_1(): Boolean begin exit(true); end;")]
        ));
        // A file with a parse error.
        let bad = "page 1 P { field(A; B) { 42 } }";
        assert!(none(bad, &[(bad.rfind('}').expect("close"), r)]));
    }
}
