//! The order in which the report extensions that change a report's run apply: the one owner of that order (dataset, triggers,
//! Report.WordXmlPart).
//!
//! Pinned:
//! - one app: object id order (corpus reports/extensions/TestReportExtensionBaseMembersContracts.al:256, ADR-025);
//! - apps that a dependency path orders: dependency order, where the object ids rise along the path (al-runner issue 6653;
//!   reports/extensions/Y64XappRunOrder.al: A <- B <- C <- the test app);
//! - two apps with no dependency path between them: app B before app D, whatever their extensions' object ids and names
//!   (reports/extensions/IndependentAppsReportExtensions.al:93-102 and :118-127, al-runner issue 7358; the two reports swap
//!   the order of the extension ids and names).
//!
//! Assumed (assumption `report-ext-independent-app-order`, a flagged pass until the corpus probe lands): two apps with no
//! dependency path apply by app id where the app name sorts the same way. The one measured pair does not tell that rule
//! from the publisher (descending), the publish order (descending) or the SQL order of the app id (descending), which
//! all put B first (al-language-xapp-b and -d app.json; the corpus CI publishes D before B). The reporter's own case needs
//! an order (al-runner issue 7358).
//!
//! Everything else is refused: other apps on a path with ids against it, more than two apps where one pair has no path, and two
//! apps without a path whose app id and app name sort differently.

use crate::workspace::Workspace;
use al_builtins::assumptions::Assumption;
use std::cmp::Ordering;

/// The report extensions in the order they apply, and the unprobed assumptions that order rests on.
pub(crate) struct ExtOrder {
    pub(crate) exts: Vec<usize>,
    pub(crate) assumes: Vec<Assumption>,
}

/// The report extensions `exts` in the order they apply, or why that order is not pinned.
pub(crate) fn report_extension_order(ws: &Workspace, base: usize, exts: &[usize]) -> Result<ExtOrder, String> {
    let pinned = |exts| ExtOrder {
        exts,
        assumes: Vec::new(),
    };
    let mut by_id = exts.to_vec();
    by_id.sort_by_key(|&e| (ws.objects[e].id, e));
    if by_id.len() < 2 {
        return Ok(pinned(by_id));
    }
    let refused =
        || "reportextensions of different apps change the run (their order across apps is not pinned)".to_string();
    if by_id.iter().any(|&e| ws.objects[e].id.is_none()) {
        return Err(refused());
    }
    let app = |e: usize| ws.objects[e].app;
    let along_path = by_id
        .windows(2)
        .all(|w| app(w[0]) == app(w[1]) || ws.app_sees(app(w[1]), app(w[0])));
    if along_path {
        return Ok(pinned(by_id));
    }
    if let [consumer, owner] = by_id[..]
        && app(consumer) != app(owner)
        && crate::reports::direct_report_dependency(ws, consumer, owner)
        && modifies_dependency_item(ws, base, consumer, owner)
    {
        return Ok(pinned(vec![owner, consumer]));
    }
    let mut apps: Vec<usize> = by_id.iter().map(|&e| app(e)).collect();
    apps.sort_unstable();
    apps.dedup();
    let [a, b] = apps[..] else {
        return Err(refused());
    };
    if ws.app_sees(a, b) || ws.app_sees(b, a) {
        return Err(refused());
    }
    let first = match independent_app_order(ws, a, b) {
        Some(Ordering::Less) => a,
        Some(Ordering::Greater) => b,
        _ => return Err(refused()),
    };
    by_id.sort_by_key(|&e| (app(e) != first, ws.objects[e].id, e));
    Ok(ExtOrder {
        exts: by_id,
        assumes: vec![Assumption::ReportExtIndependentAppOrder],
    })
}

/// al-runner issue 7878 and corpus PR #1342: a consumer with a lower ID modifies the owner's item.
fn modifies_dependency_item(ws: &Workspace, base: usize, consumer: usize, owner: usize) -> bool {
    let consumer_layout = crate::reports::ext_layout(&ws.object_ast(consumer));
    crate::reports::dependency_dataset_items(ws, consumer, base)
        .iter()
        .any(|(source, _, item)| {
            *source == owner
                && item.parent.is_none()
                && consumer_layout.modifies.iter().any(|(name, sec)| {
                    name.eq_ignore_ascii_case(&item.name)
                        && sec.properties().next().is_none()
                        && sec
                            .triggers()
                            .all(|t| t.name().eq_ignore_ascii_case("OnAfterAfterGetRecord"))
                })
        })
}

/// How app `a` sorts against app `b` when no dependency path orders them: by app id, pinned only where the app name sorts
/// the same way (the module comment).
fn independent_app_order(ws: &Workspace, a: usize, b: usize) -> Option<Ordering> {
    let (x, y) = (ws.apps.get(a)?, ws.apps.get(b)?);
    let by_id = x.id.to_lowercase().cmp(&y.id.to_lowercase());
    let by_name = x.name.to_lowercase().cmp(&y.name.to_lowercase());
    (by_id == by_name && by_id != Ordering::Equal).then_some(by_id)
}
