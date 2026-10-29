//! Texts added to an XmlDocument. Pinned: a non-blank text raises the Argument error (xml/TestProbeXmlContracts.al:268,
//! xml/XmlDocReadWrite.al:556, misc/lt-probes-needed-1/PnAXmlNodeAdds.al:38). An empty or blank text is not pinned
//! (.NET takes white space at document level) runs under the assumption `xml-doc-blank-text`; a second one is refused.

use al_builtins::native::Signal;
use al_testkit::text;

fn run(body: &str) -> Result<al_builtins::value::Value, Signal> {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var Doc: XmlDocument; S: Text; begin {body} end; }}");
    al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v)
}

const ARG: &str = "Exception of type 'Microsoft.Dynamics.Nav.Types.Exceptions.NavNCLArgumentException' was thrown.";

#[test]
fn a_non_blank_text_into_a_document_raises_the_argument_error() {
    let r = run("Doc.Add(XmlElement.Create('r'));
        asserterror Doc.Add(XmlText.Create('text at doc level')); S := GetLastErrorText() + '/' + GetLastErrorCode();
        exit(S);");
    assert_eq!(text(r), format!("{ARG}/Argument"));
}

#[test]
fn an_empty_or_blank_text_into_a_document_runs_and_a_second_is_refused() {
    // misc/lt-probes-needed-1/PnAXmlNodeAdds.al TT_EmptyOrBlankTextIntoXmlDocument on corpus writer TT's branch
    // al-runtime-rs-staging-tt (assumption `xml-doc-blank-text`): an empty Text adds no node; a blank Text or XmlText is kept
    // before or after the root and written as it is.
    // Expected texts: PnAXmlNodeAdds.al:284-291 on that branch.
    for (body, want) in [
        (
            "Doc := XmlDocument.Create('', ''); Doc.Add(XmlElement.Create('root')); Doc.WriteTo(S); exit(S);",
            "<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n<root />",
        ),
        (
            "Doc.Add(XmlText.Create('')); Doc.Add(XmlElement.Create('root')); Doc.WriteTo(S); exit(S);",
            "<?xml version=\"1.0\" encoding=\"utf-16\"?><root />",
        ),
        (
            "Doc.Add(XmlElement.Create('r')); Doc.Add(XmlText.Create(' ')); Doc.WriteTo(S); exit(S);",
            "<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n<r /> ",
        ),
    ] {
        assert_eq!(text(run(body)), want, "{body}");
    }
    let r =
        run("Doc.Add(XmlText.Create(' ')); Doc.Add(XmlElement.Create('r')); Doc.Add(XmlText.Create(' ')); exit('ok');");
    assert!(matches!(&r, Err(Signal::Unsupported(_))), "{r:?}");
}
