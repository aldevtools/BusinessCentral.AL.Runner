//! AL0271 for an event subscriber of `OnBeforeActionEvent` or `OnAfterActionEvent` whose element names no action of the page
//! (must-not-compile 3113-page-action-subscriber-unknown-action: `The action 'NoSuchAction' is not found in the target '<page>'`;
//! corpus events/page-events/PageActionEventsWithoutTrigger.al header; al-runner issue 7519).
//!
//! Only a page of the workspace is judged, named by a plain name or an id, that resolves to one page. The actions are those the
//! page declares and those its page extensions of the workspace add. A page of a dependency is not judged (its source is not
//! at hand), and a subscriber without an element is left alone.

use super::{offset_of, push};
use crate::workspace::{Diagnostic, ObjKind, Resolution, Workspace};
use al_syntax::ast::NodeText;
use al_syntax::{SyntaxKind, SyntaxNode, ast};

/// The text of a string literal argument, without its quotes.
fn literal(n: &SyntaxNode) -> String {
    let t = n.text_string();
    let t = t.trim();
    t.strip_prefix('\'')
        .and_then(|r| r.strip_suffix('\''))
        .map(|r| r.replace("''", "'"))
        .unwrap_or_default()
}

/// AL0271 for the action event subscribers of object `oi` (see the module).
pub(super) fn check(ws: &Workspace, oi: usize, obj: &ast::Object, out: &mut Vec<Diagnostic>) {
    let file = ws.objects[oi].file;
    for r in ast::routines_in(obj.syntax()) {
        for attr in r
            .attributes()
            .filter(|a| a.name().eq_ignore_ascii_case("EventSubscriber"))
        {
            let Some(args) = attr.args() else { continue };
            let args: Vec<SyntaxNode> = args.children().collect();
            let [kind, publisher, event, element, ..] = args.as_slice() else {
                continue;
            };
            let scope = |n: &SyntaxNode| {
                let mut k = n.children();
                match (n.kind(), k.next(), k.next()) {
                    (SyntaxKind::ScopeExpr, Some(l), Some(r)) if l.kind() == SyntaxKind::NameRef => Some((l, r)),
                    _ => None,
                }
            };
            let (Some((kl, kr)), Some((pl, pr))) = (scope(kind), scope(publisher)) else {
                continue;
            };
            if !kl.text_string().eq_ignore_ascii_case("ObjectType")
                || !kr.text_string().eq_ignore_ascii_case("Page")
                || !pl.text_string().eq_ignore_ascii_case("Page")
                || pr.kind() != SyntaxKind::NameRef
            {
                continue;
            }
            let event = literal(event).to_ascii_lowercase();
            let element = literal(element);
            if element.is_empty() || !matches!(event.as_str(), "onbeforeactionevent" | "onafteractionevent") {
                continue;
            }
            let name = pr
                .children_with_tokens()
                .filter_map(|e| e.into_token())
                .find(|t| {
                    matches!(
                        t.kind(),
                        SyntaxKind::Ident | SyntaxKind::QuotedIdent | SyntaxKind::Integer
                    )
                })
                .map(|t| ast::ident_text(&t))
                .unwrap_or_default();
            let Resolution::Found(page) = ws.resolve_ref_certain(file, ObjKind::Page, &[name]) else {
                continue;
            };
            if ws.is_dependency(page) {
                continue;
            }
            let has = |o: usize| {
                crate::pages::control_sections(&ws.object_ast(o)).iter().any(|s| {
                    matches!(s.name().to_ascii_lowercase().as_str(), "action" | "systemaction")
                        && s.args().first().is_some_and(|n| n.eq_ignore_ascii_case(&element))
                })
            };
            if has(page) || ws.extensions_of(page).iter().any(|&e| has(e)) {
                continue;
            }
            push(
                out,
                ws,
                file,
                offset_of(attr.syntax()),
                "AL0271",
                format!(
                    "The action '{element}' is not found in the target '{}'",
                    ws.objects[page].name
                ),
            );
        }
    }
}
