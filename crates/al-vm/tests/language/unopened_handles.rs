//! Neighbors of the cells corpus PR 1408 measured on handles nobody opened or assigned (recordref-fieldref/UhUnopenedRecordRef.al,
//! xml/UhUnassignedXmlElement.al, streams/instream-outstream/UhUntouchedStreams.al): a shape the probe did not measure keeps the
//! answer it had before (a refusal), and a RecordRef closed after it was open keeps the answers main gave a RecordRef not open.
//! The measured cells themselves are the corpus cases.

use al_builtins::native::Signal;
use al_builtins::value::Value;

const TABLE: &str = "table 50100 Row { fields { field(1; Id; Integer) { } field(2; Pic; Media) { } field(3; Pics; MediaSet) { } field(4; Data; Blob) { } }
    keys { key(PK; Id) { Clustered = true; } } }";

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!("{TABLE} codeunit 50101 C {{ {body} }}");
    al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v)
}

fn refused(body: &str) {
    let r = run(body);
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{r:?}");
}

#[test]
fn members_measured_only_on_a_recordref_never_opened_refuse_once_it_was_closed() {
    for call in [
        "B := RR.FindLast();",
        "B := RR.Next() = 0;",
        "T := RR.GetFilters();",
        "KR := RR.KeyIndex(1);",
        "RR.SetRecFilter();",
        "RR.SetLoadFields(1);",
    ] {
        refused(&format!(
            "procedure F(): Text var RR: RecordRef; KR: KeyRef; B: Boolean; T: Text; begin RR.Open(50100); RR.Close(); {call} end;"
        ));
    }
}

#[test]
fn forms_and_arguments_the_probe_did_not_use_refuse_on_a_recordref_never_opened() {
    for call in [
        // Statement forms of members measured with the result used.
        "RR.FindSet();",
        "RR.FindFirst();",
        // Arguments the probe did not pass.
        "KR := RR.KeyIndex(2);",
        "RR.SetLoadFields(1, 2);",
        "B := RR.FindSet(true);",
        // A temporary record as the source of CopyLinks.
        "RR.CopyLinks(Tmp);",
    ] {
        refused(&format!(
            "procedure F(): Text var RR: RecordRef; KR: KeyRef; Tmp: Record Row temporary; B: Boolean; begin {call} end;"
        ));
    }
}

#[test]
fn a_closed_recordref_keeps_the_answers_of_a_recordref_not_open() {
    let r = run("procedure F(): Text
        var
            RR: RecordRef;
            Id: RecordId;
            T: Text;
        begin
            RR.Open(50100);
            RR.Close();
            T := Format(RR.Number()) + '|' + Format(RR.Get(Id)) + '|' + Format(RR) + '|';
            asserterror T := RR.Name();
            T += GetLastErrorCode() + '|';
            RR.Close();
            exit(T + Format(RR.Number()));
        end;");
    assert_eq!(al_testkit::text(r), "0|No||RecordNotOpened|0");
}

#[test]
fn gettable_of_a_variant_refuses_values_and_states_the_probe_did_not_measure() {
    for (setup, value) in [
        ("", "'x'"),
        ("", "6"),
        ("", "1.5"),
        // A RecordRef that was open, closed, and one open.
        ("RR.Open(50100); RR.Close();", "''"),
        ("RR.Open(50100);", "''"),
    ] {
        refused(&format!(
            "procedure F(): Text var RR: RecordRef; V: Variant; begin {setup} V := {value}; RR.GetTable(V); end;"
        ));
    }
}

#[test]
fn keyref_members_with_other_arguments_refuse() {
    refused("procedure F(): Text var KR: KeyRef; FR: FieldRef; begin FR := KR.FieldIndex(2); end;");
}

#[test]
fn xml_element_members_in_forms_the_probe_did_not_use_refuse() {
    for call in [
        "E.SetAttribute('c', 'd');",
        "E.SetAttribute('a', 'b', 'urn:x');",
        "Nodes := E.GetChildElements('a');",
        "B := E.HasAttributes();",
    ] {
        refused(&format!(
            "procedure F(): Text var E: XmlElement; Nodes: XmlNodeList; B: Boolean; begin {call} end;"
        ));
    }
}

#[test]
fn stream_shapes_the_probe_did_not_use_refuse() {
    for call in [
        // A cleared InStream, not one never created.
        "Clear(InS); InS.ResetPosition();",
        // Media imports with other arguments, as a statement, and into a field that holds a media.
        "G := R.Pic.ImportStream(InS, 'other');",
        "R.Pic.ImportStream(InS, 'icon');",
        "G := R.Pics.ImportStream(InS, 'icon', 'image/png');",
        "R.Data.CreateOutStream(O); O.WriteText('x'); R.Data.CreateInStream(Src); G := R.Pic.ImportStream(Src, 'a'); G := R.Pic.ImportStream(InS, 'icon');",
    ] {
        refused(&format!(
            "procedure F(): Text var R: Record Row; InS: InStream; Src: InStream; O: OutStream; G: Guid; begin {call} end;"
        ));
    }
}

/// The control of the tests above: the measured shapes, built the same way, raise BC's error (or answer) instead of refusing.
#[test]
fn the_measured_shapes_run_in_this_harness() {
    for (vars, call, code) in [
        ("RR: RecordRef; B: Boolean;", "B := RR.FindLast();", "RecordNotOpened"),
        (
            "RR: RecordRef; B: Boolean;",
            "RR.Open(50100); RR.Close(); B := RR.FindFirst();",
            "RecordNotOpened",
        ),
        ("RR: RecordRef; R: Record Row;", "RR.CopyLinks(R);", "RecordNotOpened"),
        ("RR: RecordRef; V: Variant;", "V := ''; RR.GetTable(V);", "Conversion"),
        ("KR: KeyRef; FR: FieldRef;", "FR := KR.FieldIndex(1);", "NotInitialized"),
        ("E: XmlElement;", "E.SetAttribute('a', 'b');", "NotInitialized"),
        ("InS: InStream;", "InS.ResetPosition();", "NotInitialized"),
    ] {
        let r = run(&format!(
            "procedure F(): Text var {vars} begin asserterror begin {call} end; exit(GetLastErrorCode()); end;"
        ));
        assert_eq!(al_testkit::text(r), code, "{call}");
    }
    let r = run(
        "procedure F(): Text var R: Record Row; InS: InStream; begin exit(Format(R.Pic.ImportStream(InS, 'icon'))); end;",
    );
    assert_eq!(al_testkit::text(r), "{00000000-0000-0000-0000-000000000000}");
}
