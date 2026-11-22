//! AL0185 `Enum 'X' is missing` for a local variable of a procedure declared with an enum type by a one-segment name, in
//! two shapes:
//! - no app the file sees declares an enum of that name (must-not-compile 0214-unknown-enum-type.al:7);
//! - the file has a namespace, no `using` of the enum's, and the one enum of that name it sees is declared in the file's
//!   own app in a sibling namespace: the same parent, another last part (al-runner issue 7921, the
//!   reporter's alc 18.1 repro: `namespace Acme.Logic;` names `Enum "Widget Kind"` of `namespace Acme.Kinds;`; with
//!   `using Acme.Kinds;` it compiles; assumption `enum-plain-name-outside-namespace`). The corpus probe of corpus PR 1363
//!   measures the cross-app shapes (must-not-compile 3134, 3135); until they are pinned, an enum of a dependency app keeps
//!   compiling.
//!
//! Every other shape keeps compiling: a global variable, a parameter, a return type, a trigger's variable, an id, a
//! qualified name, a file without a namespace that finds the enum in any namespace, an enum of a dependency app in another
//! namespace, two visible enums of that name, a namespace with another parent, and a parent or child namespace (not
//! measured).

use super::{offset_of, push};
use crate::workspace::{Diagnostic, ObjKind, Workspace};
use al_builtins::assumptions::Assumption;
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
        let missing =
            !ws.names_object(file, ObjKind::Enum, &parts) || out_of_scope_in_own_app(ws, file, name).is_some();
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

/// The issue 7921 shape: `file` has a namespace, the enum `name` is not in its scope tiers (its namespace, its `using`s,
/// no namespace), and the one enum of that name it sees is declared in its own app in a sibling namespace of the file's:
/// the same parent, another last part (assumption `enum-plain-name-outside-namespace`).
fn out_of_scope_in_own_app(ws: &Workspace, file: usize, name: &str) -> Option<Assumption> {
    let f = &ws.files[file];
    let own = f.namespace.as_deref()?;
    if ws.in_scope_tiers(file, ObjKind::Enum, name) {
        return None;
    }
    let [i] = ws.named_objects_visible(file, ObjKind::Enum, name)[..] else {
        return None;
    };
    let o = &ws.objects[i];
    // A compile refusal ends before any test runs, so it cannot carry a flagged-pass mark. The gate returns the registry
    // entry it rests on, so that deleting the entry at the corpus pin names this gate.
    (o.app == f.app && o.namespace.as_deref().is_some_and(|ns| sibling_namespaces(own, ns)))
        .then_some(Assumption::EnumPlainNameOutsideNamespace)
}

/// Two namespaces with the same parent and another last part (`Acme.Logic` and `Acme.Kinds`, the issue 7921 pair).
fn sibling_namespaces(a: &str, b: &str) -> bool {
    match (a.rsplit_once('.'), b.rsplit_once('.')) {
        (Some((pa, la)), Some((pb, lb))) => pa.eq_ignore_ascii_case(pb) && !la.eq_ignore_ascii_case(lb),
        _ => false,
    }
}
