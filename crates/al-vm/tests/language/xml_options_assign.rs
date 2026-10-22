//! The assignment form of `XmlReadOptions.PreserveWhitespace` and `XmlWriteOptions.PreserveWhitespace` (al-runner issue
//! 5647): `Options.PreserveWhitespace := true` is the call `Options.PreserveWhitespace(true)`, whose effect is pinned
//! (xml/XmlDocReadWrite.al:215-217; misc/W4XmlJsonHttp.al:18-19, ReadFrom_InnerWhitespace).

use al_testkit::text;

#[test]
fn preserve_whitespace_assigns_like_the_call_and_read_from_reads_as_without_options() {
    let src = "codeunit 50100 C { procedure F(): Text var R: XmlReadOptions; W: XmlWriteOptions; Doc: XmlDocument; Root: XmlElement; Ok: Boolean;
        begin
            R.PreserveWhitespace := true;
            W.PreserveWhitespace := true;
            Ok := XmlDocument.ReadFrom('<a> <b/> </a>', R, Doc);
            Doc.GetRoot(Root);
            exit(Format(R.PreserveWhitespace()) + ',' + Format(W.PreserveWhitespace()) + ',' + Format(Ok) + ',' + Root.Name());
        end; }";
    assert_eq!(
        text(al_testkit::run_with(src, "C", "F", vec![]).map(|(v, _)| v)),
        "Yes,Yes,Yes,a"
    );
}
