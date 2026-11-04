//! DrillDown on a FlowField control without a trigger or a control DrillDownPageId opens the DrillDownPageId of the
//! CalcFormula's table (record/table-properties/XTablePropsTests.al:646-658). Which rows and filters that page gets is not
//! pinned, so the drill-down is refused, not run on a guessed view and not answered with the error of a plain field.

use al_builtins::native::Signal;

const SRC: &str = "table 50100 T { DrillDownPageId = L; fields { field(1; Id; Integer) { } field(2; Cnt; Integer) { FieldClass = FlowField; CalcFormula = count(T); } } keys { key(PK; Id) { } } } \
    page 50101 L { PageType = List; SourceTable = T; layout { area(content) { repeater(R) { field(IdC; Rec.Id) { } } } } } \
    page 50102 P { PageType = Card; SourceTable = T; layout { area(content) { field(CntC; Rec.Cnt) { } } } } \
    codeunit 50100 C { procedure F() var R: Record T; TP: TestPage P; begin R.Id := 1; R.Insert(); TP.OpenEdit(); TP.CntC.DrillDown(); end; }";

#[test]
fn drill_down_on_a_flowfield_control_is_refused() {
    match al_testkit::run_in(SRC, "C", "F") {
        Err(Signal::Unsupported(why)) => assert!(why.contains("FlowField"), "{why}"),
        other => panic!("expected a refusal, got {other:?}"),
    }
}
