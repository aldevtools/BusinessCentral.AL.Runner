//! `DefaultFieldsValidation` and `FieldValidate` on an import (corpus
//! xmlport/lt-bj-xmlport-default-validation/BjXmlPortDefaultValidation.al:255-334): with the property true or left out,
//! every field element without `FieldValidate = No` is validated (OnValidate runs with xRec already holding the new
//! value, a TableRelation is checked); with false only `FieldValidate = Yes` is. Per field in document order, before
//! OnInsert. Pinned for Format = Xml only: another format is refused where validation would be seen.

use al_builtins::native::Signal;
use al_testkit::run;

fn src(port_props: &str, format: &str, doc: &str) -> String {
    format!(
        r#"
table 69970 "XV Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69971 "XV Rel" {{ fields {{ field(1; Code; Code[10]) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69972 "XV Row"
{{
    fields
    {{
        field(1; Id; Integer) {{ }}
        field(2; Plain; Text[20]) {{ trigger OnValidate() var L: Codeunit "XV Log"; begin L.Add('val Plain=' + Plain + ' xRec=' + xRec.Plain); end; }}
        field(3; Forced; Text[20]) {{ trigger OnValidate() var L: Codeunit "XV Log"; begin L.Add('val Forced=' + Forced); end; }}
        field(4; Skipped; Text[20]) {{ trigger OnValidate() var L: Codeunit "XV Log"; begin L.Add('val Skipped=' + Skipped); end; }}
        field(5; Rel; Code[10]) {{ TableRelation = "XV Rel".Code; }}
    }}
    keys {{ key(PK; Id) {{ }} }}
    trigger OnInsert() var L: Codeunit "XV Log"; begin L.Add('insert ' + Format(Id)); end;
}}
codeunit 69973 "XV Log" {{ SingleInstance = true; var Log: Text; procedure Add(E: Text) begin Log += E + ';'; end; procedure Take(): Text begin exit(Log); end; }}
xmlport 69974 "XV Port"
{{
    Format = {format}; Direction = Import; UseRequestPage = false; {port_props}
    schema
    {{
        textelement(Root)
        {{
            tableelement(Row; "XV Row")
            {{
                fieldattribute(Id; Row.Id) {{ }}
                fieldelement(Plain; Row.Plain) {{ MinOccurs = Zero; }}
                fieldelement(Forced; Row.Forced) {{ MinOccurs = Zero; FieldValidate = Yes; }}
                fieldelement(Skipped; Row.Skipped) {{ MinOccurs = Zero; FieldValidate = No; }}
                fieldelement(Rel; Row.Rel) {{ MinOccurs = Zero; }}
            }}
        }}
    }}
}}
codeunit 69975 C
{{
    procedure F(): Text
    var B: Record "XV Blob" temporary; Rel: Record "XV Rel"; R: Record "XV Row"; L: Codeunit "XV Log"; OutS: OutStream; InS: InStream; T: Text;
    begin
        Rel.Code := 'R1'; Rel.Insert();
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('{doc}');
        B.Data.CreateInStream(InS);
        if not TryImport(InS) then T := 'error ' + GetLastErrorText() + ';';
        if R.FindSet() then repeat T += Format(R.Id) + ':' + R.Plain + ':' + R.Rel + ','; until R.Next() = 0;
        exit(L.Take() + T);
    end;
    [TryFunction] local procedure TryImport(InS: InStream) begin Xmlport.Import(Xmlport::"XV Port", InS); end;
}}
"#
    )
}

const GOOD: &str = "<Root><Row Id=\"1\"><Plain>p1</Plain><Forced>f1</Forced><Skipped>s1</Skipped><Rel>R1</Rel></Row><Row Id=\"2\"><Plain>p2</Plain><Skipped>s2</Skipped></Row></Root>";
const BAD_REL: &str = "<Root><Row Id=\"3\"><Plain>p3</Plain><Rel>NOPE</Rel></Row></Root>";

#[test]
fn true_and_unset_validate_every_field_element_but_field_validate_no() {
    let want = "val Plain=p1 xRec=p1;val Forced=f1;insert 1;val Plain=p2 xRec=p2;insert 2;1:p1:R1,2:p2:,";
    for props in ["DefaultFieldsValidation = true;", ""] {
        assert_eq!(al_testkit::text(run(&src(props, "Xml", GOOD), "F")), want, "{props}");
    }
}

#[test]
fn false_validates_only_field_validate_yes() {
    let got = al_testkit::text(run(&src("DefaultFieldsValidation = false;", "Xml", GOOD), "F"));
    assert_eq!(got, "val Forced=f1;insert 1;insert 2;1:p1:R1,2:p2:,");
}

#[test]
fn a_value_the_related_table_lacks_fails_the_import_when_validated_and_is_stored_when_not() {
    let got = al_testkit::text(run(&src("", "Xml", BAD_REL), "F"));
    assert_eq!(
        got,
        "val Plain=p3 xRec=p3;error The field Rel of table XV Row contains a value (NOPE) that cannot be found in the related table (XV Rel).;"
    );
    let got = al_testkit::text(run(&src("DefaultFieldsValidation = false;", "Xml", BAD_REL), "F"));
    assert_eq!(got, "insert 3;3:p3:NOPE,");
}

#[test]
fn validation_on_another_format_is_refused() {
    let s = src(
        "DefaultFieldsValidation = false; FieldSeparator = ';';",
        "VariableText",
        "1;p1;f1;s1;R1",
    );
    assert!(
        matches!(run(&s, "F"), Err(Signal::Unsupported(r)) if r.contains("not pinned")),
        "refused"
    );
}
