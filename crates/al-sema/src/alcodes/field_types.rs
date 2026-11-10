//! AL0156 for a table field of type Char or Byte (must-not-compile 3112-table-field-type-char-and-byte). Both types exist as
//! variable types only. Only these two types are measured; other types and table extension fields are left alone.

use super::{offset_of, push};
use crate::workspace::{Diagnostic, Workspace};
use al_syntax::ast;

/// AL0156 for each Char or Byte field of table `oi`, in field order.
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for fields in obj.sections().filter(|s| s.name().eq_ignore_ascii_case("fields")) {
        for field in fields.sections().filter(|s| s.name().eq_ignore_ascii_case("field")) {
            let Some(ty) = field.args().get(2).map(|t| t.trim().to_string()) else {
                continue;
            };
            let shown = if ty.eq_ignore_ascii_case("Char") {
                "Char"
            } else if ty.eq_ignore_ascii_case("Byte") {
                "Byte"
            } else {
                continue;
            };
            push(
                out,
                ws,
                file,
                offset_of(field.syntax()),
                "AL0156",
                format!(
                    "'{shown}' is not a valid field type. Use basic types (Text, Integer, Decimal, etc.) or reference custom types (Record, Enum, etc.)."
                ),
            );
        }
    }
}
