//! Texts added to an XmlDocument. Pinned: a non-blank text raises the Argument error (xml/TestProbeXmlContracts.al:268,
//! xml/XmlDocReadWrite.al:556, misc/lt-probes-needed-1/PnAXmlNodeAdds.al:38). An empty or blank text is not pinned
//! (.NET takes white space at document level) and is refused.

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
fn an_empty_or_blank_text_into_a_document_is_refused() {
    for body in [
        "Doc := XmlDocument.Create('', ''); exit('ok');",
        "Doc.Add(XmlText.Create('')); exit('ok');",
        "Doc.Add(XmlElement.Create('r')); Doc.Add(XmlText.Create('  ')); exit('ok');",
    ] {
        let r = run(body);
        assert!(matches!(&r, Err(Signal::Unsupported(_))), "{body}: {r:?}");
    }
}
