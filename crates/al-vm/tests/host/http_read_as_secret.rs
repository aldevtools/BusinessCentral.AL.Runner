//! `HttpContent.ReadAs(var SecretText)` as a statement and on the content of a request that has none: the call sequences the
//! probe al-language-onprem/http/N4ReadAsSecretStatement.al measured run through assumption http-readas-secret-statement, the
//! sequence corpus http/CzHttpNoNetwork.al:25-57 pins runs, and every other one keeps main's refusal.

use al_builtins::assumptions::{Assumption, take_hits};
use al_builtins::native::Signal;

const SRC: &str = r#"
codeunit 50192 C
{
    var
        Kept: HttpContent;

    local procedure Show(S: SecretText): Text
    begin
        exit(Format(S.IsEmpty()) + '|' + S.Unwrap());
    end;

    [TryFunction]
    local procedure TryStatement(var Ct: HttpContent; var S: SecretText)
    begin
        Ct.ReadAs(S);
    end;

    [TryFunction]
    local procedure TryUsed(var Ct: HttpContent; var S: SecretText; var Ok: Boolean)
    begin
        Ok := Ct.ReadAs(S);
    end;

    [TryFunction]
    local procedure TryRequestStatement(var Req: HttpRequestMessage; var S: SecretText)
    begin
        Req.Content.ReadAs(S);
    end;

    [TryFunction]
    local procedure TryRequestUsed(var Req: HttpRequestMessage; var S: SecretText; var Ok: Boolean)
    begin
        Ok := Req.Content.ReadAs(S);
    end;

    local procedure Statement(var Ct: HttpContent): Text
    var
        S: SecretText;
    begin
        S := SecretStrSubstNo('before');
        if TryStatement(Ct, S) then
            exit('ok|' + Show(S) + ';');
        exit(GetLastErrorCode() + '|' + Show(S) + ';');
    end;

    local procedure Used(var Ct: HttpContent): Text
    var
        S: SecretText;
        Ok: Boolean;
    begin
        S := SecretStrSubstNo('before');
        if TryUsed(Ct, S, Ok) then
            exit(Format(Ok) + '|' + Show(S) + ';');
        exit(GetLastErrorCode() + '|' + Show(S) + ';');
    end;

    local procedure RequestStatement(var Req: HttpRequestMessage): Text
    var
        S: SecretText;
    begin
        S := SecretStrSubstNo('before');
        if TryRequestStatement(Req, S) then
            exit('ok|' + Show(S) + ';');
        exit(GetLastErrorCode() + '|' + Show(S) + ';');
    end;

    local procedure RequestUsed(var Req: HttpRequestMessage): Text
    var
        S: SecretText;
        Ok: Boolean;
    begin
        S := SecretStrSubstNo('before');
        if TryRequestUsed(Req, S, Ok) then
            exit(Format(Ok) + '|' + Show(S) + ';');
        exit(GetLastErrorCode() + '|' + Show(S) + ';');
    end;

    // The calls of N4ReadAsSecretStatement.al:120-127, in its order.
    procedure Contents(): Text
    var
        Secret: HttpContent;
        Plain: HttpContent;
        Fresh: HttpContent;
        Empty: HttpContent;
        R: Text;
    begin
        Secret.WriteFrom(SecretStrSubstNo('s3cret body'));
        Plain.WriteFrom('plain body');
        Empty.WriteFrom('');
        R := Statement(Secret);
        R += Statement(Plain);
        R += Statement(Fresh);
        R += Statement(Empty);
        R += Statement(Secret);
        exit(R);
    end;

    // The calls of N4ReadAsSecretStatement.al:167-177.
    procedure RequestWithContent(): Text
    var
        Req: HttpRequestMessage;
        PlainReq: HttpRequestMessage;
        Ct: HttpContent;
        P: HttpContent;
        Copy: HttpContent;
        R: Text;
    begin
        Ct.WriteFrom(SecretStrSubstNo('token=abc'));
        Req.Content(Ct);
        P.WriteFrom('{"a":1}');
        PlainReq.Content(P);
        R := RequestStatement(Req);
        R += RequestUsed(Req);
        R += RequestStatement(PlainReq);
        R += RequestUsed(PlainReq);
        Copy := Req.Content;
        R += Statement(Copy);
        R += Used(Copy);
        exit(R);
    end;

    // The calls of N4ReadAsSecretStatement.al:193-198.
    procedure RequestWithoutContent(): Text
    var
        Req: HttpRequestMessage;
        Copy: HttpContent;
        R: Text;
    begin
        Req.Method := 'GET';
        R := RequestStatement(Req);
        R += RequestUsed(Req);
        Copy := Req.Content;
        R += Statement(Copy);
        R += Used(Copy);
        exit(R);
    end;

    // N4ReadAsSecretStatement.al:216-218.
    procedure RequestWithoutContentPlain(): Text
    var
        Req: HttpRequestMessage;
        S: SecretText;
    begin
        Req.Method := 'GET';
        S := SecretStrSubstNo('before');
        asserterror Req.Content.ReadAs(S);
        exit(GetLastErrorCode() + '|' + Show(S));
    end;

    // The calls Http Content Impl. makes for Create(SecretText) and GetHttpContent, read as DescribeContent reads them
    // (CzHttpNoNetwork.al:25-57).
    procedure ImplShape(): Text
    begin
        ImplCreate();
        exit(Describe(GetContent()));
    end;

    local procedure ImplCreate()
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

    // ---- statement reads no evidence covers

    procedure FreshTwice(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.ReadAs(S);
        Ct.ReadAs(S);
    end;

    procedure FreshReadThenWritten(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.ReadAs(S);
        Ct.WriteFrom('abc');
        Ct.ReadAs(S);
    end;

    procedure FreshUsedThenStatement(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        if Ct.ReadAs(S) then;
        Ct.ReadAs(S);
    end;

    procedure SecretThrice(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        Ct.ReadAs(S);
        Ct.ReadAs(S);
        Ct.ReadAs(S);
    end;

    procedure PlainTwice(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Ct.ReadAs(S);
        Ct.ReadAs(S);
    end;

    procedure SecretUsedThenStatement(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        if Ct.ReadAs(S) then;
        Ct.ReadAs(S);
    end;

    procedure SecretAfterACopyWasTaken(): Text
    var
        Ct: HttpContent;
        Copy: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        Copy := Ct;
        Ct.ReadAs(S);
    end;

    procedure SecretAfterSetOnARequest(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        Req.Content(Ct);
        Ct.ReadAs(S);
    end;

    procedure FreshAfterACopyWasTaken(): Text
    var
        Ct: HttpContent;
        Copy: HttpContent;
        S: SecretText;
    begin
        Copy := Ct;
        Ct.ReadAs(S);
    end;

    procedure FreshAfterSetOnARequest(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Req.Content(Ct);
        Ct.ReadAs(S);
    end;

    procedure EmptySecretWritten(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(S);
        Ct.ReadAs(S);
    end;

    procedure SecretAfterPost(): Text
    var
        Client: HttpClient;
        Resp: HttpResponseMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        if Client.Post('http://localhost/x', Ct, Resp) then;
        Ct.ReadAs(S);
    end;

    procedure RequestAfterSend(): Text
    var
        Client: HttpClient;
        Resp: HttpResponseMessage;
        Req: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Req.Content(Ct);
        Req.SetRequestUri('http://localhost/x');
        if Client.Send(Req, Resp) then;
        Req.Content.ReadAs(S);
    end;

    procedure SecretCopy(): Text
    var
        Ct: HttpContent;
        Copy: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        Copy := Ct;
        Copy.ReadAs(S);
    end;

    procedure SecretByValue(): Text
    var
        Ct: HttpContent;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        exit(Describe(Ct));
    end;

    procedure SecretWithHeaders(): Text
    var
        Ct: HttpContent;
        H: HttpHeaders;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        Ct.GetHeaders(H);
        H.Add('X-A', 'b');
        Ct.ReadAs(S);
    end;

    procedure ImplReadInPlace(): Text
    var
        S: SecretText;
    begin
        ImplCreate();
        Kept.ReadAs(S);
    end;

    procedure ImplCopyWithoutIsSecretContent(): Text
    var
        Copy: HttpContent;
        S: SecretText;
    begin
        ImplCreate();
        Copy := Kept;
        Copy.ReadAs(S);
    end;

    procedure ImplReadTwice(): Text
    var
        Copy: HttpContent;
        S: SecretText;
    begin
        ImplCreate();
        Copy := Kept;
        if Copy.IsSecretContent() then;
        Copy.ReadAs(S);
        Copy.ReadAs(S);
    end;

    procedure ImplWithoutHeaders(): Text
    begin
        ClearAll();
        Kept.Clear();
        Kept.WriteFrom(SecretStrSubstNo('test'));
        exit(Describe(GetContent()));
    end;

    procedure ClearedThenRead(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.Clear();
        Ct.ReadAs(S);
    end;

    procedure ClearedThenText(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.Clear();
        Ct.WriteFrom('abc');
        Ct.ReadAs(S);
    end;

    procedure RequestInAVariant(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        V: Variant;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        Req.Content(Ct);
        V := Req;
        Req.Content.ReadAs(S);
    end;

    procedure SecretInAVariant(): Text
    var
        Ct: HttpContent;
        V: Variant;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        V := Ct;
        Ct.ReadAs(S);
    end;

    procedure FreshInAVariant(): Text
    var
        Ct: HttpContent;
        V: Variant;
        S: SecretText;
    begin
        V := Ct;
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        Ct.ReadAs(S);
    end;

    procedure FreshAssignedTwice(): Text
    var
        Ct: HttpContent;
        C2: HttpContent;
        C3: HttpContent;
        S: SecretText;
    begin
        C2 := Ct;
        C3 := Ct;
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        Ct.ReadAs(S);
    end;

    procedure RequestCopied(): Text
    var
        Req: HttpRequestMessage;
        Req2: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Req.Content(Ct);
        Req2 := Req;
        Req2.Content.ReadAs(S);
    end;

    procedure RequestCopiedReadOnTheOriginal(): Text
    var
        Req: HttpRequestMessage;
        Req2: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Req.Content(Ct);
        Req2 := Req;
        Req.Content.ReadAs(S);
    end;

    procedure RequestCopiedThenContentSet(): Text
    var
        Req: HttpRequestMessage;
        Req2: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Req2 := Req;
        Ct.WriteFrom('abc');
        Req2.Content(Ct);
        Req2.Content.ReadAs(S);
    end;

    procedure RequestByValue(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
    begin
        Ct.WriteFrom('abc');
        Req.Content(Ct);
        ReadByValue(Req);
    end;

    local procedure ReadByValue(Req: HttpRequestMessage)
    var
        S: SecretText;
    begin
        Req.Content.ReadAs(S);
    end;

    procedure RequestWithoutContentCopied(): Text
    var
        Req: HttpRequestMessage;
        Req2: HttpRequestMessage;
        S: SecretText;
    begin
        Req2 := Req;
        Req2.Method := 'GET';
        Req2.Content.ReadAs(S);
    end;

    procedure RequestContentSetTwice(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        Ct2: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Ct2.WriteFrom('def');
        Req.Content(Ct);
        Req.Content(Ct2);
        Req.Content.ReadAs(S);
    end;

    procedure RequestContentFromAReadContent(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        Ct.ReadAs(S);
        Req.Content(Ct);
        Req.Content.ReadAs(S);
    end;

    procedure RequestInPlaceTwice(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('s3cret'));
        Req.Content(Ct);
        Req.Content.ReadAs(S);
        Req.Content.ReadAs(S);
    end;

    procedure RequestPlainInPlaceTwice(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Req.Content(Ct);
        Req.Content.ReadAs(S);
        Req.Content.ReadAs(S);
    end;

    procedure RequestSecretCopyBeforeTheReadsInPlace(): Text
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
    end;

    procedure RequestPlainCopy(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        Copy: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Req.Content(Ct);
        Req.Content.ReadAs(S);
        if Req.Content.ReadAs(S) then;
        Copy := Req.Content;
        Copy.ReadAs(S);
    end;

    procedure RequestSecretCopyOfACopy(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        Copy: HttpContent;
        Copy2: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom(SecretStrSubstNo('token=abc'));
        Req.Content(Ct);
        Req.Content.ReadAs(S);
        if Req.Content.ReadAs(S) then;
        Copy := Req.Content;
        Copy2 := Copy;
        Copy2.ReadAs(S);
    end;

    procedure RequestContentWrittenInPlace(): Text
    var
        Req: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Req.Content(Ct);
        Req.Content.WriteFrom('x');
        Req.Content.ReadAs(S);
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
    end;

    procedure ResponseFresh(): Text
    var
        R: HttpResponseMessage;
        S: SecretText;
    begin
        R.Content.ReadAs(S);
    end;

    procedure ResponseWritten(): Text
    var
        R: HttpResponseMessage;
        S: SecretText;
    begin
        R.Content.WriteFrom('resp');
        R.Content.ReadAs(S);
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
    end;

    procedure AfterClear(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        Ct.Clear();
        Ct.ReadAs(S);
    end;

    procedure RequestClearedContent(): Text
    var
        Req: HttpRequestMessage;
        S: SecretText;
    begin
        Req.Content.Clear();
        Req.Content.ReadAs(S);
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
    end;

    procedure PlainIsSecretContent(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('abc');
        if Ct.IsSecretContent() then;
        Ct.ReadAs(S);
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
    end;

    procedure WrittenTwice(): Text
    var
        Ct: HttpContent;
        S: SecretText;
    begin
        Ct.WriteFrom('a');
        Ct.WriteFrom('b');
        Ct.ReadAs(S);
    end;

    procedure NoContentStatementTwice(): Text
    var
        Req: HttpRequestMessage;
        S: SecretText;
    begin
        if TryRequestStatement(Req, S) then;
        Req.Content.ReadAs(S);
    end;

    procedure NoContentCopyStatementFirst(): Text
    var
        Req: HttpRequestMessage;
        Copy: HttpContent;
        S: SecretText;
    begin
        Copy := Req.Content;
        Copy.ReadAs(S);
    end;

    // ---- reads with the result used on a request without content that no evidence covers

    procedure NoContentUsedFirst(): Text
    var
        Req: HttpRequestMessage;
        S: SecretText;
    begin
        exit(Format(Req.Content.ReadAs(S)));
    end;

    procedure NoContentCopyUsedFirst(): Text
    var
        Req: HttpRequestMessage;
        Copy: HttpContent;
        S: SecretText;
    begin
        Copy := Req.Content;
        exit(Format(Copy.ReadAs(S)));
    end;

    procedure NoContentUsedTwice(): Text
    var
        Req: HttpRequestMessage;
        S: SecretText;
    begin
        if TryRequestStatement(Req, S) then;
        if Req.Content.ReadAs(S) then;
        exit(Format(Req.Content.ReadAs(S)));
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

    procedure NoContentRequestCopiedUsed(): Text
    var
        Req: HttpRequestMessage;
        Req2: HttpRequestMessage;
        S: SecretText;
    begin
        if TryRequestStatement(Req, S) then;
        Req2 := Req;
        exit(Format(Req2.Content.ReadAs(S)));
    end;

    // ---- reads with the result used on a content with a body: answered whatever was done before

    procedure UsedAfterAnything(): Text
    var
        Req: HttpRequestMessage;
        Req2: HttpRequestMessage;
        Ct: HttpContent;
        S: SecretText;
        R: Text;
    begin
        Ct.WriteFrom('a');
        Ct.WriteFrom('b');
        R := Format(Ct.ReadAs(S)) + S.Unwrap();
        Req.Content(Ct);
        Req2 := Req;
        R += Format(Req2.Content.ReadAs(S)) + S.Unwrap();
        exit(R);
    end;
}
"#;

const STATEMENT: &str = "HttpContent.ReadAs(SecretText) as a statement (not pinned)";
const NO_CONTENT: &str = "HttpContent.ReadAs(SecretText) on the content of a request that has none (not pinned)";

fn assumed(routine: &str) -> String {
    take_hits();
    let got = al_testkit::text(al_testkit::run_in(SRC, "C", routine));
    assert!(
        take_hits().contains(&Assumption::HttpReadasSecretStatement),
        "{routine} not flagged"
    );
    got
}

fn refused(routine: &str, want: &str) {
    match al_testkit::run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(m)) => assert_eq!(m, want, "{routine}"),
        other => panic!("{routine}: {other:?}"),
    }
}

#[test]
fn the_probe_sequences_on_content_variables_are_assumed() {
    // N4ReadAsSecretStatement.al:120-132.
    assert_eq!(
        assumed("Contents"),
        "ok|No|s3cret body;ok|No|plain body;ok|Yes|;ok|Yes|;ok|No|s3cret body;"
    );
}

#[test]
fn the_probe_sequence_on_a_request_with_content_is_assumed() {
    // N4ReadAsSecretStatement.al:167-183.
    assert_eq!(
        assumed("RequestWithContent"),
        "ok|No|token=abc;Yes|No|token=abc;ok|No|{\"a\":1};Yes|No|{\"a\":1};ok|No|token=abc;Yes|No|token=abc;"
    );
}

#[test]
fn the_probe_sequences_on_a_request_without_content_are_assumed() {
    // N4ReadAsSecretStatement.al:193-206 and :216-223.
    assert_eq!(
        assumed("RequestWithoutContent"),
        "NotInitialized|No|before;No|No|before;NotInitialized|No|before;No|No|before;"
    );
    assert_eq!(assumed("RequestWithoutContentPlain"), "NotInitialized|No|before");
}

#[test]
fn the_pinned_http_content_impl_sequence_runs() {
    // CzHttpNoNetwork.al:34 with :80 pins `empty=No`; the body is the assumption's.
    assert_eq!(assumed("ImplShape"), "No");
}

#[test]
fn a_cleared_variable_is_a_new_content() {
    // http/client/UfHttpClear.al:83-104: after `Clear(Cnt)` the content reads, lists its headers and takes a body as a new one.
    assert_eq!(assumed("AfterClearVariable"), "");
}

#[test]
fn every_statement_read_without_evidence_keeps_the_refusal() {
    for routine in [
        "FreshTwice",
        "FreshReadThenWritten",
        "FreshUsedThenStatement",
        "SecretThrice",
        "PlainTwice",
        "SecretUsedThenStatement",
        "SecretAfterACopyWasTaken",
        "SecretAfterSetOnARequest",
        "FreshAfterACopyWasTaken",
        "FreshAfterSetOnARequest",
        "EmptySecretWritten",
        "SecretAfterPost",
        "RequestAfterSend",
        "SecretCopy",
        "SecretByValue",
        "SecretWithHeaders",
        "ImplReadInPlace",
        "ImplCopyWithoutIsSecretContent",
        "ImplReadTwice",
        "ImplWithoutHeaders",
        "ClearedThenRead",
        "ClearedThenText",
        "RequestCopied",
        "RequestCopiedReadOnTheOriginal",
        "RequestCopiedThenContentSet",
        "RequestByValue",
        "RequestWithoutContentCopied",
        "RequestContentSetTwice",
        "RequestContentFromAReadContent",
        "RequestInPlaceTwice",
        "RequestPlainInPlaceTwice",
        "RequestSecretCopyBeforeTheReadsInPlace",
        "RequestPlainCopy",
        "RequestSecretCopyOfACopy",
        "RequestContentWrittenInPlace",
        "FromStream",
        "StreamThenText",
        "ResponseFresh",
        "ResponseWritten",
        "ResponseCopy",
        "AfterTextRead",
        "AfterClear",
        "RequestClearedContent",
        "PlainWithHeaders",
        "PlainIsSecretContent",
        "PlainCopy",
        "WrittenTwice",
        "NoContentStatementTwice",
        "NoContentCopyStatementFirst",
    ] {
        refused(routine, STATEMENT);
    }
}

#[test]
fn a_content_or_request_that_went_into_a_variant_or_was_assigned_untouched_keeps_the_refusal() {
    // Reviewer probe app http4 K03 and K09 of PR 7926: on main these refuse; the probe has no Variant and no copy of an
    // untouched content.
    let wrong: Vec<String> = [
        "RequestInAVariant",
        "SecretInAVariant",
        "FreshInAVariant",
        "FreshAssignedTwice",
    ]
    .into_iter()
    .filter_map(|routine| match al_testkit::run_in(SRC, "C", routine) {
        Err(Signal::Unsupported(m)) if m == STATEMENT => None,
        other => Some(format!("{routine}: {other:?}")),
    })
    .collect();
    assert!(wrong.is_empty(), "{wrong:?}");
}

#[test]
fn a_used_read_without_content_and_without_evidence_keeps_the_refusal() {
    for routine in [
        "NoContentUsedFirst",
        "NoContentCopyUsedFirst",
        "NoContentUsedTwice",
        "NoContentAfterTextReadUsed",
        "NoContentRequestCopiedUsed",
    ] {
        refused(routine, NO_CONTENT);
    }
}

#[test]
fn a_used_read_of_a_content_with_a_body_is_answered_as_on_main() {
    // xml/xml/P97ContentSecret.al:21-28 with :83-150: the result-used form gives the body; it is not part of the assumption.
    take_hits();
    assert_eq!(
        al_testkit::text(al_testkit::run_in(SRC, "C", "UsedAfterAnything")),
        "YesbYesb"
    );
    assert!(!take_hits().contains(&Assumption::HttpReadasSecretStatement));
}
