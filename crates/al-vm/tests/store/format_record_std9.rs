//! `Format(Record, 0, 9)` and `Format(RecordRef, 0, 9)` give the text of standard format 0 (a Date stays `03/05/24`), and
//! an unopened RecordRef formats as the empty text with 0 and with 9 (errors/lt-ms-refusals-133/
//! P133DateEvaluateFormat.al:441-463 NP3_FormatRecord_StandardFormats_0And9, :465-498 _Temporary_AndFlowField_0And9,
//! :500-550 NP3_FormatRecordRef_StandardFormats_0And9, :552-566 NP3_FormatRecordRef_NotOpen).

use al_testkit::text;

const SRC: &str = r#"
table 69990 "FR9 T"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Cd; Code[10]) { }
        field(3; Dt; Date) { }
        field(4; Amount; Decimal) { }
        field(5; Ok; Boolean) { }
    }
    keys { key(PK; Id) { } }
}
codeunit 69991 C
{
    procedure Rec9(): Text
    var R: Record "FR9 T";
    begin
        R.Id := 3; R.Cd := 'ABC'; R.Dt := 20240305D; R.Amount := 12.5; R.Ok := true;
        exit(Format(R, 0, 9) + '|' + Format(R, 0, 0));
    end;

    procedure Ref9(): Text
    var R: Record "FR9 T"; RecRef: RecordRef;
    begin
        R.Id := 3; R.Cd := 'ABC'; R.Dt := 20240305D; R.Amount := 12.5;
        RecRef.GetTable(R);
        exit(Format(RecRef, 0, 9) + '|' + Format(RecRef));
    end;

    procedure NotOpen(): Text
    var RecRef: RecordRef;
    begin
        exit('[' + Format(RecRef, 0, 9) + '][' + Format(RecRef) + ']');
    end;
}
"#;

#[test]
fn format_of_a_record_with_standard_format_9_equals_format_0() {
    let t = text(al_testkit::run_in(SRC, "C", "Rec9"));
    assert_eq!(t, "3\tABC\t03/05/24\t12.5\tYes|3\tABC\t03/05/24\t12.5\tYes");
}

#[test]
fn format_of_a_recordref_with_standard_format_9_equals_format_0() {
    let t = text(al_testkit::run_in(SRC, "C", "Ref9"));
    assert_eq!(t, "3\tABC\t03/05/24\t12.5\tNo|3\tABC\t03/05/24\t12.5\tNo");
}

#[test]
fn format_of_an_unopened_recordref_is_empty() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "NotOpen")), "[][]");
}
