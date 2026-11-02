//! The "Page Action" rows of a page's `area(Promoted)`. Pinned: no row is named after an actionref or after the area, and the
//! action an actionref names has its one row in the area that declares it (pages/lt-y72-page-probes/Y72PageActionControlRows.al
//! header, PageAction_FilterByName_ActionRef `Tools` 1, `Tools_Promoted` empty, `Promoted` empty; PageAction_ActionRef_Rows `rows
//! named *_Promoted` 0). Which other rows the area adds is not pinned: a page that declares no group there lists the groups
//! Category_New to Category_Category20 and rows without a name (record/TestPageActionVirtualTable.al QZ_PageAction_CaptionColumn),
//! and a group the area declares has a row of its own (Y72 `Category_Process` 1, type ActionGroup, caption 'Task'). So the area
//! answers a lookup by a name it cannot hold (none) and refuses one by a name it may hold ('', a group's or action's name, a
//! Category_ name) and every read of another shape.

use super::mat::Entry;
use super::pos;
use al_builtins::value::Value;
use al_data::TableDef;
use al_ir::Object;

/// Action IDs of the stand-in rows start here, far above the ids of the real rows (the numbers are not pinned).
const STAND_IN_ID: i32 = 1_000_000;

/// The stand-in rows of area `n` (`area(Promoted)`) of `page`: one per name a row of the area may have, each seen only by a lookup
/// by that name ([`Entry::by_name_only`]), which refuses.
pub(super) fn promoted_rows(
    o: &Object,
    page: &al_ir::Page,
    area: &al_ir::ActionArea,
    n: usize,
    t: &TableDef,
    key: &al_data::Row,
) -> Result<Vec<Entry>, String> {
    use al_ir::ControlKind as K;
    let mut names: Vec<String> = vec![
        String::new(),
        "Category_New".into(),
        "Category_Process".into(),
        "Category_Report".into(),
    ];
    names.extend((4..=20).map(|i| format!("Category_Category{i}")));
    let mut stack: Vec<usize> = area.roots.clone();
    while let Some(ci) = stack.pop() {
        let ctl = &page.controls[ci];
        match ctl.kind {
            K::Action if ctl.action_ref.is_some() => {}
            K::Action | K::ActionGroup => names.push(ctl.name.clone()),
            _ => {
                return Err(format!(
                    "page '{}': the action '{}' of the Promoted area is a {:?}, whose row is not pinned",
                    o.name, ctl.name, ctl.kind
                ));
            }
        }
        stack.extend(
            page.controls
                .iter()
                .enumerate()
                .filter(|(_, k)| k.parent == Some(ci))
                .map(|(i, _)| i),
        );
    }
    let why = format!(
        "page '{}': the rows of its Promoted area other than its actionrefs are not pinned (their ids, types and which exist)",
        o.name
    );
    let base = STAND_IN_ID + 1_000 * n as i32;
    Ok(names
        .into_iter()
        .enumerate()
        .map(|(i, name)| {
            let mut r = key.clone();
            r[pos(t, 2)] = Value::Integer(base + i as i32);
            r[pos(t, 3)] = Value::Integer(base + i as i32);
            r[pos(t, 8)] = Value::Text(super::objects::truncate(&name, 120).into());
            Entry::ok(r).in_name_set(pos(t, 8), vec![pos(t, 2), pos(t, 3)], why.clone())
        })
        .collect())
}
