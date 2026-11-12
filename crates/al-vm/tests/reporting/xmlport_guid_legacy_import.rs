//! An XmlPort without FormatEvaluate imports a text into a Guid field as corpus pull request 1340 measured
//! (xmlports/import/XpLegacyGuidImport.al), under the assumption `xmlport-guid-legacy-import` (al-runner issue 7854). The
//! texts that file does not measure at a site, and FixedText, keep main's refusal.

use al_builtins::native::Signal;
use al_testkit::text;

const SRC: &str = r#"
table 50320 GRow { fields { field(1; Id; Integer) { } field(2; Ref; Guid) { } field(3; Ref2; Guid) { } } keys { key(PK; Id) { } } }
table 50322 Buf { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }
xmlport 50321 XmlImp
{
    Direction = Import; Format = Xml; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Row; GRow) { fieldattribute(Id; Row.Id) { } fieldattribute(Ref; Row.Ref) { Occurrence = Optional; } fieldelement(Ref2; Row.Ref2) { MinOccurs = Zero; } } } }
}
xmlport 50323 VtImp
{
    Direction = Import; Format = VariableText; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Row; GRow) { fieldelement(Id; Row.Id) { } fieldelement(Ref; Row.Ref) { } } } }
}
xmlport 50325 FtImp
{
    Direction = Import; Format = FixedText; UseRequestPage = false;
    schema { textelement(Root) { tableelement(Row; GRow) { fieldelement(Id; Row.Id) { Width = 1; } fieldelement(Ref; Row.Ref) { Width = 38; } } } }
}
codeunit 50324 C
{
    procedure Import(XpId: Integer; Doc: Text): Text
    var B: Record Buf temporary; R: Record GRow; OutS: OutStream; InS: InStream;
    begin
        R.DeleteAll();
        B.Data.CreateOutStream(OutS);
        OutS.WriteText(Doc);
        B.Data.CreateInStream(InS);
        Xmlport.Import(XpId, InS);
        R.Get(1);
        exit(Format(R.Ref) + '/' + Format(R.Ref2));
    end;
    procedure Attr(RefText: Text): Text begin exit(Import(Xmlport::XmlImp, '<Root><Row Id="1" Ref="' + RefText + '" /></Root>')); end;
    procedure Elem(RefText: Text): Text begin exit(Import(Xmlport::XmlImp, '<Root><Row Id="1"><Ref2>' + RefText + '</Ref2></Row></Root>')); end;
    procedure Vt(RefText: Text): Text begin exit(Import(Xmlport::VtImp, '"1","' + RefText + '"')); end;
    procedure Ft(RefText: Text): Text begin exit(Import(Xmlport::FtImp, '1' + RefText)); end;
"#;

const G: &str = "{6F9619FF-8B86-D011-B42D-00C04FC964FF}";
const G2: &str = "{0A1B2C3D-4E5F-6789-ABCD-EF0123456789}";
const NULL: &str = "{00000000-0000-0000-0000-000000000000}";
const INVALID: &str = "Invalid format of GUID string. The correct format of the GUID string is: CDEF7890-ABCD-0123-1234-567890ABCDEF where 0-9, A-F symbolizes hexadecimal digits.";

/// The import of `t` by `routine` fails with the InvalidGuidFormat error (XpLegacyGuidImport.al:226-227).
fn assert_invalid(routine: &str, t: &str) {
    match call(routine, t) {
        Err(Signal::Error(e)) => {
            assert_eq!(e.message, INVALID, "{routine} {t}");
            assert_eq!(e.code, "InvalidGuidFormat", "{routine} {t}");
        }
        other => panic!("{routine} {t}: {other:?}"),
    }
}

/// Runs `C.<routine>(arg)` through a wrapper procedure.
fn call(routine: &str, arg: &str) -> Result<al_builtins::value::Value, Signal> {
    let src = format!("{SRC}    procedure Probe(): Text begin exit({routine}('{arg}')); end;\n}}\n");
    al_testkit::run_in(&src, "C", "Probe")
}

#[test]
fn attribute_forms_as_measured() {
    // XpLegacyGuidImport.al:203-224.
    for (t, want) in [
        (G, G),
        (NULL, NULL),
        ("{6f9619ff-8b86-d011-b42d-00c04fc964ff}", G),
        ("6F9619FF-8B86-D011-B42D-00C04FC964FF", G),
        ("6f9619ff-8b86-d011-b42d-00c04fc964ff", G),
        ("(6F9619FF-8B86-D011-B42D-00C04FC964FF)", G),
        ("6F9619FF8B86D011B42D00C04FC964FF", G),
        (" {6F9619FF-8B86-D011-B42D-00C04FC964FF} ", G),
        ("", NULL),
    ] {
        assert_eq!(text(call("Attr", t)), format!("{want}/{NULL}"), "{t}");
    }
    // :225-230.
    assert_invalid("Attr", "not-a-guid");
    assert_invalid("Attr", "{6F9619FF-8B86-D011-B42D-00C04FC964F}");
}

#[test]
fn element_and_variable_text_forms_as_measured() {
    // XpLegacyGuidImport.al:240-253.
    for t in [
        G2,
        "{0a1b2c3d-4e5f-6789-abcd-ef0123456789}",
        "0A1B2C3D-4E5F-6789-ABCD-EF0123456789",
    ] {
        assert_eq!(text(call("Elem", t)), format!("{NULL}/{G2}"), "{t}");
    }
    assert_invalid("Elem", "");
    assert_invalid("Elem", "not-a-guid");
    // :263-276.
    for t in [
        G,
        "{6f9619ff-8b86-d011-b42d-00c04fc964ff}",
        "6F9619FF-8B86-D011-B42D-00C04FC964FF",
    ] {
        assert_eq!(text(call("Vt", t)), format!("{G}/{NULL}"), "{t}");
    }
    assert_invalid("Vt", "");
    assert_invalid("Vt", "not-a-guid");
}

#[test]
fn unmeasured_texts_and_fixed_text_keep_mains_refusal() {
    for (routine, t) in [
        // Measured in an attribute only.
        ("Elem", "6f9619ff-8b86-d011-b42d-00c04fc964ff"),
        ("Elem", "(6F9619FF-8B86-D011-B42D-00C04FC964FF)"),
        ("Vt", "6F9619FF8B86D011B42D00C04FC964FF"),
        ("Vt", "{6F9619FF-8B86-D011-B42D-00C04FC964F}"),
        // Measured nowhere.
        ("Attr", "{6F9619FF-8b86-D011-B42D-00C04FC964FF}"),
        ("Attr", "(6f9619ff-8b86-d011-b42d-00c04fc964ff)"),
        ("Attr", "{6F9619FF-8B86-D011-B42D-00C04FC964FF)"),
        // FixedText is not measured.
        ("Ft", G),
    ] {
        match call(routine, t) {
            Err(Signal::Unsupported(why)) => assert!(why.contains("field of type Guid"), "{routine} {t}: {why}"),
            other => panic!("{routine} {t}: {other:?}"),
        }
    }
}
