//! Neighbors of the cells corpus PR 1408 measured on handles nobody opened or assigned (recordref-fieldref/UhUnopenedRecordRef.al,
//! xml/UhUnassignedXmlElement.al, streams/instream-outstream/UhUntouchedStreams.al): a shape the probe did not measure keeps the
//! answer it had before (a refusal), and a RecordRef closed after it was open keeps the answers main gave a RecordRef not open.
//! The measured cells themselves are the corpus cases.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;
use al_builtins::value::Value;

const TABLE: &str = "table 50100 Row { fields { field(1; Id; Integer) { } field(2; Pic; Media) { } field(3; Pics; MediaSet) { } field(4; Data; Blob) { } field(5; Name; Text[30]) { } }
    keys { key(PK; Id) { Clustered = true; } } }";

fn run(body: &str) -> Result<Value, Signal> {
    let src = format!("{TABLE} codeunit 50101 C {{ {body} }}");
    al_testkit::run_with(&src, "C", "F", vec![]).map(|(v, _)| v)
}

fn refused(body: &str) {
    let r = run(body);
    assert!(matches!(r, Err(Signal::Unsupported(_))), "{body}: {r:?}");
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

#[test]
fn a_recordref_whose_open_or_get_failed_is_not_one_never_opened() {
    // A failed Open and a Get(RecordId) that missed leave a RecordRef not open; the probe measured a new variable only.
    for setup in [
        "asserterror RR.Open(99999999);",
        "R.Id := 7; Id := R.RecordId(); B := RR.Get(Id);",
    ] {
        for call in ["B := RR.FindLast();", "T := RR.GetFilters();", "RR.SetRecFilter();"] {
            refused(&format!(
                "procedure F(): Text var RR: RecordRef; R: Record Row; Id: RecordId; B: Boolean; T: Text; begin {setup} {call} end;"
            ));
        }
        // It keeps the answers of a RecordRef not open (a closed one).
        let r = run(&format!(
            "procedure F(): Text var RR: RecordRef; R: Record Row; Id: RecordId; B: Boolean; begin {setup} asserterror B := RR.FindFirst(); exit(Format(RR.Number()) + '|' + GetLastErrorCode()); end;"
        ));
        assert_eq!(al_testkit::text(r), "0|RecordNotOpened", "{setup}");
    }
}

#[test]
fn copylinks_sources_the_probe_did_not_pass_refuse() {
    for (setup, call) in [
        // A Variant that holds a Record.
        ("V := R;", "RR.CopyLinks(V);"),
        // A source with a filter, and one standing on a stored row.
        ("R.SetRange(Id, 1);", "RR.CopyLinks(R);"),
        ("R.Id := 1; R.Insert(); R.Get(1);", "RR.CopyLinks(R);"),
        ("Src.Open(50100); Src.Field(1).SetRange(1);", "RR.CopyLinks(Src);"),
    ] {
        refused(&format!(
            "procedure F(): Text var RR: RecordRef; Src: RecordRef; R: Record Row; V: Variant; begin {setup} {call} end;"
        ));
    }
}

#[test]
fn media_imports_from_an_uncreated_stream_into_records_the_probe_did_not_use_refuse() {
    for call in [
        // An inserted record and a temporary one.
        "R.Id := 1; R.Insert(); G := R.Pic.ImportStream(InS, 'icon');",
        "T.Id := 1; G := T.Pic.ImportStream(InS, 'icon');",
        "T.Id := 1; G := T.Pics.ImportStream(InS, 'icon');",
        // What the record holds after the import is not measured: its field, its Insert and a copy of it.
        "R.Id := 1; G := R.Pic.ImportStream(InS, 'icon'); B := R.Pic.HasValue();",
        "R.Id := 1; G := R.Pics.ImportStream(InS, 'icon'); N := R.Pics.Count();",
        "R.Id := 1; G := R.Pic.ImportStream(InS, 'icon'); R.Insert();",
        "R.Id := 1; G := R.Pic.ImportStream(InS, 'icon'); R2 := R; R2.Insert();",
        "R.Id := 1; G := R.Pic.ImportStream(InS, 'icon'); R2.TransferFields(R); R2.Insert();",
        "R.Id := 1; G := R.Pic.ImportStream(InS, 'icon'); R2.Copy(R); R2.Insert();",
        "R.Id := 1; G := R.Pic.ImportStream(InS, 'icon'); RR.GetTable(R); RR.Insert();",
        "R.Id := 1; G := R.Pic.ImportStream(InS, 'icon'); V := R; R2 := V; R2.Insert();",
        "R.Id := 1; G := R.Pic.ImportStream(InS, 'icon'); Keep(R);",
    ] {
        refused(&format!(
            "procedure F(): Text var R: Record Row; R2: Record Row; T: Record Row temporary; RR: RecordRef; V: Variant; InS: InStream; G: Guid; B: Boolean; N: Integer; begin {call} end;
            local procedure Keep(R: Record Row) begin R.Insert(); end;"
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
        (
            "RR: RecordRef; Src: RecordRef;",
            "Src.Open(50100); RR.CopyLinks(Src);",
            "RecordNotOpened",
        ),
        (
            "RR: RecordRef; R: Record Row;",
            "RR.Open(50100); RR.Close(); RR.CopyLinks(R);",
            "RecordNotOpened",
        ),
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

/// A copy or whole-buffer read of a record after a media import from an uncreated stream refuses, also when a later test of the
/// codeunit reaches the copy through a global (the codeunit's globals outlive a test: `L1` and `L2` stand for two tests).
#[test]
fn the_buffer_of_an_uncreated_import_is_not_handed_on_through_a_recordref_or_format() {
    for (l1, l2) in [
        // GetTable of the record (Record and Variant argument), then a field read of the copy in the same and in a later test.
        ("RR.GetTable(R); FR := RR.Field(2); G := FR.Value;", ""),
        ("RR.GetTable(R); G := RR.FieldIndex(2).Value;", ""),
        ("V := R; RR.GetTable(V); FR := RR.Field(2); G := FR.Value;", ""),
        ("GRR.GetTable(R);", "FR := GRR.Field(2); G := FR.Value;"),
        // GetTable, then SetTable into a record and Insert in a later test.
        ("GRR.GetTable(R);", "GRR.SetTable(R2); R2.Insert();"),
        // Format of the record, in the same and in a later test.
        ("T := Format(R);", ""),
        ("GR.Id := 7; G := GR.Pic.ImportStream(InS, 'icon');", "T := Format(GR);"),
    ] {
        refused(&format!(
            "var GRR: RecordRef; GR: Record Row;
            procedure F(): Text begin L1(); L2(); end;
            local procedure L1() var R: Record Row; RR: RecordRef; FR: FieldRef; V: Variant; InS: InStream; G: Guid; T: Text;
            begin R.Id := 5; G := R.Pic.ImportStream(InS, 'icon'); {l1} end;
            local procedure L2() var R2: Record Row; FR: FieldRef; G: Guid; T: Text; begin {l2} end;"
        ));
    }
}

/// ModifyAll and DeleteAll on a record after a media import from an uncreated stream run: they write no value of its buffer.
#[test]
fn modifyall_and_deleteall_on_the_record_of_an_uncreated_import_run() {
    for call in ["R.ModifyAll(Name, 'x');", "R.DeleteAll();"] {
        let r = run(&format!(
            "procedure F(): Text var R: Record Row; InS: InStream; G: Guid; begin R.Id := 1; G := R.Pic.ImportStream(InS, 'icon'); {call} exit('ran'); end;"
        ));
        assert_eq!(al_testkit::text(r), "ran", "{call}");
    }
}

/// The control of the test above: Format of a record whose mark leaves its fields readable (SetCurrentKey on a Media field, whose
/// GetTable refuses as before) keeps its answer, and GetTable of a record nobody imported into still copies it.
#[test]
fn format_and_gettable_of_records_without_a_refusing_mark_keep_their_answer() {
    let r = run("procedure F(): Text var R: Record Row; begin R.Id := 3; R.SetCurrentKey(Pic); exit(Format(R)); end;");
    assert!(al_testkit::text(r).starts_with('3'));
    refused("procedure F(): Text var R: Record Row; RR: RecordRef; begin R.SetCurrentKey(Pic); RR.GetTable(R); end;");
    let r = run(
        "procedure F(): Text var R: Record Row; RR: RecordRef; begin R.Id := 4; RR.GetTable(R); exit(Format(RR.Field(1).Value)); end;",
    );
    assert_eq!(al_testkit::text(r), "4");
}

/// The states corpus PR 1411 measured (UhUntouchedStreamsAfter.al, codeunit 66996): a RecordRef after Open(0) and after Open then
/// Clear raises "The record is not open." on the never-opened members, a KeyRef that held a key raises NotInitialized on Active
/// after Clear. Each runs flagged.
#[test]
fn the_states_after_open_zero_and_clear_answer_as_measured() {
    for (setup, call, text) in [
        ("RR.Open(0);", "B := RR.FindLast();", "The record is not open."),
        (
            "RR.Open(50100); Clear(RR);",
            "T := RR.GetFilters();",
            "The record is not open.",
        ),
        (
            "RR.Open(50100); Clear(RR);",
            "KR := RR.KeyIndex(1);",
            "The record is not open.",
        ),
        (
            "RR.Open(50100); KR := RR.KeyIndex(1); Clear(KR);",
            "B := KR.Active();",
            "Microsoft.Dynamics.Nav.Runtime.NavKeyRef variable not initialized.",
        ),
    ] {
        take_hits();
        let r = run(&format!(
            "procedure F(): Text var RR: RecordRef; KR: KeyRef; B: Boolean; T: Text; N: Integer;
            begin {setup} asserterror begin {call} end; exit(GetLastErrorText() + '|' + GetLastErrorCode()); end;"
        ));
        let hits = take_hits();
        assert!(
            hits.contains(&Assumption::RecordRefNotOpenMembers) || hits.contains(&Assumption::KeyRefUnassignedMembers),
            "{setup} {call}: {hits:?}"
        );
        let code = if text.starts_with("The record") {
            "RecordNotOpened"
        } else {
            "NotInitialized"
        };
        assert_eq!(al_testkit::text(r), format!("{text}|{code}"), "{setup} {call}");
    }
}

/// Members corpus PR 1411 did not measure after Clear and after Open(0), and the sequences around them it did not use, keep their
/// answer before the probe (a refusal): the members UhUnopenedRecordRef.al measured on a RecordRef never opened and closed, FieldCount
/// after Clear, Clear of a RecordRef never opened, Open(0) after Clear, and a failed Open after Clear.
#[test]
fn recordref_members_not_measured_after_clear_or_open_zero_refuse() {
    for setup in ["RR.Open(0);", "RR.Open(50100); Clear(RR);"] {
        for call in [
            "B := RR.FindSet();",
            "B := RR.ReadPermission();",
            "B := RR.WritePermission();",
            "B := RR.FindFirst();",
            "RR.CopyLinks(R);",
            "RR.CopyLinks(Src);",
        ] {
            refused(&format!(
                "procedure F(): Text var RR: RecordRef; Src: RecordRef; R: Record Row; B: Boolean; begin Src.Open(50100); {setup} {call} end;"
            ));
        }
    }
    for (setup, call) in [
        ("RR.Open(50100); Clear(RR);", "N := RR.FieldCount();"),
        ("Clear(RR);", "B := RR.FindLast();"),
        ("Clear(RR);", "B := RR.FindSet();"),
        ("RR.Open(50100); Clear(RR); RR.Open(0);", "B := RR.FindLast();"),
        ("RR.Open(0); RR.Open(0);", "B := RR.FindLast();"),
        (
            "RR.Open(50100); Clear(RR); asserterror RR.Open(99999999);",
            "B := RR.FindLast();",
        ),
        ("RR.Open(50100); RR.Close(); Clear(RR);", "T := RR.GetFilters();"),
    ] {
        refused(&format!(
            "procedure F(): Text var RR: RecordRef; B: Boolean; T: Text; N: Integer; begin {setup} {call} end;"
        ));
    }
}

/// After Clear and after Open(0) a RecordRef keeps the answers main gave a RecordRef not open: Number, Format and the members of
/// RfTOpen.al Closed_Errors. FieldCount after Open(0) is pinned (RfTOpen.al:630) and runs unflagged.
#[test]
fn recordref_after_clear_or_open_zero_keeps_the_answers_of_one_not_open() {
    for setup in ["RR.Open(0);", "RR.Open(50100); Clear(RR);"] {
        let r = run(&format!(
            "procedure F(): Text var RR: RecordRef; T: Text; begin {setup}
            T := Format(RR.Number()) + '|' + Format(RR) + '|';
            asserterror T := RR.Name();
            exit(T + GetLastErrorCode()); end;"
        ));
        assert_eq!(al_testkit::text(r), "0||RecordNotOpened", "{setup}");
    }
    take_hits();
    let r = run(
        "procedure F(): Text var RR: RecordRef; N: Integer; begin RR.Open(0); asserterror N := RR.FieldCount(); exit(GetLastErrorCode()); end;",
    );
    assert_eq!(al_testkit::text(r), "RecordNotOpened");
    assert!(!take_hits().contains(&Assumption::RecordRefNotOpenMembers));
}

/// A KeyRef after Clear: only Active was asserted by corpus PR 1411 (FieldCount was observed, not asserted). FieldIndex(1), Record
/// and FieldCount refuse, and so does Active on a KeyRef cleared while it held nothing.
#[test]
fn keyref_members_not_measured_after_clear_refuse() {
    for call in ["N := KR.FieldCount();", "FR := KR.FieldIndex(1);", "R2 := KR.Record();"] {
        refused(&format!(
            "procedure F(): Text var RR: RecordRef; R2: RecordRef; KR: KeyRef; FR: FieldRef; N: Integer;
            begin RR.Open(50100); KR := RR.KeyIndex(1); Clear(KR); {call} end;"
        ));
    }
    refused("procedure F(): Text var KR: KeyRef; B: Boolean; begin Clear(KR); B := KR.Active(); end;");
}

/// GetTable of a Variant that holds no record on a RecordRef after Clear or Open(0): no probe measured it, so every value keeps its
/// answer before the probe (a refusal).
#[test]
fn gettable_of_a_variant_after_clear_or_open_zero_refuses() {
    for setup in ["RR.Open(50100); Clear(RR);", "RR.Open(0);"] {
        for value in ["''", "'abc'", "5", "'ALT Universal'"] {
            refused(&format!(
                "procedure F(): Text var RR: RecordRef; V: Variant; begin {setup} V := {value}; RR.GetTable(V); end;"
            ));
        }
        refused(&format!(
            "procedure F(): Text var RR: RecordRef; V: Variant; begin {setup} RR.GetTable(V); end;"
        ));
    }
}
