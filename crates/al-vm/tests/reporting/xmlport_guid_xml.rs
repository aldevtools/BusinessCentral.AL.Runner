//! A Guid field with FormatEvaluate = Xml (assumption `xmlport-guid-xml`; corpus branch al-runtime-rs-staging-pub-ts,
//! xmlport/lt-qr-xmlport-container-occurs/QrXpContainerOccurs.al TS_Guid_*, xmlport 90086 in TsXpObjects.al).

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 95 "GX Row"
{
    fields
    {
        field(1; "Entry No."; Integer) { }
        field(2; G; Guid) { }
        field(3; G2; Guid) { }
    }
    keys { key(PK; "Entry No.") { Clustered = true; } }
}

table 96 "GX Blob"
{
    fields
    {
        field(1; Id; Integer) { }
        field(2; Data; Blob) { }
    }
    keys { key(PK; Id) { Clustered = true; } }
}

xmlport 97 "GX Port"
{
    Format = Xml;
    FormatEvaluate = Xml;
    Encoding = UTF8;
    UseRequestPage = false;
    schema
    {
        textelement(guids)
        {
            MaxOccurs = Once;
            tableelement(row; "GX Row")
            {
                MinOccurs = Zero;
                UseTemporary = true;
                fieldattribute(no; row."Entry No.") { }
                fieldattribute(ga; row.G2) { Occurrence = Optional; }
                fieldelement(g; row.G) { MinOccurs = Zero; }
            }
        }
    }

    procedure AddRow(No: Integer; NewG: Guid; NewG2: Guid)
    begin
        row.Init();
        row."Entry No." := No;
        row.G := NewG;
        row.G2 := NewG2;
        row.Insert();
    end;

    procedure Rows(): Text
    var
        T: Text;
    begin
        if row.FindSet() then
            repeat
                T += Format(row."Entry No.") + ':g=' + Format(row.G) + ' ga=' + Format(row.G2) + ';';
            until row.Next() = 0;
        exit(T);
    end;
}

codeunit 98 C
{
    procedure Import(Body: Text): Text
    var
        B: Record "GX Blob" temporary;
        P: XmlPort "GX Port";
        OutS: OutStream;
        InS: InStream;
    begin
        B.Id := 1;
        B.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText(Body);
        B.Modify();
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        P.SetSource(InS);
        P.Import();
        exit(P.Rows());
    end;

    procedure Element(): Text
    var
        R: Text;
    begin
        R := Import('<guids><row no="1"><g>{5a2f6c9e-1b3d-4e8f-9a0b-1c2d3e4f5a6b}</g></row></guids>');
        R += Import('<guids><row no="1"><g>5A2F6C9E1B3D4E8F9A0B1C2D3E4F5A6B</g></row></guids>');
        R += Import('<guids><row no="1"><g>(5A2F6C9E-1B3D-4E8F-9A0B-1C2D3E4F5A6B)</g></row></guids>');
        R += Import('<guids><row no="1"><g> 5a2f6c9e-1b3d-4e8f-9a0b-1c2d3e4f5a6b </g></row></guids>');
        R += Import('<guids><row no="1"><g></g></row></guids>');
        exit(R);
    end;

    procedure Attribute(): Text
    begin
        exit(Import('<guids><row no="1" ga="5a2f6c9e-1b3d-4e8f-9a0b-1c2d3e4f5a6b"/></guids>') + Import('<guids><row no="1" ga=""/></guids>'));
    end;

    procedure Invalid(): Text
    begin
        exit(Import('<guids><row no="1"><g>not-a-guid</g></row></guids>'));
    end;

    procedure OneShort(): Text
    begin
        exit(Import('<guids><row no="1" ga="5A2F6C9E-1B3D-4E8F-9A0B-1C2D3E4F5A6"/></guids>'));
    end;

    procedure Unpinned(): Text
    begin
        exit(Import('<guids><row no="1"><g>{5A2F6C9E-1B3D-4E8F-9A0B-1C2D3E4F5A6B)</g></row></guids>'));
    end;

    procedure ImportTwice(): Text
    var
        B: Record "GX Blob" temporary;
        P: XmlPort "GX Port";
        OutS: OutStream;
        InS: InStream;
    begin
        B.Id := 1;
        B.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        OutS.WriteText('<guids><row no="1"/></guids>');
        B.Modify();
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        P.SetSource(InS);
        P.Import();
        P.Import();
        exit(P.Rows());
    end;

    procedure Export(): Text
    var
        B: Record "GX Blob" temporary;
        P: XmlPort "GX Port";
        Known: Guid;
        Null: Guid;
        OutS: OutStream;
        InS: InStream;
        Line: Text;
        Xml: Text;
    begin
        Known := '{5A2F6C9E-1B3D-4E8F-9A0B-1C2D3E4F5A6B}';
        P.AddRow(1, Known, Null);
        B.Id := 1;
        B.Insert();
        B.Data.CreateOutStream(OutS, TextEncoding::UTF8);
        P.SetDestination(OutS);
        P.Export();
        B.Modify();
        B.CalcFields(Data);
        B.Data.CreateInStream(InS, TextEncoding::UTF8);
        InS.ReadText(Line);
        while not InS.EOS() do begin
            InS.ReadText(Line);
            Xml += Line.Trim() + '|';
        end;
        exit(Xml);
    end;
}
"#;

const KNOWN: &str = "{5A2F6C9E-1B3D-4E8F-9A0B-1C2D3E4F5A6B}";
const NULL: &str = "{00000000-0000-0000-0000-000000000000}";

fn run(routine: &str) -> Result<al_builtins::value::Value, Signal> {
    al_testkit::run_in(SRC, "C", routine)
}

// TS_Guid_Import_Element: braces or none, any case, the 32 digits only, parentheses, surrounding spaces; empty is the null Guid.
#[test]
fn a_guid_element_imports_its_xml_forms() {
    let one = format!("1:g={KNOWN} ga={NULL};");
    assert_eq!(
        text(run("Element")),
        format!("{one}{one}{one}{one}1:g={NULL} ga={NULL};")
    );
}

// TS_Guid_Import_Attribute.
#[test]
fn a_guid_attribute_imports_its_xml_forms() {
    assert_eq!(
        text(run("Attribute")),
        format!("1:g={NULL} ga={KNOWN};1:g={NULL} ga={NULL};")
    );
}

// TS_Guid_Import_Element `invalid` and TS_Guid_Import_Attribute `invalid`, `one digit short`.
#[test]
fn a_text_that_is_no_guid_fails_with_invalid_guid_format() {
    for routine in ["Invalid", "OneShort"] {
        let e = al_testkit::error_message(run(routine));
        assert!(e.starts_with("Invalid format of GUID string."), "{routine}: {e}");
    }
}

// A form the probe did not try (mismatched brackets) is refused.
#[test]
fn an_unpinned_form_is_refused() {
    assert!(matches!(run("Unpinned"), Err(Signal::Unsupported(_))));
}

// TS_Guid_Export: `{UPPER-CASE}`, the null Guid too.
#[test]
fn a_guid_exports_in_braces_upper_case() {
    assert_eq!(
        text(run("Export")),
        format!("<guids>|<row no=\"1\" ga=\"{NULL}\">|<g>{KNOWN}</g>|</row>|</guids>|")
    );
}

// A second Import on the same instance whose source an earlier Import read to its end (a partner app's SOAP handler imports
// the port it was handed again): what BC does is not pinned, so it is refused rather than failed as an empty file.
#[test]
fn a_second_import_of_a_read_source_is_refused() {
    let r = run("ImportTwice");
    assert!(
        matches!(&r, Err(Signal::Unsupported(m)) if m.contains("second Import")),
        "{r:?}"
    );
}
