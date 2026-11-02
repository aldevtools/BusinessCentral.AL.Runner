//! `TextType = BigText` on a text attribute of a table element: its variable is a BigText, an import fills it and an export
//! writes it (corpus report/lt-u-report-props/UReportObjects.al:150, UPropertyTests.al:311-330 `bigLength=8 big=bigvalue`,
//! :399-400 `BigAttr="big"`). TextType on a text attribute of a root text element, or the value Text there, is not pinned.

use al_builtins::native::Signal;
use al_testkit::run;

fn src(port_schema: &str) -> String {
    format!(
        r#"
table 69990 "XBA Row" {{ fields {{ field(1; Id; Integer) {{ }} }} keys {{ key(PK; Id) {{ }} }} }}
table 69991 "XBA Blob" {{ fields {{ field(1; Code; Code[10]) {{ }} field(2; Data; Blob) {{ }} }} keys {{ key(PK; Code) {{ }} }} }}
codeunit 69992 "XBA Log"
{{
    SingleInstance = true;
    var Entries: Text;
    procedure Add(E: Text) begin Entries += E + ';'; end;
    procedure Take(): Text var R: Text; begin R := Entries; Entries := ''; exit(R); end;
}}
xmlport 69993 "XBA Port"
{{
    Direction = Both; Format = Xml; Encoding = UTF8; UseRequestPage = false;
    schema {{ {port_schema} }}
}}
codeunit 69994 C
{{
    procedure Import(): Text
    var B: Record "XBA Blob" temporary; L: Codeunit "XBA Log"; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<?xml version="1.0" encoding="utf-8"?><Root><Row Id="10" BigAttr="bigvalue" /></Root>');
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        Xmlport.Import(Xmlport::"XBA Port", InS);
        exit(L.Take());
    end;

    procedure Export(): Text
    var B: Record "XBA Blob" temporary; R: Record "XBA Row"; OutS: OutStream; InS: InStream; T: Text; Line: Text;
    begin
        R.Id := 1; R.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        Xmlport.Export(Xmlport::"XBA Port", OutS);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        while not InS.EOS() do begin InS.ReadText(Line); T += Line; end;
        exit(T);
    end;
}}
"#
    )
}

const IN_TABLE: &str = r#"
textelement(Root)
{
    tableelement(Row; "XBA Row")
    {
        MinOccurs = Zero;
        fieldattribute(Id; Row.Id) { }
        textattribute(BigAttr)
        {
            Occurrence = Optional;
            TextType = BigText;
            trigger OnBeforePassVariable() begin Clear(BigAttr); BigAttr.AddText('big'); end;
            trigger OnAfterAssignVariable() var L: Codeunit "XBA Log"; begin L.Add(Format(BigAttr.Length()) + '=' + Format(BigAttr)); end;
        }
    }
}"#;

#[test]
fn a_bigtext_attribute_of_a_table_element_imports_and_exports() {
    assert_eq!(al_testkit::text(run(&src(IN_TABLE), "Import")), "8=bigvalue;");
    let out = al_testkit::text(run(&src(IN_TABLE), "Export"));
    assert!(out.contains(r#"<Row Id="1" BigAttr="big" />"#), "{out}");
}

#[test]
fn texttype_on_an_attribute_of_the_root_or_the_value_text_is_refused() {
    for schema in [
        "textelement(Root) { textattribute(V) { TextType = BigText; } }",
        r#"textelement(Root) { tableelement(Row; "XBA Row") { textattribute(V) { TextType = Text; } } }"#,
    ] {
        match run(&src(schema), "Import") {
            Err(Signal::Unsupported(r)) => assert!(r.contains("texttype"), "{r}"),
            other => panic!("{schema}: {other:?}"),
        }
    }
}
