//! A text element with `TextType = BigText`: its variable is a BigText, an export writes its whole text and an import fills
//! it without the 2048-character limit of a Text (corpus xmlports/shapes-properties/XmlPortBigText.al:178, :198, :218).

use al_testkit::run;

const SRC: &str = r#"
table 69980 "XB Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
codeunit 69981 "XB Log"
{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}
xmlport 69982 "XB Export"
{
    Direction = Export; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema { textelement(Root) { textelement(Big) { TextType = BigText; } } }
    trigger OnPreXmlPort() begin Big.AddText('a<b'); Big.AddText('cd'); end;
}
xmlport 69983 "XB Import"
{
    Direction = Import; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema { textelement(Root) { textelement(Big) { TextType = BigText; } } }
    trigger OnPostXmlPort() var L: Codeunit "XB Log"; begin L.Add(Format(Big.Length()) + '/' + Format(Big)); end;
}
codeunit 69984 C
{
    procedure Export(): Text
    var B: Record "XB Blob" temporary; OutS: OutStream; InS: InStream; T: Text; Line: Text;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        Xmlport.Export(Xmlport::"XB Export", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line; end;
        exit(T);
    end;

    procedure Import(): Text
    var B: Record "XB Blob" temporary; L: Codeunit "XB Log"; OutS: OutStream; InS: InStream; Long: TextBuilder; i: Integer;
    begin
        for i := 1 to 3000 do Long.Append('x');
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<Root><Big>' + Long.ToText() + '</Big></Root>');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"XB Import", InS);
        exit(CopyStr(L.Take(), 1, 6));
    end;
}
"#;

#[test]
fn an_export_writes_the_text_of_a_bigtext_element() {
    assert_eq!(
        al_testkit::text(run(SRC, "Export")),
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?><Root>  <Big>a&lt;bcd</Big></Root>"
    );
}

#[test]
fn an_import_fills_a_bigtext_element_past_2048_characters() {
    assert_eq!(al_testkit::text(run(SRC, "Import")), "3000/x");
}
