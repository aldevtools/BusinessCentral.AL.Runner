//! `MaxOccurs = Once` on a table element: the export writes the first row only, the import fails on a second element
//! (corpus xmlport/lt-sg-xmlport-occurs/SgXmlPortOccurs.al:229-445 SG_OnceTemp_*, SG_OnceReal_*).

use al_builtins::native::Signal;
use al_testkit::run;

const SRC: &str = r#"
table 69960 "XO Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
table 69961 "XO Row" { fields { field(1; Id; Integer) { } } keys { key(PK; Id) { } } }
xmlport 69962 "XO Port"
{
    Direction = Both; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(T; "XO Row") { MaxOccurs = Once; fieldelement(Id; T.Id) { } } } }
}
codeunit 69963 C
{
    procedure Export(): Text
    var B: Record "XO Blob" temporary; R: Record "XO Row"; OutS: OutStream; InS: InStream; Line: Text; T: Text;
    begin
        R.Id := 1; R.Insert(); R.Id := 2; R.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XO Port", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line + '|'; end;
        exit(T);
    end;

    procedure Import(Doc: Text): Text
    var B: Record "XO Blob" temporary; R: Record "XO Row"; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText(Doc);
        B.Data.CreateInStream(InS);
        Xmlport.Import(Xmlport::"XO Port", InS);
        exit(Format(R.Count()));
    end;

    procedure ImportOne(): Text
    begin
        exit(Import('<Root><T><Id>1</Id></T></Root>'));
    end;

    procedure ImportTwo(): Text
    begin
        exit(Import('<Root><T><Id>1</Id></T><T><Id>2</Id></T></Root>'));
    end;
}
"#;

#[test]
fn the_export_writes_the_first_row_only() {
    assert_eq!(
        al_testkit::text(run(SRC, "Export")),
        "<?xml version=\"1.0\" encoding=\"UTF-16\" standalone=\"no\"?>|<Root>|  <T>|    <Id>1</Id>|  </T>|</Root>|"
    );
}

#[test]
fn the_import_takes_one_element_and_fails_on_a_second() {
    assert_eq!(al_testkit::text(run(SRC, "ImportOne")), "1");
    match run(SRC, "ImportTwo") {
        Err(Signal::Error(e)) => assert_eq!(
            e.message,
            "Imported XML cannot validate with the schema: The element 'Root' has invalid child element 'T'."
        ),
        other => panic!("{other:?}"),
    }
}
