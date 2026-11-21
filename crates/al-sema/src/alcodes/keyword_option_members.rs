//! A member of an option type, or of a table field's OptionMembers property, named unquoted by a word the AL compiler takes
//! as a keyword there (al-runner issue 8657). The AL compilers of the BC artifacts of every supported major and a BC
//! container give, at the word, the first diagnostic below; alc's recovery after it is not reproduced. A quoted member
//! compiles, and so do the other words.
//!
//! - An option type (`O: Option Zero,Key;`, in a variable, a global, a parameter or a return type, as any member): the
//!   words of [`PARAM_NAME_KEYWORDS`]. AL0105 (`'key' is a keyword`) for 21 of them; `end` is AL0104 (`';' expected`),
//!   or AL0519 in a return type. `begin` and `var` stop this parser already, so they are not judged here.
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
        let in_return = ty.parent().is_some_and(|p| p.kind() == SyntaxKind::ReturnSpec);
        for (word, _rests_on) in keyword_members(members, PARAM_NAME_KEYWORDS) {
            let lower = word.text().to_ascii_lowercase();
            let (code, message) = match lower.as_str() {
                "begin" | "var" => continue,
                "end" if in_return => (
                    "AL0519",
                    format!("'{}' is not valid value in this context.", word.text()),
                ),
                "end" => ("AL0104", "Syntax error, ';' expected".to_string()),
                _ => (
                    "AL0105",
                    format!("Syntax error, identifier expected; '{lower}' is a keyword"),
                ),
            };
            push(out, ws, file, word.text_range().start().into(), code, message);
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
        for (word, _rests_on) in keyword_members(&significant(&value), &OPTION_MEMBERS_KEYWORDS) {
            let lower = word.text().to_ascii_lowercase();
            push(
                out,
                ws,
                file,
                word.text_range().start().into(),
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

/// The members of a comma-separated list that are one unquoted word of `words`, with the registry entry the rule rests on
/// (a compile refusal ends before any test runs, so it carries no flagged-pass mark; the gate names the entry so that
/// deleting it at the pin names this check).
fn keyword_members(list: &[SyntaxToken], words: &[&str]) -> Vec<(SyntaxToken, Assumption)> {
    list.split(|t| t.kind() == SyntaxKind::Comma)
        .filter_map(|member| match member {
            [one] if one.kind() == SyntaxKind::Ident && words.contains(&one.text().to_ascii_lowercase().as_str()) => {
                Some((one.clone(), Assumption::KeywordOptionMembers))
            }
            _ => None,
        })
        .collect()
}
