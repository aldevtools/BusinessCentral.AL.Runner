//! Two FlowFields whose conditions filter on each other's FlowField, across two tables (al-runner issue 7623). Each source row's
//! condition FlowField is calculated with the other table's formula, which calculates the first one again: BC's error text
//! for that cycle is not pinned, so the runner refuses it. It used to recurse until the native stack overflowed, which
//! ended the test thread.

use al_builtins::native::Signal;

const SRC: &str = r#"
table 69610 "FC A"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Hits; Integer) { FieldClass = FlowField; CalcFormula = count("FC B" where(Hits = filter(> 0))); }
    }
    keys { key(PK; Id) { } }
}
table 69611 "FC B"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Hits; Integer) { FieldClass = FlowField; CalcFormula = count("FC A" where(Hits = filter(> 0))); }
    }
    keys { key(PK; Id) { } }
}
codeunit 69612 C
{
    procedure F(): Integer
    var A: Record "FC A"; B: Record "FC B";
    begin
        A.Id := 1; A.Insert();
        B.Id := 1; B.Insert();
        A.CalcFields(Hits);
        exit(A.Hits);
    end;
}
"#;

#[test]
fn a_flowfield_cycle_across_two_tables_is_refused() {
    match al_testkit::run_in(SRC, "C", "F") {
        Err(Signal::Unsupported(text)) => {
            assert!(text.contains("needs itself through the condition FlowField"), "{text}")
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}
