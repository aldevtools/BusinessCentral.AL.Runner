//! `WriteTo` of attributes, document types made without a subset, and `WriteTo(XmlWriteOptions, Text)` (corpus
//! xml/xml/P97AttrWrite.al, xml/lt-xml-remaining-node-methods/P104Write*.al, xml/XmlDocReadWrite.al).

use al_testkit::text;

fn run(body: &str) -> String {
    let src = format!(
        "codeunit 50100 C {{ procedure F(): Text var A: XmlAttribute; Dt: XmlDocumentType; Doc: XmlDocument; Opt: XmlWriteOptions; C: XmlComment; S: Text; T: Text; begin {body} end; }}"
    );
    text(al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v))
}

#[test]
fn an_attribute_on_its_own_is_written_name_equals_value() {
    // P97AttrWrite.al:86 WriteTo_Text_Kinds `t1`, `t3`, `t7`, `newline_tab`.
    let r = run("A := XmlAttribute.Create('a', 'plain'); A.WriteTo(S);
         A := XmlAttribute.Create('a', '<&>\"x'); A.WriteTo(T); S += '|' + T;
         A := XmlAttribute.CreateNamespaceDeclaration('n', 'urn:n'); A.WriteTo(T); exit(S + '|' + T);");
    assert_eq!(r, "a=\"plain\"|a=\"&lt;&amp;&gt;&quot;x\"|xmlns:n=\"urn:n\"");
}

#[test]
fn a_document_type_made_without_a_subset_is_written_without_brackets() {
    // P104WriteDocType.al:146 `v1_m1`, :150 `v2_m1`, :154 `v3_m1`; P104DocTypeCreate.al `doc1`.
    let r = run(
        "Dt := XmlDocumentType.Create('r'); Dt.WriteTo(S);
         Dt := XmlDocumentType.Create('r', 'pub'); Dt.WriteTo(T); S += '|' + T;
         Dt := XmlDocumentType.Create('r', 'pub', 'sys.dtd'); Dt.WriteTo(T); S += '|' + T;
         Doc := XmlDocument.Create(); Doc.Add(XmlDocumentType.Create('r')); Doc.Add(XmlElement.Create('r')); Doc.WriteTo(T);
         exit(S + '|' + T);",
    );
    assert_eq!(
        r,
        "<!DOCTYPE r >|<!DOCTYPE r PUBLIC \"pub\" \"\">|<!DOCTYPE r PUBLIC \"pub\" \"sys.dtd\">|<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n<!DOCTYPE r >\r\n<r />"
    );
}

#[test]
fn write_options_unindent_a_document_and_leave_a_comment_alone() {
    // xml/XmlDocReadWrite.al:392 WriteTo_Options `preserve_true`, `preserve_false`; P104WriteComment.al `m2`.
    let r = run(
        "XmlDocument.ReadFrom('<a><b>1</b><c/></a>', Doc); Opt.PreserveWhitespace(true); Doc.WriteTo(Opt, S);
         Opt.PreserveWhitespace(false); Doc.WriteTo(Opt, T); S += '|' + T;
         C := XmlComment.Create('c'); Opt.PreserveWhitespace(true); C.WriteTo(Opt, T); exit(S + '|' + T);",
    );
    assert_eq!(
        r,
        "<?xml version=\"1.0\" encoding=\"utf-16\"?><a><b>1</b><c /></a>|<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n<a>\r\n  <b>1</b>\r\n  <c />\r\n</a>|<!--c-->"
    );
}

#[test]
fn preserve_whitespace_false_on_new_options_writes_unindented() {
    // P97DocWrite.al:123 WriteTo_TextOptions_Variants `drop2`; XmlDocReadWrite.al:392 `preserve_false` after `true`.
    let r = run(
        "Doc := XmlDocument.Create(); Doc.Add(XmlElement.Create('r', XmlElement.Create('c')));
         Opt.PreserveWhitespace(false); Doc.WriteTo(Opt, S);
         Opt.PreserveWhitespace(true); Opt.PreserveWhitespace(false); Doc.WriteTo(Opt, T); exit(S + '|' + T);",
    );
    assert_eq!(
        r,
        "<?xml version=\"1.0\" encoding=\"utf-16\"?><r><c /></r>|<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n<r>\r\n  <c />\r\n</r>"
    );
}

#[test]
fn attributes_are_removed_by_object_name_or_namespace_and_a_missing_one_is_ignored() {
    // P104AttrCollection.al:31 P104_Remove_Attribute, :126 P104_Remove_ByName_Overloads; AyXmlRemoveNsAttribute.al:71, :146.
    let src = "codeunit 50100 C { procedure F(): Text var Doc: XmlDocument; E: XmlElement; A: XmlAttribute; S: Text; T: Text; begin
        XmlDocument.ReadFrom('<r xmlns:p=\"urn:p\" p:a=\"1\" a=\"3\" b=\"4\"/>', Doc); Doc.GetRoot(E);
        E.Attributes().Get('b', A); E.Attributes().Remove(A); E.RemoveAttribute('zz'); E.Attributes().Remove('missing', 'urn:y');
        E.RemoveAttribute('a', 'urn:p'); asserterror E.RemoveAttribute('p:a'); S := GetLastErrorCode(); E.WriteTo(T);
        exit(S + '|' + T + '|' + A.Value()); end; }";
    let r = text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v));
    assert_eq!(r, "Xml:Xml|<r xmlns:p=\"urn:p\" a=\"3\" />|4");
}

#[test]
fn a_processing_instruction_target_is_checked_and_its_end_marker_is_split() {
    // xml/XmlNodes.al:203-208 `empty_target`, `xml_target`, `space_target`; P90PIRead.al:2110 `pi-end-doc`.
    let r = run(
        "asserterror XmlProcessingInstruction.Create('', 'd'); S := GetLastErrorCode();
         asserterror XmlProcessingInstruction.Create('xml', 'd'); S += '|' + GetLastErrorCode();
         asserterror XmlProcessingInstruction.Create('a b', 'd'); S += '|' + GetLastErrorCode();
         XmlProcessingInstruction.Create('t', '?>').WriteTo(T); exit(S + '|' + T);",
    );
    assert_eq!(r, "Argument|Argument|Xml:Xml|<?t ? >?>");
}

const STREAM: &str = r#"
table 50311 B { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } keys { key(PK; Id) { } } }

codeunit 50312 Z
{
    procedure AttributeOnce(): Integer
    var R: Record B; OutS: OutStream; A: XmlAttribute;
    begin
        R.Data.CreateOutStream(OutS); A := XmlAttribute.Create('a', 'plain'); A.WriteTo(OutS);
        R.CalcFields(Data); exit(R.Data.Length);
    end;

    procedure AttributeTwice(): Integer
    var R: Record B; OutS: OutStream; A: XmlAttribute;
    begin
        R.Data.CreateOutStream(OutS); A := XmlAttribute.Create('a', 'plain'); A.WriteTo(OutS); A.WriteTo(OutS);
        exit(R.Data.Length);
    end;

    procedure DocTypeTwice(): Integer
    var R: Record B; OutS: OutStream; Dt: XmlDocumentType;
    begin
        R.Data.CreateOutStream(OutS); Dt := XmlDocumentType.Create('r'); Dt.WriteTo(OutS); Dt.WriteTo(OutS);
        exit(R.Data.Length);
    end;

    procedure AttributeAfterText(): Integer
    var R: Record B; OutS: OutStream; A: XmlAttribute;
    begin
        R.Data.CreateOutStream(OutS); OutS.WriteText('x'); A := XmlAttribute.Create('a', 'plain'); A.WriteTo(OutS);
        exit(R.Data.Length);
    end;
}
"#;

#[test]
fn an_attribute_written_into_a_fresh_stream_is_its_markup() {
    // P97AttrWrite.al:150 WriteTo_OutStream: `a="plain"` is 9 bytes.
    let r = al_testkit::run_in(STREAM, "Z", "AttributeOnce");
    assert!(matches!(r, Ok(al_builtins::value::Value::Integer(9))), "{r:?}");
}

#[test]
fn a_second_write_of_an_attribute_or_document_type_into_the_same_stream_is_refused() {
    // Only a stream nothing was written to is pinned for these kinds (P97AttrWrite.al:150, P104WriteDocType.al:199):
    // whether a second WriteTo writes over the first or appends is not.
    for routine in ["AttributeTwice", "DocTypeTwice", "AttributeAfterText"] {
        let r = al_testkit::run_in(STREAM, "Z", routine);
        assert!(
            matches!(&r, Err(al_builtins::native::Signal::Unsupported(_))),
            "{routine}: {r:?}"
        );
    }
}
