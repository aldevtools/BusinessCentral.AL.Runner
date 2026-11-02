//! `FieldRef.Relation` of the system fields: SystemId, SystemCreatedAt and SystemModifiedAt relate to nothing, SystemCreatedBy and
//! SystemModifiedBy to the User table 2000000120. Every literal is what real BC answered in the corpus:
//! fieldref/lt-y67-system-field-relation/Y67SystemFieldRelation.al:71-126 (codeunit 93633, writer Y67).

use al_testkit::text;

const SRC: &str = r#"
table 50199 SF
{
    fields
    {
        field(1; Id; Integer) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

codeunit 50199 C
{
    procedure Relations(): Text
    var
        RecRef: RecordRef;
        Txt: Text;
        No: Integer;
    begin
        RecRef.Open(Database::SF);
        for No := 2000000000 to 2000000004 do
            Txt += Format(RecRef.Field(No).Relation) + ' ';
        exit(Txt);
    end;
}
"#;

#[test]
fn the_relation_of_each_system_field() {
    assert_eq!(text(al_testkit::run(SRC, "Relations")), "0 0 2000000120 0 2000000120 ");
}
