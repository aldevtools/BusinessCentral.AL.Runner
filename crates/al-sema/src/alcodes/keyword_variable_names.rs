//! AL0104, AL0107, AL0303 and AL0519: a variable named, unquoted, by a word the AL compiler does not read as a name in that
//! place (al-runner issue 8539). The words are those of [`PARAM_NAME_KEYWORDS`], the ones it rejects as a parameter name:
//!
//! - As the first name of a declaration every one of them is rejected (`begin`, `end`, `of`, `procedure`, `trigger` and
//!   `var` the parser already does not take as one).
//! - As a later name of the list (`Other, Key: Integer;`) every one of them is rejected, AL0107 at the name.
//! - After an attribute (`[NonDebuggable] Key: Text;`) only `event`, `true` and `false` are rejected; the others compile.
//!
//! The code and text are those of the first diagnostic alc gives at the name, which depend on the section, on whether an
//! earlier declaration stands in it, and on the word (the tables below); alc's recovery after it is not reproduced. One
//! diagnostic per name.
//!
//! Measured with the AL compilers of the BC artifacts of every supported major (the same codes and places on all of them;
//! the oldest gives AL0107 without the sentence after "identifier expected") and in a BC container, for every word, in the
//! global var section, `protected var`, a report's request page var section, procedure var sections (local procedures and
//! event subscribers too) and trigger var sections (object triggers; field, action, data item, xmlport element and request
//! page triggers) of codeunits, tables, table extensions, pages, page extensions, reports, report extensions, xmlports and
//! queries. Assumption `keyword-variable-names` until corpus PR 1476 (must-not-compile 3222-3275) is pinned; the page
//! global `area` is pinned by must-not-compile 0334-variable-named-area.al:1 (AL0104 `'}' expected`). Other object types
//! and other var sections (an xmlport's or report extension's request page) are not judged.

use super::{IDENTIFIER_EXPECTED, PARAM_NAME_KEYWORDS, offset_of, push};
use crate::workspace::{Diagnostic, ObjKind, Workspace};
use al_builtins::assumptions::Assumption;
use al_syntax::{SyntaxKind, SyntaxNode, SyntaxToken, ast};

/// AL0519 for these as the first declaration of a procedure's var section.
const NOT_A_VALUE_FIRST: &[&str] = &["area", "extends", "false", "fieldgroups", "key", "true"];

/// AL0519 for these as a later declaration of any var section.
const NOT_A_VALUE_LATER: &[&str] = &[
    "area",
    "extends",
    "false",
    "fieldgroups",
    "key",
    "true",
    "pageextension",
    "profileextension",
    "tableextension",
];

/// The words alc rejects after an attribute.
const AFTER_ATTRIBUTE: &[&str] = &["event", "true", "false"];

/// The kind of var section a declaration stands in.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    /// The object's global var section, `protected var`, or a report's request page var section.
    Global,
    Procedure,
    Trigger,
}

/// Where a diagnostic stands.
enum Place {
    Word,
    AfterWord,
    FirstAttribute,
}

/// The keyword-name diagnostics of the variable declarations `decls` of object `oi` (see the module).
pub(super) fn check(ws: &Workspace, oi: usize, decls: &[ast::VarDecl], out: &mut Vec<Diagnostic>) {
    if !matches!(
        ws.objects[oi].kind,
        ObjKind::Codeunit
            | ObjKind::Table
            | ObjKind::TableExtension
            | ObjKind::Page
            | ObjKind::PageExtension
            | ObjKind::Report
            | ObjKind::ReportExtension
            | ObjKind::XmlPort
            | ObjKind::Query
    ) {
        return;
    }
    let file = ws.objects[oi].file;
    for d in decls {
        let Some(section) = section_of(ws.objects[oi].kind, d.syntax()) else {
            continue;
        };
        let first_decl = !d
            .syntax()
            .siblings(rowan::Direction::Prev)
            .skip(1)
            .any(|s| s.kind() == SyntaxKind::VarDecl);
        let attributes = d.attributes();
        for (i, name) in names(d.syntax()).into_iter().enumerate() {
            let shape = Shape {
                section,
                first_decl,
                later_name: i > 0,
                attributed: !attributes.is_empty(),
            };
            if name.kind() != SyntaxKind::Ident {
                continue;
            }
            let Some((code, message, place, _rests_on)) = first_diagnostic(&shape, name.text()) else {
                continue;
            };
            let at = match place {
                Place::Word => name.text_range().start().into(),
                Place::AfterWord => name.text_range().end().into(),
                Place::FirstAttribute => attributes.first().map_or(0, |a| offset_of(a.syntax())),
            };
            push(out, ws, file, at, code, message);
        }
    }
}

/// The section `decl` stands in, if it is one the measurement covers.
fn section_of(kind: ObjKind, decl: &SyntaxNode) -> Option<Section> {
    let var_section = decl.parent().filter(|p| p.kind() == SyntaxKind::VarSection)?;
    let holder = var_section.parent()?;
    match holder.kind() {
        SyntaxKind::Procedure => Some(Section::Procedure),
        SyntaxKind::Trigger => Some(Section::Trigger),
        SyntaxKind::ObjectBody => {
            let owner = holder.parent()?;
            let request_page = owner.kind() == SyntaxKind::Section
                && kind == ObjKind::Report
                && first_word(&owner).is_some_and(|w| w.eq_ignore_ascii_case("requestpage"))
                && owner
                    .parent()
                    .and_then(|b| b.parent())
                    .is_some_and(|o| o.kind() == SyntaxKind::Object);
            (owner.kind() == SyntaxKind::Object || request_page).then_some(Section::Global)
        }
        _ => None,
    }
}

fn first_word(n: &SyntaxNode) -> Option<String> {
    n.children_with_tokens()
        .filter_map(|e| e.into_token())
        .find(|t| !t.kind().is_trivia())
        .map(|t| t.text().to_string())
}

/// The names of a declaration, in order, quoted ones included (they keep the place of the names after them).
fn names(decl: &SyntaxNode) -> Vec<SyntaxToken> {
    decl.children_with_tokens()
        .filter_map(|e| e.into_token())
        .take_while(|t| t.kind() != SyntaxKind::Colon)
        .filter(|t| matches!(t.kind(), SyntaxKind::Ident | SyntaxKind::QuotedIdent))
        .collect()
}

/// The facts of a name's place the first diagnostic depends on.
struct Shape {
    section: Section,
    /// No declaration stands before this one in its section.
    first_decl: bool,
    /// A name after a comma.
    later_name: bool,
    /// The declaration has attributes.
    attributed: bool,
}

/// The first diagnostic alc gives for the unquoted name `written` in `shape`, with the registry entry the rule rests on
/// until the corpus pins it (a compile refusal ends before any test runs, so it carries no flagged-pass mark; the gate
/// names the entry so that deleting it at the pin names this check). `None` when the name compiles.
fn first_diagnostic(shape: &Shape, written: &str) -> Option<(&'static str, String, Place, Assumption)> {
    let word = written.to_ascii_lowercase();
    if !PARAM_NAME_KEYWORDS.contains(&word.as_str()) {
        return None;
    }
    let (code, message, place) = first_code(shape, &word, written)?;
    Some((code, message, place, Assumption::KeywordVariableNames))
}

/// [`first_diagnostic`] without the entry, for a word of [`PARAM_NAME_KEYWORDS`] (`word` in lower case).
fn first_code(shape: &Shape, word: &str, written: &str) -> Option<(&'static str, String, Place)> {
    let identifier_expected = |place| Some(("AL0107", IDENTIFIER_EXPECTED.to_string(), place));
    let expected = |what: &str| Some(("AL0104", format!("Syntax error, '{what}' expected"), Place::Word));
    let not_a_value = || {
        Some((
            "AL0519",
            format!("'{written}' is not valid value in this context."),
            Place::Word,
        ))
    };
    if shape.later_name {
        return identifier_expected(Place::Word);
    }
    if shape.attributed {
        if !AFTER_ATTRIBUTE.contains(&word) {
            return None;
        }
        return match (shape.section, word) {
            (Section::Global, "event") => identifier_expected(Place::AfterWord),
            (Section::Global, _) => expected("procedure"),
            _ => Some((
                "AL0303",
                "Attributes can only be defined on variables and methods.".to_string(),
                Place::FirstAttribute,
            )),
        };
    }
    let al0519 = if shape.first_decl {
        shape.section == Section::Procedure && NOT_A_VALUE_FIRST.contains(&word)
    } else {
        NOT_A_VALUE_LATER.contains(&word)
    };
    match shape.section {
        _ if al0519 => not_a_value(),
        Section::Trigger => expected("begin"),
        _ if word == "event" => identifier_expected(Place::AfterWord),
        // A page's first global `area: Integer;` is must-not-compile 0334-variable-named-area.al:8.
        _ => expected("}"),
    }
}
