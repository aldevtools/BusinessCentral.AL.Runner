//! An import without FormatEvaluate reads the caption of an Enum or Option member as that member, for Format = Xml and
//! VariableText (xmlports/import/XpEnumOptionRoundTrip.al:221-257, al-runner issue 7781). The shapes the corpus did not measure keep
//! the error they had before: FixedText, a caption in another case, and a port that declares FormatEvaluate = Legacy.

use al_testkit::{error_message, text};

const SRC: &str = r#"
enum 50100 Dir { Extensible = true; value(0; Both) { Caption = 'Both ways'; } value(1; FromTarget) { Caption = 'Get data update from target'; } }
table 50101 Row { fields { field(1; Name; Code[20]) { } field(2; Direction; Enum Dir) { } } keys { key(PK; Name) { } } }
table 50102 Buf { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
xmlport 50103 XmlLegacy
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(R; Row) { fieldattribute(Name; R.Name) { } fieldattribute(Direction; R.Direction) { } } } }
}
xmlport 50104 XmlDeclaredLegacy
{
    Direction = Import; Format = Xml; FormatEvaluate = Legacy; UseRequestPage = false;
    schema { textelement(Root) { tableelement(R; Row) { fieldattribute(Name; R.Name) { } fieldattribute(Direction; R.Direction) { } } } }
}
xmlport 50105 Fixed
{
    Direction = Import; Format = FixedText; UseRequestPage = false;
    schema { textelement(Root) { tableelement(R; Row) { fieldelement(Name; R.Name) { Width = 1; } fieldelement(Direction; R.Direction) { Width = 27; } } } }
}
codeunit 50106 C
{
    local procedure Import(XpId: Integer; Doc: Text)
    var B: Record Buf temporary; OutS: OutStream; InS: InStream;
    begin
        B.Data.CreateOutStream(OutS);
        OutS.WriteText(Doc);
        B.Data.CreateInStream(InS);
        Xmlport.Import(XpId, InS);
    end;

    local procedure Stored(): Text
    var R: Record Row;
    begin
        R.Get('A');
        exit(Format(R.Direction.AsInteger()));
    end;

    procedure XmlCaption(): Text
    begin
        Import(Xmlport::XmlLegacy, '<Root><R Name="A" Direction="Get data update from target" /></Root>');
        exit(Stored());
    end;

    procedure XmlCaptionOtherCase(): Text
    begin
        Import(Xmlport::XmlLegacy, '<Root><R Name="A" Direction="get data update from target" /></Root>');
        exit(Stored());
    end;

    procedure DeclaredLegacyCaption(): Text
    begin
        Import(Xmlport::XmlDeclaredLegacy, '<Root><R Name="A" Direction="Get data update from target" /></Root>');
        exit(Stored());
    end;

    procedure FixedCaption(): Text
    begin
        Import(Xmlport::Fixed, 'AGet data update from target');
        exit(Stored());
    end;
}
"#;

#[test]
fn an_xml_import_without_format_evaluate_reads_the_caption() {
    assert_eq!(text(al_testkit::run_in(SRC, "C", "XmlCaption")), "1");
}

#[test]
fn shapes_outside_the_measured_ones_keep_the_error() {
    for routine in ["XmlCaptionOtherCase", "DeclaredLegacyCaption", "FixedCaption"] {
        let e = error_message(al_testkit::run_in(SRC, "C", routine));
        assert!(e.contains("can't be evaluated into type Enum"), "{routine}: {e}");
    }
}
