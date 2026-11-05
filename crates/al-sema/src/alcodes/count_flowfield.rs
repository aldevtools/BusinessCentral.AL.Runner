//! AL0202 for a FlowField of a table whose CalcFormula is a `count(...)` and whose type is Decimal (must-not-compile 2970 on
//! corpus branch al-runtime-rs-staging-r6961, al-runner issue 6986). An Integer or a BigInteger FlowField takes a count
//! (record/flowfields/R6986BigIntegerCount.al). Only the Decimal shape of a table field is measured; other types and table
//! extension fields are left alone.

use super::{offset_of, push};
use crate::workspace::{Diagnostic, Workspace};
use al_syntax::ast;

/// AL0202 for the count FlowFields of type Decimal of table `oi` (see the module).
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for fields in obj.sections().filter(|s| s.name().eq_ignore_ascii_case("fields")) {
        for field in fields.sections().filter(|s| s.name().eq_ignore_ascii_case("field")) {
            let args = field.args();
            let (Some(name), Some(ty)) = (args.get(1), args.get(2)) else {
                continue;
            };
            if !ty.trim().eq_ignore_ascii_case("Decimal") {
                continue;
            }
            let count = field.properties().any(|p| {
                p.name().eq_ignore_ascii_case("CalcFormula") && {
                    let v = p.value_text().to_ascii_lowercase();
                    let v = v.trim_start().trim_start_matches('-').trim_start();
                    v.strip_prefix("count").is_some_and(|r| r.trim_start().starts_with('('))
                }
            });
            if count {
                push(
                    out,
                    ws,
                    file,
                    offset_of(field.syntax()),
                    "AL0202",
                    format!(
                        "The {name} FlowField is not an Integer field. If a FlowField CalcFormula starts with 'Count', then the FlowField must be an Integer type field."
                    ),
                );
            }
        }
    }
}
