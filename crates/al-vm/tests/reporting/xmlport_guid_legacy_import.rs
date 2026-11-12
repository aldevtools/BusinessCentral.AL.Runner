//! A Format = Xml import without FormatEvaluate reads into a Guid field only the text its export writes, upper case in braces,
//! under the assumption `xmlport-guid-legacy-import` (al-runner issue 7854). Every other text and VariableText stay refused.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 50320 GRow { fields { field(1; Id; Integer) { } field(2; Ref; Guid) { } } keys { key(PK; Id) { } } }
table 50322 Buf { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
xmlport 50321 XmlImp
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Row; GRow) { fieldelement(Id; Row.Id) { } fieldattribute(Ref; Row.Ref) { } } } }
}
xmlport 50323 VtImp
{
    Direction = Import; Format = VariableText; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Row; GRow) { fieldelement(Id; Row.Id) { } fieldelement(Ref; Row.Ref) { } } } }
}
codeunit 50324 C
{
    local procedure Import(XpId: Integer; Doc: Text): Text
    var B: Record Buf temporary; R: Record GRow; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText(Doc);
        B.Data.CreateInStream(InS);
        Xmlport.Import(XpId, InS);
        R.Get(1);
        exit(Format(R.Ref));
    end;
    procedure ExportForm(): Text begin exit(Import(Xmlport::XmlImp, '<Root><Row Ref="{6F9619FF-8B86-D011-B42D-00C04FC964FF}"><Id>1</Id></Row></Root>')); end;
    procedure LowerCase(): Text begin exit(Import(Xmlport::XmlImp, '<Root><Row Ref="{6f9619ff-8b86-d011-b42d-00c04fc964ff}"><Id>1</Id></Row></Root>')); end;
    procedure NoBraces(): Text begin exit(Import(Xmlport::XmlImp, '<Root><Row Ref="6F9619FF-8B86-D011-B42D-00C04FC964FF"><Id>1</Id></Row></Root>')); end;
    procedure VarText(): Text begin exit(Import(Xmlport::VtImp, '"1","{6F9619FF-8B86-D011-B42D-00C04FC964FF}"')); end;
}
"#;

#[test]
fn the_export_form_of_a_guid_imports() {
    assert_eq!(
        text(al_testkit::run_in(SRC, "C", "ExportForm")),
        "{6F9619FF-8B86-D011-B42D-00C04FC964FF}"
    );
}

#[test]
fn other_guid_texts_and_variable_text_stay_refused() {
    for routine in ["LowerCase", "NoBraces", "VarText"] {
        match al_testkit::run_in(SRC, "C", routine) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("field of type Guid"), "{routine}: {why}"),
            other => panic!("{routine}: {other:?}"),
        }
    }
}
