//! A page extension anchors on promoted parts of the base page that the base does not declare (al-runner issues 6646 and
//! 6647). alc compiles (corpus writer Y64's compile facts, alc 30 with BC 29 symbols):
//! - every page has the groups `Category_Category4` to `Category_Category20`; `modify` of one may set Caption, Visible or
//!   ShowAs, also on a page without actions;
//! - an action promoted the legacy way (`Promoted = true`) has the actionref `<action>_Promoted`; `addafter` and `addbefore`
//!   anchor on it, `modify` may set Visible.
//!
//! A run of the extension's action and actionref from a TestPage is pinned for `addafter` and `addlast` of an undeclared
//! category (pages/lt-up-page-ext/UpPromotedCategories.al, groups 7 and 12), and by corpus writer Y64 (branch
//! al-runtime-rs-staging-y64 85d86e7b, pages/lt-up-page-ext/Y64PromotedImplicit.al:229-308) for modify and addlast of groups 5 and 6
//! and addafter/addbefore of `<action>_Promoted`. Here the group and the implicit actionref are anchors only, so a Caption or
//! ShowAs on them is not seen by AL code. Reading the implicit actionref from a TestPage and setting its Visible (Y64 pins both)
//! are not supported yet and stay refused, as do the forms nothing pins.

use al_sema::workspace::{AppKind, AppSource};
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const BASE: &str = r#"
table 50100 "IP Entry" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { Clustered = true; } } }
codeunit 50104 "IP Log" { SingleInstance = true; var L: Text; procedure Add(E: Text) begin L += E; end; procedure Take(): Text begin exit(L); end; }
page 50101 "IP Card"
{
    PageType = Card;
    SourceTable = "IP Entry";
    layout { area(Content) { field("No."; Rec."No.") { } } }
    actions
    {
        area(Processing)
        {
            action("My Action")
            {
                Promoted = true;
                PromotedCategory = Process;
                PromotedOnly = true;
                trigger OnAction() var Log: Codeunit "IP Log"; begin Log.Add('base;'); end;
            }
        }
    }
}
"#;

fn objects(anchors: &str) -> String {
    format!(
        r#"
pageextension 50102 "IP Card Ext" extends "IP Card"
{{
    actions
    {{
        addlast(Processing)
        {{
            action("My New Action") {{ trigger OnAction() var Log: Codeunit "IP Log"; begin Log.Add('new;'); end; }}
        }}
        {anchors}
    }}
}}
codeunit 50103 T
{{
    Subtype = Test;

    [Test]
    procedure P()
    var
        Card: TestPage "IP Card";
        Log: Codeunit "IP Log";
        R: Record "IP Entry";
    begin
        R."No." := 'A';
        R.Insert();
        Card.OpenView();
        Card."My New Action".Invoke();
        Card."My New Action_Promoted".Invoke();
        Card."My Action".Invoke();
        Card.Close();
        if Log.Take() <> 'new;new;base;' then
            Error('log %1', Log.Take());
    end;
}}"#
    )
}

fn run(anchors: &str) -> Vec<(String, Outcome)> {
    let mut sources = Vec::new();
    for (id, deps, src) in [
        ("base", vec![], BASE.to_string()),
        ("ext", vec!["base".to_string()], objects(anchors)),
    ] {
        sources.push(AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps,
            files: vec![(format!("{id}.al"), src)],
        });
    }
    let (ws, l) = al_sema::lower::compile_apps(
        sources,
        vec![AppKind::Dependency, AppKind::Workspace],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    run_codeunit(&Engine::new(l.program.clone(), blocked), &l, oi)
}

const REF: &str = r#"{ actionref("My New Action_Promoted"; "My New Action") { } }"#;

#[test]
fn issue_6646_modify_and_add_to_an_undeclared_category_group() {
    for n in [4, 5, 9, 20] {
        let anchors = format!(
            "modify(Category_Category{n}) {{ Caption = 'My Category'; ShowAs = SplitButton; }} addlast(Category_Category{n}) {REF}"
        );
        let results = run(&anchors);
        assert!(!results.is_empty());
        for (name, outcome) in results {
            assert_eq!(outcome, Outcome::Pass, "{n}: {name}");
        }
    }
    for anchors in [
        format!("addafter(Category_Category5) {REF}"),
        format!("addlast(Category_Category20) {REF}"),
    ] {
        assert!(run(&anchors).iter().all(|(_, o)| *o == Outcome::Pass), "{anchors}");
    }
}

#[test]
fn issue_6647_add_after_the_implicit_actionref_of_a_legacy_promoted_action() {
    for kind in ["addafter", "addbefore"] {
        let results = run(&format!(r#"{kind}("My Action_Promoted") {REF}"#));
        assert!(!results.is_empty());
        for (name, outcome) in results {
            assert_eq!(outcome, Outcome::Pass, "{kind}: {name}");
        }
    }
}

#[test]
fn unpinned_uses_of_implicit_promoted_parts_stay_refused() {
    for anchors in [
        // Visible of an implicit group or actionref: what the run shows is not pinned.
        format!("modify(Category_Category5) {{ Visible = false; }} addlast(Category_Category5) {REF}"),
        format!(r#"modify("My Action_Promoted") {{ Visible = false; }} addlast(Category_Process) {REF}"#),
        // Not a group every page has (alc: AL0271).
        format!("addlast(Category_Category3) {REF}"),
        format!("addlast(Category_Category21) {REF}"),
    ] {
        let results = run(&anchors);
        assert!(
            matches!(&results[..], [(_, Outcome::NotRun(_))]),
            "{anchors}: {results:?}"
        );
    }
}
