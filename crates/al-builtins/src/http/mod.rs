//! `HttpClient`, `HttpRequestMessage`, `HttpResponseMessage`, `HttpContent`, `HttpHeaders` and the handler-side
//! `TestHttpRequestMessage` / `TestHttpResponseMessage` (ADR-027 section 5, owner decision 1).
//!
//! No request ever leaves the process. `HttpClient.Get`, `Post`, `Put`, `Delete`, `Patch` and `Send` hand a
//! request to the session (`Ctx::http_send`), which gives it to the `[HttpClientHandler]` of the running test,
//! raises the policy error, or refuses.
//!
//! Pinned by: corpus network/TestHttpRequestMessage.al and misc/W4XmlJsonHttp.al; the staging probes
//! tests/al-language-onprem/session/PZHttpHandler.al and PZHttpBlock.al (BC 29.0.54011.55816) and
//! tests/al-language/network/lt-http/*.al (the laptop's HTTP topic, BC 29.0.54011.55816). What those do not
//! measure is refused or marked in the assumption registry.

use crate::describe::Describe;
use crate::native::{Ctx, HttpOutgoing, NativeFn, Signal, raise};
use crate::signature::Signature;
use crate::value::{
    CookieState, HeaderBag, HeaderKind, HttpClientState, HttpContentState, HttpHeaderList, HttpRequestState,
    HttpResponseState, LibraryValue, TestHttpRequestState, TestHttpResponseState, Value, new_header_list,
};
use std::cell::RefCell;
use std::rc::Rc;

type Out = Result<Value, Signal>;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn assume(a: crate::assumptions::Assumption) {
    crate::assumptions::assume(a);
}

fn lib(name: &str, v: LibraryValue) -> Value {
    Value::Library(name.into(), Rc::new(RefCell::new(v)))
}

fn text(a: &[Value], i: usize) -> Result<String, Signal> {
    match a.get(i).and_then(crate::native::text_of) {
        Some(t) => Ok(t.to_string()),
        None => unsupported(format!("a text argument, got {}", a.get(i).describe())),
    }
}

/// The text of a Text or SecretText argument, and whether it was a SecretText.
fn text_or_secret(a: &[Value], i: usize) -> Result<(String, bool), Signal> {
    match a.get(i) {
        Some(Value::Library(_, h)) => match &*h.borrow() {
            LibraryValue::Empty => Ok((String::new(), true)),
            LibraryValue::Text(s) => Ok((s.clone(), true)),
            other => unsupported(format!("a SecretText argument, got {}", other.describe())),
        },
        _ => Ok((text(a, i)?, false)),
    }
}

// ------------------------------------------------------------------------------------------------ receivers

fn receiver_cell(r: Option<&mut Value>, what: &str) -> Result<Rc<RefCell<LibraryValue>>, Signal> {
    match r {
        Some(Value::Library(_, cell)) => Ok(cell.clone()),
        _ => unsupported(format!("{what} method without a receiver")),
    }
}

fn with_request<R>(r: Option<&mut Value>, f: impl FnOnce(&mut HttpRequestState) -> R) -> Result<R, Signal> {
    let cell = receiver_cell(r, "HttpRequestMessage")?;
    let mut lib = cell.borrow_mut();
    if matches!(&*lib, LibraryValue::Empty) {
        *lib = LibraryValue::HttpRequest(HttpRequestState::default());
    }
    match &mut *lib {
        LibraryValue::HttpRequest(s) => Ok(f(s)),
        other => unsupported(format!("HttpRequestMessage method on {}", other.describe())),
    }
}

fn with_content<R>(r: Option<&mut Value>, f: impl FnOnce(&mut HttpContentState) -> R) -> Result<R, Signal> {
    let cell = receiver_cell(r, "HttpContent")?;
    let mut lib = cell.borrow_mut();
    if matches!(&*lib, LibraryValue::Empty) {
        *lib = LibraryValue::HttpContent(HttpContentState::default());
    }
    match &mut *lib {
        LibraryValue::HttpContent(s) => Ok(f(s)),
        other => unsupported(format!("HttpContent method on {}", other.describe())),
    }
}

fn with_client<R>(r: Option<&mut Value>, f: impl FnOnce(&mut HttpClientState) -> R) -> Result<R, Signal> {
    let cell = receiver_cell(r, "HttpClient")?;
    let mut lib = cell.borrow_mut();
    if matches!(&*lib, LibraryValue::Empty) {
        *lib = LibraryValue::HttpClient(HttpClientState::default());
    }
    match &mut *lib {
        LibraryValue::HttpClient(s) => Ok(f(s)),
        other => unsupported(format!("HttpClient method on {}", other.describe())),
    }
}

fn new_content() -> Value {
    lib("HttpContent", LibraryValue::HttpContent(HttpContentState::default()))
}

fn fresh_response() -> HttpResponseState {
    // The content of a response nobody filled has no Content-Type yet (lt-http-header-rules/P87Content.al:ResponseContentHeaders,
    // `freshKeys` is `0:`); the first WriteFrom gives it one.
    let content = HttpContentState {
        headers: new_header_list(HeaderKind::Content),
        ..HttpContentState::default()
    };
    HttpResponseState {
        status: 200,
        reason: "OK".into(),
        reason_unpinned: false,
        content: lib("HttpContent", LibraryValue::HttpContent(content)),
        headers: new_header_list(HeaderKind::Response),
        blocked: false,
    }
}

fn with_response<R>(r: Option<&mut Value>, f: impl FnOnce(&mut HttpResponseState) -> R) -> Result<R, Signal> {
    let cell = receiver_cell(r, "HttpResponseMessage")?;
    let mut lib = cell.borrow_mut();
    if matches!(&*lib, LibraryValue::Empty) {
        *lib = LibraryValue::HttpResponse(fresh_response());
    }
    match &mut *lib {
        LibraryValue::HttpResponse(s) => Ok(f(s)),
        other => unsupported(format!("HttpResponseMessage method on {}", other.describe())),
    }
}

/// What a handler is given to fill: 200 OK, successful, nothing blocked, an empty body
/// (network/lt-http/TestHttpMessages.al:FreshTestResponse).
pub fn fresh_test_response() -> TestHttpResponseState {
    TestHttpResponseState {
        request_failed: false,
        failure: None,
        status: 200,
        reason: "OK".into(),
        reason_set: false,
        reason_unpinned: false,
        success: true,
        blocked: false,
        content: new_content(),
        headers: new_header_list(HeaderKind::Response),
    }
}

fn with_test_response<R>(r: Option<&mut Value>, f: impl FnOnce(&mut TestHttpResponseState) -> R) -> Result<R, Signal> {
    let cell = receiver_cell(r, "TestHttpResponseMessage")?;
    let mut lib = cell.borrow_mut();
    if matches!(&*lib, LibraryValue::Empty) {
        *lib = LibraryValue::TestHttpResponse(fresh_test_response());
    }
    match &mut *lib {
        LibraryValue::TestHttpResponse(s) => Ok(f(s)),
        other => unsupported(format!("TestHttpResponseMessage method on {}", other.describe())),
    }
}

fn with_test_request<R>(r: Option<&mut Value>, f: impl FnOnce(&mut TestHttpRequestState) -> R) -> Result<R, Signal> {
    let cell = receiver_cell(r, "TestHttpRequestMessage")?;
    let mut lib = cell.borrow_mut();
    if matches!(&*lib, LibraryValue::Empty) {
        *lib = LibraryValue::TestHttpRequest(TestHttpRequestState {
            path: String::new(),
            query: Value::Dictionary(Rc::new(RefCell::new(Vec::new()))),
            has_secret_uri: false,
            request_type: Value::Null,
        });
    }
    match &mut *lib {
        LibraryValue::TestHttpRequest(s) => Ok(f(s)),
        other => unsupported(format!("TestHttpRequestMessage method on {}", other.describe())),
    }
}

fn headers_value(list: &HttpHeaderList) -> Value {
    lib("HttpHeaders", LibraryValue::HttpHeaders(list.clone()))
}

/// Points the `var Headers` argument at `list`.
fn hand_out_headers(arg: Option<&Value>, list: &HttpHeaderList) -> Result<(), Signal> {
    let Some(Value::Library(_, cell)) = arg else {
        return unsupported("GetHeaders without an HttpHeaders variable");
    };
    *cell.borrow_mut() = LibraryValue::HttpHeaders(list.clone());
    Ok(())
}

mod content;
mod cookies;
mod dates;
mod header_rules;
mod headers;
mod messages;
mod send;
mod uri;

use content::*;
use cookies::*;
use dates::*;
use header_rules::*;
use headers::*;
use messages::*;
use send::*;
use uri::*;
pub use uri::{copy_content, is_empty_uri_form, normalize_uri};

pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    let ty = |i: usize| sig.params.get(i).map(|p| p.ty.as_str()).unwrap_or("");
    let n = sig.params.len();
    match (sig.receiver.as_str(), sig.name.as_str()) {
        ("HttpContent", "WriteFrom") if n == 1 && ty(0) == "Text" => Some(write_from),
        ("HttpContent", "WriteFrom") if n == 1 && ty(0) == "InStream" => Some(write_from_stream),
        ("HttpContent", "WriteFrom") if n == 1 && ty(0) == "SecretText" => Some(write_from_secret),
        ("HttpContent", "ReadAs") if n == 1 && ty(0) == "SecretText" => Some(read_as_secret),
        ("HttpContent", "IsSecretContent") => Some(is_secret_content),
        ("HttpContent", "ReadAs") if n == 1 && ty(0) == "Text" => Some(read_as),
        ("HttpContent", "ReadAs") if n == 1 && ty(0) == "InStream" => Some(read_as_stream),
        ("HttpContent", "GetHeaders") => Some(get_headers),
        ("HttpContent", "Clear") => Some(content_clear),
        ("HttpHeaders", "Add") if n == 2 => Some(headers_add),
        ("HttpHeaders", "Remove") => Some(headers_remove),
        ("HttpHeaders", "Contains") => Some(headers_contains),
        ("HttpHeaders", "ContainsSecret") => Some(headers_contains_secret),
        ("HttpHeaders", "Clear") => Some(headers_clear),
        ("HttpHeaders", "Keys") => Some(headers_keys),
        ("HttpHeaders", "GetValues") if ty(1).starts_with("Array") || ty(1).starts_with("List") => {
            Some(headers_get_values)
        }
        ("HttpHeaders", "GetSecretValues") if ty(1).starts_with("Array") || ty(1).starts_with("List") => {
            Some(headers_get_secret_values)
        }
        ("HttpHeaders", "TryAddWithoutValidation") => Some(headers_try_add),
        ("HttpRequestMessage", "Content") => Some(request_content),
        ("HttpRequestMessage", "GetHeaders") => Some(request_get_headers),
        ("HttpRequestMessage", "Method") => Some(method),
        ("HttpRequestMessage", "SetRequestUri") if n == 1 && ty(0) == "Text" => Some(set_request_uri),
        ("HttpRequestMessage", "GetRequestUri") => Some(get_request_uri),
        ("HttpRequestMessage", "SetSecretRequestUri") if n == 1 => Some(set_secret_request_uri),
        ("HttpRequestMessage", "GetSecretRequestUri") => Some(get_secret_request_uri),
        ("HttpClient", "Get") => Some(client_get),
        ("HttpClient", "Delete") => Some(client_delete),
        ("HttpClient", "Post") => Some(client_post),
        ("HttpClient", "Put") => Some(client_put),
        ("HttpClient", "Patch") => Some(client_patch),
        ("HttpClient", "Send") => Some(client_send),
        ("HttpClient", "Timeout") => Some(client_timeout),
        ("HttpClient", "GetBaseAddress") => Some(client_get_base),
        ("HttpClient", "SetBaseAddress") => Some(client_set_base),
        ("HttpClient", "DefaultRequestHeaders") => Some(client_default_headers),
        ("HttpClient", "Clear") => Some(client_clear),
        ("HttpClient", "UseResponseCookies") => Some(client_use_cookies),
        ("HttpClient", "UseServerCertificateValidation") => Some(client_use_cert_validation),
        ("HttpClient", "AddCertificate") => Some(client_add_certificate),
        ("HttpResponseMessage", "HttpStatusCode") => Some(response_status),
        ("HttpResponseMessage", "ReasonPhrase") => Some(response_reason),
        ("HttpResponseMessage", "IsBlockedByEnvironment") => Some(response_blocked),
        ("HttpResponseMessage", "IsSuccessStatusCode") => Some(response_success),
        ("HttpResponseMessage", "Content") => Some(response_content),
        ("HttpResponseMessage", "Headers") => Some(response_headers),
        ("TestHttpRequestMessage", "RequestType") => Some(test_request_type),
        ("TestHttpRequestMessage", "Path") => Some(test_request_path),
        ("TestHttpRequestMessage", "QueryParameters") => Some(test_request_query),
        ("TestHttpRequestMessage", "HasSecretUri") => Some(test_request_secret),
        ("TestHttpResponseMessage", "HttpStatusCode") => Some(test_response_status),
        ("TestHttpResponseMessage", "ReasonPhrase") => Some(test_response_reason),
        ("TestHttpResponseMessage", "IsBlockedByEnvironment") => Some(test_response_blocked),
        ("TestHttpResponseMessage", "IsSuccessfulRequest") => Some(test_response_success),
        ("TestHttpResponseMessage", "Content") => Some(test_response_content),
        ("TestHttpResponseMessage", "Headers") => Some(test_response_headers),
        ("HttpResponseMessage", "GetCookieNames") => Some(response_cookie_names),
        ("HttpResponseMessage", "GetCookie") => Some(response_get_cookie),
        ("Cookie", "Name") => Some(cookie_name),
        ("Cookie", "Value") => Some(cookie_value),
        ("Cookie", "Domain") => Some(cookie_domain),
        ("Cookie", "Path") => Some(cookie_path),
        ("Cookie", "Secure") => Some(cookie_secure),
        ("Cookie", "HttpOnly") => Some(cookie_http_only),
        ("Cookie", "Expires") => Some(cookie_expires),
        ("FileUpload", "FileName") => Some(file_upload_name),
        ("FileUpload", "CreateInStream") => Some(file_upload_create_in),
        _ => None,
    }
}

/// How a real request to `url` fails before any connection is made, as (error text, error code), or None when its outcome
/// needs the network (the runtime never opens a connection, ADR-027). Two cases are pinned:
/// - an internal address is refused: 127.x, localhost, [::1], 10.x, 192.168.x, 172.16.x and 169.254.169.254
///   (network/lt-cz-http-leads/CzHttpOutbound.al:91-108, BC 27, 28, 29; `http://localhost:1` also in
///   al-language-onprem/network/lt-ay-http-handler/PyHttpPolicy.al:86-90 and AyHttpHandler.al:Handler_FallThrough_ConnectionRefused);
/// - a host under the `.invalid` top-level domain, which never resolves (RFC 6761), fails with HttpRequest:Socket
///   (network/lt-http-send-failure/LtDaSendFailure.al:45-49 `lt-da.invalid`, PyHttpPolicy.al:113-127 `example.invalid`).
///
/// 0.0.0.0 raises HttpRequest:Argument with a text no test pins: None.
pub fn failure_without_connection(url: &str) -> Option<(&'static str, &'static str)> {
    const SECURITY: &str = "There was an error while executing the HTTP request, the connecting endpoint violates security requirements. Make sure you are connecting to a valid endpoint.";
    const SOCKET: &str =
        "There was an error while executing the HTTP request. Make sure you are connecting to a valid endpoint.";
    let (scheme, rest) = url.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("http") && !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return None;
    }
    let host = if authority.starts_with('[') {
        &authority[..=authority.find(']')?]
    } else {
        authority.split(':').next().unwrap_or("")
    }
    .to_ascii_lowercase();
    let octets: Option<Vec<u8>> = host.split('.').map(|p| p.parse::<u8>().ok()).collect();
    let internal = match octets.as_deref() {
        Some([127, _, _, _] | [10, _, _, _] | [192, 168, _, _] | [172, 16, _, _] | [169, 254, 169, 254]) => true,
        Some(_) => false,
        None => host == "localhost" || host == "[::1]",
    };
    if internal {
        return Some((SECURITY, "HttpRequest:Security"));
    }
    if host.len() > ".invalid".len() && host.ends_with(".invalid") {
        return Some((SOCKET, "HttpRequest:Socket"));
    }
    None
}

/// The request the session gives a handler: `Path` without the query, the decoded query parameters in order.
pub fn split_url(url: &str) -> (String, Vec<(String, String)>) {
    let no_fragment = url.split('#').next().unwrap_or(url);
    let (path, query) = match no_fragment.split_once('?') {
        Some((p, q)) => (p, q),
        None => (no_fragment, ""),
    };
    let params = query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|p| match p.split_once('=') {
            Some((k, v)) => (percent_decode(k), percent_decode(v)),
            None => (percent_decode(p), String::new()),
        })
        .collect();
    (path.to_string(), params)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uris_take_their_normal_form() {
        // network/lt-http/HttpRequest.al:SetRequestUriValid, SetRequestUriRelativeAndInvalid, SetRequestUriOtherSchemes.
        let ok = |i: &str, o: &str| assert_eq!(normalize_uri(i).as_deref(), Ok(o), "{i}");
        ok("https://example.invalid/path?q=1", "https://example.invalid/path?q=1");
        ok("http://example.invalid", "http://example.invalid/");
        ok("HTTPS://EXAMPLE.invalid/A/B", "https://example.invalid/A/B");
        ok("https://example.invalid:8443/x", "https://example.invalid:8443/x");
        ok("https://example.invalid:443/x", "https://example.invalid/x");
        ok("https://example.invalid/a b", "https://example.invalid/a%20b");
        ok("https://example.invalid/x#frag", "https://example.invalid/x#frag");
        ok("https://user@example.invalid/x", "https://user@example.invalid/x");
        ok("https://example.invalid/ä", "https://example.invalid/%C3%A4");
        ok("javascript:alert(1)", "javascript:alert(1)");
        ok("ftp://example.invalid/x", "ftp://example.invalid/x");
        for bad in [
            "",
            "/relative/path",
            "orders",
            "http://",
            "ht!tp://x",
            "https://exa mple.invalid/",
            "https://example.invalid:99999/",
        ] {
            assert_eq!(normalize_uri(bad), Err(false), "{bad}");
        }
    }

    #[test]
    fn a_web_scheme_with_nothing_after_the_colon_is_no_uri() {
        // misc/lt-probes-needed-4/PnD06_SetRequestUri.al:65-67: `http:`, `https:` and `http://` are invalid.
        for bad in ["http:", "https:", "http://"] {
            assert_eq!(normalize_uri(bad), Err(false), "{bad}");
        }
        // mailto:a@b.c stays as written (:70); `http:x` is not measured.
        assert_eq!(normalize_uri("mailto:a@b.c").as_deref(), Ok("mailto:a@b.c"));
        assert_eq!(normalize_uri("http:x"), Err(true));
    }

    #[test]
    fn a_url_splits_into_path_and_decoded_query() {
        let (p, q) = split_url("https://example.invalid/head?q=%20a&x=1#f");
        assert_eq!(p, "https://example.invalid/head");
        assert_eq!(
            q,
            vec![("q".to_string(), " a".to_string()), ("x".to_string(), "1".to_string())]
        );
    }

    #[test]
    fn a_user_agent_product_outside_the_pinned_shapes_answers_false() {
        // Pinned: `A/1 (comment) B/2` and `((((` (P87Values.al:329-336), `a b`, `a/1 b/2`, `Mozilla/5.0 (X11; Linux) Foo/1`
        // (SeHeaderValues.al:302-311). A product that is not `token` or `token/token` (a lone `/`, spaces around `/`) is not
        // measured: .NET may read `a / b` as one product. Refused, not answered with an invented Format error.
        let mut b = HeaderBag::default();
        assert!(add_header(&mut b, "User-Agent", "a/1 b/2", false).is_ok());
        let mut b = HeaderBag::default();
        assert!(matches!(
            add_header(&mut b, "User-Agent", "((((", false),
            Err(Signal::Error(_))
        ));
        // Blanks around `/` are dropped; `/`, `a/b/c`, `a@b` raise an error marked with the probe (Add answers false):
        // UserAgentProductForms.al:110-121.
        let mut b = HeaderBag::default();
        assert!(add_header(&mut b, "User-Agent", "A B / C D 365 E", false).is_ok());
        assert_eq!(
            find(&b.items, "User-Agent").unwrap().1,
            vec!["A", "B/C", "D", "365", "E"]
        );
        for v in ["/", "a/b/c", "a@b"] {
            let mut b = HeaderBag::default();
            assert!(
                matches!(add_header(&mut b, "User-Agent", v, false), Err(Signal::Error(e)) if e.message.ends_with(UA_BAD_PRODUCT_PROBE)),
                "{v}"
            );
        }
    }

    #[test]
    fn header_names_and_values_are_checked_as_the_probes_measured() {
        let mut b = HeaderBag::default();
        let code = |r: Result<(), Signal>| match r {
            Err(Signal::Error(e)) => e.code,
            Ok(()) => "ok".into(),
            Err(other) => format!("{other:?}"),
        };
        assert_eq!(code(add_header(&mut b, "X-Test", "one", false)), "ok");
        assert_eq!(code(add_header(&mut b, "", "v", false)), "InvalidHeader:Argument");
        assert_eq!(code(add_header(&mut b, "Bad Name", "v", false)), "InvalidHeader:Format");
        assert_eq!(code(add_header(&mut b, "X-Bad", "a\nb", false)), "InvalidHeader:Format");
        assert_eq!(
            code(add_header(&mut b, "Content-Type", "text/plain", false)),
            "InvalidOperation"
        );
        assert_eq!(code(add_header(&mut b, "Accept", "bad", false)), "InvalidHeader:Format");
        assert_eq!(code(add_header(&mut b, "Accept", "application/json", false)), "ok");
        assert_eq!(
            code(add_header(&mut b, "Authorization", "", false)),
            "InvalidHeader:Format"
        );
        assert_eq!(code(add_header(&mut b, "Authorization", "Bearer abc", false)), "ok");
        assert_eq!(
            code(add_header(&mut b, "Date", "Tue, 15 Nov 1994 08:12:31 GMT", false)),
            "ok"
        );
        assert_eq!(
            code(add_header(&mut b, "Date", "Tue, 15 Nov 1994 08:12:32 GMT", false)),
            "InvalidHeader:Format"
        );
        assert_eq!(b.items.iter().filter(|(n, _)| n == "X-Test").count(), 1);
    }
}
