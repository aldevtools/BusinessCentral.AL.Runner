//! `Format(Record, Length)` with a length above 0 cuts the record's text to it
//! (errors/lt-ms-refusals-133/P133DateEvaluateFormat.al:338-361 `length 5`, `length 20`).

use al_testkit::{run, text};

const SRC: &str = r#"
table 60050 "FL Row"
{
    fields { field(1; Id; Integer) { } field(2; Name; Text[20]) { } field(3; Qty; Integer) { } field(4; Ok; Boolean) { } }
    keys { key(PK; Id) { } }
}
codeunit 50180 C
{
    procedure F(): Text
    var
        Rec: Record "FL Row";
    begin
        Rec.Id := 1;
        exit(Format(Rec, 3) + '|' + Format(Rec, 6) + '|' + Format(Rec, 0));
    end;
}
"#;

#[test]
fn a_length_cuts_the_text_of_a_record() {
    assert_eq!(text(run(SRC, "F")), "1\t\t|1\t\t0\tN|1\t\t0\tNo");
}
