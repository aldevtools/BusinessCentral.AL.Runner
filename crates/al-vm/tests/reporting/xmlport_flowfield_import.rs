//! An Xml-format import skips a field element bound to a FlowField: the field keeps its value and the element's
//! OnAfterAssignField does not run (corpus xmlports/shapes-properties/AyXmlPortLookupFlowField.al:Import_AutoCalc, Import_NoAutoCalc
//! for Lookup; xmlports/shapes-properties/XmlPortFlowFieldSumCount.al:326-347 for Sum with OnAfterAssignField, and
//! :350-372 for Sum and Count). Other FlowField methods are refused.

use al_builtins::native::Signal;
use al_testkit::run;

fn src(calc: &str, trigger: &str) -> String {
    format!(
        r#"
table 69960 "XF Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69961 "XF Master" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Name; Text[30]) {{ }} field(3; Amt; Decimal) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
table 69962 "XF Row"
{{
    fields
    {{
        field(1; Id; Integer) {{ }}
        field(2; MasterCode; Code[10]) {{ }}
        field(3; Flow; Text[30]) {{ FieldClass = FlowField; CalcFormula = {calc}; }}
    }}
    keys {{ key(PK; Id) {{ }} }}
}}
xmlport 69963 "XF Port"
{{
    Format = Xml; Direction = Import; UseRequestPage = false;
    schema
    {{
        textelement(Root)
        {{
            tableelement(Rw; "XF Row")
            {{
                XmlName = 'Row';
                fieldelement(Id; Rw.Id) {{ }}
                fieldelement(MasterCode; Rw.MasterCode) {{ }}
                fieldelement(Flow; Rw.Flow) {{ {trigger} }}
                trigger OnBeforeInsertRecord() begin Seen += 'before=' + Format(Rw.Flow) + ';'; end;
            }}
        }}
    }}
    var Seen: Text;
    procedure GetSeen(): Text begin exit(Seen); end;
}}
codeunit 69964 C
{{
    procedure F(): Text
    var B: Record "XF Blob" temporary; R: Record "XF Row"; P: XmlPort "XF Port"; OutS: OutStream; InS: InStream;
    begin
        B.Code := 'X'; B.Insert();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Root><Row><Id>9</Id><MasterCode>A</MasterCode><Flow>Typed</Flow></Row></Root>');
        B.Data.CreateInStream(InS);
        P.SetSource(InS); P.Import();
        R.Get(9);
        exit(P.GetSeen() + R.MasterCode);
    end;
}}
"#
    )
}

const LOOKUP: &str = r#"lookup("XF Master".Name where(Code = field(MasterCode)))"#;

#[test]
fn an_import_does_not_assign_a_lookup_flowfield() {
    assert_eq!(al_testkit::text(run(&src(LOOKUP, ""), "F")), "before=;A");
}

#[test]
fn an_import_does_not_run_the_trigger_of_a_sum_flowfield_element() {
    let s = src(
        r#"sum("XF Master".Amt where(Code = field(MasterCode)))"#,
        "trigger OnAfterAssignField() begin Seen += 'assigned;'; end;",
    )
    .replace("field(3; Flow; Text[30])", "field(3; Flow; Decimal)")
    .replace("<Flow>Typed</Flow>", "<Flow>123.5</Flow>");
    assert_eq!(al_testkit::text(run(&s, "F")), "before=0;A");
}

// AyXmlPortLookupFlowField.al:364 PYD_Import_Xml_ExistMinMaxAverage: an Exist FlowField is skipped too; :402 a text that is
// no Boolean is skipped without an error, :377 one that is no Integer fails the schema check.
#[test]
fn an_import_skips_an_exist_flowfield_and_checks_an_integer_one() {
    let s = src(r#"exist("XF Master" where(Code = field(MasterCode)))"#, "")
        .replace("field(3; Flow; Text[30])", "field(3; Flow; Boolean)")
        .replace("<Flow>Typed</Flow>", "<Flow>maybe</Flow>");
    assert_eq!(al_testkit::text(run(&s, "F")), "before=No;A");
    let s = src(r#"count("XF Master" where(Code = field(MasterCode)))"#, "")
        .replace("field(3; Flow; Text[30])", "field(3; Flow; Integer)")
        .replace("<Flow>Typed</Flow>", "<Flow>many</Flow>");
    match run(&s, "F") {
        Err(Signal::Error(e)) => assert_eq!(
            e.message,
            "Imported XML cannot validate with the schema: The 'Flow' element is invalid - The value 'many' is invalid according \
             to its datatype 'http://www.w3.org/2001/XMLSchema:int' - The string 'many' is not a valid Int32 value."
        ),
        other => panic!("{other:?}"),
    }
}

// xmlports/shapes-properties/XmlPortFlowFilterElement.al:224 Import_FlowFilterElement_AssignedWithoutSave: a field
// element bound to a FlowFilter is skipped too, its OnAfterAssignField does not run, and the import goes on.
#[test]
fn an_import_skips_a_flowfilter_element() {
    let s = src(r#"lookup("XF Master".Name where(Code = field(MasterCode)))"#, "")
        .replace(
            "field(3; Flow; Text[30]) { FieldClass = FlowField; CalcFormula = lookup(\"XF Master\".Name where(Code = field(MasterCode))); }",
            "field(3; Flow; Integer) { FieldClass = FlowFilter; }",
        )
        .replace(
            "fieldelement(Flow; Rw.Flow) {  }",
            "fieldelement(Flow; Rw.Flow) { trigger OnAfterAssignField() begin Seen += 'assigned;'; end; }",
        )
        .replace("<Flow>Typed</Flow>", "<Flow>2</Flow>")
        .replace("Format(Rw.Flow)", "''");
    assert!(s.contains("FlowFilter") && s.contains("'assigned;'"), "{s}");
    assert_eq!(al_testkit::text(run(&s, "F")), "before=;A");
}
