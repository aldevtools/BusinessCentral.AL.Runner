//! A RecordId FlowField with `CalcFormula = -lookup(...)` gives the stored RecordId unchanged (al-runner issue 7266). Pinned by
//! corpus pull request 1199 (measured on real BC 27.5, 28.5 and 29.0, not yet on master):
//! record/flowfields/TestSignedRecordIdLookup.al:13 (the formula) and :25 CalcFormula_SignedRecordIdLookup_ReturnsStoredTarget.
//! A sign on a numeric lookup, on min or on max stays refused.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const SRC: &str = r#"
table 69980 "SRL Target" { fields { field(1; "Entry No."; Integer) { } } keys { key(PK; "Entry No.") { Clustered = true; } } }
table 69990 "REC Signed RecordId Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Target; RecordId) { }
        field(3; "Signed Target"; RecordId)
        {
            FieldClass = FlowField;
            CalcFormula = -lookup("REC Signed RecordId Row".Target where(Id = field(Id)));
        }
        field(4; Num; Integer) { }
        field(5; "Signed Num"; Integer)
        {
            FieldClass = FlowField;
            CalcFormula = -lookup("REC Signed RecordId Row".Num where(Id = field(Id)));
        }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69991 T
{
    procedure Signed(): Text
    var
        Row: Record "REC Signed RecordId Row";
        Target: Record "SRL Target";
    begin
        Target."Entry No." := 202;
        Target.Insert(false);
        Row.Id := 1;
        Row.Target := Target.RecordId();
        Row.Insert(false);
        Row.CalcFields("Signed Target");
        if Row."Signed Target" <> Target.RecordId() then
            exit('differs: ' + Format(Row."Signed Target"));
        exit(Format(Row."Signed Target"));
    end;

    procedure SignedNumber(): Integer
    var
        Row: Record "REC Signed RecordId Row";
    begin
        Row.Id := 1;
        Row.Num := 5;
        Row.Insert(false);
        Row.CalcFields("Signed Num");
        exit(Row."Signed Num");
    end;
}
"#;

#[test]
fn a_signed_lookup_of_a_record_id_gives_the_stored_record_id() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "T", "Signed")),
        "SRL Target: 202"
    );
}

#[test]
fn a_signed_lookup_of_a_number_stays_refused() {
    let r: Result<Value, Signal> = al_testkit::run_in(SRC, "T", "SignedNumber");
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}
