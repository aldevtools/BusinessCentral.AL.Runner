//! Name scope across apps and namespaces: a moved table stub of a dependency's table (al-runner issue 7914) and an enum
//! variable named outside its namespace (issue 7921, must-not-compile 0214).
use super::*;

const LIVE: &str =
    "table 50100 \"Probe Row\" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { Clustered = true; } } }";

fn stub(props: &str, name: &str) -> String {
    format!(
        "table 50100 \"{name}\" {{ {props} fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ Clustered = true; }} }} }}"
    )
}

const MOVED: &str = "ObsoleteState = Moved; ObsoleteReason = 'Moved'; ObsoleteTag = '1.0'; MovedTo = 'dep';";

// Issue 7914 (the reporter's alc 18.1 repro): the Moved stub of a table its direct dependency declares live compiles.
#[test]
fn al0264_not_reported_for_the_moved_stub_of_a_dependency_table() {
    assert!(codes_with_dependency(&[&stub(MOVED, "Probe Row")], Some(LIVE)).is_empty());
    // MovedTo in other letter case names the same app.
    let upper = MOVED.replace("'dep'", "'DEP'");
    assert!(codes_with_dependency(&[&stub(&upper, "Probe Row")], Some(LIVE)).is_empty());
}

// Neighbors of the issue 7914 pair keep main's AL0264 (must-not-compile 2149-table-id-of-dependency-app.al).
#[test]
fn al0264_kept_for_every_other_shape_of_a_moved_stub() {
    let al0264 = vec!["AL0264".to_string()];
    for (props, name) in [
        // No ObsoleteState, another state, no MovedTo, MovedTo another app, another name.
        ("", "Probe Row"),
        ("ObsoleteState = Pending; MovedTo = 'dep';", "Probe Row"),
        ("ObsoleteState = Removed; MovedTo = 'dep';", "Probe Row"),
        ("ObsoleteState = Moved;", "Probe Row"),
        ("ObsoleteState = Moved; MovedTo = 'other';", "Probe Row"),
        (MOVED, "Other Row"),
    ] {
        assert_eq!(
            codes_with_dependency(&[&stub(props, name)], Some(LIVE)),
            al0264,
            "{props} {name}"
        );
    }
    // The dependency's table is itself obsolete.
    let obsolete = LIVE.replace("{ fields", "{ ObsoleteState = Pending; fields");
    assert_eq!(
        codes_with_dependency(&[&stub(MOVED, "Probe Row")], Some(&obsolete)),
        al0264
    );
    // A codeunit is not a table (the properties are not a codeunit's either: AL0124, AL0169).
    let codeunit = codes_with_dependency(
        &["codeunit 50100 \"Probe C\" { ObsoleteState = Moved; MovedTo = 'dep'; }"],
        Some("codeunit 50100 \"Probe C\" { }"),
    );
    assert!(codeunit.contains(&"AL0264".to_string()), "{codeunit:?}");
}

// Issue 7914: with the pair compiled, a name that finds both tables resolves to the dependency's, and the program carries
// the assumption every test of it is flagged with.
#[test]
fn the_moved_stub_gives_way_to_the_dependency_table() {
    let apps = vec![
        AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec!["dep".into()],
            files: vec![("f.al".into(), stub(MOVED, "Probe Row"))],
        },
        AppSource {
            id: "dep".into(),
            name: "dep".into(),
            dependencies: vec![],
            files: vec![("d.al".into(), LIVE.to_string())],
        },
    ];
    let kinds = vec![
        crate::workspace::AppKind::Workspace,
        crate::workspace::AppKind::Dependency,
    ];
    let ws = Workspace::load_apps(apps, kinds, &Default::default());
    let stub_i = ws.objects.iter().position(|o| o.app == 0).expect("stub");
    let live_i = ws.objects.iter().position(|o| o.app == 1).expect("live");
    assert_eq!(ws.moved_table_of_dependency(stub_i), Some(live_i));
    assert!(ws.has_moved_table_stub());
    let file = ws.objects[stub_i].file;
    assert!(matches!(
        ws.resolve_ref(file, ObjKind::Table, &["Probe Row".to_string()]),
        crate::workspace::Resolution::Found(i) if i == live_i
    ));
    assert!(matches!(
        ws.resolve_ref(file, ObjKind::Table, &["50100".to_string()]),
        crate::workspace::Resolution::Found(i) if i == live_i
    ));
}

const KIND: &str =
    "namespace Acme.Kinds; enum 50100 \"Widget Kind\" { Extensible = true; value(0; A) { } value(1; B) { } }";

fn user(head: &str, var: &str) -> String {
    format!(
        "{head} codeunit 50101 \"Widget User\" {{ procedure UnqDecl(): Integer var K: {var}; begin exit(K.AsInteger()); end; }}"
    )
}

// Issue 7921 (the reporter's alc 18.1 repro): an enum of another namespace of the same app named by its plain name in a
// procedure variable, without a using, is AL0185 `Enum 'Widget Kind' is missing`.
#[test]
fn al0185_enum_of_another_namespace_without_its_using() {
    assert_eq!(
        messages_with_platform(&[KIND, &user("namespace Acme.Logic;", "Enum \"Widget Kind\"")], ""),
        vec!["AL0185 Enum 'Widget Kind' is missing".to_string()]
    );
    // The reporter's controls: with the using, and named qualified, it compiles.
    for (head, var) in [
        ("namespace Acme.Logic; using Acme.Kinds;", "Enum \"Widget Kind\""),
        ("namespace Acme.Logic;", "Enum Acme.Kinds.\"Widget Kind\""),
        ("namespace Acme.Kinds;", "Enum \"Widget Kind\""),
    ] {
        assert!(
            codes_with_platform(&[KIND, &user(head, var)], "").is_empty(),
            "{head} {var}"
        );
    }
}

// Neighbors of the issue 7921 shape that no measurement covers keep main's result (they compile).
#[test]
fn al0185_enum_not_judged_outside_the_measured_shapes() {
    let global = "namespace Acme.Logic; codeunit 50101 \"Widget User\" { var K: Enum \"Widget Kind\"; procedure F(): Integer begin exit(K.AsInteger()); end; }";
    let param = "namespace Acme.Logic; codeunit 50101 \"Widget User\" { procedure F(K: Enum \"Widget Kind\"): Integer begin exit(K.AsInteger()); end; }";
    let trigger = "namespace Acme.Logic; codeunit 50101 \"Widget User\" { trigger OnRun() var K: Enum \"Widget Kind\"; begin end; }";
    let no_ns = user("", "Enum \"Widget Kind\"");
    let child = user("namespace Acme.Kinds.Sub;", "Enum \"Widget Kind\"");
    let parent = user("namespace Acme;", "Enum \"Widget Kind\"");
    for src in [global, param, trigger, &no_ns, &child, &parent] {
        assert!(codes_with_platform(&[KIND, src], "").is_empty(), "{src}");
    }
    // An enum of a dependency app in another namespace is not judged.
    assert!(codes_with_dependency(&[&user("namespace Acme.Logic;", "Enum \"Widget Kind\"")], Some(KIND)).is_empty());
}

// must-not-compile 0214-unknown-enum-type.al:7: a procedure variable of an enum type no app declares.
#[test]
fn al0185_variable_of_an_unknown_enum_type() {
    let src = "codeunit 50019 \"MNC C214\" { procedure Run1() var E: Enum \"MNC No Such Enum\"; begin end; }";
    assert_eq!(
        messages_with_platform(&[src], ""),
        vec!["AL0185 Enum 'MNC No Such Enum' is missing".to_string()]
    );
    // Without the platform app loaded the name may exist there: not judged.
    assert!(codes(&[src]).is_empty());
}
