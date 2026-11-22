//! A member of an option type, or of a table field's OptionMembers property, named unquoted by a word the AL compiler takes
//! as a keyword there (al-runner issue 8657). The AL compilers of the BC artifacts of every supported major and a BC
//! container give, at the word, the first diagnostic below; alc's recovery after it is not reproduced. A quoted member
//! compiles, and so do the other words.
//!
//! - An option type (`O: Option Zero,Key;`, in a variable, a global, a parameter or a return type, as any member): the
//!   words of [`PARAM_NAME_KEYWORDS`]. AL0105 (`'key' is a keyword`) for 21 of them; `begin`, `var` and `end` give a
//!   diagnostic of their own per place (`type_diagnostic`).
//! - The OptionMembers property of a table or table extension field (`OptionMembers = Zero,If;`): the seven words of
//!   [`OPTION_MEMBERS_KEYWORDS`], AL0105. `Key` and the rest of the 24 compile there.

use super::{PARAM_NAME_KEYWORDS, push};
use crate::workspace::{Diagnostic, ObjKind, Workspace};
use al_builtins::assumptions::Assumption;
use al_syntax::{SyntaxKind, SyntaxNode, SyntaxToken, ast};

/// The words the AL compilers reject unquoted in the OptionMembers property; of 99 words measured, these seven give
/// AL0105 at the word, in a table and a table extension field, first or later in the list.
const OPTION_MEMBERS_KEYWORDS: [&str; 7] = [
    "if",
    "else",
    "true",
    "false",
    "pageextension",
    "profileextension",
    "tableextension",
];

/// The diagnostics for the option types `types` of object `oi` (its type references, from `declarations`) and the
/// OptionMembers properties of its fields, in source order.
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, types: &[SyntaxNode], out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for ty in types {
        let toks = significant(ty);
        let Some((first, members)) = toks.split_first() else {
            continue;
        };
        if !(first.kind() == SyntaxKind::Ident && first.text().eq_ignore_ascii_case("option")) {
            continue;
        }
        let place = match ty.parent().map(|p| p.kind()) {
            Some(SyntaxKind::ReturnSpec) => Place::Return,
            Some(SyntaxKind::Param) => Place::Parameter,
            Some(SyntaxKind::VarDecl) => Place::Variable,
            // An option inside another type (`array[2] of Option Zero,Key`): not measured, not judged.
            _ => continue,
        };
        for member in keyword_members(members, PARAM_NAME_KEYWORDS) {
            if let Some((at, code, message)) = type_diagnostic(&member, place) {
                push(out, ws, file, at, code, message);
            }
        }
    }
    if !matches!(ws.objects[oi].kind, ObjKind::Table | ObjKind::TableExtension) {
        return;
    }
    let properties = obj
        .syntax()
        .descendants()
        .filter_map(ast::Property::cast)
        .filter(|p| p.name().eq_ignore_ascii_case("optionmembers"));
    for property in properties {
        let in_field = property
            .syntax()
            .parent()
            .and_then(|b| b.parent())
            .and_then(ast::Section::cast)
            .is_some_and(|s| s.name().eq_ignore_ascii_case("field"));
        if !in_field {
            continue;
        }
        let Some(value) = property
            .syntax()
            .children()
            .find(|n| n.kind() == SyntaxKind::PropertyValue)
        else {
            continue;
        };
        for member in keyword_members(&significant(&value), &OPTION_MEMBERS_KEYWORDS) {
            let lower = member.word.text().to_ascii_lowercase();
            push(
                out,
                ws,
                file,
                member.word.text_range().start().into(),
                "AL0105",
                format!("Syntax error, identifier expected; '{lower}' is a keyword"),
            );
        }
    }
}

fn significant(n: &SyntaxNode) -> Vec<SyntaxToken> {
    n.descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
        .collect()
}

/// Where an option type is written; the AL compilers' first diagnostic for `begin`, `var` and `end` depends on it.
#[derive(Clone, Copy)]
enum Place {
    Variable,
    Parameter,
    Return,
}

/// One unquoted keyword member of a comma-separated list: whether it is the first member, the token after it, and the
/// registry entry the rule rests on (a compile refusal ends before any test runs, so it carries no flagged-pass mark; the
/// gate names the entry so that deleting it at the pin names this check).
struct Member {
    word: SyntaxToken,
    first: bool,
    next: Option<SyntaxToken>,
    _rests_on: Assumption,
}

/// The first diagnostic the AL compilers give for `member` of an option type written at `place`, as (offset, code,
/// message), or `None` where this check does not judge it. Each cell is a must-not-compile case of corpus PR 1502
/// (al-runner issue 8657; the PR body lists them by place and position). `begin` and `var` as the last member stop this
/// parser already. The AL compiler fails with an internal error (AL0999 at the start of the file) for `var` as the first
/// member of a parameter's type and `end` as the first member of a parameter's or a return type's, which are not judged.
fn type_diagnostic(member: &Member, place: Place) -> Option<(usize, &'static str, String)> {
    let word = &member.word;
    let lower = word.text().to_ascii_lowercase();
    let at_word = word.text_range().start().into();
    let comma = member.next.as_ref().filter(|t| t.kind() == SyntaxKind::Comma);
    let at_comma = comma.map(|t| t.text_range().start().into());
    let expected = |what: &str| ("AL0104", format!("Syntax error, '{what}' expected"));
    let (at, (code, message)) = match (lower.as_str(), place) {
        ("begin" | "var", _) if comma.is_none() => return None,
        ("begin", Place::Parameter) => (at_word, expected(")")),
        ("begin", Place::Return) => (at_comma?, expected("end")),
        ("var", Place::Parameter) | ("end", Place::Parameter | Place::Return) if member.first => return None,
        ("var", Place::Return) => (
            at_comma?,
            ("AL0519", "',' is not valid value in this context.".to_string()),
        ),
        ("end", Place::Return) => (
            at_word,
            (
                "AL0519",
                format!("'{}' is not valid value in this context.", word.text()),
            ),
        ),
        ("begin" | "var" | "end", _) => (at_word, expected(";")),
        _ => (
            at_word,
            (
                "AL0105",
                format!("Syntax error, identifier expected; '{lower}' is a keyword"),
            ),
        ),
    };
    Some((at, code, message))
}

/// The members of a comma-separated list that are one unquoted word of `words`.
fn keyword_members(list: &[SyntaxToken], words: &[&str]) -> Vec<Member> {
    let comma = |t: &SyntaxToken| t.kind() == SyntaxKind::Comma;
    let mut out = Vec::new();
    for (i, one) in list.iter().enumerate() {
        let alone = (i == 0 || comma(&list[i - 1])) && list.get(i + 1).is_none_or(comma);
        if alone && one.kind() == SyntaxKind::Ident && words.contains(&one.text().to_ascii_lowercase().as_str()) {
            out.push(Member {
                word: one.clone(),
                first: i == 0,
                next: list.get(i + 1).cloned(),
                _rests_on: Assumption::KeywordOptionMembers,
            });
        }
    }
    out
}
