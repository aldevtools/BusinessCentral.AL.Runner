//! Builtins and conversions that kept about 580 Tests-ERM tests from running (issue 23):
//! `RecordRef.FilterGroup`, the conversion of an Option member to an Enum, a TableRelation shape.

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;

const OBJECTS: &str = r#"
table 62900 "B4 Row" { fields { field(1; Code; Code[20]) { } } keys { key(PK; Code) { } } }
enum 62902 "B4 Kind" { value(0; A) { } value(1; B) { } }
"#;

fn outcomes(tests: &str) -> Vec<(String, Outcome)> {
    let src = format!("{OBJECTS}\ncodeunit 62901 T {{ Subtype = Test; {tests} }}");
    al_testkit::outcomes_in(&src, "T")
}

// record/TestRecordRuntimeTextsAndViews.al:374 (Record_FilterGroup_ReturnsTheGroupJustSet_AndReadsOnlyTheCurrentGroup),
// the same through a RecordRef: the RecordRef shares the record's filter groups. -1 is the OR group
// (record/PWFilterGroupOr.al); other negative groups stay refused.
#[test]
fn record_ref_filter_group_returns_the_group_just_set_and_reads_only_the_current_group() {
    let o = outcomes(
        "[Test] procedure Groups() var RR: RecordRef; begin
             RR.Open(62900);
             if RR.FilterGroup() <> 0 then Error('start');
             if RR.FilterGroup(2) <> 2 then Error('set 2');
             RR.Field(1).SetRange('X');
             if RR.FilterGroup(0) <> 0 then Error('set 0');
             if RR.GetFilters() <> '' then Error('group 0 shows %1', RR.GetFilters());
             if RR.FilterGroup(2) <> 2 then Error('set 2 again');
             if RR.GetFilters() <> 'Code: X' then Error('group 2 shows %1', RR.GetFilters());
         end;
         [Test] procedure OrGroup() var R: Record \"B4 Row\"; RR: RecordRef; begin RR.GetTable(R); RR.FilterGroup(-2); end;",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0]);
    assert!(
        matches!(&o[1].1, Outcome::NotRun(why) if why.contains("FilterGroup(-2)") && why.contains("only -1")),
        "{:?}",
        o[1]
    );
}

// An Option converts to an Enum by its ordinal (corpus types/PEOptionEnumBigText.al:68-97, probed on BC 27.5);
// it was refused before the probe (issue 29).
#[test]
fn an_option_assigned_to_an_enum_takes_its_ordinal() {
    let o = outcomes(
        "[Test] procedure Convert() var O: Option A,B; begin O := O::B; E := O; if E.AsInteger() <> 1 then Error('got %1', E.AsInteger()); end;
         var E: Enum \"B4 Kind\";",
    );
    assert_eq!(o[0].1, Outcome::Pass, "{:?}", o[0]);
}

/// The `relation_arms` of the first table field named `field` of table `table` in `src`.
fn arms(src: &str, table: &str, field: &str) -> usize {
    let l = al_testkit::compile(src);
    let o = l.program.objects.iter().find(|o| o.name == table).unwrap();
    o.table
        .as_ref()
        .unwrap()
        .fields
        .iter()
        .find(|f| f.name == field)
        .unwrap()
        .relation_arms
        .len()
}

const REL: &str = r#"
enum 62910 "B4 Acc Type" { value(0; "G/L Account") { } value(1; Customer) { } value(2; "IC Partner") { } }
enum 62911 "B4 IC Type" { value(0; "G/L Account") { } value(1; "Bank Account") { } }
enum 62912 "B4 GL Type" { value(0; Posting) { } value(1; Heading) { } }
table 62913 "B4 IC GL" { fields { field(1; "No."; Code[20]) { } field(2; "Account Type"; Enum "B4 GL Type") { } field(3; Blocked; Boolean) { } } keys { key(PK; "No.") { } } }
table 62914 "B4 IC Bank" { fields { field(1; "No."; Code[20]) { } field(2; "IC Partner Code"; Code[20]) { } field(3; Blocked; Boolean) { } } keys { key(PK; "No.") { } } }
table 62915 "B4 Line"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; "Account Type"; Enum "B4 Acc Type") { }
        field(3; "IC Account Type"; Enum "B4 IC Type") { }
        field(4; "IC Partner Code"; Code[20]) { }
        field(5; "Account No."; Code[20]) { }
        field(6; "A"; Code[20]) { TableRelation = if ("IC Account Type" = const("G/L Account")) "B4 IC GL" where("Account Type" = const(Posting), Blocked = const(false)); }
        field(7; "B"; Code[20]) { TableRelation = if ("Account Type" = const(Customer), "IC Account Type" = const("Bank Account")) "B4 IC Bank" where("IC Partner Code" = field("IC Partner Code"), Blocked = const(false)); }
        field(8; "C"; Code[20]) { TableRelation = if ("Account Type" = const("IC Partner"), "IC Account Type" = const("Bank Account")) "B4 IC Bank" where("IC Partner Code" = field("Account No."), Blocked = const(false)); }
    }
    keys { key(PK; "No.") { } }
}
"#;

// issue 23: Gen. Journal Line."IC Account No." combines these three arm shapes.
#[test]
fn the_arm_shapes_of_a_journal_line_ic_account_relation_are_understood() {
    for field in ["A", "B", "C"] {
        assert!(arms(REL, "B4 Line", field) > 0, "arm shape of {field} not understood");
    }
}

// An enum named by a condition of a relation, in a namespace the dependency's file imports with `using`:
// the dependency's file resolves it as the compiler did (corpus fieldref/TestRelationTargetNamespaceScope.al
// pins the target's scope the same way).
#[test]
fn a_condition_on_an_enum_of_another_namespace_in_a_dependency_is_understood() {
    let dep = r#"
namespace Dep.Journal;
enum 62920 "B4 Ns Kind" { value(0; Alpha) { } value(1; Beta) { } }
"#;
    let tables = r#"
namespace Dep.Lines;
using Dep.Journal;
table 62921 "B4 Ns Target" { fields { field(1; "No."; Code[20]) { } } keys { key(PK; "No.") { } } }
table 62922 "B4 Ns Line"
{
    fields
    {
        field(1; "No."; Code[20]) { }
        field(2; Kind; Enum "B4 Ns Kind") { }
        field(3; Ref; Code[20]) { TableRelation = if (Kind = const(Beta)) "B4 Ns Target"; }
    }
    keys { key(PK; "No.") { } }
}
"#;
    let (ws, l) = al_sema::lower::compile_apps(
        vec![
            AppSource {
                id: "dep0".into(),
                name: "dep0".into(),
                dependencies: vec![],
                files: vec![("e.al".into(), dep.into())],
            },
            AppSource {
                id: "dep1".into(),
                name: "dep1".into(),
                dependencies: vec!["dep0".into()],
                files: vec![("t.al".into(), tables.into())],
            },
            AppSource {
                id: "main".into(),
                name: "main".into(),
                dependencies: vec!["dep1".into()],
                files: vec![(
                    "m.al".into(),
                    "codeunit 62923 M { procedure P() var L: Record \"B4 Ns Line\"; begin L.Validate(Ref, 'X'); end; }"
                        .into(),
                )],
            },
        ],
        vec![
            al_sema::workspace::AppKind::Dependency,
            al_sema::workspace::AppKind::Dependency,
            al_sema::workspace::AppKind::Workspace,
        ],
        &Default::default(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let o = l
        .program
        .objects
        .iter()
        .find(|o| o.name == "B4 Ns Line")
        .expect("the table is compiled");
    let f = o
        .table
        .as_ref()
        .unwrap()
        .fields
        .iter()
        .find(|f| f.name == "Ref")
        .unwrap();
    assert_eq!(f.relation_arms.len(), 1, "{:?}", f.relation_arms);
}
