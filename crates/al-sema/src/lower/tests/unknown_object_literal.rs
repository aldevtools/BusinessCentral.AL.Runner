//! `Database::"X"` or `Report::"X"` of an object that neither the app nor its dependencies declare (al-runner issues
//! 8101, 8099). The reason says so instead of "is not lowered yet", which reads as a missing feature (a runner text,
//! ADR-009). It names a cause only when the workspace shows it: a dependency skipped as not found, or (when the app sees
//! Base Application) another country's Base Application.
use super::*;
use crate::workspace::AppKind;

fn reasons_of(l: &Lowered) -> Vec<String> {
    l.unsupported
        .iter()
        .flatten()
        .flatten()
        .map(|r| r.to_string())
        .collect()
}

fn reasons(src: &str) -> Vec<String> {
    let (_, l) = lower_files(&[("f.al", src)], false);
    reasons_of(&l)
}

fn app(id: &str, name: &str, deps: &[&str], files: &[(&str, &str)]) -> AppSource {
    AppSource {
        id: id.into(),
        name: name.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: files.iter().map(|(p, s)| (p.to_string(), s.to_string())).collect(),
    }
}

fn reasons_of_apps(apps: Vec<AppSource>, kinds: Vec<AppKind>) -> Vec<String> {
    let ws = Workspace::load_apps(apps, kinds, &Default::default());
    let members = crate::members::build(&ws);
    let checked = crate::check::check(&ws, &members, &al_builtins::catalog::Catalog::load());
    reasons_of(&lower_with_positions(&ws, &members, &checked, false))
}

const USES_DELIVERY: &str =
    "codeunit 50100 C { procedure P(): Integer begin exit(Database::\"Delivery Reminder Header\"); end; }";

#[test]
fn an_object_literal_that_names_no_declared_object_says_so_and_names_no_cause() {
    for (literal, kind) in [
        ("Database::\"Delivery Reminder Header\"", "table"),
        ("Report::\"Vendor Payment List\"", "report"),
        ("Page::\"Missing Card\"", "page"),
        ("Codeunit::\"Missing Unit\"", "codeunit"),
        ("Query::\"Missing Query\"", "query"),
        ("XmlPort::\"Missing Port\"", "xmlport"),
    ] {
        let src = format!("codeunit 50100 C {{ procedure P(): Integer begin exit({literal}); end; }}");
        let r = reasons(&src);
        assert!(
            r.iter().any(|r| r.contains(&format!(
                "`{literal}` names no {kind} that this app or its dependencies declare"
            )) && !r.contains("--country")
                && !r.contains("skipped")
                && !r.contains("not lowered yet")),
            "{literal}: {r:?}"
        );
    }
}

// A table of a loaded app that the app does not depend on: it is not visible, and neither hint applies.
#[test]
fn a_table_of_a_loaded_app_that_is_not_a_dependency_gets_no_cause() {
    let other = app(
        "other",
        "other",
        &[],
        &[(
            "o.al",
            "table 50200 \"Delivery Reminder Header\" { fields { field(1; K; Integer) { } } }",
        )],
    );
    let main = app("main", "main", &[], &[("m.al", USES_DELIVERY)]);
    let r = reasons_of_apps(vec![main, other], vec![AppKind::Workspace, AppKind::Workspace]);
    assert!(
        r.iter().any(
            |r| r.contains("names no table that this app or its dependencies declare")
                && !r.contains("--country")
                && !r.contains("skipped")
        ),
        "{r:?}"
    );
}

#[test]
fn a_dependency_that_is_not_loaded_is_named_as_a_cause() {
    let main = app("main", "main", &["absent"], &[("m.al", USES_DELIVERY)]);
    let r = reasons_of_apps(vec![main], vec![AppKind::Workspace]);
    assert!(
        r.iter().any(
            |r| r.contains("names no table that this app or its dependencies declare")
                && r.contains("a dependency was skipped as not found")
                && !r.contains("--country")
        ),
        "{r:?}"
    );
}

#[test]
fn an_app_that_sees_base_application_gets_the_country_hint() {
    let base = app(
        "base",
        "Base Application",
        &[],
        &[("b.al", "table 18 Customer { fields { field(1; K; Code[20]) { } } }")],
    );
    let main = app("main", "main", &["base"], &[("m.al", USES_DELIVERY)]);
    let r = reasons_of_apps(vec![main, base], vec![AppKind::Workspace, AppKind::Dependency]);
    assert!(
        r.iter().any(
            |r| r.contains("names no table that this app or its dependencies declare")
                && r.contains("--country")
                && !r.contains("skipped")
        ),
        "{r:?}"
    );
}

#[test]
fn an_object_literal_of_a_loaded_object_is_lowered() {
    let r = reasons(
        "table 50101 K { fields { field(1; A; Integer) { } } } codeunit 50100 C { procedure P(): Integer begin exit(Database::K); end; }",
    );
    assert!(r.is_empty(), "{r:?}");
}
