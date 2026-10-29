//! `XmlDocument.ReadFrom(InStream, XmlReadOptions, var XmlDocument)` (al-runner issue 6049).
//! PreserveWhitespace true: xml/XmlDocReadWrite.al:290-307 ReadFrom_InStream_Options. PreserveWhitespace false and a
//! document that does not parse: writer TJ's branch al-runtime-rs-staging-pub-tj, xml/XmlDocReadWrite.al:330-389
//! (assumption xml-readfrom-stream-options).

use al_builtins::assumptions::take_hits;
use al_builtins::native::Signal;
use al_builtins::value::Value;

const TABLE: &str = r#"
table 60008 "ALT Blob"
{
    fields
    {
        field(1; "Code"; Code[20]) { }
        field(3; Data; Blob) { }
    }
    keys { key(PK; "Code") { } }
}
"#;

/// Writes `body` (`~` stands for a line feed) to a blob, reads it with the stream overload with options and
/// PreserveWhitespace `preserve`, and returns `ok|children|WriteTo` or, when the read answers false,
/// `No|GetLastErrorText`. `statement` uses the statement form.
fn read(body: &str, preserve: bool, statement: bool) -> Result<Value, Signal> {
    let call = if statement {
        "XmlDocument.ReadFrom(I, Opt, D); Ok := true;"
    } else {
        "Ok := XmlDocument.ReadFrom(I, Opt, D);"
    };
    let src = format!(
        "{TABLE}\ncodeunit 50100 C {{
         procedure F(): Text var B: Record \"ALT Blob\"; O: OutStream; I: InStream; D: XmlDocument; R: XmlElement;
           Opt: XmlReadOptions; Ok: Boolean; Lf: Text[1]; T: Text;
         begin Lf[1] := 10; B.Data.CreateOutStream(O); O.WriteText('{body}'.Replace('~', Lf)); B.Data.CreateInStream(I);
         Opt.PreserveWhitespace({preserve}); {call}
         if not Ok then exit('No|' + GetLastErrorText());
         D.GetRoot(R); D.WriteTo(T); exit(Format(Ok) + '|' + Format(R.GetChildNodes().Count()) + '|' + T); end; }}"
    );
    al_testkit::run_in(&src, "C", "F")
}

fn text(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(s)) => s.to_string(),
        other => panic!("{other:?}"),
    }
}

#[test]
fn preserve_whitespace_true_reads_as_the_stream_form_does() {
    // xml/XmlDocReadWrite.al:290-307 ReadFrom_InStream_Options: pinned on master, no assumption.
    take_hits();
    assert_eq!(
        text(read("<a>~ <b/></a>", true, false)),
        "Yes|2|<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n<a>\r\n <b /></a>"
    );
    assert!(take_hits().is_empty());
}

#[test]
fn preserve_whitespace_false_keeps_the_whitespace_nodes() {
    // Writer TJ's branch, xml/XmlDocReadWrite.al:331-370 ReadFrom_InStream_Options_WhitespaceNodes: true and false
    // both give three children and the same text.
    for preserve in [true, false] {
        take_hits();
        assert_eq!(
            text(read("<a>~  <b/>~</a>", preserve, false)),
            "Yes|3|<?xml version=\"1.0\" encoding=\"utf-16\"?>\r\n<a>\r\n  <b />\r\n</a>"
        );
        assert!(take_hits().is_empty(), "preserve {preserve}");
    }
}

#[test]
fn invalid_document_answers_false_with_the_error_text() {
    // Writer TJ's branch, xml/XmlDocReadWrite.al:371-389 ReadFrom_InStream_Options_Invalid.
    take_hits();
    assert_eq!(
        text(read("<a><b></a>", true, false)),
        "No|The following exception was encountered when processing XML data: 'The 'b' start tag on line 1 \
         position 5 does not match the end tag of 'a'. Line 1, position 9.' at line 1 and position 9."
    );
    assert!(take_hits().is_empty());
}

#[test]
fn invalid_document_in_the_statement_form_is_refused() {
    // Not pinned for the overload with options.
    assert!(matches!(read("<a><b></a>", true, true), Err(Signal::Unsupported(_))));
}
