//! `Rec.Field::Member` names a member of the field's own enum, whatever enum of that name the file finds (al-runner issue
//! 8317). Measured shapes, under assumption `enum-member-of-field-type` until corpus PR 1442
//! (merged: types/enum-option/EnumMemberFieldType_Tests.al:29, :59, :77) is pinned: the file finds the name twice, or finds another enum of that
//! name in its own namespace or through a `using`. The corpus probe pins the values; these tests pin where the assumption is
//! marked, main's wrong value in the twin shapes, and the neighbors that keep main's answer.

use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::Signal;
use al_builtins::value::Value;
use al_sema::workspace::{AppSource, PLATFORM_APP_ID};

const FIELD_APP: &str = r#"namespace T.Enums;
enum 70110 "TF Kind" { Extensible = true; value(0; " ") { } value(1; Production) { } value(2; Transfer) { } }
"#;

const FIELD_TABLE: &str = r#"namespace T.Data;
using T.Enums;
table 70110 "TF Line" { fields { field(1; Id; Integer) { } field(2; Kind; Enum "TF Kind") { } } keys { key(PK; Id) { } } }
"#;

/// An enum of the field's enum's name, with Production at ordinal 5, in namespace `namespace` (none: the global one).
fn twin(namespace: Option<&str>) -> String {
    let header = namespace.map(|n| format!("namespace {n};\n")).unwrap_or_default();
    format!(
        "{header}enum 70111 \"TF Kind\" {{ Extensible = true; value(0; \" \") {{ }} value(5; Production) {{ }} }}\n"
    )
}

/// The reader codeunit C, in a file with the header `header` (namespace and usings).
fn reader(header: &str) -> String {
    format!(
        "{header}\n\
         codeunit 70112 C\n\
         {{\n\
             procedure ProductionOrdinal(): Integer\n\
             var Line: Record \"TF Line\";\n\
             begin Line.Kind := Line.Kind::Production; exit(Line.Kind.AsInteger()); end;\n\
         }}\n"
    )
}

/// App "a" holds the field's enum and table; app "b" (depends on "a") holds `files`. Runs C.ProductionOrdinal.
fn run(files: Vec<String>) -> (Result<Value, Signal>, Vec<Assumption>) {
    let app = |id: &str, deps: &[&str], files: Vec<String>| AppSource {
        id: id.into(),
        name: id.into(),
        dependencies: deps.iter().map(|d| d.to_string()).collect(),
        files: files
            .into_iter()
            .enumerate()
            .map(|(i, f)| (format!("{id}{i}.al"), f))
            .collect(),
    };
    let apps = vec![
        app(PLATFORM_APP_ID, &[], vec![]),
        app("a", &[], vec![FIELD_APP.to_string(), FIELD_TABLE.to_string()]),
        app("b", &["a"], files),
    ];
    let mut b = al_testkit::Built::new(al_testkit::compile_apps(apps));
    assumptions::clear_hits();
    let r = b.run_in("C", "ProductionOrdinal");
    (r, assumptions::take_hits())
}

const MARK: Assumption = Assumption::EnumMemberOfFieldType;

#[test]
fn a_name_the_file_finds_twice_is_the_fields_enum_under_the_assumption() {
    // EnumMemberFieldType_Tests.al FieldMember_NameFoundTwice_IsTheFieldsEnumMember.
    let (r, hits) = run(vec![twin(Some("T.Twin")), reader("namespace T.Tests;\nusing T.Data;")]);
    assert_eq!(al_testkit::int(r), 1);
    assert!(hits.contains(&MARK), "{hits:?}");
}

#[test]
fn a_same_named_enum_of_the_files_own_namespace_gives_way_to_the_fields_enum() {
    // FieldMember_SameNamedEnumInOwnNamespace_IsTheFieldsEnumMember: main read the twin's Production (5).
    let (r, hits) = run(vec![twin(Some("T.Twin")), reader("namespace T.Twin;\nusing T.Data;")]);
    assert_eq!(al_testkit::int(r), 1);
    assert!(hits.contains(&MARK), "{hits:?}");
}

#[test]
fn a_same_named_enum_through_a_using_gives_way_to_the_fields_enum() {
    // FieldMember_SameNamedEnumThroughUsing_IsTheFieldsEnumMember.
    let (r, hits) = run(vec![
        twin(Some("T.Twin")),
        reader("namespace T.Reader;\nusing T.Data;\nusing T.Twin;"),
    ]);
    assert_eq!(al_testkit::int(r), 1);
    assert!(hits.contains(&MARK), "{hits:?}");
}

#[test]
fn a_file_that_names_the_fields_enum_marks_nothing() {
    let (r, hits) = run(vec![
        twin(Some("T.Twin")),
        reader("namespace T.Tests;\nusing T.Data;\nusing T.Enums;"),
    ]);
    assert_eq!(al_testkit::int(r), 1);
    assert!(!hits.contains(&MARK), "{hits:?}");
}

#[test]
fn a_same_named_enum_of_the_global_namespace_keeps_mains_answer() {
    // Not measured: the twin found through the global namespace. No mark, whatever main answers.
    let (_, hits) = run(vec![twin(None), reader("namespace T.Tests;\nusing T.Data;")]);
    assert!(!hits.contains(&MARK), "{hits:?}");
}
