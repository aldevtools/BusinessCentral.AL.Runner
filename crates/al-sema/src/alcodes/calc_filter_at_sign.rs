//! AL0870 and AL0224 for a table FlowField whose CalcFormula filters a field by `filter(@)` without quotes (must-not-compile
//! 3805 on corpus pull request 1495, al-runner issue 8637): the compiler expects a verbatim string literal there. Without quotes,
//! `filter(<>@)` is AL0726, AL0104 and AL0870 (must-not-compile 3806). The quoted `filter('@')` and `filter('<>@')` compile
//! (record/filters/AtSignAloneReads.al:579, :752). Blanks anywhere in the unquoted forms change nothing (cases 3807 and 3808).
//! Only `@` alone and `<>@` alone are measured; other unquoted texts with `@`
//! and table extension fields are left alone (the lowering refuses an unquoted `@` alone or `<>@` alone wherever it is).

use super::{offset_of, push};
use crate::calcformula::{CondValue, parse};
use crate::workspace::{Diagnostic, Workspace};
use al_syntax::ast;

/// An unquoted `@` alone or `<>@` alone in a CalcFormula's `filter(...)`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnquotedAtSign {
    /// `filter(@)`: AL0870 and AL0224 (case 3805).
    Alone,
    /// `filter(<>@)`: AL0726, AL0104 and AL0870 (case 3806).
    Negated,
}

impl UnquotedAtSign {
    fn codes(self) -> &'static [(&'static str, &'static str)] {
        match self {
            UnquotedAtSign::Alone => &[
                ("AL0870", "Expected verbatim string literal."),
                (
                    "AL0224",
                    "Expression expected. Provide a valid expression (variable, constant, calculation, or method call).",
                ),
            ],
            UnquotedAtSign::Negated => &[
                (
                    "AL0726",
                    "An identifier, a literal, or an option access is expected as the value of a filter expression",
                ),
                ("AL0104", "Syntax error, ')' expected"),
                ("AL0870", "Expected verbatim string literal."),
            ],
        }
    }
}

/// The diagnostics of each FlowField of table `oi` with an unquoted `filter(@)` or `filter(<>@)` (see the module).
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for fields in obj.sections().filter(|s| s.name().eq_ignore_ascii_case("fields")) {
        for field in fields.sections().filter(|s| s.name().eq_ignore_ascii_case("field")) {
            let unquoted = field.properties().find_map(|p| {
                if !p.name().eq_ignore_ascii_case("CalcFormula") {
                    return None;
                }
                parse(&p.value_text()).ok()?.conds.iter().find_map(|c| match &c.value {
                    CondValue::Filter(raw) => unquoted_at_sign(raw),
                    _ => None,
                })
            });
            if let Some(kind) = unquoted {
                let at = offset_of(field.syntax());
                for (code, text) in kind.codes() {
                    push(out, ws, file, at, code, (*text).into());
                }
            }
        }
    }
}

/// Whether the raw text of a CalcFormula's `filter(...)` is `@` alone or `<>@` alone, without quotes, with blanks anywhere in it or
/// not: BC reports the same diagnostics (cases 3807 and 3808).
pub(crate) fn unquoted_at_sign(raw: &str) -> Option<UnquotedAtSign> {
    let text: String = raw.chars().filter(|c| !c.is_whitespace()).collect();
    match text.as_str() {
        "@" => Some(UnquotedAtSign::Alone),
        "<>@" => Some(UnquotedAtSign::Negated),
        _ => None,
    }
}
