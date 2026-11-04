//! The frame names of the AL_PROFILE file (al-runner issue 6924): a field trigger carries its field, as in
//! `table Sales Line.Quantity.OnValidate`, so the triggers of different fields do not merge into one line.

use al_testkit::Built;

const SRC: &str = r#"
table 50600 "Sales Line"
{
    fields
    {
        field(1; "No."; Integer) { }
        field(2; Quantity; Decimal) { trigger OnValidate() begin end; }
        field(3; Amount; Decimal) { trigger OnValidate() begin end; }
    }
    keys { key(PK; "No.") { } }
    trigger OnInsert() begin end;
}
codeunit 50601 "Calc" { procedure Run() begin end; }
"#;

#[test]
fn a_field_trigger_frame_names_its_field() {
    let b = Built::from_source(SRC);
    let program = &b.lowered.program;
    let (t, _) = b.routine_in("Sales Line", "OnInsert");
    let mut names: Vec<String> = (0..program.objects[t].routines.len())
        .map(|r| al_vm::profile_frame_name(program, t, r))
        .collect();
    names.sort();
    for want in [
        "table Sales Line.Quantity.OnValidate",
        "table Sales Line.Amount.OnValidate",
        "table Sales Line.OnInsert",
    ] {
        assert!(names.iter().any(|n| n == want), "{want} not in {names:?}");
    }
    let (c, r) = b.routine_in("Calc", "Run");
    assert_eq!(al_vm::profile_frame_name(program, c, r), "codeunit Calc.Run");
}
