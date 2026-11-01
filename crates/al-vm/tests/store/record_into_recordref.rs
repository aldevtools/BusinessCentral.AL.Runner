//! A Record assigned to a RecordRef or passed to a by-value RecordRef parameter: the RecordRef holds a copy of the record's
//! table, filters and current row, not shared with the record (misc/lt-cx-odd-leads/CxVariants.al:174-210
//! RecordToRecordRef_Implicit). A temporary record is not pinned and stays refused.

use al_builtins::native::Signal;
use al_testkit::text;

fn run(temp: &str, body: &str) -> Result<al_builtins::value::Value, Signal> {
    let src = format!(
        r#"
table 69984 "RIR Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Name; Text[30]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
codeunit 69985 C
{{
    local procedure RefInfo(RR: RecordRef): Text
    begin
        exit(Format(RR.Number) + '|' + Format(RR.Field(1).Value) + '|' + Format(RR.Count()));
    end;

    procedure F(): Text
    var Row: Record "RIR Row" {temp}; RR: RecordRef; I: Integer;
    begin
        for I := 1 to 4 do begin Row.Id := I; Row.Name := 'n' + Format(I); Row.Insert(); end;
        Row.SetRange(Id, 2, 3);
        Row.FindFirst();
        {body}
    end;
}}
"#
    );
    al_testkit::run_in(&src, "C", "F")
}

#[test]
fn a_record_assigned_to_a_recordref_is_a_copy() {
    let body = "RR := Row; RR.Field(2).Value := 'changed'; exit(RefInfo(RR) + '|' + RR.GetFilters() + '|' + Row.Name);";
    assert_eq!(text(run("", body)), "69984|2|2|Id: 2..3|n2");
}

#[test]
fn a_record_passed_to_a_recordref_parameter() {
    assert_eq!(text(run("", "exit(RefInfo(Row));")), "69984|2|2");
}

#[test]
fn a_temporary_record_stays_refused() {
    assert!(matches!(
        run("temporary", "exit(RefInfo(Row));"),
        Err(Signal::Unsupported(_))
    ));
}
