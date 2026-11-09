//! A table field named like a built-in Record method (al-runner issue 6162: a field `RecordID` of type RecordId next to
//! `RecordId()`). A call with `()` is the method; the bare name is the field. Real BC compiles the reporter's app
//! (hard rule 6); no corpus test names a field like a Record method that is called.

use al_testkit::text;

const SRC: &str = r#"
table 69790 "FNM Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; RecordID; RecordId) { }
        field(3; "Option"; Option) { OptionMembers = A,B; }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69791 C
{
    procedure Assign(): Text
    var MyRec: Record "FNM Row";
    begin
        MyRec.Id := 7;
        MyRec.Insert();
        MyRec.RecordID := MyRec.RecordId();
        MyRec.Modify();
        MyRec.Get(7);
        exit(Format(MyRec.RecordID = MyRec.RecordId()) + ' ' + Format(MyRec.RecordID));
    end;

    procedure Chain(): Text
    var MyRec: Record "FNM Row";
    begin
        MyRec.Id := 8;
        MyRec.Insert();
        exit(FieldName(MyRec.RecordId().GetRecord().Field(MyRec.FieldNo("Option"))));
    end;

    local procedure FieldName(FieldReference: FieldRef): Text
    begin
        exit(FieldReference.Name);
    end;
}
"#;

#[test]
fn record_id_call_is_the_method_when_a_field_is_named_record_id() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Assign")), "Yes FNM Row: 7");
}

#[test]
fn record_id_call_chain_is_the_method_when_a_field_is_named_record_id() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "Chain")), "Option");
}
