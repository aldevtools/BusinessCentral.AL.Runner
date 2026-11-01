//! Regressions found by comparing two full Microsoft runs (main 13988e40 and e3c8f818).

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

const OBJECTS: &str = "
table 69410 \"MR Asset\" { TableType = Temporary; fields { field(1; \"Entry No.\"; Integer) { } field(2; Name; Text[20]) { } }
  keys { key(PK; \"Entry No.\") { Clustered = true; } }
  trigger OnInsert() begin \"Entry No.\" := LastNo(Rec) + 1; end;
  procedure LastNo(Src: Variant): Integer var RR: RecordRef; begin
    ToRef(Src, RR); RR.Reset(); if RR.FindLast() then exit(RR.Field(1).Value); exit(0); end;
  local procedure ToRef(Src: Variant; var RR: RecordRef) begin
    if Src.IsRecordRef then RR := Src else if Src.IsRecord then RR.GetTable(Src) else Error('x'); end; }
";

fn run(tests: &str) -> Vec<(String, Outcome)> {
    let (ws, l) = al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![(
                "f.al".into(),
                format!("{OBJECTS} codeunit 69411 T {{ Subtype = Test; {tests} }}"),
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

// A record passed to a Variant parameter and read through RecordRef.GetTable inside the callee, as Base Application's
// Find Record Management.GetLastEntryIntFieldValue does for "Price Asset".OnInsert: two inserts must get two numbers.
#[test]
fn a_record_passed_as_variant_into_a_record_ref_numbers_the_next_row() {
    all_pass(
        "[Test] procedure Two() var A: Record \"MR Asset\"; begin
             A.Insert(true); A.Name := 'x'; A.Insert(true);
             if A.\"Entry No.\" <> 2 then Error('no %1', A.\"Entry No.\");
         end;",
    );
}
