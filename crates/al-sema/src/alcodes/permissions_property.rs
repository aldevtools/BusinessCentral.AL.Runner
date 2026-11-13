//! The syntax errors of an object's `Permissions` property that corpus writer Y93 measured (must-not-compile 2830 and 2831 on
//! corpus branch al-runtime-rs-staging-y93, checked with BC 27.5, 28.5 and 29.0 symbols). The property of an object other than a
//! permission set takes table data only:
//! - `tabledata * = i` (a wildcard, as a permission set may use): AL0107 "identifier expected" and AL0104 for `=` and `,` (2830);
//! - `codeunit "X" = X` (another object kind): AL0104 "'tabledata' expected" (2831).
//!
//! Only these two shapes are reported; other malformed values are left to the parser.

use super::{offset_of, push};
use crate::workspace::{Diagnostic, ObjKind, Workspace};
use al_syntax::ast;

/// AL0107 and AL0104 for the `Permissions` property of object `oi` (see the module).
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    // A permission set (and an entitlement, an object of kind Other here) declares its grants in the same property, wildcards
    // and every object kind included.
    if matches!(
        ws.objects[oi].kind,
        ObjKind::PermissionSet | ObjKind::PermissionSetExtension | ObjKind::Other
    ) {
        return;
    }
    let file = ws.objects[oi].file;
    for p in obj
        .properties()
        .filter(|p| p.name().eq_ignore_ascii_case("Permissions"))
    {
        let at = offset_of(p.syntax());
        for entry in entries(&p.value_text()) {
            let entry = entry.trim_start();
            let word: String = entry.chars().take_while(|c| c.is_ascii_alphanumeric()).collect();
            if !word.eq_ignore_ascii_case("tabledata") {
                if !word.is_empty() {
                    push(out, ws, file, at, "AL0104", "Syntax error, 'tabledata' expected".into());
                }
                continue;
            }
            if entry[word.len()..].trim_start().starts_with('*') {
                push(out, ws, file, at, "AL0107", super::IDENTIFIER_EXPECTED.into());
                push(out, ws, file, at, "AL0104", "Syntax error, '=' expected".into());
                push(out, ws, file, at, "AL0104", "Syntax error, ',' expected".into());
            }
        }
    }
}

/// The entries of a property value, split at the commas outside quoted names.
fn entries(value: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut start, mut quoted) = (0, false);
    for (i, c) in value.char_indices() {
        match c {
            '"' => quoted = !quoted,
            ',' if !quoted => {
                out.push(&value[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&value[start..]);
    out
}
