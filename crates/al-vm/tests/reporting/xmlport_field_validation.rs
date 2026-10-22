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

// platform/lt-ag-contoso/AgXmlPortImport.al:678 (QX_VariableText_TextElementEvaluateAndShortLines, xmlport 87567): on a
// VariableText port with DefaultFieldsValidation = false, a field element without FieldValidate is not validated (the Iso
// field's OnValidate never runs). The same port with the property true or left out is not pinned.
fn text_src(port_props: &str) -> String {
    format!(
        r#"
table 69980 "XT Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69981 "XT Row"
{{
    fields
    {{
        field(1; Code; Code[10]) {{ }}
        field(2; Iso; Code[3]) {{ trigger OnValidate() begin Validated := true; end; }}
        field(3; Validated; Boolean) {{ }}
    }}
    keys {{ key(PK; Code) {{ }} }}
}}
xmlport 69982 "XT Port"
{{
    Format = VariableText; Direction = Import; UseRequestPage = false; FieldSeparator = ';'; FieldDelimiter = '<None>'; {port_props}
    schema {{ textelement(Root) {{ tableelement(Cur; "XT Row") {{ fieldelement(cc; Cur.Code) {{ }} fieldelement(IsoCode; Cur.Iso) {{ }} }} }} }}
}}
codeunit 69983 C
{{
    procedure F(): Text
    var B: Record "XT Blob" temporary; R: Record "XT Row"; OutS: OutStream; InS: InStream; T: Text;
    begin
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('AED;784');
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XT Port", InS);
        if R.FindSet() then repeat T += R.Code + ':' + R.Iso + ':' + Format(R.Validated) + ','; until R.Next() = 0;
        exit(T);
    end;
}}
"#
    )
}

#[test]
fn a_variable_text_port_with_default_fields_validation_false_does_not_validate() {
    assert_eq!(
        al_testkit::text(run(&text_src("DefaultFieldsValidation = false;"), "F")),
        "AED:784:No,"
    );
    for props in ["DefaultFieldsValidation = true;", ""] {
        assert!(
            matches!(run(&text_src(props), "F"), Err(Signal::Unsupported(r)) if r.contains("not pinned")),
            "{props}"
        );
    }
}

// AgXmlPortImport.al:678 (xmlport 87567): a VariableText record that ends early leaves its lacking `MinOccurs = Zero` field
// elements as initialized, and one value too many is left out. A record lacking a field element without MinOccurs = Zero, or with two values too many, is not pinned.
fn short_src(doc: &str) -> String {
    format!(
        r#"
table 69984 "XS Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69985 "XS Row" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; N; Integer) {{ }} field(3; D; Text[5]) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
xmlport 69986 "XS Port"
{{
    Format = VariableText; Direction = Import; UseRequestPage = false; FieldSeparator = ';'; FieldDelimiter = '<None>'; DefaultFieldsValidation = false;
    schema {{ textelement(Root) {{ tableelement(R; "XS Row") {{ fieldelement(C; R.Code) {{ }} fieldelement(N; R.N) {{ MinOccurs = Zero; }} fieldelement(D; R.D) {{ MinOccurs = Zero; }} }} }} }}
}}
codeunit 69987 C
{{
    procedure F(): Text
    var B: Record "XS Blob" temporary; R: Record "XS Row"; OutS: OutStream; InS: InStream; T: Text; Lf: Text; Ch: Char;
    begin
        Ch := 10; Lf += Ch;
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('{doc}'.Replace('#', Lf));
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XS Port", InS);
        if R.FindSet() then repeat T += R.Code + ':' + Format(R.N) + ':' + R.D + ','; until R.Next() = 0;
        exit(T);
    end;
}}
"#
    )
}

#[test]
fn a_variable_text_record_of_another_length() {
    assert_eq!(al_testkit::text(run(&short_src("A;1;02:02#"), "F")), "A:1:02:02,");
    assert_eq!(al_testkit::text(run(&short_src("A;1"), "F")), "A:1:,");
    assert_eq!(al_testkit::text(run(&short_src("A;1;x;extra#"), "F")), "A:1:x,");
    assert!(matches!(run(&short_src("A;1;x;e1;e2"), "F"), Err(Signal::Unsupported(r)) if r.contains("not pinned")));
}

/// A port whose fields arrive as attributes (corpus pages/lt-report-page-properties/06-xmlport-fvat.al:7-30, :107-152).
fn attr_src(port_props: &str, txt_props: &str) -> String {
    format!(
        r#"
table 69980 "XA Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69981 "XA Row"
{{
    fields
    {{
        field(1; Id; Integer) {{ }}
        field(2; Txt; Text[20]) {{ trigger OnValidate() var L: Codeunit "XA Log"; begin L.Add('val Txt=' + Txt + ' xRec=' + xRec.Txt); end; }}
        field(3; Qty; Integer) {{ trigger OnValidate() var L: Codeunit "XA Log"; begin L.Add('val Qty=' + Format(Qty)); end; }}
    }}
    keys {{ key(PK; Id) {{ }} }}
}}
codeunit 69982 "XA Log" {{ SingleInstance = true; var Log: Text; procedure Add(E: Text) begin Log += E + ';'; end; procedure Take(): Text begin exit(Log); end; }}
xmlport 69983 "XA Port"
{{
    Format = Xml; Direction = Import; UseRequestPage = false; {port_props}
    schema
    {{
        textelement(Root)
        {{
            tableelement(Row; "XA Row")
            {{
                fieldattribute(Id; Row.Id) {{ }}
                fieldattribute(Txt; Row.Txt) {{ {txt_props} }}
                fieldattribute(Qty; Row.Qty) {{ FieldValidate = No; }}
            }}
        }}
    }}
}}
codeunit 69984 C
{{
    procedure F(): Text
    var B: Record "XA Blob" temporary; R: Record "XA Row"; L: Codeunit "XA Log"; OutS: OutStream; InS: InStream; T: Text;
    begin
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Root><Row Id="21" Txt="t21" Qty="5"/></Root>');
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XA Port", InS);
        if R.FindSet() then repeat T += Format(R.Id) + ':' + R.Txt + ':' + Format(R.Qty) + ','; until R.Next() = 0;
        exit(L.Take() + T);
    end;
}}
"#
    )
}

#[test]
fn a_field_attribute_is_validated_with_field_validate_yes_and_not_with_no() {
    // 06-xmlport-fvat.al:118-119: only Txt (FieldValidate = Yes) is validated, Qty (No) is stored as it is.
    let r = run(&attr_src("", "FieldValidate = Yes;"), "F");
    assert_eq!(al_testkit::text(r), "val Txt=t21 xRec=t21;21:t21:5,");
}

#[test]
fn a_field_attribute_without_field_validate_or_under_default_fields_validation_false_stays_refused() {
    for (port, txt) in [("", ""), ("DefaultFieldsValidation = false;", "FieldValidate = Yes;")] {
        let r = run(&attr_src(port, txt), "F");
        assert!(
            matches!(&r, Err(Signal::Unsupported(m)) if m.contains("field attribute Txt")),
            "{port} {txt}: {r:?}"
        );
    }
}
