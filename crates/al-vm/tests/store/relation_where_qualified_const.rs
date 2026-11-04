//! A TableRelation where() whose constant is written with `::`: `const("Enum"::Value)` on an Enum field of the related table,
//! `const(Database::"Table")` and `filter(Database::A | Database::B)` on an Integer field, also inside an if/else chain
//! (al-runner issue 6999).

use al_testkit::text;

const SRC: &str = r#"
enum 69890 "Rel Status" { Extensible = true; value(0; Open) { } value(1; Certified) { } value(2; Closed) { } }
table 69890 "Rel Header"
{
    fields { field(1; "Code"; Code[20]) { } field(2; Status; Enum "Rel Status") { } field(3; TabNo; Integer) { } }
    keys { key(PK; "Code") { Clustered = true; } }
}
table 69891 "Rel Line"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Tmpl; Code[20]) { TableRelation = "Rel Header" where(Status = const("Rel Status"::Certified)); }
        field(3; DocNo; Code[20]) { TableRelation = "Rel Header"."Code" where(Status = const("Rel Status"::Open)); }
        field(4; TabConst; Code[20]) { TableRelation = "Rel Header"."Code" where(TabNo = const(Database::"Rel Line")); }
        field(5; TabFilter; Code[20]) { TableRelation = "Rel Header"."Code" where(TabNo = filter(Database::"Rel Line" | Database::"Rel Header")); }
        field(6; Typ; Option) { OptionMembers = Option,Decimal,Other; }
        field(7; Feature; Code[20])
        {
            TableRelation = if (Typ = const(Option)) "Rel Header"."Code" where(TabNo = const(Database::"Rel Line"), Status = const(Certified))
            else if (Typ = const(Decimal)) "Rel Header"."Code" where(TabNo = filter(Database::"Rel Line" | Database::"Rel Header"));
        }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
codeunit 69890 C
{
    local procedure Seed()
    var H: Record "Rel Header";
    begin
        H."Code" := 'CERT-LINE'; H.Status := H.Status::Certified; H.TabNo := Database::"Rel Line"; H.Insert();
        H."Code" := 'OPEN-HEAD'; H.Status := H.Status::Open; H.TabNo := Database::"Rel Header"; H.Insert();
        H."Code" := 'CLOSED-OTHER'; H.Status := H.Status::Closed; H.TabNo := 18; H.Insert();
    end;

    [TryFunction]
    local procedure TryValidate(var L: Record "Rel Line"; FieldNo: Integer; Value: Code[20])
    begin
        case FieldNo of
            2: L.Validate(Tmpl, Value);
            3: L.Validate(DocNo, Value);
            4: L.Validate(TabConst, Value);
            5: L.Validate(TabFilter, Value);
            7: L.Validate(Feature, Value);
        end;
    end;

    local procedure R(var L: Record "Rel Line"; FieldNo: Integer; Value: Code[20]): Text
    begin
        if TryValidate(L, FieldNo, Value) then
            exit('ok ');
        exit('err ');
    end;

    procedure Run(): Text
    var L: Record "Rel Line"; Out: Text;
    begin
        Seed();
        Out := R(L, 2, 'CERT-LINE') + R(L, 2, 'OPEN-HEAD') + R(L, 3, 'OPEN-HEAD') + R(L, 3, 'CERT-LINE') + '| ';
        Out += R(L, 4, 'CERT-LINE') + R(L, 4, 'OPEN-HEAD') + R(L, 5, 'CERT-LINE') + R(L, 5, 'OPEN-HEAD') + R(L, 5, 'CLOSED-OTHER') + '| ';
        L.Typ := L.Typ::Option;
        Out += R(L, 7, 'CERT-LINE') + R(L, 7, 'OPEN-HEAD');
        L.Typ := L.Typ::Decimal;
        Out += R(L, 7, 'OPEN-HEAD') + R(L, 7, 'CLOSED-OTHER');
        L.Typ := L.Typ::Other;
        Out += R(L, 7, 'ANYTHING');
        exit(Out.TrimEnd());
    end;
}
"#;

#[test]
fn qualified_consts_in_where_select_the_related_rows() {
    assert_eq!(
        text(al_testkit::run(SRC, "Run")),
        "ok err ok err | ok err ok ok err | ok err ok err ok"
    );
}
