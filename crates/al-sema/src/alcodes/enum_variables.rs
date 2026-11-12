//! AL0185 `Enum 'X' is missing` for a local variable of a procedure declared with an enum type by a one-segment name, in
//! two measured shapes:
//! - no app the file sees declares an enum of that name (must-not-compile 0214-unknown-enum-type.al:7);
//! - the file has a namespace and the enum is declared only in other namespaces of the file's own app, none of them the
//!   file's namespace, a `using` of the file, or a namespace above or below the file's (al-runner issue 7921, the
//!   reporter's alc 18.1 repro: `namespace Acme.Logic;` names `Enum "Widget Kind"` of `namespace Acme.Kinds;`; with
//!   `using Acme.Kinds;` it compiles).
//!
//! Every other shape keeps compiling: a global variable, a parameter, a return type, a trigger's variable, an id, a
//! qualified name, a file without a namespace that finds the enum in any namespace, an enum of a dependency app in another
//! namespace, and a parent or child namespace (not measured).

use super::{offset_of, push};
use crate::workspace::{Diagnostic, ObjKind, Workspace};
use al_syntax::{SyntaxKind, ast};

/// The AL0185 diagnostics of the enum variables of object `oi` (see the module).
pub(super) fn check(ws: &Workspace, oi: usize, decls: &[ast::VarDecl], out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for d in decls {
        let in_procedure = d.syntax().ancestors().any(|a| a.kind() == SyntaxKind::Procedure);
        let Some(t) = d
            .type_ref()
            .filter(|t| in_procedure && t.type_name().eq_ignore_ascii_case("Enum"))
        else {
            continue;
        };
        let parts = t.object_name_parts();
        let [name] = parts.as_slice() else { continue };
        if name.parse::<u32>().is_ok() {
            continue;
        }
        let missing = !ws.names_object(file, ObjKind::Enum, &parts) || out_of_scope_in_own_app(ws, file, name);
        if missing {
            // alc points at the name, after `Enum ` (the issue's `Use.al(7,17)`).
            let at = t
                .syntax()
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .filter(|k| !k.kind().is_trivia())
                .nth(1)
                .map_or_else(|| offset_of(t.syntax()), |k| k.text_range().start().into());
            push(out, ws, file, at, "AL0185", format!("Enum '{name}' is missing"));
        }
    }
}

/// The issue 7921 shape: `file` has a namespace, the enum `name` is not in its scope tiers, and every enum of that name
/// it sees is declared in its own app in a namespace that is neither above nor below the file's.
fn out_of_scope_in_own_app(ws: &Workspace, file: usize, name: &str) -> bool {
    let f = &ws.files[file];
    let Some(own) = f.namespace.as_deref() else {
        return false;
    };
    if ws.in_scope_tiers(file, ObjKind::Enum, name) {
        return false;
    }
    let found = ws.named_objects_visible(file, ObjKind::Enum, name);
    !found.is_empty()
        && found.iter().all(|&i| {
            let o = &ws.objects[i];
            o.app == f.app && o.namespace.as_deref().is_some_and(|ns| unrelated_namespaces(own, ns))
        })
}

/// Two different namespaces of which neither contains the other (`Acme.Logic` and `Acme.Kinds`).
fn unrelated_namespaces(a: &str, b: &str) -> bool {
    let (a, b) = (a.to_ascii_lowercase(), b.to_ascii_lowercase());
    let within = |inner: &str, outer: &str| inner.starts_with(&format!("{outer}."));
    a != b && !within(&a, &b) && !within(&b, &a)
}
