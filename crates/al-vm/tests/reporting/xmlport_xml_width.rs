//! A Width on a field attribute or field element of an Xml port changes nothing in the export: no padding, no cut (corpus
//! xmlport/lt-ay-xmlport/AyXmlPortAttributeWidth.al:199 Width_Xml_FieldAttribute_Export).

use al_testkit::{run, text};

const SRC: &str = r#"
table 69960 "XW Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69961 "XW Row" { fields { field(1; Code; Code[20]) { } field(2; Name; Text[30]) { } } keys { key(PK; Code) { } } }
xmlport 69962 "XW Port"
{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Rw; "XW Row")
            {
                XmlName = 'Row';
                fieldattribute(Code; Rw.Code) { Width = 3; }
                fieldelement(Name; Rw.Name) { Width = 3; }
            }
        }
    }
}
codeunit 69963 C
{
    procedure F(): Text
    var B: Record "XW Blob" temporary; R: Record "XW Row"; Xp: XmlPort "XW Port"; OutS: OutStream; InS: InStream; L: Text; T: Text;
    begin
        R.Code := 'ABCDEFG'; R.Name := 'longer name'; R.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        Xp.SetDestination(OutS);
        Xp.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(L); T += L + '|'; end;
        exit(T);
    end;
}
"#;

#[test]
fn a_width_on_an_xml_attribute_or_element_is_ignored() {
    assert_eq!(
        text(run(SRC, "F")),
        "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root>|  <Row Code=\"ABCDEFG\">|    <Name>longer name</Name>|  </Row>|</Root>|"
    );
}
