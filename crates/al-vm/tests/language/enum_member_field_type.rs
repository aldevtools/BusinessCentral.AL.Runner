//! `Field::Member` on an Enum field names a member of the field's own enum, whatever enum of that name the file finds
//! (al-runner issue 8317), and `Rec.Field` itself is of the field's enum type there. Measured shapes, under assumption
//! `enum-member-of-field-type` until corpus PR 1442 (merged: types/enum-option/EnumMemberFieldType_Tests.al:29, :59, :77)
//! corpus PR 1492 (open: the global record of a codeunit) and corpus PR 1480 (merged: types/enum-option/EnumMemberFieldTypeShapes_Tests.al, must-not-compile 3290 to 3293) are
//! pinned. The corpus probes pin the values; these tests pin where the assumption is marked, main's wrong value in the twin
//! shapes, the refusal of the shapes no probe measured, and the messages of the type errors.

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

/// App "a" holds the field's enum and table; app "b" (depends on "a") holds `files`.
fn apps(files: Vec<String>) -> Vec<AppSource> {
    apps_over(vec![FIELD_APP.to_string(), FIELD_TABLE.to_string()], files)
}

/// App "a" holds `field_files`; app "b" (depends on "a") holds `files`.
fn apps_over(field_files: Vec<String>, files: Vec<String>) -> Vec<AppSource> {
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
    vec![
        app(PLATFORM_APP_ID, &[], vec![]),
        app("a", &[], field_files),
        app("b", &["a"], files),
    ]
}

/// Runs C.ProductionOrdinal of [`apps`] and the assumptions it marked.
fn run(files: Vec<String>) -> (Result<Value, Signal>, Vec<Assumption>) {
    run_apps(apps(files))
}

fn run_apps(apps: Vec<AppSource>) -> (Result<Value, Signal>, Vec<Assumption>) {
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
fn a_same_named_enum_of_the_global_namespace_gives_way_to_the_fields_enum() {
    // EnumMemberFieldTypeShapes_Tests.al FieldMember_TwinInGlobalNamespace_IsTheFieldsEnumMember: main read the twin's
    // Production (5).
    let (r, hits) = run(vec![twin(None), reader("namespace T.Tests;\nusing T.Data;")]);
    assert_eq!(al_testkit::int(r), 1);
    assert!(hits.contains(&MARK), "{hits:?}");
}

#[test]
fn a_bare_field_in_a_table_extension_reads_the_fields_enum() {
    // FieldMember_InTableExtension_IsTheFieldsEnumMember: main read the twin's Production (5) for the bare `Kind`.
    let ext = "namespace T.Twin;\nusing T.Data;\n\
               tableextension 70113 X extends \"TF Line\"\n\
               {\n\
                   procedure BareOrdinal(): Integer\n\
                   begin Kind := Kind::Production; exit(Kind.AsInteger()); end;\n\
               }\n\
               codeunit 70112 C\n\
               {\n\
                   procedure ProductionOrdinal(): Integer\n\
                   var Line: Record \"TF Line\";\n\
                   begin exit(Line.BareOrdinal()); end;\n\
               }\n";
    let (r, hits) = run(vec![twin(Some("T.Twin")), ext.to_string()]);
    assert_eq!(al_testkit::int(r), 1);
    assert!(hits.contains(&MARK), "{hits:?}");
}

#[test]
fn a_global_record_variable_of_a_codeunit_reads_the_fields_enum() {
    // EnumMemberFieldTypeShapes_Tests.al:143 (corpus PR 1492): main read the twin's Production (5).
    let src = "namespace T.Twin;\nusing T.Data;\n\
               codeunit 70112 C\n\
               {\n\
                   var Line: Record \"TF Line\";\n\
                   procedure ProductionOrdinal(): Integer\n\
                   begin Line.Kind := Line.Kind::Production; exit(Line.Kind.AsInteger()); end;\n\
               }\n";
    let (r, hits) = run(vec![twin(Some("T.Twin")), src.to_string()]);
    assert_eq!(al_testkit::int(r), 1);
    assert!(hits.contains(&MARK), "{hits:?}");
}

/// Asserts that `r` is a refusal that names the assumption.
fn refused(r: Result<Value, Signal>) {
    match r {
        Err(Signal::Unsupported(why)) => assert!(why.contains("enum-member-of-field-type"), "{why}"),
        r => panic!("not refused: {r:?}"),
    }
}

#[test]
fn a_global_record_variables_field_without_a_member_has_the_fields_enum() {
    // EnumMemberFieldTypeShapes_Tests.al:154 (corpus PR 1492): `Format(GlobalLine.Kind)` is the field's member name.
    let src = "namespace T.Twin;\nusing T.Data;\n\
               codeunit 70112 C\n\
               {\n\
                   var Line: Record \"TF Line\";\n\
                   procedure ProductionOrdinal(): Integer\n\
                   begin Line.Kind := Line.Kind::Transfer; if Format(Line.Kind) <> 'Transfer' then exit(-1); exit(Line.Kind.AsInteger()); end;\n\
               }\n";
    let (r, hits) = run(vec![twin(Some("T.Twin")), src.to_string()]);
    assert_eq!(al_testkit::int(r), 2);
    assert!(hits.contains(&MARK), "{hits:?}");
}

#[test]
fn a_global_record_variable_of_a_page_is_not_measured_and_refuses() {
    // Only a codeunit's global record is measured (EnumMemberFieldTypeShapes_Tests.al:143): a page's refuses.
    let src = "namespace T.Twin;\nusing T.Data;\n\
               page 70114 P\n\
               {\n\
                   var Line: Record \"TF Line\";\n\
                   procedure Ordinal(): Integer\n\
                   begin Line.Kind := Line.Kind::Production; exit(Line.Kind.AsInteger()); end;\n\
               }\n\
               codeunit 70112 C\n\
               {\n\
                   procedure ProductionOrdinal(): Integer\n\
                   var Pg: Page P;\n\
                   begin exit(Pg.Ordinal()); end;\n\
               }\n";
    refused(run(vec![twin(Some("T.Twin")), src.to_string()]).0);
}

#[test]
fn a_field_enum_of_the_global_namespace_refuses_where_the_file_finds_a_twin() {
    // No qualified name reaches an enum of the global namespace from a file that finds a twin, so the field's type could
    // not be its enum's: refused (EnumMemberFieldTypeShapes_Tests.al measures the value, 1).
    let field_app =
        "enum 70110 \"TF Kind\" { Extensible = true; value(0; \" \") { } value(1; Production) { } }\n".to_string();
    let table = "namespace T.Data;\n\
                 table 70110 \"TF Line\" { fields { field(1; Id; Integer) { } field(2; Kind; Enum \"TF Kind\") { } } keys { key(PK; Id) { } } }\n"
        .to_string();
    let (r, _) = run_apps(apps_over(
        vec![field_app, table],
        vec![twin(Some("T.Twin")), reader("namespace T.Twin;\nusing T.Data;")],
    ));
    refused(r);
}

#[test]
fn this_field_in_a_table_extension_is_not_measured_and_refuses() {
    // `this.Kind::Production` in a table extension in the twin's namespace: no probe; main read the twin's Production (5).
    let ext = "namespace T.Twin;\nusing T.Data;\n\
               tableextension 70113 X extends \"TF Line\"\n\
               {\n\
                   procedure ThisOrdinal(): Integer\n\
                   begin this.Kind := this.Kind::Production; exit(this.Kind.AsInteger()); end;\n\
               }\n\
               codeunit 70112 C\n\
               {\n\
                   procedure ProductionOrdinal(): Integer\n\
                   var Line: Record \"TF Line\";\n\
                   begin exit(Line.ThisOrdinal()); end;\n\
               }\n";
    refused(run(vec![twin(Some("T.Twin")), ext.to_string()]).0);
}

#[test]
fn the_field_into_a_variable_of_the_same_named_enum_names_both_enums_by_their_namespaces() {
    // must-not-compile 3291: AL0122 `Cannot implicitly convert type 'Enum <field's namespace>."<name>"' ...`; the internal
    // name separator is never shown.
    let src = "namespace T.Twin;\nusing T.Data;\n\
               codeunit 70112 C\n\
               {\n\
                   procedure P()\n\
                   var Line: Record \"TF Line\"; Twin: Enum \"TF Kind\";\n\
                   begin Twin := Line.Kind; end;\n\
               }\n";
    let l = al_testkit::compile_apps(apps(vec![twin(Some("T.Twin")), src.to_string()]));
    let al0122: Vec<&str> = l
        .al_diagnostics
        .iter()
        .filter(|d| d.code == "AL0122")
        .map(|d| d.message.as_str())
        .collect();
    assert_eq!(al0122.len(), 1, "{:?}", l.al_diagnostics);
    assert!(
        al0122[0].starts_with("Cannot implicitly convert type 'Enum T.Enums.\"TF Kind\"' to"),
        "{}",
        al0122[0]
    );
}
