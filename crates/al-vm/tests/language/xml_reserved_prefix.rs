//! The `xml` prefix is bound without a declaration: an attribute under it is written back as it was read
//! (xml/XmlDocReadWrite.al:259-265 ReadFrom_Whitespace_TextNode `xmlspace`).

use al_testkit::text;

#[test]
fn an_xml_prefixed_attribute_is_written_without_a_declaration() {
    let src = "codeunit 50100 C { procedure F(): Text var Doc: XmlDocument; T: Text;
        begin
            XmlDocument.ReadFrom('<a xml:space=\"preserve\">  <b/>  </a>', Doc);
            Doc.WriteTo(T);
            exit(T);
        end; }";
    let r = text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v));
    assert_eq!(
        r,
        "<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n<a xml:space=\"preserve\">  <b />  </a>"
    );
}

// Once the earlier test of its codeunit runs, xml/XmlDocReadWrite.al:409-430 WriteTo_OutStream_RoundTrip runs too: a line end
// read right after the document's own declaration is written on a line of its own.
#[test]
fn a_line_end_after_a_read_declaration_is_written_on_its_own_line() {
    // The stream the corpus test reads back holds this text after its byte order mark; the read is the same.
    let src = "codeunit 50100 C { procedure F(): Text var Doc: XmlDocument; CrLf: Text[2]; T: Text;
        begin
            CrLf[1] := 13;
            CrLf[2] := 10;
            XmlDocument.ReadFrom('<?xml version=\"1.0\" encoding=\"utf-8\"?>' + CrLf + '<a x=\"1\"><b>t</b></a>', Doc);
            Doc.WriteTo(T);
            exit(T);
        end; }";
    let r = text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v));
    assert_eq!(
        r,
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\r\n\r\n<a x=\"1\">\r\n  <b>t</b>\r\n</a>"
    );
}
