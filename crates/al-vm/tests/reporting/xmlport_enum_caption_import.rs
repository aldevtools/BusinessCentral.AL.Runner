//! An import without FormatEvaluate reads the caption of an Enum or Option member as that member, for Format = Xml and
//! VariableText (xmlports/import/XpEnumOptionRoundTrip.al:221-257, al-runner issue 7781). The shapes the corpus did not measure keep
//! the error they had before: FixedText, a caption in another case, a port that declares FormatEvaluate = Legacy, a caption two
//! members share, the caption of an enumextension member, another GlobalLanguage, and a field whose captions the member list
//! cannot give (CaptionML, OptionCaptionML).

use al_testkit::{error_message, text};

const SRC: &str = r#"
enum 50100 Dir { Extensible = true; value(0; Both) { Caption = 'Both ways'; } value(1; FromTarget) { Caption = 'Get data update from target'; } }
enum 50107 Twin { Extensible = true; value(0; A) { Caption = 'Same cap'; } value(1; B) { Caption = 'Same cap'; } value(2; C) { Caption = 'Own cap'; } }
enumextension 50108 TwinExt extends Twin { value(50; Ext) { Caption = 'Ext cap'; } }
enum 50109 Ml { value(0; A) { CaptionML = ENU = 'Alpha cap', DEU = 'Alpha de'; } }
table 50101 Row { fields { field(1; Name; Code[20]) { } field(2; Direction; Enum Dir) { } field(3; Tw; Enum Twin) { } field(4; M; Enum Ml) { }
  field(5; Opt; Option) { OptionMembers = X,Y; OptionCaptionML = ENU = 'Ex cap,Why cap', DEU = 'Ix,Yps'; } } keys { key(PK; Name) { } } }
xmlport 50110 XmlMore
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(R; Row) { fieldattribute(Name; R.Name) { }
        fieldattribute(Tw; R.Tw) { Occurrence = Optional; } fieldattribute(M; R.M) { Occurrence = Optional; }
        fieldattribute(Opt; R.Opt) { Occurrence = Optional; } } } }
}
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

    procedure SharedCaption(): Text
    begin
        Import(Xmlport::XmlMore, '<Root><R Name="A" Tw="Same cap" /></Root>');
        exit(Stored());
    end;

    procedure ExtensionCaption(): Text
    begin
        Import(Xmlport::XmlMore, '<Root><R Name="A" Tw="Ext cap" /></Root>');
        exit(Stored());
    end;

    procedure OwnCaptionOfExtendedEnum(): Text
    var R: Record Row;
    begin
        Import(Xmlport::XmlMore, '<Root><R Name="A" Tw="Own cap" /></Root>');
        R.Get('A');
        exit(Format(R.Tw.AsInteger()));
    end;

    procedure GermanCaption(): Text
    begin
        GlobalLanguage(1031);
        Import(Xmlport::XmlLegacy, '<Root><R Name="A" Direction="Both ways" /></Root>');
        exit(Stored());
    end;

    procedure CaptionMlGarbage(): Text
    begin
        Import(Xmlport::XmlMore, '<Root><R Name="A" M="zzz" /></Root>');
        exit(Stored());
    end;

    procedure OptionCaptionMlGarbage(): Text
    begin
        Import(Xmlport::XmlMore, '<Root><R Name="A" Opt="zzz" /></Root>');
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
    for routine in [
        "XmlCaptionOtherCase",
        "DeclaredLegacyCaption",
        "FixedCaption",
        "SharedCaption",
        "ExtensionCaption",
        "OwnCaptionOfExtendedEnum",
        "GermanCaption",
        "CaptionMlGarbage",
    ] {
        let e = error_message(al_testkit::run_in(SRC, "C", routine));
        assert!(e.contains("can't be evaluated into type Enum"), "{routine}: {e}");
    }
    let e = error_message(al_testkit::run_in(SRC, "C", "OptionCaptionMlGarbage"));
    assert!(e.contains("can't be evaluated into type Option"), "{e}");
}
