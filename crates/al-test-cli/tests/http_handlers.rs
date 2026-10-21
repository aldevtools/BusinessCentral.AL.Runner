//! `HttpClient` served by `[HttpClientHandler]` mocks, and the HTTP message types around it. Never a network:
//! a request is answered by a handler of the test, or it raises the policy error, or it is refused.
//!
//! Pins: the staging probes tests/al-language-onprem/session/PZHttpHandler.al (BC 29.0.54011.55816, codeunit
//! 61297) and PZHttpBlock.al (61298), and tests/al-language/network/lt-http/*.al (the laptop's HTTP topic, BC
//! 29.0.54011.55816). Each test names its source.

use al_sema::workspace::AppSource;
use al_test_cli::report::Outcome;
use al_test_cli::runtime::run_codeunit;
use al_vm::Engine;

fn compile(src: String) -> (al_sema::workspace::Workspace, al_sema::lower::Lowered) {
    al_sema::lower::compile(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![("f.al".into(), src)],
        }],
        &Default::default(),
    )
}

/// Runs the tests of codeunit `T`, which gets `header` as its properties and `body` as its members.
fn run_with(header: &str, body: &str) -> Vec<(String, Outcome)> {
    let (ws, l) = compile(format!("codeunit 69400 T {{ Subtype = Test; {header} {body} }}"));
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::new(l.program.clone(), blocked);
    run_codeunit(&engine, &l, oi)
}

fn all_pass(header: &str, body: &str) {
    for (name, outcome) in run_with(header, body) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

fn not_run(header: &str, body: &str) -> String {
    match run_with(header, body).remove(0).1 {
        Outcome::NotRun(w) => w,
        other => panic!("expected a refusal, got {other:?}"),
    }
}

const HANDLER: &str = r#"
    var Seen: Text; Calls: Integer; Answer: Boolean;
    [HttpClientHandler]
    procedure Handler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    var Params: Dictionary of [Text, Text]; ParamName: Text; ParamText: Text;
    begin
        Calls += 1;
        Params := Request.QueryParameters;
        foreach ParamName in Params.Keys() do
            ParamText += ParamName + '=' + Params.Get(ParamName) + '&';
        Seen += Format(Request.RequestType) + ' [' + Request.Path + '] params=[' + ParamText + '] secret=' + Format(Request.HasSecretUri) + ';';
        Response.HttpStatusCode := 201;
        Response.ReasonPhrase := 'Made Up';
        Response.Content.WriteFrom('reply ' + Format(Calls));
        Response.Headers.Add('X-Mock', 'yes');
        exit(Answer);
    end;
    local procedure Describe(Ok: Boolean; var Resp: HttpResponseMessage): Text
    var Body: Text; Values: array[1] of Text; Hdr: Text;
    begin
        Resp.Content.ReadAs(Body);
        if Resp.Headers.GetValues('X-Mock', Values) then Hdr := Values[1];
        exit('ok=' + Format(Ok) + ' status=' + Format(Resp.HttpStatusCode) + ' reason=' + Resp.ReasonPhrase + ' body=[' + Body + '] success=' + Format(Resp.IsSuccessStatusCode) + ' blocked=' + Format(Resp.IsBlockedByEnvironment) + ' header=[' + Hdr + ']');
    end;
"#;

// PZHttpHandler.al:EveryMethod_ReachesTheHandler_WithItsRequestMembers.
#[test]
fn every_method_reaches_the_handler_with_its_request_members() {
    all_pass(
        "TestHttpRequestPolicy = AllowOutboundFromHandler;",
        &format!(
            r#"{HANDLER}
    [Test] [HandlerFunctions('Handler')] procedure EveryMethod()
    var Client: HttpClient; Resp: HttpResponseMessage; Req: HttpRequestMessage; Content: HttpContent; Ok: Boolean; D: Text;
    begin
        Ok := Client.Get('https://example.invalid/a/b?x=1&y=two', Resp);
        D := Describe(Ok, Resp);
        if D <> 'ok=Yes status=201 reason=Made Up body=[reply 1] success=Yes blocked=No header=[yes]' then Error('get %1', D);
        Content.WriteFrom('payload');
        Ok := Client.Post('https://example.invalid/post', Content, Resp);
        D := Describe(Ok, Resp);
        if D <> 'ok=Yes status=201 reason=Made Up body=[reply 2] success=Yes blocked=No header=[yes]' then Error('post %1', D);
        Ok := Client.Put('https://example.invalid/put', Content, Resp);
        Ok := Client.Delete('https://example.invalid/del', Resp);
        Ok := Client.Patch('https://example.invalid/patch', Content, Resp);
        Req.Method := 'HEAD';
        Req.SetRequestUri('https://example.invalid/head?q=%20a');
        Ok := Client.Send(Req, Resp);
        D := Describe(Ok, Resp);
        if D <> 'ok=Yes status=201 reason=Made Up body=[reply 6] success=Yes blocked=No header=[yes]' then Error('send %1', D);
        if Calls <> 6 then Error('calls %1', Calls);
        if Seen <> 'Get [https://example.invalid/a/b] params=[x=1&y=two&] secret=No;Post [https://example.invalid/post] params=[] secret=No;Put [https://example.invalid/put] params=[] secret=No;Delete [https://example.invalid/del] params=[] secret=No;Patch [https://example.invalid/patch] params=[] secret=No;Head [https://example.invalid/head] params=[q= a&] secret=No;' then Error('seen %1', Seen);
    end;"#
        ),
    );
}

// PZHttpHandler.al:EmptyHandler_AnswersADefaultResponse.
#[test]
fn a_handler_that_sets_nothing_answers_200_ok() {
    all_pass(
        "TestHttpRequestPolicy = AllowOutboundFromHandler;",
        r#"
    [HttpClientHandler]
    procedure EmptyHandler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
    end;
    [Test] [HandlerFunctions('EmptyHandler')] procedure Empty()
    var Client: HttpClient; Resp: HttpResponseMessage; Body: Text; Ok: Boolean;
    begin
        Ok := Client.Get('https://example.invalid/empty', Resp);
        Resp.Content.ReadAs(Body);
        if not Ok or (Resp.HttpStatusCode <> 200) or (Resp.ReasonPhrase <> 'OK') or (Body <> '') or not Resp.IsSuccessStatusCode or Resp.IsBlockedByEnvironment then
            Error('default response');
    end;"#,
    );
}

// PZHttpHandler.al:HandlerAnsweringTrue_SendsTheRealRequest (BC 29.0): a handler that answers true lets the real request
// go; the call answers false, the handler ran once, and the response is not the one the handler could have filled (BC asks
// whether to allow the request first: see on_bc_29_a_handler_answering_true_ends_in_the_unhandled_strmenu).
#[test]
fn a_handler_that_answers_true_makes_the_call_answer_false() {
    all_pass(
        "TestHttpRequestPolicy = AllowOutboundFromHandler;",
        r#"
    var Calls: Integer;
    [HttpClientHandler]
    procedure FallThrough(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Calls += 1;
        Response.HttpStatusCode := 202;
        exit(true);
    end;
    [Test] [HandlerFunctions('FallThrough')] procedure Fall()
    var Client: HttpClient; Resp: HttpResponseMessage; Ok: Boolean;
    begin
        Ok := Client.Get('https://example.invalid/fall-through', Resp);
        if Ok or (Calls <> 1) or (Resp.HttpStatusCode = 202) then
            Error('fall through');
    end;"#,
    );
}

// PZHttpHandler.al:NoHandler_UnderAllowOutboundFromHandler and PZHttpBlock.al:NoHandler_UnderBlockOutboundRequests.
#[test]
fn a_request_without_a_handler_raises_the_policy_error() {
    for policy in ["AllowOutboundFromHandler", "BlockOutboundRequests"] {
        all_pass(
            &format!("TestHttpRequestPolicy = {policy};"),
            &format!(
                r#"
    [TryFunction] local procedure TryGet(Url: Text) var Client: HttpClient; Resp: HttpResponseMessage; begin if not Client.Get(Url, Resp) then Error('get returned false'); end;
    [Test] procedure NoHandler()
    var Ok: Boolean;
    begin
        Ok := TryGet('https://example.invalid/none');
        if Ok then Error('no error');
        if GetLastErrorText() <> 'Unhandled outbound HTTP requests are not permitted when the TestHttpRequestPolicy property is set to ''{policy}''. Please ensure that an HTTP client handler is configured.' then Error('text %1', GetLastErrorText());
        if GetLastErrorCode() <> 'TestCodeunitUnhandledHttpRequest' then Error('code %1', GetLastErrorCode());
    end;"#
            ),
        );
    }
}

// PZHttpBlock.al:Handler_UnderBlockOutboundRequests_StillAnswers (BC 29).
#[test]
fn a_handler_still_answers_under_block_outbound_requests() {
    all_pass(
        "TestHttpRequestPolicy = BlockOutboundRequests;",
        r#"
    [HttpClientHandler]
    procedure Handler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Response.HttpStatusCode := 202;
    end;
    [Test] [HandlerFunctions('Handler')] procedure Handled()
    var Client: HttpClient; Resp: HttpResponseMessage;
    begin
        if not Client.Get('https://example.invalid/handled', Resp) then Error('false');
        if Resp.HttpStatusCode <> 202 then Error('status %1', Resp.HttpStatusCode);
    end;"#,
    );
}

// Without TestHttpRequestPolicy a request to a public host goes out for real (network/TestHttpClient.al); the runtime refuses.
#[test]
fn a_request_with_no_handler_and_no_policy_is_refused_not_sent() {
    let why = not_run(
        "",
        r#"
    [Test] procedure Real()
    var Client: HttpClient; Resp: HttpResponseMessage;
    begin
        Client.Get('https://example.com', Resp);
    end;"#,
    );
    assert!(why.contains("connection"), "{why}");
}

// An HttpContent passed by reference is the caller's own object: headers handed out before still show it afterwards
// (lt-http-header-rules/P87Content.al:RewriteKeepsHeader reads the body through `Read(var C)` between header changes).
#[test]
fn an_http_content_passed_by_reference_keeps_the_headers_handed_out_before() {
    all_pass(
        "",
        r#"
    local procedure Rd(var C: HttpContent): Text
    var T: Text;
    begin
        C.ReadAs(T);
        exit(T);
    end;
    [Test] procedure Alias()
    var C: HttpContent; H: HttpHeaders; H2: HttpHeaders;
    begin
        C.WriteFrom('x');
        C.GetHeaders(H);
        if Rd(C) <> 'x' then Error('read');
        H.Remove('Content-Type');
        C.GetHeaders(H2);
        if H2.Contains('Content-Type') then Error('the old handle no longer reaches the content');
    end;"#,
    );
}

// Cache-Control directives are written in .NET's order (network/lt-http-header-names-cache-control/CacheControlPairs.al: `public, no-cache`
// reads back `public, no-cache`, `no-cache, public` too), and a known header written in lower case is stored under its canonical
// name (HeaderNameCaseRequest.al).
#[test]
fn cache_control_is_written_in_net_order_and_a_lower_case_known_name_is_canonical() {
    all_pass(
        "",
        r#"
    [Test] procedure Order()
    var Req: HttpRequestMessage; H: HttpHeaders; V: array[3] of Text;
    begin
        Req.GetHeaders(H);
        H.Add('Cache-Control', 'no-cache, public');
        if not H.GetValues('Cache-Control', V) then Error('missing');
        if V[1] <> 'public, no-cache' then Error('order %1', V[1]);
    end;"#,
    );
    all_pass(
        "",
        r#"
    [Test] procedure Case()
    var Req: HttpRequestMessage; H: HttpHeaders; Keys: List of [Text];
    begin
        Req.GetHeaders(H);
        H.Add('user-agent', 'A/1');
        Keys := H.Keys();
        if Keys.Get(1) <> 'User-Agent' then Error('key %1', Keys.Get(1));
    end;"#,
    );
}

/// The tests of codeunit `T` on the target of BC `bc`.
fn run_on(bc: &str, header: &str, body: &str) -> Vec<(String, Outcome)> {
    let target = al_version::Target::for_bc(al_version::BcVersion::parse(bc).unwrap());
    let (ws, l) = al_sema::lower::compile_for(
        vec![AppSource {
            id: "a".into(),
            name: "a".into(),
            dependencies: vec![],
            files: vec![(
                "f.al".into(),
                format!("codeunit 69400 T {{ Subtype = Test; {header} {body} }}"),
            )],
        }],
        &Default::default(),
        &target,
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let oi = l.program.objects.iter().position(|o| o.name == "T").unwrap();
    let blocked = l
        .unsupported
        .iter()
        .map(|o| o.iter().map(|r| r.first().cloned()).collect())
        .collect();
    let engine = Engine::with_target(l.program.clone(), blocked, Some(target));
    run_codeunit(&engine, &l, oi)
}

#[test]
fn before_bc_29_a_handler_does_not_answer_under_block_outbound_requests() {
    // al-language-onprem/session/PZHttpBlock.al:54-57 (Handler_UnderBlockOutboundRequests_StillAnswers): Yes on 29, the
    // unhandled-request error before.
    let body = |expected: &str| {
        format!(
            r#"
    [TryFunction]
    local procedure TryGet(Url: Text)
    var Client: HttpClient; Resp: HttpResponseMessage;
    begin
        if not Client.Get(Url, Resp) then Error('get returned false');
    end;
    [HttpClientHandler]
    procedure Handler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Response.HttpStatusCode := 202;
    end;
    [Test]
    [HandlerFunctions('Handler')]
    procedure Handled()
    var Ok: Boolean; Got: Text;
    begin
        Ok := TryGet('https://example.invalid/handled');
        Got := Format(Ok) + ' ' + GetLastErrorText() + '|' + GetLastErrorCode();
        if Got <> '{expected}' then Error('got %1', Got);
    end;
"#
        )
    };
    let header = "TestHttpRequestPolicy = BlockOutboundRequests;";
    for (name, outcome) in run_on("29.0", header, &body("Yes |")) {
        assert_eq!(outcome, Outcome::Pass, "29: {name}");
    }
    let blocked = "No Unhandled outbound HTTP requests are not permitted when the TestHttpRequestPolicy property is set to ''BlockOutboundRequests''. Please ensure that an HTTP client handler is configured.|TestCodeunitUnhandledHttpRequest";
    for bc in ["27.5", "28.5"] {
        for (name, outcome) in run_on(bc, header, &body(blocked)) {
            assert_eq!(outcome, Outcome::Pass, "{bc}: {name}");
        }
    }
}

/// The handlers and helpers of al-language-onprem/network/lt-ay-http-handler/AyHttpHandler.al (codeunit 98550).
const AY: &str = r#"
    var Seen: Text; Calls: Integer;
    [HttpClientHandler]
    procedure PathHandler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    var Params: Dictionary of [Text, Text]; ParamName: Text;
    begin
        Calls += 1;
        Seen += '[' + Request.Path + ']';
        Params := Request.QueryParameters;
        foreach ParamName in Params.Keys() do
            Seen += ParamName + '=' + Params.Get(ParamName) + '&';
        Response.HttpStatusCode := 200;
        exit(false);
    end;
    [HttpClientHandler]
    procedure NotFoundHandler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Calls += 1;
        Response.HttpStatusCode := 404;
        exit(false);
    end;
    [HttpClientHandler]
    procedure TeapotHandler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Calls += 1;
        Response.HttpStatusCode := 418;
        exit(false);
    end;
    [HttpClientHandler]
    procedure FallThroughHandler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Calls += 1;
        Response.HttpStatusCode := 202;
        exit(true);
    end;
    [TryFunction]
    local procedure TryGet(var Client: HttpClient; Url: Text; var Resp: HttpResponseMessage; var Ret: Boolean)
    begin
        Ret := Client.Get(Url, Resp);
    end;
    local procedure Path(var Client: HttpClient; Url: Text): Text
    var Resp: HttpResponseMessage; Ret: Boolean;
    begin
        Seen := '';
        ClearLastError();
        if TryGet(Client, Url, Resp, Ret) then
            exit(Format(Ret) + ' ' + Seen);
        exit('E[' + CopyStr(GetLastErrorText(), 1, 70) + '|' + GetLastErrorCode() + '] ' + Seen);
    end;
    local procedure Outcome(Url: Text): Text
    var Client: HttpClient; Resp: HttpResponseMessage; Ret: Boolean;
    begin
        Calls := 0;
        ClearLastError();
        if TryGet(Client, Url, Resp, Ret) then
            exit('ret=' + Format(Ret) + ' status=' + Format(Resp.HttpStatusCode) + ' reason=[' + Resp.ReasonPhrase + '] success=' + Format(Resp.IsSuccessStatusCode) + ' blocked=' + Format(Resp.IsBlockedByEnvironment) + ' last=' + GetLastErrorText() + '|' + GetLastErrorCode() + ' calls=' + Format(Calls));
        exit('error status=' + Format(Resp.HttpStatusCode) + ' last=' + GetLastErrorText() + '|' + GetLastErrorCode() + ' calls=' + Format(Calls));
    end;
    local procedure Check(Got: Text; Expected: Text)
    begin
        if Got <> Expected then Error('got %1', Got);
    end;
"#;

const ALLOW_FROM_HANDLER: &str = "TestHttpRequestPolicy = AllowOutboundFromHandler;";

fn ay_on(bc: &str, tests: &str) -> Vec<(String, Outcome)> {
    run_on(bc, ALLOW_FROM_HANDLER, &format!("{AY}{tests}"))
}

// AyHttpHandler.al:BaseAddress_RelativePaths_1 and _2 (BC 29): the path a handler sees for a relative path joined to the base
// address: a plain word joins the base directory, a rooted path replaces the base path, `../x` drops the last directory,
// and a query alone keeps the base path.
#[test]
fn on_bc_29_a_handler_sees_the_relative_path_joined_to_the_base_address() {
    let tests = r#"
    [Test] [HandlerFunctions('PathHandler')] procedure Joined()
    var Client: HttpClient;
    begin
        Client.SetBaseAddress('https://example.invalid/api/v1/');
        Check(Path(Client, 'orders'), 'Yes [https://example.invalid/api/v1/orders]');
        Check(Path(Client, '/orders'), 'Yes [https://example.invalid/orders]');
        Check(Path(Client, '../x'), 'Yes [https://example.invalid/api/x]');
        Check(Path(Client, '?q=1'), 'Yes [https://example.invalid/api/v1/]q=1&');
    end;"#;
    for (name, outcome) in ay_on("29.0", tests) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

// AyHttpHandler.al:NoBaseAddress_RelativePaths_1 and _2 (BC 29): with no base address, `a/b`, `a.b` and `?q=1` raise
// "Invalid URI: The URI is empty." (AL:UriFormat) before the handler runs.
#[test]
fn on_bc_29_a_relative_path_without_a_base_address_raises_the_empty_uri_error() {
    let tests = r#"
    [Test] [HandlerFunctions('PathHandler')] procedure NoBase()
    var Client: HttpClient;
    begin
        Check(Path(Client, 'a/b'), 'E[Invalid URI: The URI is empty.|AL:UriFormat] ');
        Check(Path(Client, 'a.b'), 'E[Invalid URI: The URI is empty.|AL:UriFormat] ');
        Check(Path(Client, '?q=1'), 'E[Invalid URI: The URI is empty.|AL:UriFormat] ');
        if Calls <> 0 then Error('calls %1', Calls);
    end;"#;
    for (name, outcome) in ay_on("29.0", tests) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

// AyHttpHandler.al (BC 27 and 28 legs): under AllowOutboundFromHandler a request is not given to the handler: every form
// raises the unhandled-request error, the handler is not called and the response keeps status 200.
#[test]
fn before_bc_29_a_handler_does_not_answer_under_allow_outbound_from_handler() {
    let unhandled = "Unhandled outbound HTTP requests are not permitted when the TestHttpRequestPolicy property is set to ''AllowOutboundFromHandler''. Please ensure that an HTTP client handler is configured.|TestCodeunitUnhandledHttpRequest";
    let short =
        "E[Unhandled outbound HTTP requests are not permitted when the TestHttpRe|TestCodeunitUnhandledHttpRequest] ";
    let tests = format!(
        r#"
    [Test] [HandlerFunctions('PathHandler')] procedure Paths()
    var Client: HttpClient; NoBase: HttpClient;
    begin
        Client.SetBaseAddress('https://example.invalid/api/v1/');
        Check(Path(Client, 'orders'), '{short}');
        Check(Path(Client, '../x'), '{short}');
        Check(Path(NoBase, 'a/b'), '{short}');
        Check(Path(NoBase, '?q=1'), '{short}');
    end;
    [Test] [HandlerFunctions('NotFoundHandler')] procedure NotFound()
    begin
        Check(Outcome('https://example.invalid/missing'), 'error status=200 last={unhandled} calls=0');
    end;
    [Test] [HandlerFunctions('FallThroughHandler')] procedure FallThrough()
    begin
        Check(Outcome('http://localhost:1/x'), 'error status=200 last={unhandled} calls=0');
    end;"#
    );
    for bc in ["27.5", "28.5"] {
        for (name, outcome) in ay_on(bc, &tests) {
            assert_eq!(outcome, Outcome::Pass, "{bc}: {name}");
        }
    }
}

// AyHttpHandler.al:Handler_OnlyStatus404 (BC 29): a handler that sets only status 404 answers true with reason "Not Found".
#[test]
fn on_bc_29_a_handler_that_sets_only_404_gives_the_reason_not_found() {
    let tests = r#"
    [Test] [HandlerFunctions('NotFoundHandler')] procedure NotFound()
    begin
        Check(Outcome('https://example.invalid/missing'), 'ret=Yes status=404 reason=[Not Found] success=No blocked=No last=| calls=1');
    end;"#;
    for (name, outcome) in ay_on("29.0", tests) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

// The reason phrase BC gives a status the handler set without a reason is pinned for 200 and 404 only: another is refused.
#[test]
fn a_handler_status_whose_reason_phrase_is_not_pinned_is_refused() {
    let tests = r#"
    [Test] [HandlerFunctions('TeapotHandler')] procedure Teapot()
    begin
        Check(Outcome('https://example.invalid/tea'), 'x');
    end;
    [Test] [HandlerFunctions('TeapotHandler')] procedure TeapotStatusOnly()
    var Client: HttpClient; Resp: HttpResponseMessage;
    begin
        if not Client.Get('https://example.invalid/tea', Resp) then Error('false');
        Check(Format(Resp.HttpStatusCode), '418');
    end;"#;
    let got = ay_on("29.0", tests);
    assert!(
        matches!(&got[0].1, Outcome::NotRun(w) if w.contains("ReasonPhrase")),
        "{got:?}"
    );
    // The status alone, with the phrase never read, runs.
    assert_eq!(got[1].1, Outcome::Pass, "{got:?}");
}

const SECURITY: &str = "There was an error while executing the HTTP request, the connecting endpoint violates security requirements. Make sure you are connecting to a valid endpoint.";
const SOCKET: &str =
    "There was an error while executing the HTTP request. Make sure you are connecting to a valid endpoint.";

// AyHttpHandler.al:Handler_FallThrough_ConnectionRefused (BC 29, corpus bump 43): a handler that answers true lets the real
// request go, with no approval prompt. A request to localhost is refused before any connection: the call answers false,
// the response reads status 0 and `NavHttpClient Request Failed`, and the last error is HttpRequest:Security.
#[test]
fn on_bc_29_a_handler_answering_true_sends_the_request_and_localhost_is_refused() {
    let tests = format!(
        r#"
    [Test] [HandlerFunctions('FallThroughHandler')] procedure FallThrough()
    begin
        Check(Outcome('http://localhost:1/x'), 'ret=No status=0 reason=[NavHttpClient Request Failed] success=No blocked=No last={SECURITY}|HttpRequest:Security calls=1');
    end;"#
    );
    for (name, outcome) in ay_on("29.0", &tests) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

// AyHttpHandler.al:PY_FailedCall_IntoFilledResponse (BC 29): a failed request overwrites a response that a handler filled
// before (status 0); PY_UnusedResult_AllowOnceRefused: with the result not used the call raises the error.
#[test]
fn on_bc_29_a_failed_request_overwrites_a_filled_response_and_raises_as_a_statement() {
    let tests = format!(
        r#"
    var Mode: Text;
    [HttpClientHandler]
    procedure ModeHandler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Calls += 1;
        if Mode = 'true' then exit(true);
        Response.HttpStatusCode := 201;
        exit(false);
    end;
    [TryFunction]
    local procedure TryStatement(var Client: HttpClient; Url: Text; var Resp: HttpResponseMessage)
    begin
        Client.Get(Url, Resp);
    end;
    [Test] [HandlerFunctions('ModeHandler')] procedure Filled()
    var Client: HttpClient; Resp: HttpResponseMessage; Ret: Boolean;
    begin
        Mode := 'status';
        Check(Format(TryGet(Client, 'https://example.invalid/one', Resp, Ret)) + Format(Ret) + Format(Resp.HttpStatusCode), 'YesYes201');
        Mode := 'true';
        Check(Format(TryGet(Client, 'http://localhost:1/two', Resp, Ret)) + Format(Ret) + Format(Resp.HttpStatusCode) + Resp.ReasonPhrase, 'YesNo0NavHttpClient Request Failed');
        ClearLastError();
        Check(Format(TryStatement(Client, 'http://localhost:1/three', Resp)) + ' ' + GetLastErrorText() + '|' + GetLastErrorCode() + ' ' + Format(Resp.HttpStatusCode), 'No {SECURITY}|HttpRequest:Security 0');
    end;"#
    );
    for (name, outcome) in ay_on("29.0", &tests) {
        assert_eq!(outcome, Outcome::Pass, "{name}");
    }
}

const DEFAULT_POLICY_TESTS: &str = r#"
    var Calls: Integer; Mode: Text;
    [HttpClientHandler]
    procedure ModeHandler(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Calls += 1;
        if Mode = 'true' then exit(true);
        Response.HttpStatusCode := 201;
        Response.ReasonPhrase := 'Mocked';
        exit(false);
    end;
    [TryFunction]
    local procedure TryGet(Url: Text; var Resp: HttpResponseMessage; var Ret: Boolean)
    var Client: HttpClient;
    begin
        Ret := Client.Get(Url, Resp);
    end;
    local procedure Outcome(NewMode: Text; Url: Text): Text
    var Resp: HttpResponseMessage; Ret: Boolean;
    begin
        Mode := NewMode;
        Calls := 0;
        ClearLastError();
        if not TryGet(Url, Resp, Ret) then exit('error');
        exit('ret=' + Format(Ret) + ' status=' + Format(Resp.HttpStatusCode) + ' reason=[' + Resp.ReasonPhrase + '] success=' + Format(Resp.IsSuccessStatusCode) + ' blocked=' + Format(Resp.IsBlockedByEnvironment) + ' last=' + GetLastErrorText() + '|' + GetLastErrorCode() + ' calls=' + Format(Calls));
    end;
    local procedure Check(Got: Text; Expected: Text)
    begin
        if Got <> Expected then Error('got %1', Got);
    end;
"#;

// al-language-onprem/network/lt-ay-http-handler/PyHttpPolicy.al (codeunits 98700 default policy and 98701 AllowAllOutboundRequests,
// the same values): from BC 29 a handler answers (Handler_AnswersFalse_Mocked: 201 Mocked, calls 1); before, it is not called and
// the request goes out for real: to example.invalid it fails with HttpRequest:Socket, to localhost with HttpRequest:Security.
#[test]
fn under_the_default_policy_a_handler_answers_from_bc_29_and_before_the_request_goes_out() {
    let fail = "status=0 reason=[NavHttpClient Request Failed] success=No blocked=No";
    for header in ["", "TestHttpRequestPolicy = AllowAllOutboundRequests;"] {
        for (bc, mocked, through) in [
            (
                "29.0",
                "ret=Yes status=201 reason=[Mocked] success=Yes blocked=No last=| calls=1".to_string(),
                1,
            ),
            (
                "28.5",
                format!("ret=No {fail} last={SOCKET}|HttpRequest:Socket calls=0"),
                0,
            ),
            (
                "27.5",
                format!("ret=No {fail} last={SOCKET}|HttpRequest:Socket calls=0"),
                0,
            ),
        ] {
            let body = format!(
                r#"{DEFAULT_POLICY_TESTS}
    [Test] [HandlerFunctions('ModeHandler')] procedure Mocked()
    begin
        Check(Outcome('false', 'https://example.invalid/mocked'), '{mocked}');
    end;
    [Test] [HandlerFunctions('ModeHandler')] procedure TrueToLocalhost()
    begin
        Check(Outcome('true', 'http://localhost:1/x'), 'ret=No {fail} last={SECURITY}|HttpRequest:Security calls={through}');
    end;
    [Test] procedure NoHandler()
    begin
        Check(Outcome('', 'http://localhost:1/x'), 'ret=No {fail} last={SECURITY}|HttpRequest:Security calls=0');
    end;"#
            );
            for (name, outcome) in run_on(bc, header, &body) {
                assert_eq!(outcome, Outcome::Pass, "{bc} [{header}] {name}");
            }
        }
    }
}

// network/lt-http-send-failure/LtDaSendFailure.al:37-104 and network/lt-cz-http-leads/CzHttpOutbound.al:91-108: with no handler a
// request goes out for real. A host under `.invalid` never resolves (HttpRequest:Socket) and an internal address is refused
// before any connection (HttpRequest:Security); as a statement the call raises that error and the response reads status 0.
// 0.0.0.0 (HttpRequest:Argument, text not pinned) and a public host stay refused: the runtime never opens a connection.
#[test]
fn a_request_to_an_unresolvable_or_internal_host_fails_without_a_connection() {
    let body = format!(
        r#"
    local procedure Brief(Url: Text): Text
    var Client: HttpClient; Resp: HttpResponseMessage; Ok: Boolean;
    begin
        ClearLastError();
        Ok := Client.Get(Url, Resp);
        exit(Format(Ok) + ' ' + Format(Resp.HttpStatusCode) + ' ' + Format(Resp.IsBlockedByEnvironment) + ' ' + GetLastErrorCode());
    end;
    [Test] procedure Hosts()
    begin
        if Brief('https://lt-da.invalid/get') <> 'No 0 No HttpRequest:Socket' then Error('invalid');
        if Brief('http://127.0.0.1:1/') <> 'No 0 No HttpRequest:Security' then Error('loopback');
        if Brief('http://127.10.20.30/') <> 'No 0 No HttpRequest:Security' then Error('loopback_other');
        if Brief('http://[::1]:1/') <> 'No 0 No HttpRequest:Security' then Error('ipv6');
        if Brief('http://10.255.255.1/') <> 'No 0 No HttpRequest:Security' then Error('ten');
        if Brief('http://192.168.255.1/') <> 'No 0 No HttpRequest:Security' then Error('lan');
        if Brief('http://172.16.255.1/') <> 'No 0 No HttpRequest:Security' then Error('lan172');
        if Brief('http://169.254.169.254/metadata/instance') <> 'No 0 No HttpRequest:Security' then Error('imds');
    end;
    [Test] procedure Statement()
    var Client: HttpClient; Resp: HttpResponseMessage; Body: Text;
    begin
        asserterror Client.Get('https://lt-da.invalid/statement', Resp);
        if GetLastErrorText() + '|' + GetLastErrorCode() <> '{SOCKET}|HttpRequest:Socket' then Error('got %1', GetLastErrorText());
        Resp.Content.ReadAs(Body);
        if (Resp.HttpStatusCode <> 0) or (Resp.ReasonPhrase <> 'NavHttpClient Request Failed') or Resp.IsSuccessStatusCode or (Body <> '') then
            Error('after');
    end;"#
    );
    for header in ["", "TestHttpRequestPolicy = AllowAllOutboundRequests;"] {
        for (name, outcome) in run_with(header, &body) {
            assert_eq!(outcome, Outcome::Pass, "[{header}] {name}");
        }
    }
    for url in ["http://0.0.0.0:1/", "https://example.com/"] {
        let why = not_run(
            "",
            &format!(
                r#"
    [Test] procedure Real()
    var Client: HttpClient; Resp: HttpResponseMessage;
    begin
        if Client.Get('{url}', Resp) then;
    end;"#
            ),
        );
        assert!(why.contains("connection"), "{url}: {why}");
    }
}

// network/lt-ay-http/AyCookieSetterExpression.al:CookieSetter_EmptyName: Name('') raises and keeps the old name.
#[test]
fn a_cookie_name_of_empty_text_raises_and_keeps_the_old_name() {
    all_pass(
        "",
        r#"
    [Test] procedure EmptyName()
    var C: Cookie; Ignored: Text;
    begin
        Ignored := C.Name('old-name');
        asserterror Ignored := C.Name('');
        if GetLastErrorText() <> 'The ''Name''='''' part of the cookie is invalid.' then Error('text %1', GetLastErrorText());
        if GetLastErrorCode() <> 'AL:Cookie' then Error('code %1', GetLastErrorCode());
        if C.Name() <> 'old-name' then Error('name %1', C.Name());
    end;"#,
    );
}

// network/lt-ay-http/AyContentLength.al:ContentLength_SecondWriteFrom: the Content-Length a read put in the headers stays when
// a later WriteFrom('') empties the body.
#[test]
fn a_content_length_from_a_read_stays_after_writing_empty_text() {
    all_pass(
        "",
        r#"
    [Test] procedure SecondWrite()
    var C: HttpContent; H: HttpHeaders; Vals: List of [Text]; T: Text;
    begin
        C.WriteFrom('hello');
        C.WriteFrom('a longer text');
        C.ReadAs(T);
        C.WriteFrom('');
        C.GetHeaders(H);
        if not H.GetValues('Content-Length', Vals) then Error('no length');
        if Vals.Get(1) <> '13' then Error('length %1', Vals.Get(1));
    end;"#,
    );
}

// After a read put a Content-Length in, a WriteFrom of another non-empty length leaves a value nothing pins: reading it is
// refused; the rest of the content still runs (P87Content.al:RewriteKeepsHeader reads only the keys).
#[test]
fn a_content_length_after_a_rewrite_of_another_length_is_refused_when_read() {
    let got = run_with(
        "",
        r#"
    [Test] procedure Other()
    var C: HttpContent; H: HttpHeaders; Vals: List of [Text]; T: Text;
    begin
        C.WriteFrom('hello');
        C.ReadAs(T);
        C.WriteFrom('a longer text');
        C.GetHeaders(H);
        if not H.Contains('Content-Length') then Error('gone');
        H.GetValues('Content-Length', Vals);
    end;"#,
    );
    assert!(
        matches!(&got[0].1, Outcome::NotRun(w) if w.contains("Content-Length")),
        "{got:?}"
    );
}

// Only the pinned no-base-address forms `a/b`, `a.b` and `?q=1` raise the empty-uri error (AyHttpHandler.al:
// NoBaseAddress_RelativePaths_1 and _2); another form reaching a handler on BC 29 is refused.
#[test]
fn an_unpinned_no_base_address_form_reaching_a_handler_is_refused() {
    for url in ["x/y", "a/b/c", "ab.cd"] {
        let tests = format!(
            r#"
    [Test] [HandlerFunctions('PathHandler')] procedure Other()
    var Client: HttpClient;
    begin
        Check(Path(Client, '{url}'), 'x');
    end;"#
        );
        let got = ay_on("29.0", &tests);
        assert!(matches!(&got[0].1, Outcome::NotRun(_)), "{url}: {got:?}");
    }
}

// GetDotNetType of an AL Enum is pinned for Enum "Sales Document Type" only (lt-bb-dotnet/BbGetDotNetType.al:341): another
// enum is refused when lowered.
#[test]
fn get_dotnet_type_of_an_unpinned_enum_is_refused() {
    let (ws, l) = compile(
        r#"codeunit 69400 T { Subtype = Test;
    [Test] procedure OtherEnum()
    var T: DotNet LtT; E: Enum "LT Other";
    begin
        T := GetDotNetType(E);
    end;
}
enum 69401 "LT Other" { value(0; A) { } }
dotnet { assembly("mscorlib") { type("System.Type"; LtT) { } } }"#
            .to_string(),
    );
    assert!(ws.diagnostics.is_empty(), "{:?}", ws.diagnostics);
    let reasons = format!("{:?}", l.unsupported);
    assert!(reasons.contains("GetDotNetType of an Enum"), "{reasons}");
}

// al-language-onprem/network/LtDaHttpHandlerOdd.al:119-120, :145-146: a handler that sets IsSuccessfulRequest to false makes
// the call answer false; the response is the one the handler filled, and as a statement the call raises nothing.
#[test]
fn a_handler_that_marks_the_request_unsuccessful_makes_the_call_answer_false() {
    all_pass(
        "TestHttpRequestPolicy = AllowOutboundFromHandler;",
        &format!(
            r#"{HANDLER}
    [HttpClientHandler]
    procedure Unsuccessful(Request: TestHttpRequestMessage; var Response: TestHttpResponseMessage): Boolean
    begin
        Response.IsSuccessfulRequest := false;
        Response.Content.WriteFrom('unsuccessful body');
        exit(false);
    end;
    [Test] [HandlerFunctions('Unsuccessful')] procedure Answer()
    var Client: HttpClient; Resp: HttpResponseMessage; Resp2: HttpResponseMessage; Ok: Boolean; D: Text;
    begin
        Ok := Client.Get('https://example.invalid/u', Resp);
        D := Describe(Ok, Resp);
        if D <> 'ok=No status=200 reason=OK body=[unsuccessful body] success=Yes blocked=No header=[]' then Error('get %1', D);
        Client.Get('https://example.invalid/u', Resp2);
        D := Describe(true, Resp2);
        if D <> 'ok=Yes status=200 reason=OK body=[unsuccessful body] success=Yes blocked=No header=[]' then Error('statement %1', D);
    end;"#
        ),
    );
}
