//! The bytes a VariableText export writes per `TextEncoding` (corpus xmlports/W7XmlPortTests.al:345
//! XmlPortExport_BytesPerDeclaredEncoding: MSDOS and no TextEncoding write `é` as 82, Windows as E9, UTF16 with FF FE;
//! xmlports/import/XpTEncoding.al:90-160 for `€`, `ü`, `ß` and a character Windows lacks, `?`). An
//! import into such a port is refused, unless its root is a table element (XpTEncoding.al:165-205, the root error).

use al_testkit::run;

fn src(encoding: &str, text: &str) -> String {
    format!(
        r#"
table 69970 "XE Row" {{ fields {{ field(1; Id; Integer) {{ }} field(2; Txt; Text[30]) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69971 "XE Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
xmlport 69972 "XE Port"
{{
    Format = VariableText; Direction = Both; UseRequestPage = false; {encoding}
    schema {{ tableelement(R; "XE Row") {{ fieldelement(Id; R.Id) {{ }} fieldelement(Txt; R.Txt) {{ }} }} }}
}}
codeunit 69973 C
{{
    procedure Bytes(): Text
    var Row: Record "XE Row"; B: Record "XE Blob" temporary; OutS: OutStream; InS: InStream; Byte: Byte; I: Integer; T: Text;
    begin
        Row.Id := 1; Row.Txt := '{text}'; Row.Insert();
        B.Data.CreateOutStream(OutS);
        Xmlport.Export(Xmlport::"XE Port", OutS);
        B.Data.CreateInStream(InS);
        while not InS.EOS() do begin InS.Read(Byte); I := Byte; T += Format(I) + ' '; end;
        exit(T);
    end;

    procedure Import(): Text
    var B: Record "XE Blob" temporary; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::Windows);
        OutS.WriteText('1,a');
        B.Data.CreateInStream(InS, TextEncoding::Windows);
        Xmlport.Import(Xmlport::"XE Port", InS);
        exit('imported');
    end;
}}
"#
    )
}

fn bytes(encoding: &str, text: &str) -> String {
    al_testkit::text(run(&src(encoding, text), "Bytes"))
}

#[test]
fn the_export_writes_the_declared_encoding() {
    assert_eq!(bytes("TextEncoding = MSDOS;", "xé"), "34 49 34 44 34 120 130 34 ");
    assert_eq!(bytes("", "xé"), "34 49 34 44 34 120 130 34 ");
    assert_eq!(bytes("TextEncoding = WINDOWS;", "xé"), "34 49 34 44 34 120 233 34 ");
    assert_eq!(bytes("TextEncoding = WINDOWS;", "a€"), "34 49 34 44 34 97 128 34 ");
    assert_eq!(bytes("TextEncoding = WINDOWS;", "a✓b"), "34 49 34 44 34 97 63 98 34 ");
    assert_eq!(
        bytes("TextEncoding = UTF16;", "xé"),
        "255 254 34 0 49 0 34 0 44 0 34 0 120 0 233 0 34 0 "
    );
}

#[test]
fn an_import_of_a_port_whose_root_is_a_table_element_fails_whatever_the_encoding() {
    match run(&src("TextEncoding = WINDOWS;", "x"), "Import") {
        Err(al_builtins::native::Signal::Error(e)) => assert_eq!(e.code, "XmlPortInvalidNodeType"),
        other => panic!("{other:?}"),
    }
}
