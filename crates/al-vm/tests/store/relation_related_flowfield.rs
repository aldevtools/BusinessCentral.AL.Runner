//! A TableRelation whose where() filters on a FlowField of the related table compares the calculated value of each related row,
//! not the stored one (al-runner issue 6979).

const SRC: &str = r#"
table 50975 "Rf Group"
{
    fields { field(1; Code; Code[20]) { } field(2; "Own Group"; Boolean) { } }
    keys { key(PK; Code) { } }
}
table 50976 "Rf District"
{
    fields
    {
        field(1; Code; Code[20]) { }
        field(2; "Group Code"; Code[20]) { }
        field(10; "Own Group"; Boolean)
        {
            FieldClass = FlowField;
            CalcFormula = lookup("Rf Group"."Own Group" where(Code = field("Group Code")));
        }
    }
    keys { key(PK; Code) { } }
}
table 50977 "Rf Document"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Own True"; Code[20]) { TableRelation = "Rf District" where("Own Group" = const(true)); }
    }
    keys { key(PK; Id) { } }
}
codeunit 50978 C
{
    procedure F(): Text
    var G: Record "Rf Group"; D: Record "Rf District"; Doc: Record "Rf Document"; T: Text;
    begin
        G.Code := 'G1'; G."Own Group" := true; G.Insert();
        G.Code := 'G2'; G."Own Group" := false; G.Insert();
        D.Code := 'AA'; D."Group Code" := 'G1'; D.Insert();
        D.Code := 'BB'; D."Group Code" := 'G2'; D.Insert();
        Doc.Validate("Own True", 'AA');
        T := Doc."Own True";
        asserterror Doc.Validate("Own True", 'BB');
        T += '|' + GetLastErrorText();
        exit(T);
    end;
}
"#;

#[test]
fn a_relation_on_a_flowfield_of_the_related_table_compares_the_calculated_value() {
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "F")),
        "AA|The field Own True of table Rf Document contains a value (BB) that cannot be found in the related table (Rf District)."
    );
}
