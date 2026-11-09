//! `access_by_permission_refusals`: which AccessByPermission shapes a restricted session refuses at page open (ADR-062 probe P21).

use crate::workspace::{AppSource, Workspace};

/// The restricted-session refusals of page `page` in one app holding `src`.
fn refusals(src: &str, page: &str) -> Vec<String> {
    let ws = Workspace::load(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src.into())],
        }],
        &Default::default(),
    );
    let members = crate::members::build(&ws);
    let checked = crate::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
    let lowered = super::lower_with_positions(&ws, &members, &checked, false);
    let o = lowered.program.objects.iter().find(|o| o.name == page).expect("page");
    o.page.as_ref().expect("a page").restricted_refusals.clone()
}

const OBJECTS: &str = r#"
table 50100 "Row" { fields { field(1; Id; Integer) { } } }
table 50101 "Guard" { fields { field(1; Id; Integer) { } } }
"#;

/// Y94AbpActionRef.al Y94_Ref_GrantsA, Y90PermActionRef.al Y90_ActionRef_*: an action listed again with actionref in area(Promoted)
/// is decided by its own guard, the actionref changes nothing the TestPage sees (the page opens; the action is off without the guard).
#[test]
fn an_actionref_to_a_guarded_action_opens() {
    let src = format!(
        r#"{OBJECTS}
page 50102 "Card" {{
    SourceTable = "Row";
    actions {{
        area(Processing) {{
            action(GuardAct) {{ AccessByPermission = tabledata "Guard" = R; trigger OnAction() begin end; }}
            group(Grp) {{ action(InGrpAct) {{ AccessByPermission = tabledata "Guard" = R; trigger OnAction() begin end; }} }}
        }}
        area(Promoted) {{
            actionref(GuardRef; GuardAct) {{ }}
            group(Category_Process) {{ actionref(InGrpRef; InGrpAct) {{ }} }}
        }}
    }}
}}"#
    );
    assert_eq!(refusals(&src, "Card"), Vec::<String>::new());
}

/// Y94AbpPageLevel.al Y94_PageLevel_GrantsA: the property on the page itself next to a promoted guarded action.
#[test]
fn a_page_level_guard_with_a_promoted_guarded_action_opens() {
    let src = format!(
        r#"{OBJECTS}
page 50102 "Lvl" {{
    SourceTable = "Row";
    AccessByPermission = tabledata "Guard" = R;
    actions {{
        area(Processing) {{ action(GuardAct) {{ AccessByPermission = tabledata "Guard" = R; trigger OnAction() begin end; }} }}
        area(Promoted) {{ group(Category_Process) {{ actionref(GuardRef; GuardAct) {{ }} }} }}
    }}
}}"#
    );
    assert_eq!(refusals(&src, "Lvl"), Vec::<String>::new());
}

/// An actionref that states a guard of its own is pinned by no corpus test: it refuses, and the reason names the control.
#[test]
fn an_actionref_with_a_guard_of_its_own_refuses() {
    let src = format!(
        r#"{OBJECTS}
page 50102 "Own" {{
    SourceTable = "Row";
    actions {{
        area(Processing) {{ action(PlainAct) {{ trigger OnAction() begin end; }} }}
        area(Promoted) {{ actionref(PlainRef; PlainAct) {{ AccessByPermission = tabledata "Guard" = R; }} }}
    }}
}}"#
    );
    let r = refusals(&src, "Own");
    assert_eq!(r.len(), 1, "{r:?}");
    assert!(r[0].contains("PlainRef"), "{r:?}");
}
