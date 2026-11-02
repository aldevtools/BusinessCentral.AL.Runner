//! A TableFilter field in a field element of an Xml port (corpus xmlport/lt-y67-xmlport-tablefilter/Y67XpTableFilter.al:120-171):
//! the export writes its table-qualified text, an empty one as an empty element; the import reads the text as Evaluate does.

const SRC: &str = r#"
table 69910 "XT Row" { fields { field(1; Id; Integer) { } field(2; Filt; TableFilter) { } } keys { key(PK; Id) { } } }
table 69911 "XT Blob" { fields { field(1; Code; Code[10]) { } field(2; Data; Blob) { } } keys { key(PK; Code) { } } }
xmlport 69912 "XT Port"
{
    Format = Xml; Direction = Both; UseRequestPage = false;
    schema { textelement(Root) { tableelement(R; "XT Row") { MinOccurs = Zero; fieldelement(Id; R.Id) { } fieldelement(Filter; R.Filt) { MinOccurs = Zero; } } } }
}
codeunit 69913 C
{
    procedure Export(): Text
    var Row: Record "XT Row"; B: Record "XT Blob" temporary; OutS: OutStream; InS: InStream; Line: Text; R: Text;
    begin
        Row.Id := 1; Evaluate(Row.Filt, 'XT Row: Id=2..5'); Row.Insert();
        Row.Init(); Row.Id := 2; Row.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XT Port", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF16);
        InS.ReadText(Line);
        while not InS.EOS() do begin InS.ReadText(Line); R += Line.Trim(); end;
        exit(R);
    end;

    procedure Import(): Text
    var Row: Record "XT Row"; B: Record "XT Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<Root><R><Id>10</Id><Filter>XT Row: Id=1..9</Filter></R><R><Id>12</Id><Filter /></R></Root>');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"XT Port", InS);
        Row.Get(10);
        exit('[' + Format(Row.Filt) + ']');
    end;
}
"#;

#[test]
fn a_table_filter_field_exports_its_text_and_imports_it() {
    // TableFilterField_Export, TableFilterField_Import `qualified`.
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Export")),
        "<Root><R><Id>1</Id><Filter>XT Row: Id=2..5</Filter></R><R><Id>2</Id><Filter /></R></Root>"
    );
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "Import")),
        "[XT Row: Id=1..9]"
    );
}
