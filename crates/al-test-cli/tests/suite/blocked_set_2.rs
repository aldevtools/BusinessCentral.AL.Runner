//! Behaviours that Microsoft's tests reach and that the corpus does not pin yet (rule 3 exception): each test cites
//! the Microsoft code that needs it; the assumption registry (crates/al-builtins/src/assumptions.rs) lists it.

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const OBJECTS: &str = "
enum 69300 \"CV Kind\" { Extensible = true; value(0; None) { } value(1; One) { } value(2; Two) { } }
table 69300 \"CV Row\" { fields { field(1; Code; Code[20]) { } field(7; Number; Integer) { } field(2; Kind; Option) { OptionMembers = A,B,C; }
  field(3; Flag; Boolean) { } field(4; Pick; Enum \"CV Kind\") { } field(9; Total; Integer) { FieldClass = FlowField; CalcFormula = count(\"CV Row\"); } }
  keys { key(PK; Code) { Clustered = true; } } }
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
    let (ws, l) = compile(format!("{OBJECTS} codeunit 69301 T {{ Subtype = Test; {tests} }}"));
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

// Sales Line SetLoadFieldsForInvDiscoundCalculation passes the FlowField "Reserved Quantity" to SetLoadFields, in about
// 2,350 Microsoft tests, and real BC runs it (record/lt-priority-assumptions/PriorityAssumptions.al:282-312 pins that SetLoadFields accepts it).
#[test]
fn set_load_fields_accepts_a_flow_field() {
    all_pass(
        "[Test] procedure FlowFieldInLoadFields() var Row: Record \"CV Row\"; begin
             Row.Code := 'A'; Row.Number := 5; Row.Insert();
             Row.Reset();
             Row.SetLoadFields(Row.Number, Row.Total);
             Row.FindFirst();
             if Row.Number <> 5 then Error('number %1', Row.Number);
         end;",
    );
}

// Mfg. Dimension Management does RecRef.GetTable(RecordVariant) for the temporary Purchase Line of a posting check and
// RecRef.SetTable(PurchaseLine) into a record that is not declared temporary. The target stays a permanent record: it keeps
// its own table and does not read the temporary rows (corpus recordref/lt-settable-temporary/SetTableTemporaryToPermanent.al:
// SetTable_TempIntoPermanent_Insert `perm_temp` No, SetTable_TempIntoPermanent_FilteredSource `perm_count` 0). This test used to
// assume the opposite (registry entry settable-temporary-into-permanent, retired).
#[test]
fn set_table_of_a_temporary_record_ref_into_a_permanent_record_stays_permanent() {
    all_pass(
        "[Test] procedure TemporaryIntoPermanent() var Tmp: Record \"CV Row\" temporary; Target: Record \"CV Row\"; RecRef: RecordRef; begin
             Tmp.Code := 'T'; Tmp.Number := 7; Tmp.Insert();
             RecRef.GetTable(Tmp);
             RecRef.SetTable(Target);
             if Target.Get('T') then Error('read a temporary row');
             if Target.IsTemporary() then Error('temporary');
             if Target.Number <> 7 then Error('number %1', Target.Number);
         end;",
    );
}
