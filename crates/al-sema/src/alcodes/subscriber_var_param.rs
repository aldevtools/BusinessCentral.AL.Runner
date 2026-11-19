//! AL0288 for an event subscriber that declares `var` on a parameter its publisher passes by value (must-not-compile
//! 1685-subscriber-var-on-by-value-parameter: `Parameter 'Plain' is only allowed to be 'var' if the publisher parameter is
//! 'var'.`; al-runner issue 8320).
//!
//! Judged only where the case pins it: a codeunit publisher of the workspace, named by a plain name or an id, that declares one
//! procedure of the event's name, with the `[IntegrationEvent]` attribute. The subscriber's parameter is matched by name.
//! Other publishers and event kinds are not judged.

use super::action_subscriber::subscriber_target;
use super::{offset_of, push};
use crate::workspace::{Diagnostic, ObjKind, Resolution, Workspace};
use al_syntax::ast;

/// AL0288 for the subscribers of object `oi` (see the module).
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for r in ast::routines_in(obj.syntax()) {
        for attr in r
            .attributes()
            .filter(|a| a.name().eq_ignore_ascii_case("EventSubscriber"))
        {
            let Some(t) = subscriber_target(&attr) else { continue };
            if !t.object_type.eq_ignore_ascii_case("Codeunit") || !t.publisher_kind.eq_ignore_ascii_case("Codeunit") {
                continue;
            }
            let Resolution::Found(publisher) = ws.resolve_ref_certain(file, ObjKind::Codeunit, &[t.publisher]) else {
                continue;
            };
            if ws.is_dependency(publisher) {
                continue;
            }
            let publisher_ast = ws.object_ast(publisher);
            let events: Vec<ast::Routine> = ast::routines_in(publisher_ast.syntax())
                .into_iter()
                .filter(|p| p.name().eq_ignore_ascii_case(&t.event))
                .collect();
            let [event] = events.as_slice() else { continue };
            if !event
                .attributes()
                .any(|a| a.name().eq_ignore_ascii_case("IntegrationEvent"))
            {
                continue;
            }
            for p in r.params().filter(ast::Param::is_var) {
                let by_value = event
                    .params()
                    .find(|e| e.name().eq_ignore_ascii_case(&p.name()))
                    .is_some_and(|e| !e.is_var());
                if by_value {
                    push(
                        out,
                        ws,
                        file,
                        offset_of(p.syntax()),
                        "AL0288",
                        format!(
                            "Parameter '{}' is only allowed to be 'var' if the publisher parameter is 'var'.",
                            p.name()
                        ),
                    );
                }
            }
        }
    }
}
