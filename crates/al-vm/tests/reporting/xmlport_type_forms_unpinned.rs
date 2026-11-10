//! The cells of the XmlPort type forms that corpus xmlports/PWXmlPortFormatEvaluateMoreTypes.al does not pin are refused:
//! an import into a port with a Blob field whose element is not in the document (Import_Xml_Blob_MediaAndMediaSet :683-700
//! pins only documents that hold the element, `XmlOne` :526-529), and an export of a DateFormula under a language other than
//! 1033 (Export_Xml_PopulatedRow runs under 1033).

use al_builtins::native::Signal;
use al_testkit::{run_in, text};

const SRC: &str = r#"
table 50930 "TF Row"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Bl; Blob) { }
        field(3; Df; DateFormula) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}
table 50933 "TF Blob" { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { Clustered = true; } } }
xmlport 50931 "TF Imp Blob"
{
    Format = Xml;
    Direction = Import;
    FormatEvaluate = Xml;
    schema
    {
        textelement(Root)
        {
            tableelement(T; "TF Row")
            {
                fieldelement(Id; T.Id) { MinOccurs = Zero; }
                fieldelement(Bl; T.Bl) { MinOccurs = Zero; }
            }
        }
    }
}
xmlport 50932 "TF Exp Df"
{
    Format = Xml;
    Direction = Export;
    FormatEvaluate = Xml;
    Encoding = UTF8;
    schema
    {
        textelement(Root)
        {
            tableelement(T; "TF Row")
            {
                fieldelement(Id; T.Id) { }
                fieldelement(Df; T.Df) { }
            }
        }
    }
}
codeunit 50930 C
{
    local procedure Import(Content: Text): Text
    var Row: Record "TF Row"; TempBlob: Record "TF Blob"; OutStr: OutStream; InStr: InStream; Imported: Boolean;
    begin
        TempBlob.Data.CreateOutStream(OutStr, TextEncoding::UTF8);
        OutStr.WriteText(Content);
        TempBlob.Data.CreateInStream(InStr, TextEncoding::UTF8);
        Imported := TryImport(InStr);
        exit(StrSubstNo('imported=%1 error=[%2] code=[%3] rows=%4', Imported, GetLastErrorText(), GetLastErrorCode(), Row.Count()));
    end;

    [TryFunction]
    local procedure TryImport(var InStr: InStream)
    begin
        Xmlport.Import(Xmlport::"TF Imp Blob", InStr);
    end;

    procedure ElementInDocument(): Text
    begin
        exit(Import('<?xml version="1.0" encoding="utf-8"?><Root><T><Id>1</Id><Bl></Bl></T></Root>'));
    end;

    procedure ElementAbsent(): Text
    begin
        exit(Import('<?xml version="1.0" encoding="utf-8"?><Root><T><Id>1</Id></T></Root>'));
    end;

    procedure NoRow(): Text
    begin
        exit(Import('<?xml version="1.0" encoding="utf-8"?><Root/>'));
    end;

    local procedure Export(Language: Integer): Text
    var Row: Record "TF Row"; TempBlob: Record "TF Blob"; OutStr: OutStream; InStr: InStream; Line: Text; All: Text;
    begin
        Row.Id := 1;
        Evaluate(Row.Df, '<-1W>');
        Row.Insert();
        GlobalLanguage(Language);
        TempBlob.Data.CreateOutStream(OutStr, TextEncoding::UTF8);
        Xmlport.Export(Xmlport::"TF Exp Df", OutStr);
        TempBlob.Data.CreateInStream(InStr, TextEncoding::UTF8);
        while not InStr.EOS() do begin InStr.ReadText(Line); All += Line; end;
        exit(All);
    end;

    procedure ExportEnglish(): Text begin exit(Export(1033)); end;
    procedure ExportDanish(): Text begin exit(Export(1030)); end;
}
"#;

fn refused(routine: &str) -> String {
    match run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(why)) => why,
        other => panic!("{routine}: expected a refusal, got {other:?}"),
    }
}

// Import_Xml_Blob_MediaAndMediaSet `blob_blank`.
#[test]
fn a_blob_element_in_the_document_fails_the_import() {
    assert_eq!(
        text(run_in(SRC, "C", "ElementInDocument")),
        "imported=No error=[The type BLOB is not supported.] code=[NotSupportedType] rows=0"
    );
}

#[test]
fn a_document_without_the_blob_element_is_refused() {
    for r in ["ElementAbsent", "NoRow"] {
        let why = refused(r);
        assert!(why.contains("not in the document"), "{r}: {why}");
    }
}

// Export_Xml_PopulatedRow under 1033.
#[test]
fn a_date_formula_exports_under_1033() {
    let xml = text(run_in(SRC, "C", "ExportEnglish"));
    assert!(xml.contains("<Df>-1W</Df>"), "{xml}");
}

#[test]
fn a_date_formula_export_under_another_language_is_refused() {
    let why = refused("ExportDanish");
    assert!(why.contains("DateFormula under the language 1030"), "{why}");
}
