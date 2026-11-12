//! `HttpContent.ReadAs(var SecretText)` as a statement and on the content of a request that has none: the shapes the probe
//! al-language-onprem/http/N4ReadAsSecretStatement.al measured run through assumption http-readas-secret-statement, the shape
//! corpus http/CzHttpNoNetwork.al:25-42 pins runs, and every other shape keeps the refusal.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;

const SRC: &str = r#"
codeunit 50192 C
{
    var
        Kept: HttpContent;

    procedure Written(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('plain body');
        Ct.ReadAs(S);
        exit(S.Unwrap());
    end;

    procedure SecretTwice(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        Ct.ReadAs(S);
        Ct.ReadAs(S);
        exit(S.Unwrap());
    end;

    procedure Fresh(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        S := SecretStrSubstNo('before');
        Ct.ReadAs(S);
        exit(Format(S.IsEmpty()));
    end;

    procedure RequestInPlace(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('{"a":1}');
        Req.Content(Ct);
        Req.Content.ReadAs(S);
        exit(S.Unwrap());
    end;

    procedure RequestSecretCopy(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        Copy: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('token=abc'));
        Req.Content(Ct);
        Copy := Req.Content;
        Copy.ReadAs(S);
        exit(S.Unwrap());
    end;

    // The calls Http Content Impl. makes for Create(SecretText) and GetHttpContent, read as DescribeContent reads them.
    procedure ImplShape(): Text
    var
        Headers: HttpHeaders;
    begin
        ClearAll();
        Kept.Clear();
        Kept.WriteFrom(SecretStrSubstNo('test'));
        if not Kept.GetHeaders(Headers) then
            Error('no headers');
        if Headers.Contains('Content-Type') then
            Headers.Remove('Content-Type');
        Headers.Add('Content-Type', 'application/json');
        exit(Describe(GetContent()));
    end;

    local procedure GetContent() ReturnValue: HttpContent
    begin
        ReturnValue := Kept;
    end;

    local procedure Describe(Content: HttpContent): Text
    var
        S: SecretText;
    begin
        if not Content.IsSecretContent() then
            exit('plain');
        Content.ReadAs(S);
        exit(Format(S.IsEmpty()));
    end;

    procedure NoContentUsed(): Text
    var
        Req: HttpRequestMessage;
        S: SecretText;
    begin
        S := SecretStrSubstNo('before');
        exit(Format(Req.Content.ReadAs(S)) + Format(S.IsEmpty()));
    end;

    procedure NoContentStatement(): Text
    var
        Req: HttpRequestMessage;
        S: SecretText;
    begin
        Req.Content.ReadAs(S);
        exit('');
    end;

    procedure NoContentCopyStatement(): Text
    var
        Req: HttpRequestMessage;
        Copy: HttpContent;
        S: SecretText;
    begin
        Copy := Req.Content;
        Copy.ReadAs(S);
        exit('');
    end;

    procedure NoContentAfterTextReadUsed(): Text
    var
        Req: HttpRequestMessage;
        S: SecretText;
        T: Text;
    begin
        if Req.Content.ReadAs(T) then;
        exit(Format(Req.Content.ReadAs(S)));
    end;

    procedure FromStream(): Text
    var
        Ct: HttpContent;
        Src: HttpContent;
        InS: InStream;
        S: SecretText;
    begin
        Src.WriteFrom('x');
        Src.ReadAs(InS);
        Ct.WriteFrom(InS);
        Ct.ReadAs(S);
        exit('');
    end;

    procedure StreamThenText(): Text
    var
        Ct: HttpContent;
        Src: HttpContent;
        InS: InStream;
        S: SecretText;
    begin
        Src.WriteFrom('x');
        Src.ReadAs(InS);
        Ct.WriteFrom(InS);
        Ct.WriteFrom('txt');
        Ct.ReadAs(S);
        exit('');
    end;

    procedure ResponseFresh(): Text
    var
        R: HttpResponseMessage;
        S: SecretText;
    begin
        R.Content.ReadAs(S);
        exit('');
    end;

    procedure ResponseWritten(): Text
    var
        R: HttpResponseMessage;
        S: SecretText;
    begin
        R.Content.WriteFrom('resp');
        R.Content.ReadAs(S);
        exit('');
    end;

    procedure ResponseCopy(): Text
    var
        R: HttpResponseMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        R.Content.WriteFrom('resp');
        Ct := R.Content;
        Ct.ReadAs(S);
        exit('');
    end;

    procedure AfterTextRead(): Text
    var
        Ct: HttpContent;
        S: SecretText;
        T: Text;
    begin
        Ct.WriteFrom('abc');
        Ct.ReadAs(T);
        Ct.ReadAs(S);
        exit('');
    end;

    procedure AfterClear(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Ct.Clear();
        Ct.ReadAs(S);
        exit('');
    end;

    procedure RequestClearedContent(): Text
    var
        Req: HttpRequestMessage;
        S: SecretText;
    begin
        Req.Content.Clear();
        Req.Content.ReadAs(S);
        exit('');
    end;

    procedure PlainWithHeaders(): Text
    var
        Ct: HttpContent;
        H: HttpHeaders;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Ct.GetHeaders(H);
        Ct.ReadAs(S);
        exit('');
    end;

    procedure PlainCopy(): Text
    var
        Ct: HttpContent;
        Copy: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Copy := Ct;
        Copy.ReadAs(S);
        exit('');
    end;

    procedure AfterClearVariable(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Clear(Ct);
        Ct.ReadAs(S);
        exit(S.Unwrap());
    end;

    procedure WrittenTwice(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('a');
        Ct.WriteFrom('b');
        Ct.ReadAs(S);
        exit('');
    end;
}
"#;

fn assumed(routine: &str) -> String {
    take_hits();
    let got = al_testkit::text(al_testkit::run_in(SRC, "C", routine));
    assert!(
        take_hits().contains(&Assumption::HttpReadasSecretStatement),
        "{routine} not flagged"
    );
    got
}

#[test]
fn the_measured_statement_shapes_are_assumed() {
    assert_eq!(assumed("Written"), "plain body");
    assert_eq!(assumed("SecretTwice"), "s3cret");
    assert_eq!(assumed("Fresh"), "Yes");
    assert_eq!(assumed("RequestInPlace"), "{\"a\":1}");
    assert_eq!(assumed("RequestSecretCopy"), "token=abc");
    // `Clear(Ct)` makes it a new content (http/client/UfHttpClear.al:14-62).
    assert_eq!(assumed("AfterClearVariable"), "");
}

#[test]
fn the_pinned_http_content_impl_shape_runs() {
    take_hits();
    assert_eq!(al_testkit::text(al_testkit::run_in(SRC, "C", "ImplShape")), "No");
}

#[test]
fn a_request_without_content_is_assumed() {
    assert_eq!(assumed("NoContentUsed"), "NoNo");
    for routine in ["NoContentStatement", "NoContentCopyStatement"] {
        take_hits();
        match al_testkit::run_in(SRC, "C", routine) {
            Err(Signal::Error(e)) => assert_eq!(e.code, "NotInitialized", "{routine}"),
            other => panic!("{routine}: {other:?}"),
        }
        assert!(
            take_hits().contains(&Assumption::HttpReadasSecretStatement),
            "{routine} not flagged"
        );
    }
}

#[test]
fn a_request_without_content_read_as_text_first_keeps_the_refusal() {
    match al_testkit::run_in(SRC, "C", "NoContentAfterTextReadUsed") {
        Err(Signal::Unsupported(m)) => assert!(m.contains("content of a request that has none"), "{m}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn every_unmeasured_statement_shape_keeps_the_refusal() {
    for routine in [
        "FromStream",
        "StreamThenText",
        "ResponseFresh",
        "ResponseWritten",
        "ResponseCopy",
        "AfterTextRead",
        "AfterClear",
        "RequestClearedContent",
        "PlainWithHeaders",
        "PlainCopy",
        "WrittenTwice",
    ] {
        match al_testkit::run_in(SRC, "C", routine) {
            Err(Signal::Unsupported(m)) => {
                assert_eq!(
                    m, "HttpContent.ReadAs(SecretText) as a statement (not pinned)",
                    "{routine}"
                )
            }
            other => panic!("{routine}: {other:?}"),
        }
    }
}
