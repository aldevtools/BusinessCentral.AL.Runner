//! `RecordRef.LoadFields(Fields: Integer*)`: the Record rule with field numbers
//! (fieldref/lt-fieldref-testfield-types/RecordRefLoadFieldsList.al:13-50).

use al_testkit::{run, text};

const SRC: &str = r#"
table 60030 "LF Row"
{
    fields { field(1; Id; Integer) { } field(2; A; Integer) { } field(3; B; Integer) { } field(4; C; Integer) { } field(5; D; Integer) { } }
    keys { key(PK; Id) { } }
}
codeunit 50140 C
{
    procedure F(): Text
    var
        Row: Record "LF Row";
        RecRef: RecordRef;
        R: Text;
    begin
        Row.Id := 1; Row.A := 4; Row.B := 5; Row.Insert();
        RecRef.Open(60030);
        RecRef.LoadFields(2);
        RecRef.FindFirst();
        R := Format(RecRef.AreFieldsLoaded(2)) + Format(RecRef.AreFieldsLoaded(3)) + Format(RecRef.AreFieldsLoaded(2, 3)) + Format(RecRef.AreFieldsLoaded(1));
        RecRef.Close();
        RecRef.Open(60030);
        RecRef.LoadFields(3, 5);
        RecRef.FindFirst();
        R += '|' + Format(RecRef.AreFieldsLoaded(3, 5)) + Format(RecRef.AreFieldsLoaded(4)) + '|' + Format(RecRef.Field(3).Value);
        exit(R);
    end;
}
"#;

#[test]
fn load_fields_takes_field_numbers() {
    assert_eq!(text(run(SRC, "F")), "YesYesYesYes|YesYes|5");
}
