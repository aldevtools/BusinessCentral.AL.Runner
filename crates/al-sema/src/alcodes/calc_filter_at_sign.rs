//! AL0870 and AL0224 for a table FlowField whose CalcFormula filters a field by `filter(@)` without quotes (must-not-compile
//! 3805 on corpus pull request 1495, al-runner issue 8637): the compiler expects a verbatim string literal there. The quoted
//! `filter('@')` compiles (record/filters/AtSignAloneReads.al:579). Only `@` alone is measured; other unquoted texts with `@`
//! and table extension fields are left alone (the lowering refuses an unquoted `@` alone wherever it is).

use super::{offset_of, push};
use crate::calcformula::{CondValue, parse};
use crate::workspace::{Diagnostic, Workspace};
use al_syntax::ast;

/// AL0870 and AL0224 for each FlowField of table `oi` with an unquoted `filter(@)` (see the module).
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for fields in obj.sections().filter(|s| s.name().eq_ignore_ascii_case("fields")) {
        for field in fields.sections().filter(|s| s.name().eq_ignore_ascii_case("field")) {
            let unquoted = field.properties().any(|p| {
                p.name().eq_ignore_ascii_case("CalcFormula")
                    && parse(&p.value_text()).is_ok_and(|f| {
                        f.conds
                            .iter()
                            .any(|c| matches!(&c.value, CondValue::Filter(raw) if unquoted_at_sign(raw)))
                    })
            });
            if unquoted {
                let at = offset_of(field.syntax());
                push(out, ws, file, at, "AL0870", "Expected verbatim string literal.".into());
                push(
                    out,
                    ws,
                    file,
                    at,
                    "AL0224",
                    "Expression expected. Provide a valid expression (variable, constant, calculation, or method call).".into(),
                );
            }
        }
    }
}

/// Whether the raw text of a CalcFormula's `filter(...)` is `@` alone, without quotes.
pub(crate) fn unquoted_at_sign(raw: &str) -> bool {
    raw.trim() == "@"
}
