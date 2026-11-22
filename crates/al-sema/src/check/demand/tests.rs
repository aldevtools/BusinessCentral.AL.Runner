//! Tests of the demand-driven check (`demand.rs`).

use crate::lower::{Lowered, compile_apps};

/// One pass over an object's text finds the same names as one `text_contains_ignore_case` per name.
#[test]
fn one_pass_finds_each_name_as_a_search_per_name_does() {
    let src = "codeunit 50100 X { procedure P() var r: Record Item; begin r.SETFILTER(\"No.\", 'a'); \
                   // Rendering Custom\n end; }\npage 50101 Y { CaptionClass = '1'; }\ncodeunit 50102 Z { }";
    let file = al_syntax::parse(src, &Default::default());
    let objects: Vec<_> = file
        .syntax()
        .children()
        .filter_map(al_syntax::ast::Object::cast)
        .collect();
    assert_eq!(objects.len(), 3);
    for o in &objects {
        let got = super::mentions(o.syntax());
        for (bit, name) in super::MENTION_NAMES {
            assert_eq!(
                got & bit != 0,
                al_syntax::ast::text_contains_ignore_case(o.syntax(), name),
                "{name} in {}",
                o.syntax()
            );
        }
    }
    assert_eq!(
        super::mentions(objects[0].syntax()),
        super::SET_FILTER | super::RENDERING | super::CUSTOM
    );
    assert_eq!(super::mentions(objects[1].syntax()), super::CAPTION_CLASS);
    assert_eq!(super::mentions(objects[2].syntax()), 0);
}
use crate::workspace::{AppKind, AppSource, Workspace};

/// A workspace app `main` over a dependency app `dep`, compiled with demand-driven dependencies.
fn compile(main: &str, dep: &[(&str, &str)]) -> (Workspace, Lowered) {
    let app = |id: &str, deps: &[&str], files: Vec<(String, String)>| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files,
    };
    compile_apps(
        vec![
            app("main", &["dep"], vec![("main.al".into(), main.into())]),
            app(
                "dep",
                &[],
                dep.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
            ),
        ],
        vec![AppKind::Workspace, AppKind::Dependency],
        &Default::default(),
    )
}

fn object(l: &Lowered, name: &str) -> usize {
    l.program
        .objects
        .iter()
        .position(|o| o.name == name)
        .unwrap_or_else(|| panic!("no object {name}"))
}

/// The reasons routine `name` of object `obj` cannot run (empty: it runs).
fn reasons(l: &Lowered, obj: &str, name: &str) -> Vec<String> {
    let o = object(l, obj);
    let r = l.program.objects[o]
        .routines
        .iter()
        .position(|r| r.name == name)
        .unwrap_or_else(|| panic!("no routine {name}"));
    l.unsupported[o][r].iter().map(|r| r.to_string()).collect()
}

fn compiled(l: &Lowered, obj: &str, name: &str) -> bool {
    reasons(l, obj, name).iter().all(|r| !r.contains("not compiled"))
}

/// The program, the refusals, the call graph and the diagnostics of a compile of `main` over
/// `dep` with `workers` check threads.
fn compile_with_workers(main: &str, dep: &[(String, String)], workers: usize) -> String {
    compile_with_both(main, dep, workers, workers)
}

/// Like `compile_with_workers`, with `check` threads checking and `lower` threads lowering.
fn compile_with_both(main: &str, dep: &[(String, String)], check: usize, lower: usize) -> String {
    let app = |id: &str, deps: &[&str], files: Vec<(String, String)>| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files,
    };
    let roots = crate::check::Roots {
        workers: Some(check),
        ..Default::default()
    };
    let ws = Workspace::load_apps(
        vec![
            app("main", &["dep"], vec![("main.al".into(), main.into())]),
            app("dep", &[], dep.to_vec()),
        ],
        vec![AppKind::Workspace, AppKind::Dependency],
        &Default::default(),
    );
    let members = crate::members::build(&ws);
    let checked = crate::check::check_with(&ws, &members, &al_builtins::catalog::Catalog::load(), roots);
    let l = crate::lower::lower_with_workers(&ws, &members, &checked, true, lower);
    format!(
        "{}\n{:?}\n{:?}\n{:?}",
        serde_json::to_string(&*l.program).unwrap(),
        l.unsupported,
        l.calls,
        ws.diagnostics
    )
}

#[test]
fn checking_on_several_threads_compiles_the_same_program() {
    // 80 libraries reached in one round (more than a round needs to be spread over threads),
    // each calling a helper in the next round, one of which cannot run.
    let n = 80;
    let main = format!(
        "codeunit 1 Main {{ procedure Go() var {} begin {} end; }}",
        (0..n).map(|i| format!("L{i}: Codeunit Lib{i};")).collect::<String>(),
        (0..n).map(|i| format!("L{i}.Entry();")).collect::<String>()
    );
    let dep: Vec<(String, String)> = (0..n)
        .map(|i| {
            let broken = if i % 7 == 0 { "T := NoSuchBuiltin(1);" } else { "" };
            (
                format!("lib{i}.al"),
                format!(
                    "codeunit {} Lib{i} {{ procedure Entry() begin Helper(); end; \
                         local procedure Helper() var T: Text; begin {broken} end; \
                         procedure Unused() begin end; }}",
                    100 + i
                ),
            )
        })
        .collect();
    let one = compile_with_workers(&main, &dep, 1);
    assert!(one.contains("Helper"), "the fixture compiles something");
    assert_eq!(one, compile_with_workers(&main, &dep, 4));
    assert_eq!(one, compile_with_workers(&main, &dep, 7));
    // Lowering the dependency objects on several threads does not change the program either.
    assert_eq!(one, compile_with_both(&main, &dep, 1, 5));
}

#[test]
fn only_the_dependency_routines_the_workspace_calls_are_compiled() {
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var D: Codeunit Lib; begin D.Used(); end; }",
        &[(
            "lib.al",
            "codeunit 10 Lib { procedure Used() begin Inner(); end; local procedure Inner() begin end; procedure Unused() begin Other(); end; local procedure Other() begin end; }",
        )],
    );
    assert!(compiled(&l, "Lib", "Used"));
    assert!(compiled(&l, "Lib", "Inner"), "what a reached routine calls is reached");
    assert!(!compiled(&l, "Lib", "Unused"));
    assert!(!compiled(&l, "Lib", "Other"));
    // The stub says why it cannot run.
    assert!(reasons(&l, "Lib", "Unused")[0].contains("not compiled"));
}

#[test]
fn a_dependency_report_request_page_expression_is_reached_by_a_test_request_page() {
    // reports/TestReqPageBoundVisibility.al:172-188: the handler reads `.Visible()` of a control
    // whose `Visible = FlagVisible;` is a synthetic `__visible_N` routine of Base App's report.
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var R: TestRequestPage \"Lib Rep\"; begin end; }",
        &[(
            "lib.al",
            "report 10 \"Lib Rep\" { requestpage { layout { area(content) { field(Name; Nm) { Visible = FlagVisible; } } } } var Nm: Text; FlagVisible: Boolean; }",
        )],
    );
    let o = object(&l, "Lib Rep");
    let synthetic: Vec<_> = l.program.objects[o]
        .routines
        .iter()
        .filter(|r| r.name.starts_with("__visible_"))
        .map(|r| r.name.clone())
        .collect();
    assert_eq!(
        synthetic.len(),
        1,
        "{:?}",
        l.program.objects[o]
            .routines
            .iter()
            .map(|r| &r.name)
            .collect::<Vec<_>>()
    );
    assert!(compiled(&l, "Lib Rep", &synthetic[0]));
}

#[test]
fn an_unsupported_dependency_routine_nothing_reaches_does_not_block_anything() {
    // `Rare` uses a builtin that does not exist; nothing reaches it, so nothing is refused.
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var D: Codeunit Lib; begin D.Fine(); end; }",
        &[(
            "lib.al",
            "codeunit 10 Lib { procedure Fine() begin end; procedure Rare() var T: Text; begin T := NoSuchBuiltin(1); end; }",
        )],
    );
    assert!(reasons(&l, "Lib", "Fine").is_empty());
    assert!(!compiled(&l, "Lib", "Rare"));
    assert!(reasons(&l, "Main", "Go").is_empty(), "{:?}", reasons(&l, "Main", "Go"));
}

#[test]
fn a_reached_routine_that_cannot_run_blocks_what_reaches_it() {
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var D: Codeunit Lib; begin D.Rare(); end; }",
        &[(
            "lib.al",
            "codeunit 10 Lib { procedure Rare() var T: Text; begin T := NoSuchBuiltin(1); end; }",
        )],
    );
    assert!(compiled(&l, "Lib", "Rare"));
    assert!(!reasons(&l, "Lib", "Rare").is_empty());
    let go = l.program.objects[object(&l, "Main")]
        .routines
        .iter()
        .position(|r| r.name == "Go")
        .unwrap();
    let lib = object(&l, "Lib");
    let rare = l.program.objects[lib]
        .routines
        .iter()
        .position(|r| r.name == "Rare")
        .unwrap();
    assert!(l.calls[object(&l, "Main")][go].contains(&(lib, rare)));
}

#[test]
fn objects_nothing_refers_to_are_not_compiled_and_stay_out_of_reach() {
    let (ws, l) = compile(
        "codeunit 1 Main { procedure Go() var R: Record Used; begin R.FindFirst(); end; }",
        &[
            ("used.al", "table 10 Used { fields { field(1; K; Code[10]) { } } }"),
            (
                "other.al",
                "table 11 Other { fields { field(1; K; Code[10]) { } } trigger OnInsert() begin end; }",
            ),
        ],
    );
    let used = &l.program.objects[object(&l, "Used")];
    let other = &l.program.objects[object(&l, "Other")];
    assert!(used.table.is_some());
    assert!(
        other.table.is_none() && other.routines.is_empty(),
        "an object nothing refers to is a shell"
    );
    // The workspace no longer resolves it either: code that needs it after all is refused.
    let fi = ws.files.iter().position(|f| f.path == "main.al").unwrap();
    assert_eq!(
        ws.resolve_ref(fi, crate::workspace::ObjKind::Table, &["Other".to_string()]),
        crate::workspace::Resolution::Missing
    );
}

/// `main` over the platform app (the table `Field`) and an app with the table `Other`.
fn compile_with_catalog(main: &str) -> (Workspace, Lowered) {
    let app = |id: &str, deps: &[&str], files: Vec<(&str, &str)>| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: files.into_iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
    };
    compile_apps(
        vec![
            app(
                "main",
                &[crate::workspace::PLATFORM_APP_ID, "dep"],
                vec![("main.al", main)],
            ),
            app(
                crate::workspace::PLATFORM_APP_ID,
                &[],
                vec![(
                    "field.al",
                    "table 2000000041 \"Field\" { fields { field(1; TableNo; Integer) { } field(7; \"Type\"; Integer) { } } }",
                )],
            ),
            app(
                "dep",
                &[],
                vec![("other.al", "table 11 Other { fields { field(1; K; Code[10]) { } } }")],
            ),
        ],
        vec![AppKind::Workspace, AppKind::Dependency, AppKind::Dependency],
        &Default::default(),
    )
}

/// Using the platform table `Field` as a table answers for every table of the installed apps, so
/// their shapes are compiled. A record variable that only has its fields assigned and read (Library
/// Assert's TypeNameOf) reads no row and needs none of them.
#[test]
fn a_catalog_table_used_as_a_table_compiles_every_table_shape() {
    let (_, l) = compile_with_catalog(
        "codeunit 1 Main { procedure Go() var F: Record \"Field\"; begin F.SetRange(TableNo, 18); end; }",
    );
    assert!(l.program.objects[object(&l, "Other")].table.is_some());
}

#[test]
fn a_catalog_table_variable_used_only_through_its_fields_compiles_no_other_table() {
    let (_, l) = compile_with_catalog(
        "codeunit 1 Main { procedure Go(): Integer var F: Record \"Field\"; begin F.\"Type\" := 3; exit(F.\"Type\"); end; }",
    );
    assert!(
        l.program.objects[object(&l, "Other")].table.is_none(),
        "no row of Field is read, so no table's shape is needed"
    );
}

/// A trigger of a page in a dependency app calls a procedure of the page its part shows
/// (`CurrPage.PurchLines.Page.ForceTotalsCalculation()` in Base Application's Purchase Invoice): the part's
/// page is reached with the page that names it. Pinned in the workspace by testpage/TestPageTempPart_Host.al
/// (`CurrPage.Lines.Page.SetRows`). The part's page has a group named `Page` (Base Application's Purch.
/// Invoice Subform does): `.Page` is still the part's page, not that control.
#[test]
fn a_page_part_s_procedure_called_from_a_dependency_page_is_compiled() {
    let dep = [
        (
            "t.al",
            "table 10 T { fields { field(1; K; Integer) { } field(2; V; Integer) { } } keys { key(PK; K) { } } }",
        ),
        (
            "sub.al",
            "namespace Ns.Docs; page 12 \"Sub\" { PageType = ListPart; SourceTable = T; layout { area(Content) { repeater(R) { field(K; Rec.K) { } } group(\"Page\") { } } } \
                 procedure Force() begin end; }",
        ),
        (
            "host.al",
            "namespace Ns.Docs; page 11 \"Host\" { PageType = Card; SourceTable = T; layout { area(Content) { field(V; Rec.V) { \
                 trigger OnValidate() begin AfterValid(); end; } \
                 part(Lines; \"Sub\") { ApplicationArea = Basic, Suite; Editable = IsEditable; Enabled = IsEditable; SubPageLink = K = field(K); UpdatePropagation = Both; } } } \
                 var IsEditable: Boolean; local procedure AfterValid() begin CurrPage.Lines.Page.Force(); end; }",
        ),
    ];
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var TP: TestPage \"Host\"; begin TP.OpenEdit(); TP.V.SetValue(1); end; }",
        &dep,
    );
    let why = reasons(&l, "Host", "AfterValid");
    assert!(why.is_empty(), "{why:?}");
}

#[test]
fn reading_a_table_runs_no_trigger_and_writing_runs_the_trigger_of_that_operation() {
    let dep = [(
        "t.al",
        "table 10 T { fields { field(1; K; Code[10]) { } } \
             trigger OnInsert() begin end; trigger OnModify() begin end; trigger OnDelete() begin end; }",
    )];
    let (_, read) = compile(
        "codeunit 1 Main { procedure Go() var R: Record T; begin R.FindFirst(); end; }",
        &dep,
    );
    for t in ["OnInsert", "OnModify", "OnDelete"] {
        assert!(!compiled(&read, "T", t), "{t} must not run when the table is only read");
    }
    let (_, write) = compile(
        "codeunit 1 Main { procedure Go() var R: Record T; begin R.Insert(true); end; }",
        &dep,
    );
    assert!(compiled(&write, "T", "OnInsert"));
    assert!(
        !compiled(&write, "T", "OnModify"),
        "only the operation used runs its trigger"
    );
    assert!(!compiled(&write, "T", "OnDelete"));
}

#[test]
fn writing_a_table_reaches_its_event_subscribers_for_that_operation_only() {
    let dep = [
        ("t.al", "table 10 T { fields { field(1; K; Code[10]) { } } }"),
        (
            "s.al",
            "codeunit 20 Subs {\n\
                 [EventSubscriber(ObjectType::Table, Database::T, 'OnAfterInsertEvent', '', false, false)]\n\
                 local procedure OnIns() begin end;\n\
                 [EventSubscriber(ObjectType::Table, Database::T, 'OnAfterModifyEvent', '', false, false)]\n\
                 local procedure OnMod() begin end; }",
        ),
    ];
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var R: Record T; begin R.Insert(); end; }",
        &dep,
    );
    assert!(compiled(&l, "Subs", "OnIns"));
    assert!(!compiled(&l, "Subs", "OnMod"));
    // Reading only: neither subscriber is reached.
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var R: Record T; begin R.FindFirst(); end; }",
        &dep,
    );
    assert!(!compiled(&l, "Subs", "OnIns"));
}

#[test]
fn a_reached_event_publisher_reaches_the_subscribers_of_that_event() {
    let dep = [
        (
            "p.al",
            "codeunit 10 Pub { procedure Do() begin OnThing(); OnOther(); end;\n\
                 [IntegrationEvent(false, false)] local procedure OnThing() begin end;\n\
                 [IntegrationEvent(false, false)] local procedure OnOther() begin end; }",
        ),
        (
            "s.al",
            "codeunit 20 Subs {\n\
                 [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnThing', '', false, false)]\n\
                 local procedure A() begin end;\n\
                 [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnNever', '', false, false)]\n\
                 local procedure B() begin end; }",
        ),
    ];
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var P: Codeunit Pub; begin P.Do(); end; }",
        &dep,
    );
    assert!(compiled(&l, "Subs", "A"));
    assert!(!compiled(&l, "Subs", "B"), "the publisher never raises OnNever");
}

#[test]
fn a_write_anywhere_reaches_the_global_triggers_subscribers_of_that_operation() {
    // `Global Triggers` (2000000002) is a platform codeunit no app declares here; the runtime
    // raises its events around writes, so a dependency's subscribers to them are reached by a
    // write to any table, the workspace's own included.
    let dep = [(
        "s.al",
        "codeunit 20 Subs {\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'GetDatabaseTableTriggerSetup', '', false, false)]\n\
             local procedure Setup(TableId: Integer; var OnDatabaseInsert: Boolean; var OnDatabaseModify: Boolean; var OnDatabaseDelete: Boolean; var OnDatabaseRename: Boolean) begin end;\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseInsert', '', false, false)]\n\
             local procedure Ins(RecRef: RecordRef) begin end;\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseModify', '', false, false)]\n\
             local procedure Mod(RecRef: RecordRef) begin end; }",
    )];
    let (_, l) = compile(
        "table 5 Mine { fields { field(1; K; Code[10]) { } } }\n\
             codeunit 1 Main { procedure Go() var R: Record Mine; begin R.Insert(); end; }",
        &dep,
    );
    assert!(compiled(&l, "Subs", "Setup"));
    assert!(compiled(&l, "Subs", "Ins"));
    assert!(
        !compiled(&l, "Subs", "Mod"),
        "only the operations used raise their events"
    );
    let (_, l) = compile(
        "table 5 Mine { fields { field(1; K; Code[10]) { } } }\n\
             codeunit 1 Main { procedure Go() var R: Record Mine; begin R.FindFirst(); end; }",
        &dep,
    );
    assert!(
        l.program.objects[object(&l, "Subs")].routines.is_empty(),
        "reading raises nothing, so the subscribers are not compiled"
    );
}

#[test]
fn an_interface_used_reaches_the_codeunits_that_implement_it() {
    let dep = [
        ("i.al", "interface IThing { procedure Run(); }"),
        (
            "c.al",
            "codeunit 10 Impl implements IThing { procedure Run() begin Helper(); end; local procedure Helper() begin end; procedure Extra() begin end; }",
        ),
    ];
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var I: Interface IThing; begin I.Run(); end; }",
        &dep,
    );
    assert!(compiled(&l, "Impl", "Run"));
    assert!(compiled(&l, "Impl", "Helper"));
    assert!(
        !compiled(&l, "Impl", "Extra"),
        "only the interface's procedures are called through it"
    );
}

#[test]
fn a_table_related_to_a_used_one_is_lowered_with_it_but_runs_nothing() {
    let dep = [
        (
            "a.al",
            "table 10 A { fields { field(1; K; Code[10]) { } field(2; Ref; Code[10]) { TableRelation = B.K; } } }",
        ),
        (
            "b.al",
            "table 11 B { fields { field(1; K; Code[10]) { } } trigger OnInsert() begin end; }",
        ),
    ];
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var R: Record A; begin R.FindFirst(); end; }",
        &dep,
    );
    let b = &l.program.objects[object(&l, "B")];
    assert!(
        b.table.is_some(),
        "the related table is lowered so the relation resolves"
    );
    assert!(!compiled(&l, "B", "OnInsert"));
    let a = l.program.objects[object(&l, "A")].table.as_ref().unwrap();
    assert!(a.fields[1].relation_arms.iter().all(|arm| arm.table.is_some()));
}

#[test]
fn a_dependency_app_without_a_reference_from_the_workspace_compiles_nothing() {
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() begin end; }",
        &[
            ("lib.al", "codeunit 10 Lib { procedure P() begin end; }"),
            ("t.al", "table 11 T { }"),
        ],
    );
    assert!(l.program.objects[object(&l, "Lib")].routines.is_empty());
    assert!(l.dependency[object(&l, "Lib")] && !l.dependency[object(&l, "Main")]);
}

const T_WITH_TRIGGERS: &str = "table 10 T { fields { field(1; K; Code[10]) { } field(2; V; Integer) { } } \
         trigger OnInsert() begin end; trigger OnModify() begin end; trigger OnDelete() begin end; \
         procedure Bump() begin V := 1; Modify(true); end; }";

#[test]
fn a_write_without_the_record_name_inside_table_code_runs_the_trigger_of_that_operation() {
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var R: Record T; begin R.Bump(); end; }",
        &[("t.al", T_WITH_TRIGGERS)],
    );
    assert!(
        compiled(&l, "T", "OnModify"),
        "`Modify(true)` inside Bump writes the table"
    );
    assert!(!compiled(&l, "T", "OnDelete"));
}

#[test]
fn a_workspace_page_over_a_dependency_table_runs_what_the_client_writes() {
    let (_, l) = compile(
        "page 5 Mine { PageType = Card; SourceTable = T; layout { area(Content) { field(K; Rec.K) { } } } }",
        &[("t.al", T_WITH_TRIGGERS)],
    );
    for t in ["OnInsert", "OnModify", "OnDelete"] {
        assert!(compiled(&l, "T", t), "{t}");
    }
}

#[test]
fn the_run_object_actions_of_a_workspace_page_run_their_targets() {
    let dep = [(
        "c.al",
        "codeunit 10 Lib { trigger OnRun() begin end; procedure Other() begin end; } \
             codeunit 11 Unused { trigger OnRun() begin end; }",
    )];
    let (_, l) = compile(
        "page 5 Mine { PageType = Card; layout { area(Content) { } } \
             actions { area(Processing) { action(A) { RunObject = Codeunit Lib; } } } }",
        &dep,
    );
    assert!(compiled(&l, "Lib", "OnRun"));
    assert!(!compiled(&l, "Lib", "Other"));
    assert!(l.program.objects[object(&l, "Unused")].routines.is_empty());
}

/// `reach_run_targets` walks a page once: a page named by several `TestPage` variables, in several
/// objects and on several check threads, still reaches what its actions run, and the program is the
/// one a single thread compiles.
#[test]
fn a_page_named_by_many_test_pages_reaches_its_run_targets_once_and_the_same() {
    let dep = vec![
        (
            "c.al".to_string(),
            "codeunit 10 Lib { trigger OnRun() begin end; procedure Other() begin end; } \
                 codeunit 11 Lib2 { trigger OnRun() begin end; }"
                .to_string(),
        ),
        (
            "p.al".to_string(),
            "page 20 P { PageType = Card; layout { area(Content) { } } \
                 actions { area(Processing) { action(A) { RunObject = Codeunit Lib; } \
                 action(B) { RunObject = Page Q; } } } } \
                 page 21 Q { PageType = Card; layout { area(Content) { } } \
                 actions { area(Processing) { action(C) { RunObject = Codeunit Lib2; } } } }"
                .to_string(),
        ),
    ];
    let main = (0..6)
        .map(|k| {
            format!(
                "codeunit {} M{k} {{ procedure A() var P: TestPage P; begin P.OpenView(); end; \
                     procedure B() var P1: TestPage P; P2: TestPage P; begin P1.OpenEdit(); P2.OpenView(); end; }}",
                k + 1
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let (_, l) = compile(
        &main,
        &dep.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect::<Vec<_>>(),
    );
    assert!(compiled(&l, "Lib", "OnRun"));
    assert!(!compiled(&l, "Lib", "Other"));
    // The page an action opens is driven through a variable of its own, so what its actions run is not reached.
    assert!(l.program.objects[object(&l, "Lib2")].routines.is_empty());
    let one = compile_with_workers(&main, &dep, 1);
    assert_eq!(one, compile_with_workers(&main, &dep, 4));
}

#[test]
fn a_page_the_client_opens_reaches_the_subscribers_of_the_events_it_raises_and_no_others() {
    let dep = [
        ("t.al", "table 10 T { fields { field(1; K; Code[10]) { } } }"),
        (
            "p.al",
            "page 20 P { PageType = Card; SourceTable = T; layout { area(Content) { field(K; Rec.K) { } } } }",
        ),
        (
            "s.al",
            "codeunit 30 Subs {\n\
                 [EventSubscriber(ObjectType::Page, Page::P, 'OnOpenPageEvent', '', false, false)]\n\
                 local procedure Opened() begin end;\n\
                 [EventSubscriber(ObjectType::Page, Page::P, 'OnDeleteRecordEvent', '', false, false)]\n\
                 local procedure Row(var Rec: Record T; var AllowDelete: Boolean) begin end; }",
        ),
    ];
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var P: TestPage P; begin P.OpenNew(); end; }",
        &dep,
    );
    assert!(compiled(&l, "Subs", "Opened"));
    assert!(
        !compiled(&l, "Subs", "Row"),
        "the client does not raise it; the page refuses to open"
    );
    // Both are declared: the one nobody compiled is on the program's list, the other is not.
    let events: Vec<&str> = l
        .program
        .uncompiled_subscribers
        .iter()
        .map(|u| u.event.as_str())
        .collect();
    assert!(events.is_empty(), "{events:?}: a stub is registered, not missing");
}

#[test]
fn caption_class_resolvers_are_reached_by_a_workspace_page_that_has_a_caption_class() {
    let resolver = (
        "r.al",
        "codeunit 30 Resolver {\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Caption Class\", 'OnResolveCaptionClass', '', false, false)]\n\
             local procedure R(CaptionArea: Text; CaptionExpr: Text; Language: Integer; var Caption: Text; var Resolved: Boolean) begin end; }",
    );
    let with = "table 5 Mine { fields { field(1; K; Code[10]) { } } }\n\
             page 5 Mine { PageType = Card; SourceTable = Mine; layout { area(Content) { \
             field(K; Rec.K) { CaptionClass = 'ZZ,x'; } } } }";
    let (_, l) = compile(with, &[resolver]);
    assert!(compiled(&l, "Resolver", "R"));
    let without = with.replace("CaptionClass = 'ZZ,x';", "");
    let (_, l) = compile(&without, &[resolver]);
    assert!(l.program.objects[object(&l, "Resolver")].routines.is_empty());
    // Not reached, yet declared: the page client refuses to raise the event.
    assert_eq!(l.program.uncompiled_subscribers.len(), 1);
    let u = &l.program.uncompiled_subscribers[0];
    assert_eq!(u.platform, al_ir::PlatformPublisher::CaptionClass);
    assert_eq!(u.event, "onresolvecaptionclass");
}

/// A dependency subscriber whose publisher is a bare id is reached as one written with the name is, or listed as not
/// compiled (al-runner issue 8466). A `CaptionClassTranslate` subscriber of a dependency is listed whatever code is
/// reached, except the System Application's Caption Class Impl. (by its app id: a twin of another app is listed).
/// Apps: `s` holds the subscribers, `p` the codeunits they name, `w` the code under test.
#[test]
fn a_dependency_subscriber_written_with_a_bare_id_is_reached_or_listed() {
    let subs = "codeunit 30 Subs {\n\
             [EventSubscriber(ObjectType::Codeunit, 10, 'OnX', '', false, false)] local procedure X() begin end;\n\
             [EventSubscriber(ObjectType::Codeunit, 2000000007, 'MakeDateFilter', '', false, false)]\n\
             local procedure F(var DateFilterText: Text) begin end;\n\
             [EventSubscriber(ObjectType::Codeunit, 2000000004, 'CaptionClassTranslate', '', false, false)]\n\
             local procedure U(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024]) begin end; }\n\
             codeunit 31 \"Caption Class Impl.\" {\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"UI Helper Triggers\", 'CaptionClassTranslate', '', false, false)]\n\
             local procedure V(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024]) begin end; }";
    let system = "codeunit 32 \"Caption Class Impl.\" {\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"UI Helper Triggers\", 'CaptionClassTranslate', '', false, false)]\n\
             local procedure V(Language: Integer; CaptionExpr: Text[1024]; var Translation: Text[1024]) begin end; }";
    let pubs = "codeunit 10 Pub { procedure Fire() begin OnX(); end; [IntegrationEvent(false, false)] local procedure OnX() begin end; }\n\
             codeunit 2000000007 \"Filter Helper Triggers\" { }";
    let build = |main: &str| {
        let app = |id: &str, deps: &[&str], src: &str| AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files: vec![(format!("{id}.al"), src.to_string())],
        };
        let sa = al_ir::SYSTEM_APPLICATION_APP_ID;
        compile_apps(
            vec![
                app("w", &["s", "p", sa], main),
                app("s", &["p"], subs),
                app("p", &[], pubs),
                app(sa, &[], system),
            ],
            vec![
                AppKind::Workspace,
                AppKind::Dependency,
                AppKind::Dependency,
                AppKind::Dependency,
            ],
            &Default::default(),
        )
        .1
    };
    let listed = |l: &Lowered| -> Vec<(al_ir::PlatformPublisher, String, String)> {
        let mut v: Vec<_> = (l.program.uncompiled_subscribers.iter())
            .map(|u| (u.platform, u.event.clone(), u.subscriber.clone()))
            .collect();
        v.sort_by(|a, b| (&a.1, &a.2).cmp(&(&b.1, &b.2)));
        v
    };
    let ui = |s: &str| {
        (
            al_ir::PlatformPublisher::UiHelperTriggers,
            "captionclasstranslate".to_string(),
            s.to_string(),
        )
    };
    let l = build("codeunit 1 Main { procedure Go() begin end; }");
    assert_eq!(
        listed(&l),
        [
            ui("codeunit \"Caption Class Impl.\""),
            ui("codeunit \"Subs\""),
            (
                al_ir::PlatformPublisher::FilterHelperTriggers,
                "makedatefilter".to_string(),
                "codeunit \"Subs\"".to_string()
            ),
        ]
    );
    let l = build(
        "codeunit 1 Main { procedure Go() var P: Codeunit Pub; R: Record Integer; T: Text; begin P.Fire(); \
             R.SetFilter(Number, '1'); T := CaptionClassTranslate('3,x'); end; }",
    );
    assert!(compiled(&l, "Subs", "X") && compiled(&l, "Subs", "F"));
    // The statement reaches no UI Helper subscriber: each stays listed, and al-vm refuses the translation for it.
    assert_eq!(listed(&l), [ui("codeunit \"Caption Class Impl.\"")]);
}

#[test]
fn an_event_published_by_an_extension_reaches_the_subscribers_of_its_base_object() {
    let dep = [
        ("t.al", "table 10 T { fields { field(1; K; Code[10]) { } } }"),
        (
            "x.al",
            "tableextension 11 TX extends T { procedure Fire() begin OnThing(); end; \
                 [IntegrationEvent(false, false)] local procedure OnThing() begin end; }",
        ),
        (
            "s.al",
            "codeunit 30 Subs {\n\
                 [EventSubscriber(ObjectType::Table, Database::T, 'OnThing', '', false, false)]\n\
                 local procedure A() begin end; }",
        ),
    ];
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var R: Record T; begin R.Fire(); end; }",
        &dep,
    );
    assert!(compiled(&l, "Subs", "A"));
}

#[test]
fn a_record_ref_write_raises_the_global_events_of_that_operation() {
    let dep = [(
        "s.al",
        "codeunit 20 Subs {\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseModify', '', false, false)]\n\
             local procedure Mod(RecRef: RecordRef) begin end;\n\
             [EventSubscriber(ObjectType::Codeunit, Codeunit::\"Global Triggers\", 'OnDatabaseDelete', '', false, false)]\n\
             local procedure Del(RecRef: RecordRef) begin end; }",
    )];
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var RR: RecordRef; begin RR.Modify(); end; }",
        &dep,
    );
    assert!(compiled(&l, "Subs", "Mod"));
    assert!(!compiled(&l, "Subs", "Del"));
    // The delete subscriber sits in a compiled object, so it is a stub that refuses if it is
    // asked, not a missing one.
    assert!(l.program.uncompiled_subscribers.is_empty());
    assert!(reasons(&l, "Subs", "Del")[0].contains("not compiled"));
}

#[test]
fn running_a_report_or_xmlport_reaches_its_triggers_by_variable_or_by_constant() {
    let dep = [
        (
            "r.al",
            "report 40 Rep { trigger OnPreReport() begin end; procedure Other() begin end; }",
        ),
        (
            "x.al",
            "xmlport 41 Xp { schema { textelement(Root) { } } trigger OnPreXmlPort() begin end; }",
        ),
    ];
    let (_, by_var) = compile(
        "codeunit 1 Main { procedure Go() var R: Report Rep; begin R.Run(); end; }",
        &dep,
    );
    assert!(compiled(&by_var, "Rep", "OnPreReport"));
    assert!(!compiled(&by_var, "Rep", "Other"));
    let (_, by_constant) = compile(
        "codeunit 1 Main { procedure Go() begin Report.Run(Report::Rep); XmlPort.Import(XmlPort::Xp); end; }",
        &dep,
    );
    assert!(compiled(&by_constant, "Rep", "OnPreReport"));
    assert!(compiled(&by_constant, "Xp", "OnPreXmlPort"));
    // Only named, not run: nothing is reached.
    let (_, named) = compile(
        "codeunit 1 Main { procedure Go() var I: Integer; begin I := Report::Rep; end; }",
        &dep,
    );
    assert!(!compiled(&named, "Rep", "OnPreReport"));
    // Handed to a procedure as an argument, workspace code may run it there
    // (corpus xml/TestReportDatasetHelpLink.al: `RenderDataset(Report::"G/L Register")`).
    let (_, passed) = compile(
        "codeunit 1 Main { procedure Go() begin Show(Report::Rep); end; procedure Show(Id: Integer) begin end; }",
        &dep,
    );
    assert!(compiled(&passed, "Rep", "OnPreReport"));
}

// The company data codeunit's app (`--company-data demotool`) hands its reports' ids to a procedure that runs them
// (the DemoTool's Interface Relationship Mgt.RunReport(Report::"Create Conts. from Customers")): its app counts as
// reaching them, as workspace code does. Another dependency app's mention does not.
#[test]
fn the_company_data_codeunits_app_reaches_the_reports_it_hands_to_a_call() {
    let dep = [
        ("r.al", "report 40 Rep { trigger OnPreReport() begin end; }"),
        (
            "b.al",
            "codeunit 50 Build { trigger OnRun() begin Show(Report::Rep); end; procedure Show(Id: Integer) begin Report.Run(Id); end; }",
        ),
    ];
    let build = |company_data: Vec<u32>| {
        let app = |id: &str, deps: &[&str], files: Vec<(String, String)>| AppSource {
            id: id.into(),
            name: id.into(),
            dependencies: deps.iter().map(|d| d.to_string()).collect(),
            files,
        };
        crate::lower::compile_apps_with(
            vec![
                app("main", &["dep"], vec![("main.al".into(), "codeunit 1 Main { }".into())]),
                app(
                    "dep",
                    &[],
                    dep.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
                ),
            ],
            vec![AppKind::Workspace, AppKind::Dependency],
            &Default::default(),
            None,
            crate::check::Roots {
                baseline: true,
                company_data,
                ..Default::default()
            },
        )
        .1
    };
    assert!(compiled(&build(vec![50]), "Rep", "OnPreReport"));
    let other = build(vec![]);
    let o = object(&other, "Rep");
    assert!(
        other.program.objects[o]
            .routines
            .iter()
            .all(|r| r.name != "OnPreReport")
            || !compiled(&other, "Rep", "OnPreReport")
    );
}

#[test]
fn the_triggers_before_and_after_a_table_trigger_that_an_extension_declares_are_reached() {
    let dep = [
        ("t.al", "table 10 T { fields { field(1; K; Code[10]) { } } }"),
        (
            "x.al",
            "tableextension 11 TX extends T { trigger OnBeforeInsert() begin end; trigger OnAfterInsert() begin end; \
                 trigger OnBeforeDelete() begin end; }",
        ),
    ];
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var R: Record T; begin R.Insert(); end; }",
        &dep,
    );
    assert!(compiled(&l, "TX", "OnBeforeInsert") && compiled(&l, "TX", "OnAfterInsert"));
    assert!(!compiled(&l, "TX", "OnBeforeDelete"));
}

#[test]
fn subscribers_nothing_compiled_are_listed_by_publisher_unless_they_are_manual() {
    let dep = [
        ("t.al", "table 10 T { fields { field(1; K; Code[10]) { } } }"),
        (
            "p.al",
            "codeunit 20 Pub { [IntegrationEvent(false, false)] procedure OnX() begin end; }",
        ),
        (
            "s.al",
            "codeunit 30 Auto {\n\
                 [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnX', '', false, false)]\n\
                 local procedure A() begin end; }",
        ),
        (
            "m.al",
            "codeunit 31 Manual { EventSubscriberInstance = Manual;\n\
                 [EventSubscriber(ObjectType::Codeunit, Codeunit::Pub, 'OnX', '', false, false)]\n\
                 local procedure A() begin end; }",
        ),
    ];
    // The workspace uses the publisher without calling the event routine, so its subscribers
    // are not reached; the manual one cannot fire without a bound instance.
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var P: Codeunit Pub; begin end; }",
        &dep,
    );
    let list: Vec<_> = l
        .program
        .uncompiled_subscribers
        .iter()
        .map(|u| {
            (
                u.publisher.map(|p| l.program.objects[p as usize].name.clone()),
                u.event.clone(),
                u.subscriber.clone(),
            )
        })
        .collect();
    assert_eq!(
        list,
        [(
            Some("Pub".to_string()),
            "onx".to_string(),
            "codeunit \"Auto\"".to_string()
        )]
    );
}
// A panic while checking or lowering one routine is a bug in the compiler. It must not end the
// run: the routine becomes a refusal, so the tests that reach it are not run and the rest are.
// `InjectedCheckPanic` and `InjectedLowerPanic` panic in the compiler (test builds only).
#[test]
fn a_panic_checking_a_reached_routine_refuses_it_and_nothing_else() {
    for workers in [1, 4] {
        let dep = [(
            "lib.al".to_string(),
            "codeunit 10 Lib { procedure Fine() begin end; procedure InjectedCheckPanic() begin end; \
                 procedure Other() begin end; }"
                .to_string(),
        )];
        let main = "codeunit 1 Main { procedure Go() var L: Codeunit Lib; begin L.Fine(); L.InjectedCheckPanic(); L.Other(); end; \
                        procedure Clean() var L: Codeunit Lib; begin L.Fine(); end; }";
        let out = compile_with_workers(main, &dep, workers);
        assert!(
            out.contains("internal error while checking"),
            "{workers} workers: {out}"
        );
    }
}

#[test]
fn a_panic_lowering_a_routine_refuses_it_and_nothing_else() {
    let (_, l) = compile(
        "codeunit 1 Main { procedure Go() var L: Codeunit Lib; begin L.Fine(); L.InjectedLowerPanic(); end; \
             procedure Clean() var L: Codeunit Lib; begin L.Fine(); end; }",
        &[(
            "lib.al",
            "codeunit 10 Lib { procedure Fine() begin end; procedure InjectedLowerPanic() begin end; }",
        )],
    );
    assert!(
        reasons(&l, "Lib", "InjectedLowerPanic")
            .iter()
            .any(|r| r.contains("internal error while lowering")),
        "{:?}",
        reasons(&l, "Lib", "InjectedLowerPanic")
    );
    assert!(
        reasons(&l, "Lib", "Fine").is_empty(),
        "{:?}",
        reasons(&l, "Lib", "Fine")
    );
    assert!(reasons(&l, "Main", "Clean").is_empty());
}

#[test]
fn a_panic_in_the_workspace_refuses_that_routine_only() {
    let (_, l) = compile(
        "codeunit 1 Main { procedure InjectedCheckPanic() begin end; procedure InjectedLowerPanic() begin end; \
             procedure Fine() begin end; }",
        &[("lib.al", "codeunit 10 Lib { }")],
    );
    for name in ["InjectedCheckPanic", "InjectedLowerPanic"] {
        assert!(
            reasons(&l, "Main", name).iter().any(|r| r.contains("internal error")),
            "{name}: {:?}",
            reasons(&l, "Main", name)
        );
    }
    assert!(reasons(&l, "Main", "Fine").is_empty());
}
