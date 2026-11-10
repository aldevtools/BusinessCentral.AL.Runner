//! A test selection keeps the test codeunits the install code can run (al-runner issue 6924).
//!
//! A selection leaves the unselected test codeunits out of the compile (`Roots::only_tests`) and keeps those the kept
//! code names (`named_by_kept`). The install code of an app (Install and Upgrade codeunits, subscribers to the install
//! events of `Extension Triggers` and to `Company Triggers`) runs before every test, as in the full run, and it may run a
//! codeunit by an id it computes (`Codeunit.Run(Id)`), which no name or literal in any text shows. A test codeunit run
//! that way would be a stub in the selection and a real codeunit in the full run, so the same selected test would end
//! differently. When the install code, or a workspace object it names, runs a codeunit by a computed id, no test
//! codeunit is left out.

use super::*;

/// Whether the install code that stays in the compile can run a codeunit by an id the text does not show. `skipped`
/// says which objects the selection leaves out.
pub(super) fn install_runs_by_computed_id(ws: &Workspace, skipped: &[bool]) -> bool {
    let skip = |oi: usize| skipped.get(oi).copied().unwrap_or(false);
    let mut roots: Vec<(usize, u32)> = Vec::new();
    let mut add = |oi: usize| {
        let o = &ws.objects[oi];
        if let (ObjKind::Codeunit, Some(id)) = (o.kind, o.id)
            && !ws.is_dependency(oi)
            && !skip(oi)
            && !roots.contains(&(o.app, id))
        {
            roots.push((o.app, id));
        }
    };
    for oi in 0..ws.objects.len() {
        if ws.objects[oi].kind == ObjKind::Codeunit
            && !ws.is_dependency(oi)
            && ws.object_ast(oi).properties().any(|p| {
                p.name().eq_ignore_ascii_case("Subtype")
                    && (p.value_text().eq_ignore_ascii_case("Install")
                        || p.value_text().eq_ignore_ascii_case("Upgrade"))
            })
        {
            add(oi);
        }
    }
    for (oi, _) in ws
        .extension_triggers_subscribers()
        .into_iter()
        .chain(ws.company_triggers_subscribers())
    {
        add(oi);
    }
    if roots.is_empty() {
        return false;
    }
    // What the install code names, directly or through the objects so named.
    lean_named(ws, &roots)
        .into_iter()
        .enumerate()
        .any(|(oi, named)| named && !ws.is_dependency(oi) && runs_by_computed_id(ws, oi))
}

/// Whether the text of object `oi` has a `Codeunit.Run(` whose first argument is neither an integer literal nor
/// `Codeunit::Name`.
fn runs_by_computed_id(ws: &Workspace, oi: usize) -> bool {
    let tokens: Vec<(SyntaxKind, String)> = ws
        .object_ast(oi)
        .syntax()
        .descendants_with_tokens()
        .filter_map(|e| e.into_token())
        .filter(|t| !t.kind().is_trivia())
        .map(|t| (t.kind(), t.text().to_ascii_lowercase()))
        .collect();
    let word = |i: usize, w: &str| tokens.get(i).is_some_and(|(_, t)| t == w);
    (0..tokens.len()).any(|i| {
        word(i, "codeunit")
            && word(i + 1, ".")
            && word(i + 2, "run")
            && word(i + 3, "(")
            && tokens
                .get(i + 4)
                .is_some_and(|(k, _)| *k != SyntaxKind::Integer && !(word(i + 4, "codeunit") && word(i + 5, "::")))
    })
}
