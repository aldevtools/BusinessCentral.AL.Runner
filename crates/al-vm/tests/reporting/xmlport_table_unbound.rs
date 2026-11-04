//! `Unbound` and `Width` on a table element of an Xml port change neither the export nor the import (corpus
//! xmlports/shapes-properties/YXmlPortProps.al:390 BdUnbound_TableElement_ExportAndImport).

use al_testkit::{run, text};

const SRC: &str = r#"
table 69950 "XU Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69951 "XU Row" { fields { field(1; Id; Integer) { } field(2; Name; Text[30]) { } } keys { key(PK; Id) { } } }
xmlport 69952 "XU Port"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema
    {
        textelement(Root)
        {
            tableelement(Rw; "XU Row")
            {
                XmlName = 'Item';
                Unbound = true;
                Width = 12;
                fieldelement(Id; Rw.Id) { }
            }
        }
    }
}
codeunit 69953 C
{
    procedure F(): Text
    var B: Record "XU Blob" temporary; R: Record "XU Row"; Xp: XmlPort "XU Port"; OutS: OutStream; InS: InStream; L: Text; T: Text;
    begin
        R.Id := 1; R.Name := 'one'; R.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        Xp.SetDestination(OutS);
        Xp.Export();
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(L); T += L + '|'; end;
        Clear(B);
        B.Data.CreateOutStream(OutS);
        OutS.WriteText('<Root><Item><Id>31</Id></Item><Item><Id>32</Id></Item></Root>');
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XU Port", InS);
        R.FindSet();
        repeat T += Format(R.Id) + '/' + R.Name + ';'; until R.Next() = 0;
        exit(T);
    end;
}
"#;

#[test]
fn unbound_and_width_on_a_table_element_are_ignored() {
    assert_eq!(
        text(run(SRC, "F")),
        "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root>|  <Item>|    <Id>1</Id>|  </Item>|</Root>|1/one;31/;32/;"
    );
}
