//! `HttpHeaders.Add` used as an expression and `GetValues` of a name that is not there, on request, content and response headers.
//! Every literal is what real BC answered in the corpus: network/lt-ay-http/AyHeaderAddResult.al (codeunit 80507) and
//! network/lt-http/HttpContent.al:OldHeadersHandle_AfterWriteFrom (codeunit 72019).

use al_testkit::text;

/// The helpers of AyHeaderAddResult.al:17-81: `ret=<Add result> <read back>` or `E[<error>|<code>] <read back>`, where the read back is
/// a GetValues statement inside a TryFunction (`get=E` when it raises).
const SRC: &str = r#"
codeunit 50190 C
{
    [TryFunction]
    local procedure TryAdd(var H: HttpHeaders; N: Text; V: Text; var Ret: Boolean)
    begin
        Ret := H.Add(N, V);
    end;

    [TryFunction]
    local procedure TryGetValues(var H: HttpHeaders; N: Text; var Vals: List of [Text])
    begin
        H.GetValues(N, Vals);
    end;

    local procedure ReadBack(var H: HttpHeaders; N: Text): Text
    var
        Vals: List of [Text];
        V: Text;
        Out: Text;
    begin
        if not TryGetValues(H, N, Vals) then
            exit('get=E');
        Out := 'n=' + Format(Vals.Count());
        foreach V in Vals do
            Out += '[' + V + ']';
        exit(Out);
    end;

    local procedure AddTo(var H: HttpHeaders; N: Text; V: Text): Text
    var
        Ret: Boolean;
    begin
        ClearLastError();
        if TryAdd(H, N, V, Ret) then
            exit('ret=' + Format(Ret) + ' ' + ReadBack(H, N));
        exit('E[' + CopyStr(GetLastErrorText(), 1, 60) + '|' + GetLastErrorCode() + '] ' + ReadBack(H, N));
    end;

    procedure Req(N: Text; V: Text): Text
    var
        R: HttpRequestMessage;
        H: HttpHeaders;
    begin
        R.GetHeaders(H);
        exit(AddTo(H, N, V));
    end;

    procedure Cnt(N: Text; V: Text): Text
    var
        C: HttpContent;
        H: HttpHeaders;
    begin
        C.GetHeaders(H);
        exit(AddTo(H, N, V));
    end;

    procedure Resp(N: Text; V: Text): Text
    var
        R: HttpResponseMessage;
        H: HttpHeaders;
    begin
        H := R.Headers();
        exit(AddTo(H, N, V));
    end;

    procedure Cli(N: Text; V: Text): Text
    var
        Client: HttpClient;
        H: HttpHeaders;
    begin
        H := Client.DefaultRequestHeaders();
        exit(AddTo(H, N, V));
    end;

    procedure ContentStatement(): Text
    var
        C: HttpContent;
        H: HttpHeaders;
        Vals: List of [Text];
    begin
        C.GetHeaders(H);
        C.WriteFrom('hello');
        asserterror H.GetValues('Content-Length', Vals);
        exit(GetLastErrorText() + '|' + GetLastErrorCode());
    end;

    procedure ContentUsed(): Text
    var
        C: HttpContent;
        H: HttpHeaders;
        Vals: List of [Text];
        R: Text;
    begin
        C.GetHeaders(H);
        C.WriteFrom('hello');
        ClearLastError();
        R := Format(H.GetValues('Content-Length', Vals)) + ' n=' + Format(Vals.Count());
        exit(R + '|' + GetLastErrorText() + '|' + GetLastErrorCode());
    end;
}
"#;

fn call(routine: &str, name: &str, value: &str) -> String {
    let mut b = al_testkit::Built::from_source(SRC);
    let (oi, ri) = b.routine(routine);
    let r = b
        .call(
            oi,
            ri,
            vec![
                al_builtins::value::Value::Text(name.into()),
                al_builtins::value::Value::Text(value.into()),
            ],
        )
        .map(|(v, _)| v);
    text(r)
}

const NAV_INVALID_HEADER: &str =
    "Exception of type 'Microsoft.Dynamics.Nav.Types.Exceptions.NavNclInvalidHeaderException' was thrown.";

#[test]
fn get_values_of_a_missing_content_header_raises_as_a_statement_and_sets_the_last_error_when_used() {
    // HttpContent.al:270 and :290-291 (statement), :226 and :281-283 (result used: false, nothing filled, the last error set).
    assert_eq!(
        text(al_testkit::run(SRC, "ContentStatement")),
        format!("{NAV_INVALID_HEADER}|InvalidHeader")
    );
    assert_eq!(
        text(al_testkit::run(SRC, "ContentUsed")),
        format!("No n=0|{NAV_INVALID_HEADER}|InvalidHeader")
    );
}

#[test]
fn get_values_statement_of_a_missing_name_raises_on_request_and_response_headers() {
    // AyHeaderAddResult.al:88 (accept_noslash), :200 (range_bad), :283 (enc_bad), :87 (accept_ok): the Add refused the value, GetValues raises.
    assert_eq!(call("Req", "Accept", "json"), "ret=No get=E");
    assert_eq!(call("Req", "Range", "abc"), "ret=No get=E");
    assert_eq!(call("Cnt", "Content-Encoding", "g z"), "ret=No get=E");
    assert_eq!(
        call("Req", "Accept", "application/json"),
        "ret=Yes n=1[application/json]"
    );
}

#[test]
fn add_as_an_expression_answers_false_for_every_refused_name() {
    // AyHeaderAddResult.al:159 (blank name, Argument), :186-187 (a content header on request headers and the reverse, InvalidOperation),
    // :383 (Content-Type on response headers), :157, :172 and :185 (names that are no token, Format).
    assert_eq!(call("Req", "", "v"), "ret=No get=E");
    assert_eq!(call("Req", "Bad Name", "v"), "ret=No get=E");
    assert_eq!(call("Req", "X-Ä", "v"), "ret=No get=E");
    assert_eq!(call("Req", "Content-Type", "text/plain"), "ret=No get=E");
    assert_eq!(call("Cnt", "Accept", "text/plain"), "ret=No get=E");
    assert_eq!(call("Resp", "Content-Type", "text/plain"), "ret=No get=E");
    assert_eq!(call("Resp", "Bad Name", "v"), "ret=No get=E");
}

#[test]
fn typed_request_header_values_follow_the_probe() {
    // AyHeaderAddResult.al:101 (Accept with an open quote), :241-243 (If-Range, Max-Forwards), :255-257 (Referer, Expect).
    assert_eq!(call("Req", "Accept", "a/b; q=\""), "ret=No get=E");
    assert_eq!(call("Req", "If-Range", "abc"), "ret=No get=E");
    assert_eq!(call("Req", "Max-Forwards", "5"), "ret=Yes n=1[5]");
    assert_eq!(call("Req", "Max-Forwards", "five"), "ret=No get=E");
    assert_eq!(
        call("Req", "Referer", "https://a.example/x"),
        "ret=Yes n=1[https://a.example/x]"
    );
    assert_eq!(call("Req", "Referer", "a b"), "ret=Yes n=1[a%20b]");
    assert_eq!(call("Req", "Expect", "="), "ret=No get=E");
}

#[test]
fn typed_content_header_values_follow_the_probe() {
    // AyHeaderAddResult.al:270, :284-285, :297-299, :311, :325-327.
    assert_eq!(call("Cnt", "Content-Disposition", ";;;"), "ret=No get=E");
    assert_eq!(call("Cnt", "Content-Language", "e n"), "ret=No get=E");
    assert_eq!(
        call("Cnt", "Content-Range", "bytes 0-9/10"),
        "ret=Yes n=1[bytes 0-9/10]"
    );
    assert_eq!(call("Cnt", "Content-Range", "abc"), "ret=No get=E");
    assert_eq!(
        call("Cnt", "Content-MD5", "AAECAwQFBgcICQoLDA0ODw=="),
        "ret=Yes n=1[AAECAwQFBgcICQoLDA0ODw==]"
    );
    assert_eq!(call("Cnt", "Content-MD5", "not base64!"), "ret=No get=E");
    assert_eq!(call("Cnt", "Content-Location", "a b"), "ret=Yes n=1[a%20b]");
    assert_eq!(call("Cnt", "Expires", "0"), "ret=No get=E");
    assert_eq!(call("Cnt", "Last-Modified", "soon"), "ret=No get=E");
    assert_eq!(call("Cnt", "Allow", "G E T"), "ret=No get=E");
}

#[test]
fn typed_response_header_values_follow_the_probe() {
    // AyHeaderAddResult.al:339-341, :353-355, :367-369, :381-382.
    assert_eq!(call("Resp", "Age", "12"), "ret=Yes n=1[12]");
    assert_eq!(call("Resp", "Age", "old"), "ret=No get=E");
    assert_eq!(call("Resp", "ETag", "\"abc\""), "ret=Yes n=1[\"abc\"]");
    assert_eq!(call("Resp", "ETag", "abc"), "ret=No get=E");
    assert_eq!(
        call("Resp", "Location", "https://a.example/x"),
        "ret=Yes n=1[https://a.example/x]"
    );
    assert_eq!(call("Resp", "Location", "a b"), "ret=Yes n=1[a%20b]");
    assert_eq!(call("Resp", "Retry-After", "120"), "ret=Yes n=1[120]");
    assert_eq!(call("Resp", "Retry-After", "later"), "ret=No get=E");
    assert_eq!(call("Resp", "Vary", "a b"), "ret=No get=E");
    // A lone comma is an empty list: the Add answers true and stores nothing (:381, :384).
    assert_eq!(call("Resp", "WWW-Authenticate", ","), "ret=Yes get=E");
    assert_eq!(call("Resp", "Accept-Ranges", "a b"), "ret=No get=E");
}

/// GetValues of a missing name as a statement on response headers raises an error whose text and code no corpus test pins
/// (AyHeaderAddResult.al `get=E`; the text is pinned on content headers, HttpContent.al:290-291, and on request and client headers,
/// UserAgentProductForms.al:116-118). Every read of them is
/// refused, naming the probe.
const UNPINNED_SRC: &str = r#"
table 50191 "HH Row"
{
    fields { field(1; "No."; Code[20]) { } field(2; Name; Text[50]) { } }
    keys { key(PK; "No.") { } }
}
page 50192 "HH Card"
{
    PageType = Card;
    SourceTable = "HH Row";
    layout { area(Content) { field(NameCtl; Rec.Name) { trigger OnValidate() begin Missing(); end; } } }
    local procedure Missing()
    var
        R: HttpResponseMessage;
        H: HttpHeaders;
        Vals: List of [Text];
    begin
        H := R.Headers();
        H.GetValues('X-Missing', Vals);
    end;
}
codeunit 50193 C
{
    local procedure Missing()
    var
        R: HttpResponseMessage;
        H: HttpHeaders;
        Vals: List of [Text];
    begin
        H := R.Headers();
        H.GetValues('X-Missing', Vals);
    end;

    procedure Caught(): Boolean
    begin
        asserterror Missing();
        exit(true);
    end;

    procedure Text(): Text
    begin
        asserterror Missing();
        exit(GetLastErrorText());
    end;

    procedure TextFlag(): Text
    begin
        asserterror Missing();
        exit(GetLastErrorText(true));
    end;

    procedure Code(): Text
    begin
        asserterror Missing();
        exit(GetLastErrorCode());
    end;

    procedure PageValidate()
    var
        Row: Record "HH Row";
        TP: TestPage "HH Card";
    begin
        Row."No." := 'A';
        Row.Insert();
        TP.OpenEdit();
        TP.NameCtl.SetValue('x');
    end;
}
"#;

fn refused_naming_the_probe(r: Result<al_builtins::value::Value, al_builtins::native::Signal>) -> String {
    match r {
        Err(al_builtins::native::Signal::Unsupported(why)) => {
            assert!(why.contains("http-get-values-missing-error"), "{why}");
            why
        }
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn the_unpinned_get_values_error_can_be_caught() {
    assert!(al_testkit::boolean(al_testkit::run_in(UNPINNED_SRC, "C", "Caught")));
}

#[test]
fn the_get_last_error_builtins_refuse_the_unpinned_error() {
    refused_naming_the_probe(al_testkit::run_in(UNPINNED_SRC, "C", "Text"));
    refused_naming_the_probe(al_testkit::run_in(UNPINNED_SRC, "C", "TextFlag"));
    refused_naming_the_probe(al_testkit::run_in(UNPINNED_SRC, "C", "Code"));
}

#[test]
fn a_page_validation_error_built_from_the_unpinned_error_is_refused() {
    refused_naming_the_probe(al_testkit::run_in(UNPINNED_SRC, "C", "PageValidate"));
}

// Writer SE's probes, network/lt-se-header-values/SeHeaderValues.al (codeunit 89030, corpus bump PR 737; same on BC 27.5, 28.5, 29.0).

#[test]
fn se_headers_dotnet_types_check_on_request_headers() {
    // SeHeaderValues.al:93-150 (SE_Untyped_Request_1 to _3).
    let cases = [
        ("Accept-Charset", "utf-8", "ret=Yes n=1[utf-8]"),
        ("Accept-Charset", "utf-8 ; q=0.5", "ret=Yes n=1[utf-8; q=0.5]"),
        ("Accept-Charset", "a b", "ret=No get=E"),
        ("Accept-Charset", ";", "ret=No get=E"),
        ("Accept-Language", "en-US,de;q=0.8", "ret=Yes n=2[en-US][de; q=0.8]"),
        ("Accept-Language", "e n", "ret=No get=E"),
        ("From", "a@b.example", "ret=Yes n=1[a@b.example]"),
        ("From", "not an email", "ret=Yes n=1[not an email]"),
        ("Pragma", "no-cache", "ret=Yes n=1[no-cache]"),
        ("Pragma", "a=b", "ret=Yes n=1[a=b]"),
        ("Pragma", "a b", "ret=No get=E"),
        ("TE", "trailers", "ret=Yes n=1[trailers]"),
        ("TE", "deflate;q=0.5", "ret=Yes n=1[deflate; q=0.5]"),
        ("TE", "a b", "ret=No get=E"),
        ("Trailer", "X-A", "ret=Yes n=1[X-A]"),
        ("Trailer", "a b", "ret=No get=E"),
        ("Upgrade", "websocket", "ret=Yes n=1[websocket]"),
        ("Upgrade", "HTTP/2.0", "ret=Yes n=1[HTTP/2.0]"),
        ("Upgrade", "a b", "ret=No get=E"),
        ("Via", "1.1 proxy", "ret=Yes n=1[1.1 proxy]"),
        ("Via", "1.0 fred, 1.1 p.example", "ret=Yes n=2[1.0 fred][1.1 p.example]"),
        ("Via", "bad", "ret=No get=E"),
        ("Warning", "199 - \"msg\"", "ret=Yes n=1[199 - \"msg\"]"),
        ("Warning", "bad", "ret=No get=E"),
    ];
    for (n, v, want) in cases {
        assert_eq!(call("Req", n, v), want, "{n}: {v}");
    }
}

#[test]
fn se_response_only_headers_check_on_response_headers() {
    // SeHeaderValues.al:153-175 (SE_Untyped_Response).
    let cases = [
        ("Server", "a/1.0 (x)", "ret=Yes n=2[a/1.0][(x)]"),
        ("Server", "a b", "ret=Yes n=2[a][b]"),
        ("Server", "(", "ret=No get=E"),
        (
            "Proxy-Authenticate",
            "Basic realm=\"x\"",
            "ret=Yes n=1[Basic realm=\"x\"]",
        ),
        ("Proxy-Authenticate", ",", "ret=Yes get=E"),
        ("Transfer-Encoding", "chunked", "ret=Yes n=1[chunked]"),
        ("Transfer-Encoding", "a b", "ret=No get=E"),
        ("Via", "bad", "ret=No get=E"),
        ("Pragma", "a b", "ret=No get=E"),
    ];
    for (n, v, want) in cases {
        assert_eq!(call("Resp", n, v), want, "{n}: {v}");
    }
}

#[test]
fn se_headers_of_the_other_message_kind_are_stored_as_written() {
    // SeHeaderValues.al:178-232 (SE_RequestHeaders_OnResponse, SE_ResponseHeaders_OnRequest).
    for (n, v) in [
        ("Range", "abc"),
        ("If-Match", "abc"),
        ("Accept", "json"),
        ("Authorization", "a b c"),
        ("User-Agent", "("),
        ("Host", "a b"),
        ("Referer", "a b"),
        ("Max-Forwards", "five"),
        ("Expect", "="),
    ] {
        assert_eq!(call("Resp", n, v), format!("ret=Yes n=1[{v}]"), "{n}: {v}");
    }
    for (n, v) in [
        ("ETag", "abc"),
        ("Age", "old"),
        ("Location", "a b"),
        ("Retry-After", "later"),
        ("Vary", "a b"),
        ("Accept-Ranges", "a b"),
        ("WWW-Authenticate", ","),
        ("Server", "("),
        ("Proxy-Authenticate", ","),
    ] {
        assert_eq!(call("Req", n, v), format!("ret=Yes n=1[{v}]"), "{n}: {v}");
    }
}

#[test]
fn se_client_headers_check_like_request_headers() {
    // SeHeaderValues.al:235-256 (SE_ClientHeaders).
    let cases = [
        ("Range", "abc", "ret=No get=E"),
        ("Accept", "json", "ret=No get=E"),
        ("If-Match", "abc", "ret=No get=E"),
        ("ETag", "abc", "ret=Yes n=1[abc]"),
        ("Age", "old", "ret=Yes n=1[old]"),
        ("Max-Forwards", "five", "ret=No get=E"),
        ("Referer", "a b", "ret=Yes n=1[a%20b]"),
        ("Via", "bad", "ret=No get=E"),
        ("Date", "yesterday", "ret=No get=E"),
    ];
    for (n, v, want) in cases {
        assert_eq!(call("Cli", n, v), want, "{n}: {v}");
    }
}

#[test]
fn se_value_shapes_of_the_typed_headers() {
    // SeHeaderValues.al:259-437 (SE_Shapes_*).
    let cases = [
        (
            "Req",
            "Accept",
            "text/html, application/json;q=0.9",
            "ret=Yes n=2[text/html][application/json; q=0.9]",
        ),
        (
            "Req",
            "Accept",
            "text/html;q=0.9 ; level=1",
            "ret=Yes n=1[text/html; q=0.9; level=1]",
        ),
        ("Req", "Accept", "*/*", "ret=Yes n=1[*/*]"),
        ("Req", "Accept", "text/*", "ret=Yes n=1[text/*]"),
        ("Req", "Accept", "*", "ret=No get=E"),
        (
            "Req",
            "Accept-Encoding",
            "gzip, deflate;q=0.5",
            "ret=Yes n=2[gzip][deflate; q=0.5]",
        ),
        ("Req", "Accept-Encoding", "gzip;q=1.0", "ret=Yes n=1[gzip; q=1.0]"),
        ("Req", "Connection", " close ", "ret=Yes n=1[close]"),
        (
            "Req",
            "Connection",
            "keep-alive, Upgrade",
            "ret=Yes n=2[keep-alive][Upgrade]",
        ),
        ("Req", "Authorization", "Bearer  abc", "ret=Yes n=1[Bearer abc]"),
        ("Req", "Authorization", "Basic", "ret=Yes n=1[Basic]"),
        ("Req", "Authorization", "Bearer a b", "ret=Yes n=1[Bearer a b]"),
        (
            "Req",
            "User-Agent",
            "Mozilla/5.0 (X11; Linux) Foo/1",
            "ret=Yes n=3[Mozilla/5.0][(X11; Linux)][Foo/1]",
        ),
        ("Req", "User-Agent", "a b", "ret=Yes n=2[a][b]"),
        ("Req", "User-Agent", "a/1 b/2", "ret=Yes n=2[a/1][b/2]"),
        ("Req", "Proxy-Authorization", "Basic  x", "ret=Yes n=1[Basic x]"),
        ("Req", "Range", "bytes=0-9, 20-29", "ret=Yes n=1[bytes=0-9, 20-29]"),
        ("Req", "Range", "bytes=-5", "ret=Yes n=1[bytes=-5]"),
        ("Req", "Range", "bytes=x", "ret=No get=E"),
        ("Req", "Range", "items=1-2", "ret=Yes n=1[items=1-2]"),
        ("Req", "Host", "a.example:80", "ret=Yes n=1[a.example:80]"),
        ("Req", "Host", "a.example:", "ret=Yes n=1[a.example:]"),
        ("Req", "Host", "[::1]:8080", "ret=Yes n=1[[::1]:8080]"),
        ("Req", "Host", "A.Example", "ret=Yes n=1[A.Example]"),
        (
            "Req",
            "Date",
            "Tue, 15 Nov 1994 08:12:31 GMT",
            "ret=Yes n=1[Tue, 15 Nov 1994 08:12:31 GMT]",
        ),
        (
            "Req",
            "Date",
            "Tuesday, 15-Nov-94 08:12:31 GMT",
            "ret=Yes n=1[Tue, 15 Nov 1994 08:12:31 GMT]",
        ),
        (
            "Req",
            "Date",
            "Tue Nov 15 08:12:31 1994",
            "ret=Yes n=1[Tue, 15 Nov 1994 08:12:31 GMT]",
        ),
        (
            "Req",
            "Date",
            "15 Nov 1994 08:12:31 +0100",
            "ret=Yes n=1[Tue, 15 Nov 1994 07:12:31 GMT]",
        ),
        (
            "Req",
            "If-Unmodified-Since",
            "tue, 15 nov 1994 08:12:31 gmt",
            "ret=No get=E",
        ),
        (
            "Cnt",
            "Last-Modified",
            "Tue, 15 Nov 1994 08:12:31 GMT",
            "ret=Yes n=1[Tue, 15 Nov 1994 08:12:31 GMT]",
        ),
        (
            "Cnt",
            "Expires",
            "Tue, 15 Nov 1994 08:12:31 UTC",
            "ret=Yes n=1[Tue, 15 Nov 1994 08:12:31 GMT]",
        ),
        ("Req", "If-Match", "\"a\", \"b\"", "ret=Yes n=2[\"a\"][\"b\"]"),
        ("Req", "If-Match", "W/\"a\"", "ret=Yes n=1[W/\"a\"]"),
        ("Req", "If-None-Match", "*", "ret=Yes n=1[*]"),
        ("Req", "If-None-Match", "\"a\",W/\"b\"", "ret=Yes n=2[\"a\"][W/\"b\"]"),
        ("Req", "If-Range", "\"a\"", "ret=Yes n=1[\"a\"]"),
        (
            "Req",
            "If-Range",
            "Tue, 15 Nov 1994 08:12:31 GMT",
            "ret=Yes n=1[Tue, 15 Nov 1994 08:12:31 GMT]",
        ),
        ("Resp", "ETag", "W/\"a\"", "ret=Yes n=1[W/\"a\"]"),
        ("Resp", "ETag", "*", "ret=No get=E"),
        ("Cnt", "Content-Disposition", "inline", "ret=Yes n=1[inline]"),
        (
            "Cnt",
            "Content-Disposition",
            "attachment;filename=a.txt",
            "ret=Yes n=1[attachment; filename=a.txt]",
        ),
        (
            "Cnt",
            "Content-Disposition",
            "form-data; name=\"f\"; filename=\"a b.txt\"",
            "ret=Yes n=1[form-data; name=\"f\"; filename=\"a b.txt\"]",
        ),
        (
            "Cnt",
            "Content-Type",
            "text/plain ; charset=utf-8",
            "ret=No n=1[text/plain; charset=utf-8]",
        ),
        ("Cnt", "Content-Length", " 5", "ret=Yes n=1[5]"),
        ("Cnt", "Content-Length", "05", "ret=Yes n=1[5]"),
        ("Cnt", "Content-Length", "-1", "ret=No get=E"),
        ("Cnt", "Content-Language", "en, de", "ret=Yes n=2[en][de]"),
        ("Req", "Max-Forwards", "007", "ret=Yes n=1[7]"),
        ("Resp", "Age", "0", "ret=Yes n=1[0]"),
        (
            "Resp",
            "Retry-After",
            "Tue, 15 Nov 1994 08:12:31 GMT",
            "ret=Yes n=1[Tue, 15 Nov 1994 08:12:31 GMT]",
        ),
        ("Resp", "Location", "/a b?x=1", "ret=Yes n=1[/a%20b?x=1]"),
        (
            "Resp",
            "Location",
            "HTTPS://A.example/x",
            "ret=Yes n=1[https://a.example/x]",
        ),
        ("Req", "Referer", "https://a.example", "ret=Yes n=1[https://a.example/]"),
        ("Cnt", "Content-Range", "bytes */10", "ret=Yes n=1[bytes */10]"),
        ("Req", "Expect", "100-continue; a=b", "ret=Yes n=1[100-continue; a=b]"),
    ];
    for (routine, n, v, want) in cases {
        assert_eq!(call(routine, n, v), want, "{routine} {n}: {v}");
    }
}

/// AyHeaderAddResult.al:489-498 (`AddLastError`) and :552-620 (SE_AddFalse_*): an Add that answers false sets the last error to the
/// header error; one that answers true leaves it.
const LAST_ERROR_SRC: &str = r#"
codeunit 50194 C
{
    local procedure AddLastError(var H: HttpHeaders; N: Text; V: Text): Text
    var
        Ret: Boolean;
        Out: Text;
    begin
        asserterror Error('prior error');
        Ret := H.Add(N, V);
        Out := Format(Ret) + ' [' + GetLastErrorText() + '|' + GetLastErrorCode() + ']';
        ClearLastError();
        Ret := H.Add(N, V);
        exit(Out + ' / ' + Format(Ret) + ' [' + GetLastErrorText() + '|' + GetLastErrorCode() + ']');
    end;

    procedure Request(N: Text; V: Text): Text
    var
        R: HttpRequestMessage;
        H: HttpHeaders;
    begin
        R.GetHeaders(H);
        exit(AddLastError(H, N, V));
    end;

    procedure Client(N: Text; V: Text): Text
    var
        C: HttpClient;
        H: HttpHeaders;
    begin
        H := C.DefaultRequestHeaders();
        exit(AddLastError(H, N, V));
    end;

    procedure IfNot(N: Text; V: Text): Text
    var
        R: HttpRequestMessage;
        H: HttpHeaders;
        Branch: Text;
    begin
        R.GetHeaders(H);
        ClearLastError();
        Branch := 'then-not-taken';
        if not H.Add(N, V) then
            Branch := 'then-taken [' + GetLastErrorText() + '|' + GetLastErrorCode() + ']';
        exit(Branch + ' keys=' + Format(H.Keys().Count()));
    end;
}
"#;

fn call_in(src: &str, routine: &str, name: &str, value: &str) -> String {
    let mut b = al_testkit::Built::from_source(src);
    let (oi, ri) = b.routine(routine);
    let r = b
        .call(
            oi,
            ri,
            vec![
                al_builtins::value::Value::Text(name.into()),
                al_builtins::value::Value::Text(value.into()),
            ],
        )
        .map(|(v, _)| v);
    text(r)
}

#[test]
fn an_add_that_answers_false_sets_the_last_error() {
    let format = "[Invalid HTTP header. Please, make sure the format of the header is correct.|InvalidHeader:Format]";
    let argument =
        "[Invalid HTTP header. Please, make sure the format of the header is correct.|InvalidHeader:Argument]";
    let operation = "[The requested operation cannot be performed in this context.|InvalidOperation]";
    let both = |e: &str| format!("No {e} / No {e}");
    // AyHeaderAddResult.al:499-519 (PY_AddFalse_LastError_Request), :555-570 (SE_AddFalse_LastError_EmptyName).
    assert_eq!(call_in(LAST_ERROR_SRC, "Request", "Bad Name", "x"), both(format));
    assert_eq!(
        call_in(LAST_ERROR_SRC, "Request", "Content-Type", "text/plain"),
        both(operation)
    );
    assert_eq!(call_in(LAST_ERROR_SRC, "Request", "Accept", "json"), both(format));
    assert_eq!(call_in(LAST_ERROR_SRC, "Request", "", "x"), both(argument));
    assert_eq!(
        call_in(LAST_ERROR_SRC, "Request", "X-Ok", "v"),
        "Yes [prior error|Dialog] / Yes [|]"
    );
    // :572-589 (SE_AddFalse_LastError_ClientHeaders).
    assert_eq!(call_in(LAST_ERROR_SRC, "Client", "Accept", "json"), both(format));
    assert_eq!(
        call_in(LAST_ERROR_SRC, "Client", "Content-Type", "text/plain"),
        both(operation)
    );
    assert_eq!(call_in(LAST_ERROR_SRC, "Client", "", "x"), both(argument));
    // :591-620 (SE_AddFalse_LastError_IfNotForm).
    assert_eq!(
        call_in(LAST_ERROR_SRC, "IfNot", "Accept", "json"),
        format!("then-taken {format} keys=0")
    );
    assert_eq!(
        call_in(LAST_ERROR_SRC, "IfNot", "Content-Type", "text/plain"),
        format!("then-taken {operation} keys=0")
    );
    assert_eq!(
        call_in(LAST_ERROR_SRC, "IfNot", "", "x"),
        format!("then-taken {argument} keys=0")
    );
}

/// network/lt-ay-http/AyContentLength.al:249-365 (SE_ContentLength_*).
const LENGTH_SRC: &str = r#"
codeunit 50195 C
{
    local procedure Show(var H: HttpHeaders): Text
    var
        Vals: List of [Text];
        V: Text;
        Out: Text;
    begin
        Out := Format(H.Contains('Content-Length'));
        if H.GetValues('Content-Length', Vals) then
            foreach V in Vals do
                Out += '[' + V + ']';
        exit(Out);
    end;

    local procedure Len(var C: HttpContent): Text
    var
        H: HttpHeaders;
    begin
        C.GetHeaders(H);
        exit(Show(H));
    end;

    procedure ByHand(): Text
    var
        C: HttpContent;
        H: HttpHeaders;
        T: Text;
        Out: Text;
    begin
        C.WriteFrom('hello');
        C.GetHeaders(H);
        Out := Format(H.Add('Content-Length', '3')) + ' ' + Show(H);
        C.ReadAs(T);
        exit(Out + ' ' + Show(H) + ' read=' + T + ' ' + Len(C));
    end;

    procedure EmptyBody(): Text
    var
        C: HttpContent;
        T: Text;
        Out: Text;
    begin
        C.WriteFrom('');
        Out := Len(C);
        C.ReadAs(T);
        Out += ' ' + Len(C) + ' read=' + T;
        C.WriteFrom('hello');
        C.ReadAs(T);
        exit(Out + ' ' + Len(C) + ' read=' + T);
    end;

    procedure InStreamThenText(): Text
    var
        C: HttpContent;
        InS: InStream;
        T: Text;
        Out: Text;
    begin
        C.WriteFrom('hello');
        C.ReadAs(InS);
        Out := Len(C);
        C.ReadAs(T);
        exit(Out + ' ' + Len(C) + ' read=' + T);
    end;

    procedure LongerAfterRead(): Text
    var
        C: HttpContent;
        T: Text;
        Out: Text;
    begin
        C.WriteFrom('hello');
        C.ReadAs(T);
        C.WriteFrom('a longer text');
        Out := Len(C);
        C.ReadAs(T);
        exit(Out + ' ' + Len(C) + ' read=' + T);
    end;
}
"#;

#[test]
fn a_read_adds_the_content_length_only_when_it_is_missing() {
    // :333-351 (SE_ContentLength_AddedByHandBeforeRead): a hand-set value stays.
    assert_eq!(
        text(al_testkit::run(LENGTH_SRC, "ByHand")),
        "Yes Yes[3] Yes[3] read=hello Yes[3]"
    );
    // :313-331 (SE_ContentLength_EmptyBody).
    assert_eq!(
        text(al_testkit::run(LENGTH_SRC, "EmptyBody")),
        "No Yes[0] read= Yes[0] read=hello"
    );
    // :285-300 (SE_ContentLength_InStreamThenText).
    assert_eq!(
        text(al_testkit::run(LENGTH_SRC, "InStreamThenText")),
        "No Yes[5] read=hello"
    );
    // AyContentLength.al:PY_ContentLength_LongerBodyAfterRead (`rewritten` and `second_read` Yes[5]).
    assert_eq!(
        text(al_testkit::run(LENGTH_SRC, "LongerAfterRead")),
        "Yes[5] Yes[5] read=a longer text"
    );
}

/// Writer Y2's value shapes (network/lt-se-header-values/SeHeaderValues.al:507-614).
#[test]
fn y2_date_and_quality_shapes() {
    let cases = [
        (
            "Req",
            "If-Modified-Since",
            "Sun, 06 Nov 49 08:49:37 GMT",
            "ret=No get=E",
        ),
        (
            "Req",
            "If-Modified-Since",
            "Sun, 06 Nov 50 08:49:37 GMT",
            "ret=No get=E",
        ),
        (
            "Req",
            "If-Modified-Since",
            "Sun, 06 Nov 1994 08:49:37 -0500",
            "ret=Yes n=1[Sun, 06 Nov 1994 13:49:37 GMT]",
        ),
        (
            "Req",
            "If-Modified-Since",
            "Sun, 06 Nov 1994 08:49:37 +0000",
            "ret=Yes n=1[Sun, 06 Nov 1994 08:49:37 GMT]",
        ),
        (
            "Req",
            "If-Modified-Since",
            "sun, 06 nov 1994 08:49:37 GMT",
            "ret=Yes n=1[Sun, 06 Nov 1994 08:49:37 GMT]",
        ),
        (
            "Req",
            "If-Modified-Since",
            "Sun,06 Nov 1994 08:49:37 GMT",
            "ret=Yes n=1[Sun, 06 Nov 1994 08:49:37 GMT]",
        ),
        ("Req", "If-Modified-Since", "06 Nov 1994", "ret=No get=E"),
        (
            "Req",
            "If-Modified-Since",
            "Sun, 6 Nov 1994 8:49:37 GMT",
            "ret=Yes n=1[Sun, 06 Nov 1994 08:49:37 GMT]",
        ),
        ("Req", "Accept-Charset", "utf-8;q=x", "ret=No get=E"),
    ];
    for (routine, name, value, want) in cases {
        assert_eq!(call(routine, name, value), want, "{name}: {value}");
    }
}

/// network/lt-se-header-values/UserAgentProductForms.al (codeunit 91350): the read back with the GetValues error and Contains.
const UA_SRC: &str = r#"
codeunit 50195 U
{
    [TryFunction]
    local procedure TryAdd(var H: HttpHeaders; V: Text; var Ret: Boolean)
    begin
        Ret := H.Add('User-Agent', V);
    end;

    [TryFunction]
    local procedure TryGetValues(var H: HttpHeaders; var Vals: List of [Text])
    begin
        H.GetValues('User-Agent', Vals);
    end;

    local procedure AddTo(var H: HttpHeaders; V: Text): Text
    var
        Vals: List of [Text];
        S: Text;
        Out: Text;
        Ret: Boolean;
    begin
        ClearLastError();
        if not TryAdd(H, V, Ret) then
            exit('E');
        Out := 'ret=' + Format(Ret) + ' ';
        if TryGetValues(H, Vals) then begin
            Out += 'n=' + Format(Vals.Count());
            foreach S in Vals do
                Out += '[' + S + ']';
        end else
            Out += 'get=E[' + CopyStr(GetLastErrorText(), 1, 80) + '|' + GetLastErrorCode() + ']';
        ClearLastError();
        exit(Out + ' contains=' + Format(H.Contains('User-Agent')));
    end;

    procedure Req(V: Text): Text
    var
        R: HttpRequestMessage;
        H: HttpHeaders;
    begin
        R.GetHeaders(H);
        exit(AddTo(H, V));
    end;

    procedure Cli(V: Text): Text
    var
        Client: HttpClient;
        H: HttpHeaders;
    begin
        H := Client.DefaultRequestHeaders();
        exit(AddTo(H, V));
    end;
}
"#;

#[test]
fn user_agent_product_forms_follow_the_probe() {
    // UserAgentProductForms.al:110-121 (request headers) and :132-143 (client headers).
    let bad = "ret=No get=E[Exception of type 'Microsoft.Dynamics.Nav.Types.Exceptions.NavNclInvalidHeaderEx|InvalidHeader] contains=No";
    for f in ["Req", "Cli"] {
        let call = |v: &str| {
            let mut b = al_testkit::Built::from_source(UA_SRC);
            let (oi, ri) = b.routine_in("U", f);
            text(
                b.call(oi, ri, vec![al_builtins::value::Value::Text(v.into())])
                    .map(|(v, _)| v),
            )
        };
        assert_eq!(call("A B / C D 365 E"), "ret=Yes n=5[A][B/C][D][365][E] contains=Yes");
        assert_eq!(call("a / b"), "ret=Yes n=1[a/b] contains=Yes");
        assert_eq!(call("a /1"), "ret=Yes n=1[a/1] contains=Yes");
        assert_eq!(call("a/ 1"), "ret=Yes n=1[a/1] contains=Yes");
        assert_eq!(call("/"), bad);
        assert_eq!(call("a/b/c"), bad);
        assert_eq!(call("a@b"), bad);
    }
}
