//! AL0107 for a member named, unquoted, by a word of [`MEMBER_KEYWORDS`] after `.` or `::` in the code of a procedure or
//! trigger: `Row.If := 1;`, `V := Callee.Exit();` (al-runner issue 8635), `K := K::Exit;`, `Database::Exit` (issue 8649). The
//! AL compilers of the BC artifacts of every supported major and a BC container give AL0107 (identifier expected) at the word
//! first, for each word. After `.`: in procedures and triggers of codeunits, tables, table fields, pages, page actions and
//! reports, through a record, Rec, xRec, a codeunit, an interface, a query and a TestPage, and for a namespace segment in code
//! (`Database::Ns.If."T"`). After `::`: an enum variable, an enum type, an option field, an option variable, a case label and
//! an object id; `in` there is pinned by corpus must-not-compile 3158-3160 (issue 7803). alc's recovery after it is not
//! reproduced. A quoted name compiles, and so do the other words (`Local`, `Key`, `Area`, `Value`). A namespace segment that
//! is such a word compiles in `namespace`, `using` and a variable's type, which are not code. An expression property or a
//! control's source expression (`Visible = Rec.If > 0;`) is rejected the same way by the AL compilers but is not judged
//! here: this checker sees it only as a synthetic routine (al-runner issue 8650).

use super::{IDENTIFIER_EXPECTED, push};
use crate::workspace::{Diagnostic, Workspace};
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
