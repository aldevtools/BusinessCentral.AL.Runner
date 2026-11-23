//! The resolution of a `TableRelation` target against the apps a file sees (`Workspace::resolve_relation_target`).

use super::*;

#[test]
fn run_time_resolution_only_trusts_tiers_no_absent_app_could_shadow() {
    // corpus recordref-fieldref/fieldref/TestRelationOwnNamespaceDependency.al, TestRelationTargetNamespaceScope.al:
    // a workspace table must not stand in for a Base Application one that is not here.
    let ws = load(vec![app(
        "a",
        &[],
        &[
            ("t.al", "namespace Imported; table 50900 \"Agent\" { }"),
            ("g.al", "table 50901 \"Global Agent\" { }"),
            ("own.al", "namespace Own; table 50902 \"Own Agent\" { }"),
            ("c1.al", "namespace Own; using Imported; codeunit 60100 C1 { }"),
            ("c2.al", "using Imported; codeunit 60101 C2 { }"),
            ("c3.al", "codeunit 60102 C3 { }"),
            ("c4.al", "namespace Other; codeunit 60103 C4 { }"),
        ],
    )]);
    let file = |name: &str| ws.objects.iter().find(|o| o.name == name).unwrap().file;
    let found = |r: Resolution| matches!(r, Resolution::Found(_));
    let one = |n: &str| vec![n.to_string()];
    // The ordinary compile-time resolution finds each of them.
    assert!(found(ws.resolve_ref(file("C1"), ObjKind::Table, &one("Agent"))));
    // Own namespace: certain.
    assert!(found(ws.resolve_ref_certain(
        file("C1"),
        ObjKind::Table,
        &one("Own Agent")
    )));
    // A `using` hit under a namespaced file could be shadowed by a Base Application table of the
    // file's own namespace: not certain. Without a namespace of its own it is.
    assert!(!found(ws.resolve_ref_certain(
        file("C1"),
        ObjKind::Table,
        &one("Agent")
    )));
    assert!(found(ws.resolve_ref_certain(file("C2"), ObjKind::Table, &one("Agent"))));
    // A global table is only certain for a file with neither namespace nor usings.
    assert!(!found(ws.resolve_ref_certain(
        file("C2"),
        ObjKind::Table,
        &one("Global Agent")
    )));
    assert!(found(ws.resolve_ref_certain(
        file("C3"),
        ObjKind::Table,
        &one("Global Agent")
    )));
    assert!(!found(ws.resolve_ref_certain(
        file("C4"),
        ObjKind::Table,
        &one("Global Agent")
    )));
    // Qualified names are exact.
    assert!(found(ws.resolve_ref_certain(
        file("C4"),
        ObjKind::Table,
        &["Imported".to_string(), "Agent".to_string()]
    )));
}

#[test]
fn with_base_application_loaded_a_using_resolves_a_relation_target() {
    // corpus recordref-fieldref/fieldref/TestRelationTargetNamespaceScope.al RelationScope_Usings_*: a table in a
    // file with a namespace, naming "Shipping Agent" with `using Microsoft.Foundation.Shipping`,
    // relates to Base Application's table 291. Base Application is here, so the tiers above the
    // using can be told to hold nothing (the same holds for the name written in Base
    // Application's own files, which resolve among the apps they saw).
    let base = app(
        "Base Application",
        &[],
        &[(
            "sa.al",
            "namespace Microsoft.Foundation.Shipping; table 291 \"Shipping Agent\" { }",
        )],
    );
    let mine = app(
        "mine",
        &["Base Application"],
        &[(
            "c.al",
            "namespace Mine.Scope; using Microsoft.Foundation.Shipping; codeunit 60100 C1 { }",
        )],
    );
    let ws = Workspace::load_apps(
        vec![base, mine],
        vec![AppKind::Dependency, AppKind::Workspace],
        &al_version::corpus_symbols(
            &al_version::BcVersion::new(27, 0),
            &al_version::ReleasedMinors::NotRecorded,
        ),
    );
    let file = ws.objects.iter().find(|o| o.name == "C1").unwrap().file;
    let name = vec!["Shipping Agent".to_string()];
    assert!(matches!(
        ws.resolve_relation_target(file, ObjKind::Table, &name),
        Resolution::Found(_)
    ));
    // Without Base Application, and with a dependency that is not loaded, a hit under a using stays uncertain (the
    // test above): the absent app could hold a same-named table.
    let alone = Workspace::load(
        vec![app(
            "mine",
            &["437dbf0e-84ff-417a-965d-ed2bb9650972"],
            &[
                (
                    "t.al",
                    "namespace Microsoft.Foundation.Shipping; table 291 \"Shipping Agent\" { }",
                ),
                (
                    "c.al",
                    "namespace Mine.Scope; using Microsoft.Foundation.Shipping; codeunit 60100 C1 { }",
                ),
            ],
        )],
        &al_version::corpus_symbols(
            &al_version::BcVersion::new(27, 0),
            &al_version::ReleasedMinors::NotRecorded,
        ),
    );
    let file = alone.objects.iter().find(|o| o.name == "C1").unwrap().file;
    assert!(!matches!(
        alone.resolve_relation_target(file, ObjKind::Table, &name),
        Resolution::Found(_)
    ));
}

#[test]
fn with_every_dependency_loaded_a_using_resolves_a_relation_target() {
    // al-runner issues 8191 and 8181: an app with no dependency names a table of its own in another namespace through a
    // using, and a table of another namespace from a file with neither namespace nor using (the "Table Metadata" shape). No app is absent, so the name means what it means to
    // the compiler (TestRelationTargetNamespaceScope.al:235, a table of the app reached through a using).
    let ws = load(vec![
        app(PLATFORM_APP_ID, &[], &[]),
        app(
            "mine",
            &[],
            &[
                ("g.al", "namespace Repro.Setup; table 51690 \"Repro Posting Group\" { }"),
                (
                    "h.al",
                    "namespace Repro.Assets; using Repro.Setup; codeunit 60100 C1 { }",
                ),
                (
                    "m.al",
                    "namespace System.Reflection; table 50999 \"Repro Metadata\" { }",
                ),
                ("p.al", "codeunit 60101 C2 { }"),
                ("q.al", "namespace Repro.Other; codeunit 60102 C3 { }"),
            ],
        ),
    ]);
    let file = |name: &str| ws.objects.iter().find(|o| o.name == name).unwrap().file;
    let found = |r: Resolution| matches!(r, Resolution::Found(_));
    assert!(found(ws.resolve_relation_target(
        file("C1"),
        ObjKind::Table,
        &["Repro Posting Group".to_string()]
    )));
    assert!(found(ws.resolve_relation_target(
        file("C2"),
        ObjKind::Table,
        &["Repro Metadata".to_string()]
    )));
    // The certain resolution itself is unchanged: an absent app is what it guards against.
    assert!(!found(ws.resolve_ref_certain(
        file("C1"),
        ObjKind::Table,
        &["Repro Posting Group".to_string()]
    )));
    // A file with a namespace and no using does not reach a table of another namespace (must-not-compile
    // 2081-namespace-record-integer-without-using.al): main's refusal stays.
    assert!(!found(ws.resolve_relation_target(
        file("C3"),
        ObjKind::Table,
        &["Repro Metadata".to_string()]
    )));
}

/// Without the platform app System an app is absent (`Workspace::no_app_absent`): a hit under a using stays uncertain, as
/// `names_object` treats the same app (review of al-runner PR 8220, probe rel-e0).
#[test]
fn without_the_platform_app_a_using_does_not_resolve_a_relation_target() {
    let ws = load(vec![app(
        "mine",
        &[],
        &[
            ("g.al", "namespace Repro.Setup; table 51690 \"Repro Posting Group\" { }"),
            (
                "h.al",
                "namespace Repro.Assets; using Repro.Setup; codeunit 60100 C1 { }",
            ),
        ],
    )]);
    let c1 = ws.objects.iter().find(|o| o.name == "C1").unwrap().file;
    assert!(!matches!(
        ws.resolve_relation_target(c1, ObjKind::Table, &["Repro Posting Group".to_string()]),
        Resolution::Found(_)
    ));
}
