//! An Enum from an enum of another type, and from an Integer, as the probe of corpus
//! tests/al-language/types/PXEnumRecordRef.al (codeunit 69185, corpus at
//! ae3fe88, written against a real BC 27.5 service tier) pins it. The procedures below are those probes with the observer replaced
//! by `Error` on a difference; each cites its probe lines. The compile errors (AL0122) are tested in
//! `al-sema/src/alcodes.rs`.

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const OBJECTS: &str = "
enum 69185 \"PX Src Compat\" { Extensible = true; AssignmentCompatibility = true; value(0; None) { } value(1; One) { } value(7; Seven) { } value(9; Nine) { } }
enum 69186 \"PX Dst Compat\" { Extensible = true; AssignmentCompatibility = true; value(0; None) { } value(1; Uno) { } value(2; Dos) { } value(9; Nueve) { } }
enum 69187 \"PX Dst Plain\" { Extensible = true; value(0; None) { } value(1; Uno) { } value(2; Dos) { } }
table 69185 \"PX Row\" { fields { field(1; Code; Code[20]) { } field(2; Kind; Enum \"PX Dst Compat\") { } field(3; Plain; Enum \"PX Dst Plain\") { }
  field(4; Info; Text[30]) { } field(5; Amount; Decimal) { } }
  keys { key(PK; Code, Kind) { Clustered = true; } key(Sec; Info) { } } }
";

fn compile(src: String) -> (al_sema::workspace::Workspace, al_sema::lower::Lowered) {
    al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    )
}

fn run(tests: &str) -> Vec<(String, Outcome)> {
    let (ws, l) = compile(format!("{OBJECTS} codeunit 69190 T {{ Subtype = Test; {tests} }}"));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    run_codeunit(&engine, &l, oi)
}

fn all_pass(tests: &str) {
    for (name, outcome) in run(tests) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

// PXEnumRecordRef.al:74-108 (EnumToEnum_TargetAllowsAssignment_Ordinals).
#[test]
fn an_enum_of_another_type_converts_by_its_ordinal() {
    all_pass(
        "[Test] procedure Ordinals() var S: Enum \"PX Src Compat\"; D: Enum \"PX Dst Compat\"; begin
             S := S::One; D := S;
             if Format(D.AsInteger()) <> '1' then Error('one ordinal'); if Format(D) <> 'Uno' then Error('one text %1', Format(D));
             S := S::Nine; D := S;
             if Format(D.AsInteger()) <> '9' then Error('nine ordinal'); if Format(D) <> 'Nueve' then Error('nine text');
             S := S::None; D := S;
             if Format(D.AsInteger()) <> '0' then Error('none ordinal'); if Format(D) <> 'None' then Error('none text');
             S := S::Seven; D := S;
             if Format(D.AsInteger()) <> '7' then Error('seven ordinal'); if '[' + Format(D) + ']' <> '[7]' then Error('seven text');
             if Format(D = S) <> 'Yes' then Error('seven equals source');
         end;",
    );
}

// PXEnumRecordRef.al:110-135 (EnumToEnum_InFieldAssignmentAndKey).
#[test]
fn an_enum_of_another_type_in_a_field_a_key_and_a_filter() {
    all_pass(
        "[Test] procedure FieldAndKey() var Row: Record \"PX Row\"; Found: Record \"PX Row\"; S: Enum \"PX Src Compat\"; begin
             Row.DeleteAll(); Row.Code := 'A'; Row.Kind := Row.Kind::Nueve; Row.Insert();
             S := S::Nine;
             if not Found.Get('A', S) then Error('get found');
             Row.Kind := S; if Format(Row.Kind.AsInteger()) <> '9' then Error('field assign');
             S := S::Seven;
             if Found.Get('A', S) then Error('lacking ordinal found');
             Row.SetRange(Kind, S); if Row.Count() <> 0 then Error('setrange lacking count');
         end;",
    );
}

// PXEnumRecordRef.al:137-196 (IntegerToEnum_AssignmentKeyAndSetRange).
#[test]
fn an_integer_converts_to_the_enum_value_of_that_ordinal() {
    all_pass(
        "[Test] procedure IntegerToEnum() var Row: Record \"PX Row\"; Found: Record \"PX Row\"; Int: Integer; D: Enum \"PX Dst Compat\"; P: Enum \"PX Dst Plain\"; begin
             Row.DeleteAll(); Row.Code := 'A'; Row.Kind := Row.Kind::Dos; Row.Insert();
             Int := 2;
             D := Enum::\"PX Dst Compat\".FromInteger(Int); if Format(D) <> 'Dos' then Error('frominteger');
             if not Found.Get('A', Int) then Error('get integer found');
             Int := 9; if Found.Get('A', Int) then Error('get integer missing row');
             Row.SetRange(Kind, 2); if Row.Count() <> 1 then Error('setrange literal');
             Int := 2; Row.SetRange(Kind, Int); if Row.Count() <> 1 then Error('setrange variable');
             Int := 5; Row.SetRange(Kind, Int); if Row.Count() <> 0 then Error('setrange lacking');
             if Row.GetFilter(Kind) <> '5' then Error('filter %1', Row.GetFilter(Kind));
             Row.Kind := Int; if Format(Row.Kind.AsInteger()) <> '5' then Error('field lacking');
             P := Enum::\"PX Dst Plain\".FromInteger(1); if Format(P) <> 'Uno' then Error('plain frominteger');
             Int := 9; D := Int; if Format(D) + '/' + Format(D.AsInteger()) <> 'Nueve/9' then Error('declared compat');
             Int := 5; D := Int; if '[' + Format(D) + ']/' + Format(D.AsInteger()) <> '[5]/5' then Error('lacking compat');
             Int := 2; P := Int; if Format(P) + '/' + Format(P.AsInteger()) <> 'Dos/2' then Error('declared plain');
             Int := 7; P := Int; if '[' + Format(P) + ']/' + Format(P.AsInteger()) <> '[7]/7' then Error('lacking plain');
         end;",
    );
}

// The not-found error of a Get keyed by an enum of another type names the caller's value, keyed by an Integer the key's
// enum value (the number for an ordinal it lacks): errors/lt-ms-refusals-138/P138EnumGetNotFound.al:103, :126-128.
#[test]
fn the_not_found_error_of_a_get_keyed_by_another_enum_or_an_integer_names_the_pinned_value() {
    let outcomes = run(
        "[Test] procedure ByEnum() var Found: Record \"PX Row\"; S: Enum \"PX Src Compat\"; begin
             Found.DeleteAll(); S := S::Seven; asserterror Found.Get('A', S);
             if GetLastErrorText() <> 'The PX Row does not exist. Identification fields and values: Code=''A'',Kind=''Seven''' then
                 Error('enum: %1', GetLastErrorText());
         end;
         [Test] procedure ByInteger() var Found: Record \"PX Row\"; I: Integer; begin
             Found.DeleteAll(); I := 5; asserterror Found.Get('A', I);
             if GetLastErrorText() <> 'The PX Row does not exist. Identification fields and values: Code=''A'',Kind=''5''' then
                 Error('integer: %1', GetLastErrorText());
             I := 1; asserterror Found.Get('A', I);
             if GetLastErrorText() <> 'The PX Row does not exist. Identification fields and values: Code=''A'',Kind=''Uno''' then
                 Error('integer: %1', GetLastErrorText());
         end;",
    );
    for (name, outcome) in outcomes {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}
