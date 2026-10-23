//! Corpus cases that failed before corpus bump 34 and the runtime got wrong (branch lane-old-fails).
//! Each test cites its case.

use al_builtins::native::Signal;
use al_builtins::value::Value;

fn out(r: Result<Value, Signal>) -> String {
    match r {
        Ok(Value::Text(t) | Value::Code(t)) => t.to_string(),
        Ok(Value::Integer(i)) => i.to_string(),
        Ok(Value::Boolean(b)) => b.to_string(),
        Err(Signal::Error(e)) => format!("error:{}|{}", e.message, e.code),
        Err(Signal::Unsupported(why)) => format!("refused:{why}"),
        other => format!("{other:?}"),
    }
}

fn text(vars: &str, body: &str) -> String {
    let src = format!("codeunit 50100 C {{ procedure F(): Text var {vars} begin {body} end; }}");
    out(al_testkit::run_in(&src, "C", "F"))
}

/// `Clear` of an array of JsonObject leaves every element as it was (json/lt-db-json-leads/DbJsonLeads.al:150-164).
#[test]
fn clear_of_an_array_of_json_object_leaves_the_elements() {
    let r = text(
        "A: array[3] of JsonObject; T: Text; U: Text;",
        "A[1].ReadFrom('{\"a\":1}'); A[3].ReadFrom('{\"c\":3}'); Clear(A); A[1].WriteTo(T); A[3].WriteTo(U); exit(T + U);",
    );
    assert_eq!(r, "{\"a\":1}{\"c\":3}");
}

/// `Clear` of an array of another JSON type is not pinned: refused.
#[test]
fn clear_of_an_array_of_json_array_is_refused() {
    let r = text("A: array[2] of JsonArray;", "A[1].Add(1); Clear(A); exit('ran');");
    assert!(r.starts_with("refused:"), "{r}");
}

/// `ReadAs(var Text)` after the body was handed out as a stream: a copy of a response's content reads an empty text (network/
/// lt-db-http-leads/DbHttpContentLeads.al:146), the response's own content the body (:113).
#[test]
fn http_content_read_as_text_after_a_stream_was_taken() {
    let r = text(
        "R: HttpResponseMessage; C: HttpContent; S: InStream; T: Text; Line: Text;",
        "R.Content().WriteFrom('body'); C := R.Content(); C.ReadAs(S); S.ReadText(Line); C.ReadAs(T); exit(Line + '|' + T);",
    );
    assert_eq!(r, "body|");
    let r = text(
        "R: HttpResponseMessage; S: InStream; T: Text; Line: Text;",
        "R.Content().WriteFrom('body'); R.Content().ReadAs(S); S.ReadText(Line); R.Content().ReadAs(T); exit(Line + '|' + T);",
    );
    assert_eq!(r, "body|body");
    // A variable written from a text reads the body again (xml/lt-xml-attr-doc-http-members/P97ContentRead.al:89).
    let r = text(
        "C: HttpContent; S: InStream; T: Text; Line: Text;",
        "C.WriteFrom('body'); C.ReadAs(S); S.ReadText(Line); C.ReadAs(T); exit(Line + '|' + T);",
    );
    assert_eq!(r, "body|body");
}

/// `WriteFrom(InStream)` leaves the source stream where it was: it is not at its end
/// (network/lt-db-http-leads/DbHttpContentLeads.al:310). Any other use of the source afterwards is not pinned.
/// `ReadAs(var Text)` after a stream of the body that was read only in part, or not at all: the response's own content and a
/// content written from a text still read the whole body; a copy of a content reads an empty text
/// (network/lt-db-http-leads/DbHttpContentLeads.al:384-480 QG_RespContent_*, QG_RespContentCopy_*, :506-550 QG_TextContent_*).
#[test]
fn http_content_read_as_text_after_a_partly_read_or_unread_stream() {
    for (vars, body, want) in [
        (
            "R: HttpResponseMessage; C: HttpContent; S: InStream; T: Text; Line: Text;",
            "R.Content().WriteFrom('abcdef'); C := R.Content(); C.ReadAs(S); C.ReadAs(T); exit('ran ' + T);",
            "ran ",
        ),
        (
            "R: HttpResponseMessage; S: InStream; T: Text; Line: Text; Nl: Text[1];",
            "Nl[1] := 10; R.Content().WriteFrom('a' + Nl + 'b'); R.Content().ReadAs(S); S.ReadText(Line); R.Content().ReadAs(T); exit('ran ' + T);",
            "ran a\nb",
        ),
        (
            "C: HttpContent; S: InStream; T: Text;",
            "C.WriteFrom('body'); C.ReadAs(S); C.ReadAs(T); exit('ran ' + T);",
            "ran body",
        ),
    ] {
        assert_eq!(text(vars, body), want, "{body}");
    }
}

#[test]
fn http_content_write_from_stream_leaves_the_source_not_at_its_end() {
    let src = r#"table 50101 B { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } }
codeunit 50100 C {
  procedure Fill(var R: Record B temporary; var S: InStream) var O: OutStream; begin R.Data.CreateOutStream(O); O.WriteText('picture'); R.Insert(); R.CalcFields(Data); R.Data.CreateInStream(S); end;
  procedure F(): Text var R: Record B temporary; C: HttpContent; S: InStream; T1: Text; T2: Text; begin Fill(R, S); C.WriteFrom(S); C.ReadAs(T1); C.ReadAs(T2); exit(T1 + '|' + T2 + '|' + Format(S.EOS())); end;
  procedure ReadSource(): Text var R: Record B temporary; C: HttpContent; S: InStream; T: Text; begin Fill(R, S); C.WriteFrom(S); S.ReadText(T); exit(T); end; }"#;
    assert_eq!(out(al_testkit::run_in(src, "C", "F")), "picture|picture|No");
    let r = out(al_testkit::run_in(src, "C", "ReadSource"));
    assert!(r.starts_with("refused:"), "{r}");
}

/// `Clear` on the content of a request that has none gives it a body and content headers: `GetHeaders` then hands out the
/// content's headers with `text/plain; charset=utf-8` (network/lt-db-http-leads/DbHttpContentLeads.al:176-219).
#[test]
fn http_request_content_clear_then_get_headers() {
    let src = r#"codeunit 50100 C {
  [TryFunction] local procedure TryHeaders(var Req: HttpRequestMessage; var H: HttpHeaders) begin Req.Content.GetHeaders(H); end;
  local procedure V(var H: HttpHeaders; N: Text): Text var Values: array[5] of Text; begin if not H.Contains(N) then exit('absent'); H.GetValues(N, Values); exit(Values[1]); end;
  procedure F(): Text var Req: HttpRequestMessage; H: HttpHeaders; R: Text; T: Text; begin
    if TryHeaders(Req, H) then R := 'ok1' else R := 'err1';
    Req.Content.Clear();
    Req.Content.GetHeaders(H);
    R += '|' + V(H, 'Content-Type');
    R += '|' + Format(H.Remove('Content-Type')) + Format(H.Add('Content-Type', 'application/json')) + V(H, 'Content-Type');
    R += '|' + Format(Req.Content.ReadAs(T)) + T;
    Req.Content.GetHeaders(H);
    R += '|' + V(H, 'Content-Type');
    Req.Content.WriteFrom('{"a":1}');
    Req.Content.GetHeaders(H);
    exit(R + '|' + V(H, 'Content-Type'));
  end; }"#;
    assert_eq!(
        out(al_testkit::run_in(src, "C", "F")),
        "err1|text/plain; charset=utf-8|YesYesapplication/json|Yes|application/json|text/plain; charset=utf-8"
    );
}

/// An InStream handed back from a routine whose local temporary record owns its bytes reads as empty on BC once the local
/// is out of scope: Length 0, EOS Yes, ReadText gives '' (streams/lt-dc-stream-leads/DcInStreamScope.al:107-112, :125-126).
/// A stream of a global keeps its bytes.
#[test]
fn an_instream_of_a_local_handed_back_reads_empty() {
    let src = r#"table 50101 B { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } }
codeunit 50100 C {
  var G: Record B temporary;
  local procedure FromLocal() Result: InStream var R: Record B temporary; O: OutStream; begin R.Data.CreateOutStream(O); O.WriteText('x'); R.Data.CreateInStream(Result); end;
  local procedure FromLocalVar(var S: InStream) var R: Record B temporary; O: OutStream; begin R.Data.CreateOutStream(O); O.WriteText('x'); R.Data.CreateInStream(S); end;
  local procedure FromGlobal() Result: InStream var O: OutStream; begin G.Data.CreateOutStream(O); O.WriteText('kept'); G.Data.CreateInStream(Result); end;
  local procedure Show(var S: InStream): Text var T: Text; L: Text; E: Text; begin L := Format(S.Length()); E := Format(S.EOS()); S.ReadText(T); exit(L + '|' + E + '|' + T + '|' + Format(S.EOS())); end;
  procedure Local(): Text var S: InStream; begin S := FromLocal(); exit(Show(S)); end;
  procedure LocalVar(): Text var S: InStream; begin FromLocalVar(S); exit(Show(S)); end;
  procedure Global(): Text var S: InStream; T: Text; begin S := FromGlobal(); S.ReadText(T); exit(T); end; }"#;
    assert_eq!(out(al_testkit::run_in(src, "C", "Local")), "0|Yes||Yes");
    assert_eq!(out(al_testkit::run_in(src, "C", "LocalVar")), "0|Yes||Yes");
    assert_eq!(out(al_testkit::run_in(src, "C", "Global")), "kept");
}

/// The Temp Blob codeunit as test code sees it: a stand-in with the System app's id and name, whose bytes live in a temporary
/// record it holds as a global.
const TEMP_BLOB: &str = r#"table 50102 TB { TableType = Temporary; fields { field(1; Id; Integer) { } field(2; Blob; Blob) { } } }
codeunit 4100 "Temp Blob" {
  var R: Record TB temporary;
  procedure CreateInStream(var S: InStream) begin R.Blob.CreateInStream(S); end;
  procedure CreateOutStream(var O: OutStream) begin R.Blob.CreateOutStream(O); end; }
"#;

/// al-runner issue 5688: a var InStream filled by a procedure of another codeunit named CreateInstream (as Http Web Request
/// Mgt. has) that creates it on its own local Temp Blob, directly and through two levels, empty or with text: Length 0, EOS
/// Yes, ReadText '' (streams/lt-dc-stream-leads/DcInStreamScope.al:128-166). The procedure named CreateInstream of a codeunit
/// is no Blob method: the caller that hands its stream on is not refused.
#[test]
fn an_instream_of_a_local_temp_blob_of_another_codeunit_reads_empty() {
    let src = format!(
        r#"{TEMP_BLOB}
codeunit 50103 Source {{
  procedure CreateInstream(var InStr: InStream) var TempBlob: Codeunit "Temp Blob"; begin TempBlob.CreateInStream(InStr); end;
  procedure FilledInstream(Content: Text; var InStr: InStream) var TempBlob: Codeunit "Temp Blob"; O: OutStream; begin
    TempBlob.CreateOutStream(O); O.WriteText(Content); TempBlob.CreateInStream(InStr); end; }}
codeunit 50100 C {{
  local procedure Via(Content: Text; var S: InStream) var Source: Codeunit Source; begin
    if Content = '' then Source.CreateInstream(S) else Source.FilledInstream(Content, S); end;
  local procedure Show(var S: InStream): Text var T: Text; L: Text; E: Text; begin L := Format(S.Length()); E := Format(S.EOS()); S.ReadText(T); exit(L + '|' + E + '|' + T + '|' + Format(S.EOS())); end;
  procedure TwoEmpty(): Text var S: InStream; begin Via('', S); exit(Show(S)); end;
  procedure TwoText(): Text var S: InStream; begin Via('hello', S); exit(Show(S)); end;
  procedure Direct(): Text var S: InStream; Source: Codeunit Source; begin Source.FilledInstream('hello', S); exit(Show(S)); end; }}"#
    );
    for routine in ["TwoEmpty", "TwoText", "Direct"] {
        assert_eq!(out(al_testkit::run_in(&src, "C", routine)), "0|Yes||Yes", "{routine}");
    }
}

/// A stream whose owner went out of scope: only Length, EOS and ReadText are pinned (DcInStreamScope.al:107-166); anything
/// else is refused when it runs.
#[test]
fn an_instream_of_a_local_out_of_scope_refuses_other_uses() {
    let src = r#"table 50101 B { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } }
codeunit 50100 C {
  local procedure FromLocal() Result: InStream var R: Record B temporary; O: OutStream; begin R.Data.CreateOutStream(O); O.WriteText('x'); R.Data.CreateInStream(Result); end;
  procedure ReadInt(): Text var S: InStream; I: Integer; begin S := FromLocal(); S.Read(I); exit(Format(I)); end;
  procedure Pos(): Text var S: InStream; begin S := FromLocal(); exit(Format(S.Position())); end; }"#;
    for routine in ["ReadInt", "Pos"] {
        let r = out(al_testkit::run_in(src, "C", routine));
        assert!(r.starts_with("refused:") && r.contains("out of scope"), "{routine}: {r}");
    }
}

/// A stream of a local handed back in a shape no corpus test pins (a record that is not temporary, a call that is not the
/// routine's last statement) is refused when that code runs, not when it is only reachable (al-runner issue 5688).
#[test]
fn an_instream_of_a_local_in_an_unpinned_shape_refuses_when_it_runs() {
    let src = r#"table 50101 B { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } }
codeunit 50100 C {
  local procedure NotTemp(var S: InStream) var R: Record B; begin R.Data.CreateInStream(S); end;
  local procedure NotLast(var S: InStream) var R: Record B temporary; N: Integer; begin R.Data.CreateInStream(S); N := 1; end;
  procedure Reached1(): Text var S: InStream; begin NotTemp(S); exit('ran'); end;
  procedure Reached2(): Text var S: InStream; begin NotLast(S); exit('ran'); end;
  procedure NotReached(): Text var S: InStream; N: Integer; begin if N = 1 then NotTemp(S); if N = 1 then NotLast(S); exit('ran'); end; }"#;
    assert_eq!(out(al_testkit::run_in(src, "C", "NotReached")), "ran");
    for routine in ["Reached1", "Reached2"] {
        let r = out(al_testkit::run_in(src, "C", routine));
        assert!(r.starts_with("refused:") && r.contains("out of scope"), "{routine}: {r}");
    }
}

/// The InStream-of-a-local refusal looks at calls in the syntax tree, with names resolved: a comment or a string that
/// spells the call, a stream created on a local but not handed back, and a handed-back stream created on a parameter
/// record do not refuse the routine (they hand back no stream owned by a local).
#[test]
fn an_instream_handed_back_not_owned_by_a_local_runs() {
    let src = r#"table 50101 B { fields { field(1; Id; Integer) { } field(2; Data; Blob) { } } }
codeunit 50100 C {
  var G: Record B temporary;
  local procedure Commented() Result: InStream var R: Record B temporary; O: OutStream; T: Text; begin
    // R.Data.CreateInStream(Result);
    T := 'R.Data.CreateInStream(Result)';
    G.Data.CreateOutStream(O); O.WriteText('global'); G.Data.CreateInStream(Result); end;
  local procedure NotHanded() Result: InStream var R: Record B temporary; S: InStream; O: OutStream; begin
    R.Data.CreateOutStream(O); O.WriteText('x'); R.Data.CreateInStream(S);
    G.Data.CreateOutStream(O); O.WriteText('global2'); G.Data.CreateInStream(Result); end;
  local procedure FromParam(var P: Record B temporary) Result: InStream var O: OutStream; begin
    P.Data.CreateOutStream(O); O.WriteText('param'); P.Data.CreateInStream(Result); end;
  procedure A(): Text var S: InStream; T: Text; begin S := Commented(); S.ReadText(T); exit(T); end;
  procedure B2(): Text var S: InStream; T: Text; begin S := NotHanded(); S.ReadText(T); exit(T); end;
  procedure P2(): Text var S: InStream; T: Text; R: Record B temporary; begin S := FromParam(R); S.ReadText(T); exit(T); end; }"#;
    assert_eq!(out(al_testkit::run_in(src, "C", "A")), "global");
    assert_eq!(out(al_testkit::run_in(src, "C", "B2")), "global2");
    assert_eq!(out(al_testkit::run_in(src, "C", "P2")), "param");
}
