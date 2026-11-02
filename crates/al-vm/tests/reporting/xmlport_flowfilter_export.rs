//! An Xml export of a field element bound to a FlowFilter writes the field's own value (its type's default), whatever filter the
//! table view holds on it (corpus xmlport/lt-xmlport-flowfield-sum-count/XmlPortFlowFieldSumCount.al:390-442
//! Export_Xml_SumAndCountFlowFields_Utf16, Export_Xml_FlowFilterOnView_Utf16, Export_Xml_FlowFilterRangeOnView_Utf16:
//! `<SeqFilter>0</SeqFilter>`).

use al_testkit::{run, text};

const SRC: &str = r#"
table 69973 "XFE Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69970 "XFE Parent"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; "Seq Filter"; Integer) { FieldClass = FlowFilter; }
    }
    keys { key(PK; Id) { } }
}
xmlport 69971 "XFE Port"
{
    Direction = Export; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Parent; "XFE Parent")
            {
                fieldelement(Id; Parent.Id) { }
                fieldelement(SeqFilter; Parent."Seq Filter") { }
            }
        }
    }
}
codeunit 69972 C
{
    procedure F(): Text
    var P: Record "XFE Parent"; Xp: XmlPort "XFE Port"; B: Record "XFE Blob" temporary; OutS: OutStream; InS: InStream; T: Text; L: Text;
    begin
        B.Code := 'X'; B.Insert(); P.Id := 1; P.Insert();
        P.SetRange("Seq Filter", 2);
        Xp.SetTableView(P);
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        Xp.SetDestination(OutS);
        Xp.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin
            InS.ReadText(L);
            T += L + '|';
        end;
        exit(T);
    end;
}
"#;

#[test]
fn an_export_writes_the_default_value_of_a_flowfilter_element() {
    assert_eq!(
        text(run(SRC, "F")),
        "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root>|  <Parent>|    <Id>1</Id>|    <SeqFilter>0</SeqFilter>|  </Parent>|</Root>|"
    );
}
