//! AL0107 for a member named, unquoted, by a word of [`MEMBER_KEYWORDS`] after `.` or `::` in the code of a procedure or
//! trigger: `Row.If := 1;`, `V := Callee.Exit();` (al-runner issue 8635), `K := K::Exit;`, `Database::Exit` (issue 8649). The
//! AL compilers of the BC artifacts of every supported major and a BC container give AL0107 (identifier expected) at the word
//! first, for each word. After `.`: in procedures and triggers of codeunits, tables, table fields, pages, page actions and
//! reports, through a record, Rec, xRec, a codeunit, an interface, a query and a TestPage, and for a namespace segment in code
//! (`Database::Ns.If."T"`). After `::`: an enum variable, an enum type, an option field, an option variable, a case label and
//! an object id; `in` there is pinned by corpus must-not-compile 3158-3160 (issue 7803). alc's recovery after it is not
//! reproduced. A quoted name compiles, and so do the other words (`Local`, `Key`, `Area`, `Value`). A namespace segment that
//! is such a word compiles in `namespace`, `using` and a variable's type, which are not code. The expressions the AL compilers
//! parse from properties and section arguments ([`EXPRESSIONS`]: `Visible = Rec.If > 0;`, `field(Shown; Rec.If)`) are judged
//! the same way, from their own tokens (issue 8650): the workspace turns them into synthetic routines, which the walk of
//! routines skips. Other properties keep compiling: StyleExpr, and TableRelation and CalcFormula, which reject only some
//! of the words after `.`.

use super::{IDENTIFIER_EXPECTED, push};
use crate::workspace::{Diagnostic, ObjKind, Workspace};
use al_builtins::assumptions::Assumption;
use al_syntax::{SyntaxKind, SyntaxNode, SyntaxToken, ast};

/// The words the AL compilers reject as an unquoted member name. Of 99 words measured as a record field written and read
/// through a record variable, and as an enum value after `::` on an enum variable, the same 32 give AL0107 at the word; the
/// other 67 compile (`local`, `key`, `area`, `value`, `page`, ...).
const MEMBER_KEYWORDS: [&str; 32] = [
    "and",
    "asserterror",
    "begin",
    "case",
    "div",
    "do",
    "downto",
    "else",
    "end",
    "event",
    "exit",
    "false",
    "for",
    "foreach",
    "if",
    "in",
    "mod",
    "not",
    "of",
    "or",
    "procedure",
    "repeat",
    "then",
    "this",
    "to",
    "trigger",
    "true",
    "until",
    "var",
    "while",
    "with",
    "xor",
];

/// Where an expression the AL compilers parse lives outside a routine.
enum Place {
    /// The value of the property of this name (lower case).
    Property(&'static str),
    /// The second argument of the section: a control's or column's source expression.
    Source,
}

/// The measured expression places: the object kind, the section the expression sits in (`""` for the object itself) and
/// the place. The AL compilers of the BC artifacts of every supported major give AL0107 at the word for each of the 32
/// words, after `.` and after `::`, in each (issue 8650; the control Visible, Editable, Enabled and CaptionClass, the action
/// Visible, the page DataCaptionExpression, the page control source and the report column source also in a BC container).
const EXPRESSIONS: &[(ObjKind, &str, Place)] = {
    use ObjKind::*;
    use Place::*;
    &[
        (Page, "field", Property("visible")),
        (Page, "field", Property("editable")),
        (Page, "field", Property("enabled")),
        (Page, "field", Property("captionclass")),
        (Page, "field", Property("hidevalue")),
        (Page, "field", Property("showmandatory")),
        (Page, "field", Property("quickentry")),
        (Page, "field", Source),
        (Page, "action", Property("visible")),
        (Page, "action", Property("enabled")),
        (Page, "group", Property("visible")),
        (Page, "part", Property("visible")),
        (Page, "", Property("datacaptionexpression")),
        (PageExtension, "modify", Property("visible")),
        (PageExtension, "field", Source),
        (PageExtension, "action", Property("visible")),
        (Report, "column", Source),
        (ReportExtension, "column", Source),
    ]
};

/// AL0107 for each member of object `oi` named by such a word after `.` or `::` in the body of a procedure or trigger the
/// author wrote, in source order.
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    // The routines the workspace spliced in for expression properties, by where their text was added: a procedure of the
    // author's may carry a name of the same form (`__visible_12`), and the AL compilers judge it.
    let added = ws.synthetic(file);
    let authored = |r: &ast::Routine| {
        let at: usize = r.syntax().text_range().start().into();
        !added.iter().any(|&(start, end)| start <= at && at < end)
    };
    let bodies = obj
        .syntax()
        .descendants()
        .filter_map(ast::Routine::cast)
        .filter(authored)
        .filter_map(|r| r.body());
    for body in bodies {
        let members = body
            .descendants()
            .filter(|n| matches!(n.kind(), SyntaxKind::MemberExpr | SyntaxKind::ScopeExpr));
        for member in members {
            if let Some((name, _rests_on)) = keyword_member(&member) {
                push(
                    out,
                    ws,
                    file,
                    name.text_range().start().into(),
                    "AL0107",
                    IDENTIFIER_EXPECTED.to_string(),
                );
            }
        }
    }
    for expr in expressions(ws.objects[oi].kind, obj) {
        for (word, _rests_on) in keyword_tokens(&expr) {
            push(
                out,
                ws,
                file,
                word.text_range().start().into(),
                "AL0107",
                IDENTIFIER_EXPECTED.to_string(),
            );
        }
    }
}

/// The significant tokens of each expression of object `obj` (of kind `kind`) that [`EXPRESSIONS`] lists, in source order.
fn expressions(kind: ObjKind, obj: &ast::Object) -> Vec<Vec<SyntaxToken>> {
    let significant = |n: &SyntaxNode| -> Vec<SyntaxToken> {
        n.descendants_with_tokens()
            .filter_map(|e| e.into_token())
            .filter(|t| !t.kind().is_trivia())
            .collect()
    };
    let mut out = Vec::new();
    for node in obj.syntax().descendants() {
        if let Some(property) = ast::Property::cast(node.clone()) {
            let owner = node.parent().and_then(|b| b.parent());
            let section = owner
                .and_then(ast::Section::cast)
                .map(|s| s.name().to_ascii_lowercase());
            let name = property.name().to_ascii_lowercase();
            let listed = EXPRESSIONS.iter().any(|(k, s, place)| {
                *k == kind
                    && *s == section.as_deref().unwrap_or("")
                    && matches!(place, Place::Property(p) if *p == name)
            });
            if let (true, Some(value)) = (listed, node.children().find(|n| n.kind() == SyntaxKind::PropertyValue)) {
                out.push(significant(&value));
            }
        } else if let Some(section) = ast::Section::cast(node.clone()) {
            let keyword = section.name().to_ascii_lowercase();
            let listed = EXPRESSIONS
                .iter()
                .any(|(k, s, place)| *k == kind && *s == keyword && matches!(place, Place::Source));
            if let (true, Some(args)) = (listed, node.children().find(|n| n.kind() == SyntaxKind::SectionArgs)) {
                // The tokens after the first `;` at depth 0, without the closing parenthesis.
                let toks = significant(&args);
                let Some(inner) = toks.get(1..toks.len().saturating_sub(1)) else {
                    continue;
                };
                let mut depth = 0;
                let first = inner.iter().position(|t| {
                    match t.kind() {
                        SyntaxKind::LParen | SyntaxKind::LBrack => depth += 1,
                        SyntaxKind::RParen | SyntaxKind::RBrack => depth -= 1,
                        _ => {}
                    }
                    depth == 0 && t.kind() == SyntaxKind::Semicolon
                });
                if let Some(first) = first {
                    out.push(inner[first + 1..].to_vec());
                }
            }
        }
    }
    out
}

/// The words of [`MEMBER_KEYWORDS`] that follow `.` or `::` unquoted in the tokens of an expression, with the registry entry
/// the rule rests on (a compile refusal ends before any test runs, so it carries no flagged-pass mark).
fn keyword_tokens(expr: &[SyntaxToken]) -> Vec<(SyntaxToken, Assumption)> {
    expr.windows(2)
        .filter(|w| matches!(w[0].kind(), SyntaxKind::Dot | SyntaxKind::ColonColon))
        .map(|w| &w[1])
        .filter(|t| t.kind() == SyntaxKind::Ident && MEMBER_KEYWORDS.contains(&t.text().to_ascii_lowercase().as_str()))
        .map(|t| (t.clone(), Assumption::KeywordPropertyMembers))
        .collect()
}

/// The member name token of `member` (`Base.Name` or `Base::Name`) when it is one unquoted word of [`MEMBER_KEYWORDS`], with
/// the registry entry the rule rests on, or `None` for `::in`, which the corpus pins. A compile refusal ends before any test
/// runs, so it carries no flagged-pass mark; the gate names the entry so that deleting it at the pin names this check.
fn keyword_member(member: &SyntaxNode) -> Option<(SyntaxToken, Option<Assumption>)> {
    let name = member.children().nth(1)?;
    let mut toks = name
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia());
    let (Some(word), None) = (toks.next(), toks.next()) else {
        return None;
    };
    let lower = word.text().to_ascii_lowercase();
    if word.kind() != SyntaxKind::Ident || !MEMBER_KEYWORDS.contains(&lower.as_str()) {
        return None;
    }
    let rests_on = match member.kind() {
        SyntaxKind::ScopeExpr if lower == "in" => None,
        SyntaxKind::ScopeExpr => Some(Assumption::KeywordScopeNames),
        _ => Some(Assumption::KeywordMemberNames),
    };
    Some((word, rests_on))
}
