//! A TableRelation a table extension sets, whose `const(Member)` condition names a member of the enum of a field the table
//! declares, while the extension's file cannot name that enum (its namespace has no `using` of the enum's): Base Application's
//! "Mfg. Item Journal Line" on Item Journal Line."Order Type" (al-runner issue 8297). The condition names a member of the enum
//! the table's file names, under assumption `relation-condition-enum-of-declaring-file` until corpus PR 1437
//! (merged: table-objects/extensions/ExtRelationEnumNamespace_Tests.al:49-260, :369, :400, :424) is pinned. The corpus probe pins the values; these tests pin
//! what it cannot see: where the assumption is marked, and the neighbors that keep main's answer or stay refused.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::{AppSource, PLATFORM_APP_ID};

const ENUMS: &str = r#"namespace T.Enums;
enum 70100 "TE Kind" { Extensible = true; value(0; " ") { } value(1; Production) { } value(2; Transfer) { } }
"#;

const TARGETS: &str = r#"namespace T.Targets;
table 70100 "TE Order" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
"#;

const TABLE: &str = r#"namespace T.Journal;
using T.Enums;
table 70101 "TE Line"
{
    fields { field(1; Id; Integer) { } field(2; Kind; Enum "TE Kind") { } field(3; "Order No."; Code[20]) { } }
    keys { key(PK; Id) { } }
}
"#;

/// The extension, in a namespace with the usings `usings`, sets the relation of "Order No." on the condition field `field`.
fn extension(usings: &str, field: &str) -> String {
    format!(
        "namespace T.Mfg;\nusing T.Journal;\nusing T.Targets;\n{usings}\n\
         tableextension 70100 \"TE Line Ext\" extends \"TE Line\"\n\
         {{ fields {{ modify(\"Order No.\") {{ TableRelation = if ({field} = const(Production)) \"TE Order\".\"No.\"; }} }} }}\n"
    )
}

/// A second extension that declares the field Kind2 of the enum, in a namespace that names it.
const KIND2: &str = r#"namespace T.Kind2;
using T.Enums;
using T.Journal;
tableextension 70101 "TE Line Kind2" extends "TE Line" { fields { field(70101; Kind2; Enum "TE Kind") { } } }
"#;

const TESTS: &str = r#"namespace T.Tests;
using T.Journal;
using T.Targets;
codeunit 70100 C
{
    procedure Matching(): Text
    var
        O: Record "TE Order";
        L: Record "TE Line";
    begin
        O."No." := 'O1';
        O.Insert();
        L.Kind := L.Kind::Production;
        L.Validate("Order No.", 'O1');
        exit(L."Order No.");
    end;

    procedure Missing(): Text
    var
        L: Record "TE Line";
    begin
        L.Kind := L.Kind::Production;
        L.Validate("Order No.", 'NONE');
        exit(L."Order No.");
    end;

    procedure NoArmApplies(): Text
    var
        L: Record "TE Line";
    begin
        L.Kind := L.Kind::Transfer;
        L.Validate("Order No.", 'NONE');
        exit(L."Order No.");
    end;
}
"#;

fn apps(files: Vec<String>) -> Vec<AppSource> {
    vec![
        AppSource {
            id: PLATFORM_APP_ID.into(),
            name: "System".into(),
            dependencies: vec![],
            files: vec![],
        },
        AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: files
                .into_iter()
                .enumerate()
                .map(|(i, f)| (format!("f{i}.al"), f))
                .collect(),
        },
    ]
}

/// Runs `routine` of codeunit C with the extension `ext` (and `more` files) and gives its answer with the assumptions marked.
fn run(ext: String, more: &[&str], routine: &str) -> (Result<Value, Signal>, Vec<Assumption>) {
    let mut files = vec![
        ENUMS.to_string(),
        TARGETS.to_string(),
        TABLE.to_string(),
        ext,
        TESTS.to_string(),
    ];
    files.extend(more.iter().map(|f| f.to_string()));
    let mut b = al_testkit::Built::new(al_testkit::compile_apps(apps(files)));
    assumptions::clear_hits();
    let r = b.run_in("C", routine);
    (r, assumptions::take_hits())
}

const MARK: Assumption = Assumption::RelationConditionEnumOfDeclaringFile;

#[test]
fn an_extension_without_a_using_of_the_enum_applies_the_arm_under_the_assumption() {
    let (r, hits) = run(extension("", "Kind"), &[], "Matching");
    assert_eq!(al_testkit::text(r), "O1");
    assert!(hits.contains(&MARK), "{hits:?}");
    let (r, hits) = run(extension("", "Kind"), &[], "Missing");
    assert!(
        al_testkit::error_message(r).contains("cannot be found in the related table"),
        "the arm applies"
    );
    assert!(hits.contains(&MARK), "{hits:?}");
}

#[test]
fn an_arm_that_does_not_apply_marks_the_assumption_too() {
    // That the arm does not apply rests on the same member.
    let (r, hits) = run(extension("", "Kind"), &[], "NoArmApplies");
    assert_eq!(al_testkit::text(r), "NONE");
    assert!(hits.contains(&MARK), "{hits:?}");
}

#[test]
fn an_extension_that_names_the_enum_marks_nothing() {
    let (r, hits) = run(extension("using T.Enums;", "Kind"), &[], "Matching");
    assert_eq!(al_testkit::text(r), "O1");
    assert!(!hits.contains(&MARK), "{hits:?}");
}

#[test]
fn a_condition_on_a_field_another_extension_declares_stays_refused() {
    // Not measured: the condition field is not the table's own.
    let (r, _) = run(extension("using T.Kind2;", "Kind2"), &[KIND2], "Missing");
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains("not understood yet"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

/// An app "b" over app "a" (the enum, the related table and the line table), with the extension and the tests in "b".
fn run_two(b_files: Vec<String>, routine: &str) -> (Result<Value, Signal>, Vec<Assumption>) {
    let mut apps = apps(vec![ENUMS.to_string(), TARGETS.to_string(), TABLE.to_string()]);
    apps.push(AppSource {
        id: "b".into(),
        name: "b".into(),
        dependencies: vec!["a".into()],
        files: b_files
            .into_iter()
            .enumerate()
            .map(|(i, f)| (format!("b{i}.al"), f))
            .collect(),
    });
    let mut b = al_testkit::Built::new(al_testkit::compile_apps(apps));
    assumptions::clear_hits();
    let r = b.run_in("C", routine);
    (r, assumptions::take_hits())
}

/// An enum "TE Kind" of another namespace, with Production at another ordinal than the field's enum.
fn twin(namespace: &str) -> String {
    format!(
        "{namespace}\nenum 70102 \"TE Kind\" {{ Extensible = true; value(0; \" \") {{ }} value(2; Production) {{ }} }}\n"
    )
}

/// The tests, in a file that names the field's enum (so that `L.Kind::Production` is the field's member when a twin exists).
fn tests_naming_the_enum() -> String {
    TESTS.replace("using T.Journal;", "using T.Enums;\nusing T.Journal;")
}

#[test]
fn an_extension_in_another_app_applies_the_arm_under_the_assumption() {
    // ExtRelationEnumNamespace_Tests.al ModifyRelation_ExtensionInDependentApp_Validate.
    let (r, hits) = run_two(vec![extension("", "Kind"), TESTS.to_string()], "Missing");
    assert!(al_testkit::error_message(r).contains("cannot be found in the related table"));
    assert!(hits.contains(&MARK), "{hits:?}");
}

#[test]
fn an_enum_of_the_same_name_in_the_extensions_own_namespace_gives_way_to_the_fields_enum() {
    // ExtRelationEnumNamespace_Tests.al ExtensionField_SameNamedEnumInExtensionNamespace_Validate: Production of the field's
    // enum (ordinal 1), not of the twin (ordinal 2), so the arm applies.
    let (r, hits) = run_two(
        vec![twin("namespace T.Mfg;"), extension("", "Kind"), tests_naming_the_enum()],
        "Missing",
    );
    assert!(al_testkit::error_message(r).contains("cannot be found in the related table"));
    assert!(hits.contains(&MARK), "{hits:?}");
}

#[test]
fn an_enum_of_the_same_name_through_a_using_keeps_mains_answer() {
    // Not measured: the twin reached through a `using`, not the file's own namespace. Main takes the twin's member, so the arm
    // does not apply to Production (ordinal 1).
    let (r, hits) = run_two(
        vec![
            twin("namespace T.Other;"),
            extension("using T.Other;", "Kind"),
            tests_naming_the_enum(),
        ],
        "Missing",
    );
    assert_eq!(al_testkit::text(r), "NONE");
    assert!(!hits.contains(&MARK), "{hits:?}");
}

#[test]
fn an_enum_name_two_usings_find_stays_refused() {
    // Not measured: the extension's file finds the name twice.
    let (r, _) = run_two(
        vec![
            twin("namespace T.Other;"),
            extension("using T.Enums;\nusing T.Other;", "Kind"),
            tests_naming_the_enum(),
        ],
        "Missing",
    );
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains("not understood yet"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_lookup_over_the_assumed_condition_is_refused() {
    // Not measured: the probe has no page.
    let page = r#"namespace T.Tests;
using T.Journal;
page 70103 "TE Line Card"
{
    PageType = Card; SourceTable = "TE Line"; ApplicationArea = All; UsageCategory = None;
    layout { area(Content) { field(Kind; Rec.Kind) { } field("Order No."; Rec."Order No.") { } } }
}
codeunit 70104 L
{
    procedure Lookup(): Text
    var R: Record "TE Line"; P: TestPage "TE Line Card";
    begin
        R.Id := 1; R.Kind := R.Kind::Production; R.Insert();
        P.OpenEdit();
        P."Order No.".Lookup();
        exit(P."Order No.".Value());
    end;
}
"#;
    let files = vec![
        ENUMS.to_string(),
        TARGETS.to_string(),
        TABLE.to_string(),
        extension("", "Kind"),
        page.to_string(),
    ];
    let mut b = al_testkit::Built::new(al_testkit::compile_apps(apps(files)));
    match b.run_in("L", "Lookup") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("does not name"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
