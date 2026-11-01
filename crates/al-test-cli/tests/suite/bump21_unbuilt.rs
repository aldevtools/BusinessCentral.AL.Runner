//! Behaviours pinned by corpus bump 21 that were not built yet (lane bump21-unbuilt).

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const OBJECTS: &str = "
table 69420 \"B21 Alpha\" { fields { field(1; Id; Integer) { } field(2; Txt; Text[20]) { } } keys { key(PK; Id) { Clustered = true; } } }
table 69421 \"B21 Beta\" { fields { field(1; Code; Code[10]) { } } keys { key(PK; Code) { Clustered = true; } } }
";

fn run(tests: &str) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![(
                "f.al".into(),
                format!("{OBJECTS} codeunit 69429 T {{ Subtype = Test; {tests} }}"),
            )],
        }],
        &Default::default(),
    );
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

// types/lt-variant-record-reassign/VariantRecordReassign.al:130 Variant_Reassign_ReadBack: a Variant that held a record of
// another table cannot be read into a record variable of this table.
#[test]
fn variant_of_another_table_does_not_read_into_a_record() {
    all_pass(
        "[Test] procedure Mismatch() var A: Record \"B21 Alpha\"; B: Record \"B21 Beta\"; A2: Record \"B21 Alpha\"; V: Variant; begin
             A.Id := 7; B.Code := 'BCODE'; V := A; V := B;
             asserterror A2 := V;
             if GetLastErrorCode() <> 'NotSupportedOperation' then Error('code %1', GetLastErrorCode());
             if GetLastErrorText() <> 'The requested operation is not supported.' then Error('text %1', GetLastErrorText());
             if A2.Id <> 0 then Error('changed');
         end;",
    );
}
