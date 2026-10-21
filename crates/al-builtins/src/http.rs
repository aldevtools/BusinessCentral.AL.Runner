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

/// The Content-Length value a WriteFrom of another length left after a read put the header in: `GetValues` of it is refused.
const UNPINNED_LENGTH: &str = "\u{1}unmeasured Content-Length";

// ------------------------------------------------------------------------------------------------ uri

const INVALID_URL: &str = "Failed to create the URL for the request. Please, make sure to set a valid URL.";

/// What `System.Uri` makes of a text, for the forms the probes measured (network/lt-http/HttpRequest.al:
/// SetRequestUriValid, SetRequestUriRelativeAndInvalid, SetRequestUriOtherSchemes, SetRequestUriLongAndUnicode).
/// `Err(false)` is an invalid uri, `Err(true)` a text whose validity is not measured.
pub fn normalize_uri(input: &str) -> Result<String, bool> {
    if input.is_empty() {
        return Err(false);
    }
    // A rooted path is not an absolute uri on the Windows tier (network/lt-http/HttpRequest.al:
    // SetRequestUriRelativeAndInvalid); it is the invalid-url error.
    if input.starts_with('/') && !input.starts_with("//") {
        return Err(false);
    }
    // A text with no colon has no scheme and is no absolute uri: 'orders' (misc/lt-probes-needed-4/PnD06_SetRequestUri.al is the wide pin:
    // `/api/v1/items`, `api/v1/items`, `./items`, `../items`, `?a=1`, `#top`, `/`; network/lt-http-relative-request/HttpRelativeRequestPath.al:
    // P168_Send_RelativeRequestUri `af109` for `orders`) is invalid.
    let Some(colon) = input.find(':') else {
        return Err(false);
    };
    let scheme = &input[..colon];
    let scheme_ok = !scheme.is_empty()
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme.chars().all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c));
    let rest = &input[colon + 1..];
    if !scheme_ok {
        return if rest.starts_with("//") || input.chars().any(char::is_whitespace) {
            Err(false)
        } else {
            Err(true)
        };
    }
    let scheme = scheme.to_ascii_lowercase();
    // `http:` and `https:` with nothing after the colon are no uri (misc/lt-probes-needed-4/PnD06_SetRequestUri.al:65-66);
    // a web scheme followed by something other than `//` is not measured.
    if matches!(scheme.as_str(), "http" | "https") && !rest.starts_with("//") {
        return Err(!rest.is_empty());
    }
    let Some(after) = rest.strip_prefix("//") else {
        // mailto:a@b.c, javascript:alert(1): kept as written.
        return Ok(format!("{scheme}:{rest}"));
    };
    let end = after.find(['/', '?', '#']).unwrap_or(after.len());
    let (authority, tail) = after.split_at(end);
    let (userinfo, hostport) = match authority.rfind('@') {
        Some(i) => (Some(&authority[..i]), &authority[i + 1..]),
        None => (None, authority),
    };
    let (host, port) = match hostport.rfind(':') {
        Some(i) if !hostport[i..].contains(']') => (&hostport[..i], Some(&hostport[i + 1..])),
        _ => (hostport, None),
    };
    let file_like = scheme == "file";
    if host.is_empty() && !file_like {
        return Err(false);
    }
    if host
        .chars()
        .any(|c| c.is_whitespace() || c.is_control() || "\\^|<>\"`{}".contains(c))
    {
        return Err(false);
    }
    let port = match port {
        None | Some("") => None,
        Some(p) => match p.parse::<u32>() {
            Ok(n) if n <= 65535 && p.chars().all(|c| c.is_ascii_digit()) => Some(n),
            _ => return Err(false),
        },
    };
    let default_port = match scheme.as_str() {
        "http" | "ws" => Some(80),
        "https" | "wss" => Some(443),
        "ftp" => Some(21),
        _ => None,
    };
    let port = port.filter(|p| Some(*p) != default_port);
    let (path_query, fragment) = match tail.find('#') {
        Some(i) => (&tail[..i], Some(&tail[i + 1..])),
        None => (tail, None),
    };
    let (path, query) = match path_query.find('?') {
        Some(i) => (&path_query[..i], Some(&path_query[i + 1..])),
        None => (path_query, None),
    };
    let path = if path.is_empty() && !file_like {
        "/".to_string()
    } else {
        escape(&path.replace('\\', "/"))
    };
    let mut out = format!("{scheme}://");
    if let Some(u) = userinfo {
        out.push_str(u);
        out.push('@');
    }
    out.push_str(&host.to_ascii_lowercase());
    if let Some(p) = port {
        out.push_str(&format!(":{p}"));
    }
    out.push_str(&path);
    if let Some(q) = query {
        out.push('?');
        out.push_str(&escape(q));
    }
    if let Some(f) = fragment {
        out.push('#');
        out.push_str(&escape(f));
    }
    Ok(out)
}

/// Percent-encodes what `System.Uri` encodes in a path or query: space, non-ASCII (as UTF-8) and the characters
/// `"<>^`{|}`.
fn escape(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii() && !c.is_ascii_control() && !" \"<>^`{|}".contains(c) {
            out.push(c);
        } else {
            let mut buf = [0u8; 4];
            for b in c.encode_utf8(&mut buf).bytes() {
                out.push_str(&format!("%{b:02X}"));
            }
        }
    }
    out
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && b.len() >= i + 3 {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Resolves `path` against the client's base address the way `new Uri(base, path)` does for the forms in use. The second
/// value says the join is one the corpus does not pin (assumption http-relative-request-path). A relative path with no base
/// address stays as written for the forms the corpus pins as reaching the request policy (`is_pinned_relative`); the
/// session then raises the policy error, or the empty-uri error once a handler is reached (`http_send_impl`).
fn resolve_request_path(base: &str, path: &str) -> Result<(String, bool), Signal> {
    if path.contains("://") || base.is_empty() {
        return match normalize_uri(path) {
            Ok(u) => Ok((u, false)),
            Err(_) if is_pinned_relative(path) => Ok((path.to_string(), false)),
            // Another relative path with no base address: whether it reaches the policy, the empty-uri error or the
            // invalid-url error is not measured (docs/probes-needed.md).
            Err(false) if !path.is_empty() && !path.contains(':') => unsupported(format!(
                "HttpClient with the relative request path `{path}` and no base address: which error it raises is not measured (docs/probes-needed.md)"
            )),
            Err(true) => unsupported(format!(
                "HttpClient with the request path `{path}` and no base address: whether it is valid is not measured"
            )),
            Err(false) => invalid_url(),
        };
    }
    let Some(after) = base.find("://").map(|i| i + 3) else {
        return unsupported("HttpClient with a base address that has no authority");
    };
    let path_start = base[after..].find('/').map_or(base.len(), |i| after + i);
    let (origin, base_path) = base.split_at(path_start);
    let (rel_path, rest) = path.split_at(path.find(['?', '#']).unwrap_or(path.len()));
    let merged = if let Some(rooted) = rel_path.strip_prefix('/') {
        format!("/{rooted}")
    } else if rel_path.is_empty() {
        base_path.to_string()
    } else {
        let dir = &base_path[..base_path.rfind('/').map_or(0, |i| i + 1)];
        format!("{}{rel_path}", if dir.is_empty() { "/" } else { dir })
    };
    let joined = format!("{origin}{}{rest}", remove_dot_segments(&merged));
    normalize_uri(&joined)
        .map(|u| (u, !is_pinned_join(base_path, path)))
        .or_else(|_| invalid_url())
}

/// The joins al-language-onprem/network/lt-ay-http-handler/AyHttpHandler.al:BaseAddress_RelativePaths_1 and _2 pin (BC 29, a
/// base address `https://example.invalid/api/v1/`): a plain word, a rooted word, `../` and a word, and a query alone.
fn is_pinned_join(base_path: &str, path: &str) -> bool {
    let word = |w: &str| !w.is_empty() && w.chars().all(|c| c.is_ascii_alphanumeric());
    base_path.ends_with('/')
        && (word(path)
            || path.strip_prefix('/').is_some_and(word)
            || path.strip_prefix("../").is_some_and(word)
            || path
                .strip_prefix('?')
                .is_some_and(|q| q.split_once('=').is_some_and(|(k, v)| word(k) && word(v))))
}

/// RFC 3986 section 5.2.4: `.` and `..` segments of a merged path are removed, as `System.Uri` does.
fn remove_dot_segments(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    let segments: Vec<&str> = path.split('/').skip(1).collect();
    for (i, seg) in segments.iter().enumerate() {
        let last = i + 1 == segments.len();
        match *seg {
            "." => {
                if last {
                    out.push("");
                }
            }
            ".." => {
                out.pop();
                if last {
                    out.push("");
                }
            }
            s => out.push(s),
        }
    }
    format!("/{}", out.join("/"))
}

/// The relative paths the corpus pins as reaching the request policy with no base address: a plain word and a rooted path
/// (network/lt-http-relative-request/HttpRelativeRequestPath.al:P168_Get_NoBaseAddress `orders`, `/orders`), and `a/b`, `a.b`
/// and `?q=1` (al-language-onprem/network/lt-ay-http-handler/AyHttpHandler.al:NoBaseAddress_RelativePaths_1 and _2: the
/// unhandled-request error on BC 27 and 28, where the handler is not reached).
fn is_pinned_relative(path: &str) -> bool {
    let word = path.strip_prefix('/').unwrap_or(path);
    (!word.is_empty() && word.chars().all(|c| c.is_ascii_alphanumeric())) || is_empty_uri_form(path)
}

/// The no-base-address forms that raise "Invalid URI: The URI is empty." when they reach a handler on BC 29, exactly as pinned
/// (AyHttpHandler.al:NoBaseAddress_RelativePaths_1 `a/b`, `a.b`; _2 `?q=1`). Any other form is not measured.
pub fn is_empty_uri_form(path: &str) -> bool {
    matches!(path, "a/b" | "a.b" | "?q=1")
}

fn invalid_url<T>() -> Result<T, Signal> {
    raise(INVALID_URL, "CreateUrl:UriFormat")
}

/// `SetRequestUri` and `SetBaseAddress`: a valid uri is kept in its normal form; an invalid one answers false when
/// the result is used and raises otherwise (handlers/PWRefusedBuiltins.al:HttpRequestMessage_SetRequestUri_ValidAndInvalid).
fn checked_uri(c: &dyn Ctx, u: &str) -> Result<Option<String>, Signal> {
    match normalize_uri(u) {
        Ok(n) => Ok(Some(n)),
        Err(false) if c.ret_used() => Ok(None),
        Err(false) => invalid_url(),
        Err(true) => unsupported(format!(
            "a request uri of `{u}`: whether it is valid is not measured (an absolute uri, white space without a scheme, and the forms of network/lt-http/HttpRequest.al are)"
        )),
    }
}

/// `Method := 'GET'` sets and `Method` reads it back as written (network/TestHttpRequestMessage.al:19-25;
/// network/lt-http/HttpRequest.al:MethodValues). A request nobody gave a method is `GET`. A method with a space
/// or a colon raises `The format of the HTTP method is invalid.` (`AL:Format`).
fn method(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return Ok(Value::Text(
            with_request(r, |s| s.method.clone())?
                .unwrap_or_else(|| "GET".to_string())
                .into(),
        ));
    }
    let m = text(a, 0)?;
    if m.is_empty() || m.chars().any(|c| !is_token_char(c)) {
        return raise("The format of the HTTP method is invalid.", "AL:Format");
    }
    with_request(r, |s| s.method = Some(m)).map(|()| Value::Null)
}

fn set_request_uri(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let u = text(a, 0)?;
    match checked_uri(c, &u)? {
        Some(n) => with_request(r, |s| s.uri = Some(n)).map(|()| Value::Boolean(true)),
        None => Ok(Value::Boolean(false)),
    }
}

fn get_request_uri(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    Ok(Value::Text(
        with_request(r, |s| s.uri.clone())?.unwrap_or_default().into(),
    ))
}

fn request_get_headers(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let list = with_request(r, |s| s.headers.clone())?;
    hand_out_headers(a.first(), &list)?;
    Ok(Value::Boolean(true))
}

/// `Content := C` stores a copy of the content (misc/W4XmlJsonHttp.al:RequestContent_IsCopyOrReference: writing
/// the original afterwards does not change the request's), `Content` reads the request's own content. A request
/// nobody set a content on has an uninitialized one (network/lt-http/HttpRequest.al:FreshRequestContent).
fn request_content(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if let Some(c) = a.first() {
        let copy = copy_content(c)?;
        return with_request(r, |s| s.content = Some(copy)).map(|()| Value::Null);
    }
    // The uninitialized content is the request's own: a `Clear` on it gives the request a body and content headers that a
    // later `Content` shows (network/lt-db-http-leads/DbHttpContentLeads.al:176-219).
    with_request(r, |s| {
        s.content
            .get_or_insert_with(|| {
                lib(
                    "HttpContent",
                    LibraryValue::HttpContent(HttpContentState {
                        uninitialized: true,
                        ..HttpContentState::default()
                    }),
                )
            })
            .clone()
    })
}

/// An `HttpContent` value copied: its own data and its own headers.
pub fn copy_content(v: &Value) -> Result<Value, Signal> {
    match v {
        Value::Library(_, cell) => match &*cell.borrow() {
            LibraryValue::Empty => Ok(new_content()),
            LibraryValue::HttpContent(s) => Ok(lib("HttpContent", LibraryValue::HttpContent(s.deep_copy()))),
            other => unsupported(format!("an HttpContent was expected, got {}", other.describe())),
        },
        other => unsupported(format!("an HttpContent was expected, got {}", other.describe())),
    }
}

// ------------------------------------------------------------------------------------------------ headers

const INVALID_HEADER: &str = "Invalid HTTP header. Please, make sure the format of the header is correct.";
const GET_VALUES_MISSING: &str =
    "Exception of type 'Microsoft.Dynamics.Nav.Types.Exceptions.NavNclInvalidHeaderException' was thrown.";
const INVALID_OPERATION: &str = "The requested operation cannot be performed in this context.";

fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c)
}

/// A name the collection of `kind` refuses: the content headers on request, response or client headers, and on the headers of a content
/// every known name that is no content header (network/lt-http-header-names-cache-control/HeaderNameCaseContent.al: all 39 known names;
/// lt-http-header-rules/P87Matrix.al for the first eleven).
fn not_here(kind: HeaderKind, name: &str) -> bool {
    if kind != HeaderKind::Content {
        return is_content_header(name);
    }
    canonical_name(name).is_some() && !is_content_header(name)
}

/// The headers `HttpContentHeaders` owns: a request, response or client header collection refuses them.
fn is_content_header(name: &str) -> bool {
    [
        "allow",
        "content-disposition",
        "content-encoding",
        "content-language",
        "content-length",
        "content-location",
        "content-md5",
        "content-range",
        "content-type",
        "expires",
        "last-modified",
    ]
    .iter()
    .any(|h| name.eq_ignore_ascii_case(h))
}

fn header_error<T>(code: &str) -> Result<T, Signal> {
    raise(INVALID_HEADER, &format!("InvalidHeader:{code}"))
}

/// Whether `v` is `type/subtype` with optional `;name=value` parameters.
fn is_media_type(v: &str) -> bool {
    let mut parts = v.split(';');
    let main = parts.next().unwrap_or("").trim();
    let Some((t, st)) = main.split_once('/') else {
        return false;
    };
    let token = |s: &str| !s.is_empty() && s.chars().all(is_token_char);
    token(t)
        && token(st)
        && parts.all(|p| matches!(p.split_once('='), Some((n, v)) if token(n.trim()) && param_value(v.trim())))
}

/// A parameter value: not empty, and a value that opens a quote closes it (network/lt-ay-http/AyHeaderAddResult.al:101 refuses
/// `a/b; q="`).
fn param_value(v: &str) -> bool {
    !v.is_empty() && (!v.starts_with('"') || (v.len() >= 2 && v.ends_with('"')))
}

/// An RFC 1123 date (`Tue, 15 Nov 1994 08:12:31 GMT`).
fn is_http_date(v: &str) -> bool {
    let p: Vec<&str> = v.split_whitespace().collect();
    if p.len() != 6 || !p[0].ends_with(',') || p[5] != "GMT" {
        return false;
    }
    let days = ["Mon,", "Tue,", "Wed,", "Thu,", "Fri,", "Sat,", "Sun,"];
    let months = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let time_ok = p[4].split(':').count() == 3
        && p[4]
            .split(':')
            .all(|x| x.len() == 2 && x.chars().all(|c| c.is_ascii_digit()));
    days.contains(&p[0])
        && p[1].len() <= 2
        && p[1].chars().all(|c| c.is_ascii_digit())
        && months.contains(&p[2])
        && p[3].len() == 4
        && p[3].chars().all(|c| c.is_ascii_digit())
        && time_ok
}

const KNOWN_NAMES: &[&str] = &[
    "Accept",
    "Accept-Charset",
    "Accept-Encoding",
    "Accept-Language",
    "Allow",
    "Authorization",
    "Cache-Control",
    "Connection",
    "Content-Disposition",
    "Content-Encoding",
    "Content-Language",
    "Content-Length",
    "Content-Location",
    "Content-MD5",
    "Content-Range",
    "Content-Type",
    "Date",
    "Expect",
    "Expires",
    "From",
    "Host",
    "If-Match",
    "If-Modified-Since",
    "If-None-Match",
    "If-Range",
    "If-Unmodified-Since",
    "Last-Modified",
    "Max-Forwards",
    "Pragma",
    "Proxy-Authorization",
    "Range",
    "Referer",
    "TE",
    "Trailer",
    "Transfer-Encoding",
    "Upgrade",
    "User-Agent",
    "Via",
    "Warning",
];

/// The name a known header is stored under, whatever case it was added in (lt-http-header-rules/P87Twice.al:NameCaseAndKeys, `accept`
/// reads back as `Accept`); a custom header keeps the case of its first Add.
fn canonical_name(name: &str) -> Option<&'static str> {
    KNOWN_NAMES.iter().copied().find(|k| k.eq_ignore_ascii_case(name))
}

/// The name a header is stored under. A known header written in another case than .NET's canonical form is stored canonically, in
/// all 39 known names and on request, content and response headers (network/lt-http-header-names-cache-control/HeaderNameCaseRequest.al,
/// HeaderNameCaseContent.al, HeaderNameCaseResponse.al: `keys=1:Accept-Charset`); a custom name keeps its first case (CustomNameCase.al).
fn stored_name(items: &[(String, Vec<String>)], name: &str) -> Result<String, Signal> {
    if let Some((n, _)) = find(items, name) {
        return Ok(n.clone());
    }
    Ok(canonical_name(name).map_or_else(|| name.to_string(), str::to_string))
}

/// `a; b=c` with one "; " between the parts (.NET writes a parameter list that way).
fn join_params(v: &str) -> String {
    v.split(';').map(str::trim).collect::<Vec<_>>().join("; ")
}

fn is_token(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_token_char)
}

/// Splits a list header value at the commas outside quotes.
fn split_list(v: &str) -> Vec<String> {
    let (mut out, mut cur, mut quoted) = (Vec::new(), String::new(), false);
    for c in v.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                cur.push(c);
            }
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.into_iter()
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

/// A list item `token` or `token;name=value`, written with "; " between the parts.
fn token_with_params(item: &str) -> Option<String> {
    let head = item.split(';').next().unwrap_or("").trim();
    if !is_token(head) {
        return None;
    }
    let mut parts = item.split(';').skip(1);
    parts
        .all(|p| matches!(p.split_once('='), Some((n, v)) if is_token(n.trim()) && !v.trim().is_empty()))
        .then(|| join_params(item))
}

/// `"tag"`, `W/"tag"` or `*`.
fn is_entity_tag(item: &str) -> bool {
    let t = item.strip_prefix("W/").unwrap_or(item);
    item == "*" || (t.len() >= 2 && t.starts_with('"') && t.ends_with('"') && !t[1..t.len() - 1].contains('"'))
}

/// The tokens of a User-Agent: products `name/version` and comments `(...)`, each a value of its own.
fn user_agent_parts(v: &str) -> Option<Vec<String>> {
    let chars: Vec<char> = v.chars().collect();
    let (mut i, mut out) = (0, Vec::new());
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
        } else if chars[i] == '(' {
            let (mut depth, start) = (0, i);
            loop {
                match chars.get(i)? {
                    '(' => depth += 1,
                    ')' => depth -= 1,
                    _ => {}
                }
                i += 1;
                if depth == 0 {
                    break;
                }
            }
            out.push(chars[start..i].iter().collect());
        } else {
            let start = i;
            while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '(' {
                i += 1;
            }
            let product: String = chars[start..i].iter().collect();
            let ok = match product.split_once('/') {
                Some((n, ver)) => is_token(n) && is_token(ver),
                None => is_token(&product),
            };
            if !ok {
                return None;
            }
            out.push(product);
        }
    }
    (!out.is_empty()).then_some(out)
}

/// The known Cache-Control directives in the order .NET writes them (network/lt-http-header-names-cache-control/CacheControlPairs.al,
/// CacheControlTriples.al:P198_AllDirectivesInListOrderAndReverse); extensions follow, in the order added.
const CACHE_CONTROL_ORDER: &[&str] = &[
    "no-store",
    "no-transform",
    "only-if-cached",
    "public",
    "must-revalidate",
    "proxy-revalidate",
    "no-cache",
    "max-age",
    "s-maxage",
    "max-stale",
    "min-fresh",
    "private",
];

/// One Cache-Control entry: `Ok(Some((name as written, normalized text)))`, `Ok(None)` for an invalid one (InvalidHeader:Format), an
/// `Err` for a form no test pins.
fn cache_control_item(item: &str) -> Result<Option<(String, String)>, Signal> {
    let (name, value) = match item.split_once('=') {
        Some((n, v)) => (n.trim(), Some(v.trim())),
        None => (item.trim(), None),
    };
    if !is_token(name) {
        return Ok(None);
    }
    let lower = name.to_ascii_lowercase();
    let unpinned = |what: &str| {
        unsupported(format!(
            "Cache-Control {item}: {what} is not pinned (network/lt-http-header-names-cache-control)"
        ))
    };
    match lower.as_str() {
        "max-age" | "s-maxage" | "min-fresh" | "max-stale" => match value {
            // `max-age=` with nothing, text, a sign or a number beyond Int32 are invalid; the value is read as seconds.
            Some(v) if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) || v.parse::<i32>().is_err() => Ok(None),
            Some(v) => Ok(Some((
                lower.clone(),
                format!("{lower}={}", v.parse::<i32>().expect("checked")),
            ))),
            None => unpinned("a directive that takes seconds written without them"),
        },
        "no-cache" | "private" => match value {
            None => Ok(Some((lower.clone(), lower))),
            Some(v) if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') => {
                Ok(Some((lower.clone(), format!("{lower}={v}"))))
            }
            Some(_) => unpinned("a field list without quotes"),
        },
        "no-store" | "no-transform" | "only-if-cached" | "public" | "must-revalidate" | "proxy-revalidate" => {
            match value {
                None => Ok(Some((lower.clone(), lower))),
                Some(_) => unpinned("a flag directive with a value"),
            }
        }
        _ => match value {
            None => Ok(Some((name.to_string(), name.to_string()))),
            Some(v) if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') => {
                Ok(Some((name.to_string(), format!("{name}={v}"))))
            }
            Some(v) if is_token(v) => Ok(Some((name.to_string(), format!("{name}={v}")))),
            Some(_) => Ok(None),
        },
    }
}

/// The directives of a Cache-Control value merged and written the way .NET's CacheControlHeaderValue writes them: the known ones
/// in `CACHE_CONTROL_ORDER` once each, then the extensions as added (an extension added twice is written twice). `Ok(None)` is an
/// invalid value; two different values of one known directive are not pinned.
fn cache_control(existing: &[String], added: &str) -> Result<Option<String>, Signal> {
    let mut known: Vec<(String, String)> = Vec::new();
    let mut extensions: Vec<String> = Vec::new();
    for entry in existing.iter().flat_map(|e| split_list(e)).chain(split_list(added)) {
        let Some((name, text)) = cache_control_item(&entry)? else {
            return Ok(None);
        };
        if CACHE_CONTROL_ORDER.contains(&name.as_str()) {
            match known.iter().find(|(n, _)| *n == name) {
                None => known.push((name, text)),
                Some((_, old)) if *old == text => {}
                Some(_) => {
                    return unsupported(format!(
                        "Cache-Control {entry}: a second value of a directive that is there is not pinned (network/lt-http-header-names-cache-control)"
                    ));
                }
            }
        } else {
            extensions.push(text);
        }
    }
    let mut out: Vec<String> = Vec::new();
    for k in CACHE_CONTROL_ORDER {
        if let Some((_, text)) = known.iter().find(|(n, _)| n == k) {
            out.push(text.clone());
        }
    }
    out.extend(extensions);
    Ok(Some(out.join(", ")))
}

/// What `HttpHeaders.Add` stores for a header .NET types, or `None` for one it accepts as written. The values come from the probes of
/// lt-http-header-rules/P87Values.al (KnownHeaderLists_*), P87Twice.al and P87Matrix.al; a refused value is `InvalidHeader:Format`.
/// A header .NET holds once (`single`) refuses a second Add the same way.
struct Typed {
    values: Vec<String>,
    /// The collection holds these values instead of what it held (Cache-Control is one merged value).
    replace: bool,
}

fn typed_add(bag: &HeaderBag, name: &str, value: &str) -> Result<Option<Typed>, Signal> {
    let lower = name.to_ascii_lowercase();
    let existing = find(&bag.items, name);
    let fail = || header_error::<Option<Typed>>("Format");
    let single = |values: Vec<String>| {
        if existing.is_some() {
            fail()
        } else {
            Ok(Some(Typed { values, replace: false }))
        }
    };
    let list = |values: Vec<String>| Ok(Some(Typed { values, replace: false }));
    let v = value.trim();
    match lower.as_str() {
        "accept" => {
            let items = split_list(value);
            if !items.is_empty() && items.iter().all(|m| is_media_type(m) || m.starts_with("*/")) {
                list(items.iter().map(|m| join_params(m)).collect())
            } else {
                fail()
            }
        }
        "accept-encoding" | "connection" | "content-encoding" | "content-language" | "transfer-encoding" => {
            let items = split_list(value);
            let normalized: Option<Vec<String>> = items.iter().map(|i| token_with_params(i)).collect();
            match normalized {
                Some(n) if !n.is_empty() => list(n),
                _ => fail(),
            }
        }
        "if-match" | "if-none-match" => {
            let items = split_list(value);
            if !items.is_empty() && items.iter().all(|i| is_entity_tag(i)) {
                list(items)
            } else {
                fail()
            }
        }
        "cache-control" => {
            let old: Vec<String> = existing.map(|(_, v)| v.clone()).unwrap_or_default();
            match cache_control(&old, value)? {
                Some(merged) if merged.is_empty() => Ok(Some(Typed {
                    values: Vec::new(),
                    replace: true,
                })),
                Some(merged) => Ok(Some(Typed {
                    values: vec![merged],
                    replace: true,
                })),
                None => fail(),
            }
        }
        "user-agent" => match user_agent_parts(value) {
            Some(parts) => list(parts),
            None => fail(),
        },
        "range" => match v.split_once('=') {
            Some((unit, rest)) if is_token(unit) && !rest.trim().is_empty() => single(vec![v.to_string()]),
            _ => fail(),
        },
        "host" => {
            let (host, port) = match v.rsplit_once(':') {
                Some((h, p)) => (h, Some(p)),
                None => (v, None),
            };
            if !host.is_empty()
                && !host.contains(char::is_whitespace)
                && port.is_none_or(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
            {
                single(vec![v.to_string()])
            } else {
                fail()
            }
        }
        "authorization" | "proxy-authorization" => {
            let (scheme, creds) = match v.split_once(char::is_whitespace) {
                Some((s, c)) => (s, Some(c.trim())),
                None => (v, None),
            };
            if is_token(scheme) {
                let shown = match creds {
                    Some(c) if !c.is_empty() => format!("{scheme} {c}"),
                    _ => scheme.to_string(),
                };
                single(vec![shown])
            } else {
                fail()
            }
        }
        "content-disposition" => {
            let head = v.split(';').next().unwrap_or("").trim();
            if is_token(head) {
                single(vec![join_params(v)])
            } else {
                fail()
            }
        }
        "content-length" => {
            if !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()) && v.parse::<u64>().is_ok() {
                single(vec![v.to_string()])
            } else {
                fail()
            }
        }
        "date" | "if-modified-since" | "if-unmodified-since" | "last-modified" | "expires" => {
            if existing.is_some() || !is_http_date(v) {
                fail()
            } else {
                Ok(Some(Typed {
                    values: vec![value.to_string()],
                    replace: false,
                }))
            }
        }
        // A media type is written back with "; " between its parameters (.NET's MediaTypeHeaderValue): 'text/plain;charset=utf-8' reads
        // 'text/plain; charset=utf-8', upper case and quotes stay (lt-http-header-rules/P87ContentType.al:ContentTypeVariantsStayVerbatim).
        "content-type" => {
            if existing.is_some() || !is_media_type(v) {
                fail()
            } else {
                Ok(Some(Typed {
                    values: vec![join_params(v)],
                    replace: false,
                }))
            }
        }
        _ => Ok(None),
    }
}

/// A whole number as .NET writes it back: digits with no leading zero, at most nine of them (so it fits an Int32). A number with a
/// leading zero or more digits is not measured.
fn plain_number(v: &str) -> Result<bool, Signal> {
    if v.is_empty() || !v.chars().all(|c| c.is_ascii_digit()) {
        return Ok(false);
    }
    if (v.len() > 1 && v.starts_with('0')) || v.len() > 9 {
        return unsupported(format!(
            "HttpHeaders.Add of the number '{v}': its stored form is not measured"
        ));
    }
    Ok(true)
}

/// A uri header (Referer, Location, Content-Location): a relative uri is stored with its spaces as `%20`
/// (network/lt-ay-http/AyHeaderAddResult.al:256, :311, :355 `a%20b`); an absolute one as written when it is already in its normal form
/// (:255, :354 `https://a.example/x`). Other characters of a relative uri, and an absolute uri .NET would rewrite, are not measured.
fn uri_header(v: &str) -> Result<String, Signal> {
    let has_scheme = v.split_once(':').is_some_and(|(s, _)| {
        s.starts_with(|c: char| c.is_ascii_alphabetic())
            && s.chars().all(|c| c.is_ascii_alphanumeric() || "+.-".contains(c))
    });
    if has_scheme {
        return match normalize_uri(v) {
            Ok(n) if n == v => Ok(n),
            _ => unsupported(format!(
                "HttpHeaders.Add of the uri '{v}': its stored form is not measured"
            )),
        };
    }
    if v.chars()
        .all(|c| c == ' ' || c.is_ascii_alphanumeric() || "-._~/?#[]@!$&'()*+,;=%".contains(c))
    {
        return Ok(v.replace(' ', "%20"));
    }
    unsupported(format!(
        "HttpHeaders.Add of the relative uri '{v}': its stored form is not measured"
    ))
}

/// `bytes 0-9/10` (AyHeaderAddResult.al:285; lt-http-header-names-cache-control/Hdr198Helper.al `bytes 0-1/2`): a unit, one space, a
/// range `first-last` and `/length`. `abc` is refused (AyHeaderAddResult.al:297). `*` forms are not measured.
fn content_range(v: &str) -> Result<bool, Signal> {
    let Some((unit, rest)) = v.split_once(' ') else {
        return Ok(false);
    };
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    let ok = is_token(unit)
        && rest
            .split_once('/')
            .is_some_and(|(r, len)| digits(len) && r.split_once('-').is_some_and(|(a, b)| digits(a) && digits(b)));
    if !ok && rest.contains('*') {
        return unsupported(format!("HttpHeaders.Add of the Content-Range '{v}': not measured"));
    }
    Ok(ok)
}

/// The typed headers `typed_add` leaves alone, measured by network/lt-ay-http/AyHeaderAddResult.al (codeunit 80507) on the collection
/// it used: the request headers If-Range, Max-Forwards, Referer and Expect; the content headers Content-Range, Content-MD5,
/// Content-Location and Allow; the response headers Age, ETag, Location, Retry-After, Vary, WWW-Authenticate and Accept-Ranges.
/// `None` is a header with no value rule here.
fn typed_add_more(bag: &HeaderBag, name: &str, value: &str) -> Result<Option<Typed>, Signal> {
    let lower = name.to_ascii_lowercase();
    let v = value.trim();
    let one = |s: String| {
        Ok(Some(Typed {
            values: vec![s],
            replace: false,
        }))
    };
    let fail = || header_error::<Option<Typed>>("Format");
    let request = ["if-range", "max-forwards", "referer", "expect"];
    let content = ["content-range", "content-md5", "content-location", "allow"];
    let response = [
        "age",
        "etag",
        "location",
        "retry-after",
        "vary",
        "www-authenticate",
        "accept-ranges",
    ];
    let probed_here = match bag.kind {
        HeaderKind::Request => request.contains(&lower.as_str()),
        HeaderKind::Content => content.contains(&lower.as_str()),
        HeaderKind::Response => response.contains(&lower.as_str()),
        HeaderKind::Default => false,
    };
    // On another collection the header keeps the earlier rule, stored as written (the HeaderNameCase*.al probes add valid values of
    // these names to every collection; assumption http-header-rules).
    if !probed_here {
        return Ok(None);
    }
    let existing = find(&bag.items, name).is_some();
    let single_unmeasured = || unsupported(format!("HttpHeaders.Add of {name} a second time: not measured"));
    let token_list = |v: &str| {
        let items = split_list(v);
        if !items.is_empty() && items.iter().all(|i| is_token(i)) {
            Ok(Some(Typed {
                values: items,
                replace: false,
            }))
        } else {
            fail()
        }
    };
    match lower.as_str() {
        // `abc` refused (:241); `"a"` accepted (Hdr198Helper.al); an HTTP date is not measured here.
        "if-range" => match () {
            _ if existing => single_unmeasured(),
            _ if is_entity_tag(v) && v != "*" => one(v.to_string()),
            _ if is_http_date(v) => unsupported("HttpHeaders.Add of If-Range with a date: not measured"),
            _ => fail(),
        },
        // `5` accepted, `five` refused (:242-243).
        "max-forwards" | "age" => match () {
            _ if existing => single_unmeasured(),
            _ if plain_number(v)? => one(v.to_string()),
            _ => fail(),
        },
        // `120` accepted, `later` refused (:367-368); a date is not measured here.
        "retry-after" => match () {
            _ if existing => single_unmeasured(),
            _ if plain_number(v)? => one(v.to_string()),
            _ if is_http_date(v) => unsupported("HttpHeaders.Add of Retry-After with a date: not measured"),
            _ => fail(),
        },
        "referer" | "location" | "content-location" => match () {
            _ if existing => single_unmeasured(),
            _ => one(uri_header(v)?),
        },
        // `=` refused (:257); `100-continue` accepted (Hdr198Helper.al). A value or parameters are not measured.
        "expect" => {
            let items = split_list(v);
            if items.iter().any(|i| i.contains(['=', ';']))
                && items
                    .iter()
                    .all(|i| is_token(i.split(['=', ';']).next().unwrap_or("").trim()))
            {
                return unsupported("HttpHeaders.Add of Expect with a value or parameters: not measured");
            }
            token_list(v)
        }
        // A base-64 text (:298) read back as written; `not base64!` refused (:299). Base 64 that .NET would write another way is not
        // measured.
        "content-md5" => match crate::platform_native::b64_decode(v) {
            _ if existing => single_unmeasured(),
            Ok(bytes) if crate::platform_native::b64_encode(&bytes, false) == v => one(v.to_string()),
            Ok(_) => unsupported(format!(
                "HttpHeaders.Add of the Content-MD5 '{v}': its stored form is not measured"
            )),
            Err(_) => fail(),
        },
        "content-range" => match () {
            _ if existing => single_unmeasured(),
            _ if content_range(v)? => one(v.to_string()),
            _ => fail(),
        },
        // `"abc"` accepted, `abc` refused (:339-341, :353).
        "etag" => match () {
            _ if existing => single_unmeasured(),
            _ if is_entity_tag(v) && v != "*" => one(v.to_string()),
            _ => fail(),
        },
        // Token lists: `G E T`, `a b` refused (:327, :369, :382); `GET` accepted (Hdr198Helper.al).
        "allow" | "vary" | "accept-ranges" => token_list(v),
        // A lone comma is an empty list: Add answers true and nothing is stored, so GetValues raises (:381, :384). A challenge is
        // not measured.
        "www-authenticate" => {
            if split_list(v).is_empty() && !v.is_empty() {
                Ok(Some(Typed {
                    values: Vec::new(),
                    replace: false,
                }))
            } else {
                unsupported("HttpHeaders.Add of a WWW-Authenticate challenge: not measured")
            }
        }
        _ => Ok(None),
    }
}

fn find<'a>(items: &'a [(String, Vec<String>)], name: &str) -> Option<&'a (String, Vec<String>)> {
    items.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))
}

fn push_value(items: &mut Vec<(String, Vec<String>)>, name: &str, value: String) {
    match items.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
        Some((_, v)) => v.push(value),
        None => items.push((name.to_string(), vec![value])),
    }
}

/// `HttpHeaders.Add` (network/lt-http/HttpHeaders.al, HttpClient.al, HttpContent.al, HttpResponse.al).
fn add_header(bag: &mut HeaderBag, name: &str, value: &str, secret: bool) -> Result<(), Signal> {
    if name.is_empty() {
        return header_error("Argument");
    }
    if name.chars().any(|c| !is_token_char(c)) {
        return header_error("Format");
    }
    if not_here(bag.kind, name) {
        return raise(INVALID_OPERATION, "InvalidOperation");
    }
    if value.contains(['\r', '\n']) {
        return header_error("Format");
    }
    if secret {
        push_value(&mut bag.secret, name, value.to_string());
        return Ok(());
    }
    let stored = stored_name(&bag.items, name)?;
    let typed = match typed_add(bag, name, value)? {
        Some(t) => Some(t),
        None => typed_add_more(bag, name, value)?,
    };
    // The value rules of a typed header other than Cache-Control (pinned by network/lt-http-header-names-cache-control/CacheControl*.al) rest
    // on the probes measured so far (assumption http-header-rules).
    if typed.is_some() && !name.eq_ignore_ascii_case("cache-control") {
        assume(crate::assumptions::Assumption::HttpHeaderRules);
    }
    match typed {
        Some(t) if t.replace => {
            // The value is replaced where the header stands (CacheControlTriples.al:P198_InterleavedAndRemoved `keys`).
            match bag.items.iter().position(|(n, _)| n.eq_ignore_ascii_case(name)) {
                Some(at) if t.values.is_empty() => {
                    bag.items.remove(at);
                }
                Some(at) => bag.items[at].1 = t.values,
                None if t.values.is_empty() => {}
                None => bag.items.push((stored, t.values)),
            }
        }
        Some(t) => {
            for v in t.values {
                push_value(&mut bag.items, &stored, v);
            }
        }
        None => push_value(&mut bag.items, &stored, value.to_string()),
    }
    Ok(())
}

fn with_bag<R>(r: Option<&Value>, f: impl FnOnce(&mut HeaderBag) -> R) -> Result<R, Signal> {
    let Some(Value::Library(_, cell)) = r else {
        return unsupported("HttpHeaders method without a receiver");
    };
    // A variable no GetHeaders or Headers call filled is an empty collection (network/lt-http/HttpRequest.al:
    // FreshRequestContent reads `Keys().Count()` 0 after a GetHeaders that raised).
    if matches!(&*cell.borrow(), LibraryValue::Empty) {
        *cell.borrow_mut() = LibraryValue::HttpHeaders(new_header_list(HeaderKind::Request));
    }
    match &*cell.borrow() {
        LibraryValue::HttpHeaders(h) => Ok(f(&mut h.borrow_mut())),
        other => unsupported(format!("HttpHeaders method on {}", other.describe())),
    }
}

fn headers_add(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    let (value, secret) = text_or_secret(a, 1)?;
    let ret_used = c.ret_used();
    // A name of blanks is a format error from BC 29 and an argument error on BC 28 (network/lt-http/HttpHeaders.al:304-308);
    // BC 27 is not probed.
    if !name.is_empty()
        && name.chars().all(char::is_whitespace)
        && !c.gate(al_version::Gate::HttpBlankHeaderNameIsFormatFrom29)
    {
        if !c.gate(al_version::Gate::HttpHeaderNameRulesProbedFrom28) {
            return unsupported("HttpHeaders.Add with a name of blanks on a major older than the probed ones");
        }
        return header_error("Argument");
    }
    let added = with_bag(r.as_deref(), |b| add_header(b, &name, &value, secret))?;
    match added {
        Ok(()) => Ok(Value::Boolean(true)),
        // A statement raises the invalid header; an Add whose result is used answers false instead, for a refused value
        // (network/lt-ay-http/AyHeaderAddResult.al:88-89, :115-116, :129-130 and the typed headers below them), a name that is no
        // token (:157-158, :171-173, :185), an empty name (:159, InvalidHeader:Argument) and a name the collection refuses (:186-187,
        // :383, InvalidOperation). lt-http-header-rules/P87Twice.al raises it for the statement. What the last error holds after the
        // false is not measured (assumption http-headers-add-used-result).
        Err(Signal::Error(e))
            if ret_used
                && matches!(
                    e.code.as_str(),
                    "InvalidHeader:Format" | "InvalidHeader:Argument" | "InvalidOperation"
                ) =>
        {
            assume(crate::assumptions::Assumption::HttpHeadersAddUsedResult);
            Ok(Value::Boolean(false))
        }
        Err(e) => Err(e),
    }
}

/// `Remove(Name)` drops every value of the name and answers true, whether or not it was there.
fn headers_remove(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    let removed = with_bag(r.as_deref(), |b| {
        // A content header on request, response or client headers is no header there: Remove answers false and raises nothing
        // (lt-http-header-rules/P87Matrix.al `rm=No` for every content header name of those collections).
        if not_here(b.kind, &name) {
            return Ok(false);
        }
        b.items.retain(|(n, _)| !n.eq_ignore_ascii_case(&name));
        b.secret.retain(|(n, _)| !n.eq_ignore_ascii_case(&name));
        Ok(true)
    })??;
    Ok(Value::Boolean(removed))
}

/// Asking a request, response or client header collection for a content header raises
/// `InvalidHeader:InvalidOperation` (network/lt-http/HttpHeaders.al:TryAddWithoutValidationBasics, HttpResponse.al).
fn restricted_lookup(b: &HeaderBag, name: &str) -> Result<(), Signal> {
    if not_here(b.kind, name) {
        return header_error("InvalidOperation");
    }
    if name.chars().any(|c| !is_token_char(c)) {
        return header_error("Format");
    }
    Ok(())
}

fn headers_contains(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    with_bag(r.as_deref(), |b| {
        restricted_lookup(b, &name)?;
        Ok(Value::Boolean(find(&b.items, &name).is_some()))
    })?
}

fn headers_contains_secret(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    with_bag(r.as_deref(), |b| Value::Boolean(find(&b.secret, &name).is_some()))
}

fn headers_clear(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_bag(r.as_deref(), |b| {
        b.items.clear();
        b.secret.clear();
    })?;
    Ok(Value::Null)
}

fn headers_keys(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let keys = with_bag(r.as_deref(), |b| {
        b.items
            .iter()
            .map(|(n, _)| Value::Text(n.clone().into()))
            .collect::<Vec<_>>()
    })?;
    Ok(Value::List(Rc::new(RefCell::new(keys.into()))))
}

/// `GetValues(Name, var Values)` fills an array or a list with the values in order and answers true. For a name that is not
/// there it answers false, leaves the target as it was and puts the error in GetLastErrorText when its result is used, and raises
/// that error as a statement (network/lt-http/HttpContent.al:OldHeadersHandle_AfterWriteFrom, :226 with :281-283 and :270 with
/// :290-291, on content headers; network/lt-http/HttpHeaders.al:GetValuesOnMissingLeavesArray for the result).
fn headers_get_values(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    // A content header name asked of request, response or client headers is no header there: GetValues answers false
    // (lt-http-header-rules/P87Matrix.al `get=No/n=0`), where Contains raises.
    let (values, kind) = with_bag(r.as_deref(), |b| {
        if restricted_lookup(b, &name).is_err() {
            return Ok::<_, Signal>((None, b.kind));
        }
        Ok((find(&b.items, &name).map(|(_, v)| v.clone()), b.kind))
    })??;
    if values.as_ref().is_some_and(|v| v.iter().any(|x| x == UNPINNED_LENGTH)) {
        return unsupported(
            "the Content-Length of an HttpContent that a read measured and a WriteFrom of another length rewrote: what it shows is not measured (docs/probes-needed.md)",
        );
    }
    let Some(values) = values else {
        let error = crate::native::AlError {
            message: GET_VALUES_MISSING.into(),
            code: "InvalidHeader".into(),
            ..Default::default()
        };
        if kind != HeaderKind::Content {
            // On request, response and client headers a statement raises too, for a name that is not there, no token or a content
            // header (network/lt-ay-http/AyHeaderAddResult.al `get=E` after every refused Add, :88 to :383). The text and code are
            // measured on content headers only: the error carries none, and any read of them is refused (probe
            // http-get-values-missing-error in docs/probes-needed.md). With the result used nothing about the last error is
            // measured there; it is left as it was.
            if c.ret_used() {
                return Ok(Value::Boolean(false));
            }
            return Err(Signal::Error(Box::new(crate::native::unpinned_error_for(
                "http-get-values-missing-error",
            ))));
        }
        if c.ret_used() {
            c.set_last_error(error);
            return Ok(Value::Boolean(false));
        }
        return Err(Signal::Error(Box::new(error)));
    };
    match a.get(1) {
        Some(Value::Array(arr)) => {
            let mut arr = arr.borrow_mut();
            if values.len() > arr.len() {
                return unsupported(
                    "HttpHeaders.GetValues into an array smaller than the number of values: not measured",
                );
            }
            for (i, v) in values.into_iter().enumerate() {
                arr[i] = Value::Text(v.into());
            }
        }
        Some(Value::List(list)) => {
            let mut list = list.borrow_mut();
            list.clear();
            list.extend(values.into_iter().map(|v| Value::Text(v.into())));
        }
        other => return unsupported(format!("HttpHeaders.GetValues into {}", other.describe())),
    }
    Ok(Value::Boolean(true))
}

/// `TryAddWithoutValidation` adds without checking the value or the known headers: false for a name that is no
/// token, or for a content header on request, response or client headers; true otherwise
/// (network/lt-http/HttpHeaders.al:TryAddWithoutValidationBasics, TryAddWithoutValidationRestricted).
fn headers_try_add(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    let (value, secret) = text_or_secret(a, 1)?;
    with_bag(r.as_deref(), |b| {
        if name.is_empty() || name.chars().any(|c| !is_token_char(c)) {
            return Ok(Value::Boolean(false));
        }
        if not_here(b.kind, &name) {
            return Ok(Value::Boolean(false));
        }
        if secret {
            push_value(&mut b.secret, &name, value);
            return Ok(Value::Boolean(true));
        }
        // A known header is parsed here too, so its values read back as Add would have stored them; a value that does not parse is kept
        // as written (lt-http-header-rules/P87Values.al:KnownHeaderLists_*, `raw=Yes;get2=...`).
        let stored = match stored_name(&b.items, &name) {
            Ok(n) => n,
            Err(e) => return Err(e),
        };
        match typed_add(b, &name, &value) {
            Ok(Some(t)) if t.replace => {
                b.items.retain(|(n, _)| !n.eq_ignore_ascii_case(&name));
                b.items.push((stored, t.values));
            }
            Ok(Some(t)) => {
                for v in t.values {
                    push_value(&mut b.items, &stored, v);
                }
            }
            Err(e @ Signal::Unsupported(_)) => return Err(e),
            _ => push_value(&mut b.items, &stored, value),
        }
        Ok(Value::Boolean(true))
    })?
}

// ------------------------------------------------------------------------------------------------ content

fn write_from(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let t = text(a, 0)?;
    with_content(r, |s| s.write_body(t.into_bytes())).map(|()| Value::Null)
}

/// `WriteFrom(InStream)` takes the bytes the stream has left. The source then is not at its end
/// (network/lt-db-http-leads/DbHttpContentLeads.al:310); any other use of it is not pinned and refused (`stream::hand_to_content`).
fn write_from_stream(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let bytes = crate::stream::hand_to_content(&mut a[0])?;
    with_content(r, |s| {
        s.write_body(bytes);
        s.from_stream = true;
    })
    .map(|()| Value::Null)
}

/// `ReadAs(var Text)` gives the body back, UTF-8 decoded; a content nobody wrote to reads as an empty text and
/// answers true (misc/W4XmlJsonHttp.al:ReadAs_FreshContent). On the content a request has when none was set, three shapes are
/// pinned:
/// - `Req.Content.ReadAs(Txt)` with its result used answers false and leaves the text (network/lt-http/HttpRequest.al:220,
///   FreshRequestContent);
/// - `Req.Content.ReadAs(Txt)` as a statement raises `NotInitialized` (misc/lt-probes-needed-5/PnE12HttpContent.al:205,
///   RequestWithoutContent, under asserterror);
/// - a copy of it in a variable (`Content := Req.Content`), called as a statement, raises `NotInitialized`
///   (misc/W4XmlJsonHttp.al:342-343, RequestWithoutContent_ReadAs, through a TryFunction).
///
/// A copy with the result used is refused. Probe wanted (docs/probes-needed.md).
fn read_as(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (data, uninit, copied, charset, taken) = with_content(r, |s| {
        let charset = s
            .headers
            .borrow()
            .items
            .iter()
            .find(|(n, _)| n.eq_ignore_ascii_case("Content-Type"))
            .and_then(|(_, v)| v.first().cloned())
            .and_then(|t| {
                t.split(';').skip(1).find_map(|p| {
                    let (k, v) = p.split_once('=')?;
                    k.trim()
                        .eq_ignore_ascii_case("charset")
                        .then(|| v.trim().trim_matches('"').to_ascii_lowercase())
                })
            });
        // Reading the body makes the content buffer it, and its headers then list a Content-Length (misc/lt-probes-needed-5/PnE12HttpContent.al:
        // WriteFromText_ThenReadAndHeaders reads `5` after WriteFrom('hello') and ReadAs; the headers fetched before the read show it too, and
        // none shows it before: lt-http-header-rules/P87Content.al:RewriteKeepsHeader, HttpWriteFromStream.al:P100_*). Assumption
        // http-content-length-after-read.
        if !s.uninitialized {
            assume(crate::assumptions::Assumption::HttpContentLengthAfterRead);
            let len = vec![s.data.len().to_string()];
            let mut bag = s.headers.borrow_mut();
            match bag
                .items
                .iter_mut()
                .find(|(n, _)| n.eq_ignore_ascii_case("Content-Length"))
            {
                Some((_, v)) => *v = len,
                None => bag.items.push(("Content-Length".into(), len)),
            }
        }
        let taken = s.stream_taken && (s.copied || s.from_stream);
        (s.data.clone(), s.uninitialized, s.copied, charset, taken)
    })?;
    if uninit {
        return match (copied, c.ret_used()) {
            (false, true) => Ok(Value::Boolean(false)),
            (_, false) => raise(
                "Microsoft.Dynamics.Nav.Runtime.NavHttpContent variable not initialized.",
                "NotInitialized",
            ),
            (true, true) => unsupported(
                "HttpContent.ReadAs with its result used, on a copy of the content of a request that has none (not pinned)",
            ),
        };
    }
    // Once the body was handed out as a stream, whether that stream was read to its end, in part or not at all: a copy of a
    // content (network/lt-db-http-leads/DbHttpContentLeads.al:146, :430-480 QG_RespContentCopy_*) and a content written from a
    // stream (:336, :552-600 QG_StreamContent_*) read an empty text; the response's own content (:113, :384-428 QG_RespContent_*)
    // and a variable written from a text (xml/lt-xml-attr-doc-http-members/P97ContentRead.al:89, DbHttpContentLeads.al:506-550
    // QG_TextContent_*) still read the body.
    if taken {
        a[0] = Value::Text(String::new().into());
        return Ok(Value::Boolean(true));
    }
    // The body is read with the charset of the Content-Type header (lt-http-header-rules/P87ContentType.al:ReadAsIgnoresCharsetHeader,
    // whatever its name says: iso-8859-1 reads each byte as a character, utf-16 reads little-endian units, a lone last byte is U+FFFD;
    // no charset, or another type, is UTF-8). A charset not measured stays refused.
    let decoded = match charset.as_deref() {
        None | Some("utf-8") => String::from_utf8_lossy(&data).into_owned(),
        Some("iso-8859-1" | "latin1") => data.iter().map(|b| char::from(*b)).collect(),
        Some("utf-16" | "utf-16le") => {
            let mut units: Vec<u16> = data.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            let mut text = String::from_utf16_lossy(&units);
            if data.len() % 2 == 1 {
                text.push('\u{FFFD}');
            }
            units.clear();
            text
        }
        Some(other) => {
            return unsupported(format!(
                "HttpContent.ReadAs(Text) with the charset {other}: not measured"
            ));
        }
    };
    a[0] = Value::Text(decoded.into());
    Ok(Value::Boolean(true))
}

/// `ReadAs(var InStream)` hands the body out once: a second read gets an empty stream
/// (network/lt-http/HttpContent.al:ReadAsInStreamTwice).
fn read_as_stream(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (data, uninit) = with_content(r, |s| {
        let out = if s.stream_taken { Vec::new() } else { s.data.clone() };
        s.stream_taken = true;
        (out, s.uninitialized)
    })?;
    // On the content of a request that has none, only ReadAs(Text) is pinned (see `read_as`). Probe wanted (docs/probes-needed.md).
    if uninit {
        return unsupported("HttpContent.ReadAs(InStream) on the content of a request that has none (not pinned)");
    }
    let stream = crate::stream::InState::snapshot(data, crate::stream::Encoding::Utf8);
    crate::stream::set_stream(&a[0], LibraryValue::InStream(stream))?;
    Ok(Value::Boolean(true))
}

/// `GetHeaders(var Headers)` hands out the content's own header list (network/TestHttpRequestMessage.al:39); the
/// uninitialized content of a request raises `NotInitialized` (network/lt-http/HttpRequest.al:FreshRequestContent).
fn get_headers(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (list, uninit) = with_content(r, |s| (s.headers.clone(), s.uninitialized))?;
    if uninit {
        return raise(
            "Microsoft.Dynamics.Nav.Runtime.NavHttpContent variable not initialized.",
            "NotInitialized",
        );
    }
    hand_out_headers(a.first(), &list)?;
    Ok(Value::Boolean(true))
}

/// `Clear` empties the body and gives the content its own fresh headers, the Content-Type among them
/// (network/lt-http/HttpContent.al:ClearContent).
fn content_clear(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_content(r, |s| s.clear_body()).map(|()| Value::Null)
}

// ------------------------------------------------------------------------------------------------ client

/// `HttpClient.Timeout` is 100 seconds until set; a value over 5 minutes raises
/// `HttpClientTimeoutTooLarge` naming the minutes of the value (network/lt-http/HttpClient.al:TimeoutValues).
fn client_timeout(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let Some(v) = a.first() else {
        return with_client(r, |s| Value::Duration(s.timeout_ms));
    };
    let ms = match v {
        Value::Duration(d) => *d,
        Value::Integer(i) => i64::from(*i),
        Value::BigInteger(i) => *i,
        other => return unsupported(format!("HttpClient.Timeout set from {}", other.describe())),
    };
    if ms > 300_000 {
        return raise(
            format!(
                "The timeout value of '{}' is larger than the maximum supported value of '5' minutes.",
                (ms / 60_000) % 60
            ),
            "HttpClientTimeoutTooLarge",
        );
    }
    with_client(r, |s| s.timeout_ms = ms).map(|()| Value::Null)
}

fn client_get_base(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_client(r, |s| Value::Text(s.base_address.clone().into()))
}

fn client_set_base(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let u = text(a, 0)?;
    match checked_uri(c, &u)? {
        Some(n) => with_client(r, |s| s.base_address = n).map(|()| Value::Boolean(true)),
        None => Ok(Value::Boolean(false)),
    }
}

fn client_default_headers(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_client(r, |s| headers_value(&s.default_headers))
}

/// `Clear` brings the client back to a fresh one: timeout, base address and default headers
/// (network/lt-http/HttpClient.al:ClientClear).
fn client_clear(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_client(r, |s| *s = HttpClientState::default()).map(|()| Value::Null)
}

/// `UseResponseCookies` takes a Boolean and changes nothing a test can see (network/lt-http/HttpClient.al:BooleanSettings).
fn client_use_cookies(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_client(r, |_| ()).map(|()| Value::Null)
}

/// Hands `(method, url)` to the session and writes the response it returns into the `var Response` argument.
fn send(c: &mut dyn Ctx, method: &str, url: (String, bool), response: Option<&Value>) -> Out {
    let state = c.http_send(HttpOutgoing {
        method: method.to_ascii_uppercase(),
        url: url.0,
        has_secret_uri: false,
        unpinned_join: url.1,
    })?;
    let Some(Value::Library(_, cell)) = response else {
        return unsupported("HttpClient call without a response variable");
    };
    // A request that failed before any connection (`failure_without_connection`) leaves a response with status 0 and the
    // reason `NavHttpClient Request Failed`, not successful, not blocked, an empty body and no headers, also over a response
    // a handler filled before (network/lt-http-send-failure/LtDaSendFailure.al:48, :69; al-language-onprem/network/
    // lt-ay-http-handler/AyHttpHandler.al:PY_FailedCall_IntoFilledResponse). With its result used the call answers false and
    // the error is the last error; as a statement it raises the error (LtDaSendFailure.al:81-86, :98-104).
    if state.request_failed {
        let mut failed = fresh_response();
        failed.status = 0;
        failed.reason = "NavHttpClient Request Failed".into();
        *cell.borrow_mut() = LibraryValue::HttpResponse(failed);
        let Some((message, code)) = state.failure else {
            return unsupported("a failed HttpClient call without an error");
        };
        if !c.ret_used() {
            return raise(message, &code);
        }
        c.set_last_error(crate::native::AlError {
            message,
            code,
            ..Default::default()
        });
        return Ok(Value::Boolean(false));
    }
    // A handler that set IsSuccessfulRequest to false makes the call answer false with the response it filled; as a statement
    // the call raises nothing (al-language-onprem/network/LtDaHttpHandlerOdd.al:119-120, :145-146).
    let success = state.success;
    *cell.borrow_mut() = LibraryValue::HttpResponse(HttpResponseState {
        status: state.status,
        reason: state.reason,
        reason_unpinned: state.reason_unpinned,
        content: state.content,
        headers: state.headers,
        blocked: state.blocked,
    });
    Ok(Value::Boolean(success))
}

macro_rules! verb {
    ($name:ident, $method:expr, $with_content:expr) => {
        fn $name(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
            let base = with_client(r, |s| s.base_address.clone())?;
            let path = text(a, 0)?;
            let url = resolve_request_path(&base, &path)?;
            let response = if $with_content { a.get(2) } else { a.get(1) };
            send(c, $method, url, response)
        }
    };
}
verb!(client_get, "GET", false);
verb!(client_delete, "DELETE", false);
verb!(client_post, "POST", true);
verb!(client_put, "PUT", true);
verb!(client_patch, "PATCH", true);

fn client_send(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let base = with_client(r, |s| s.base_address.clone())?;
    let Some(Value::Library(_, req)) = a.first() else {
        return unsupported("HttpClient.Send without a request");
    };
    let (method, uri) = match &*req.borrow() {
        LibraryValue::HttpRequest(s) => (s.method.clone().unwrap_or_else(|| "GET".into()), s.uri.clone()),
        LibraryValue::Empty => ("GET".into(), None),
        other => return unsupported(format!("HttpClient.Send of {}", other.describe())),
    };
    // A request whose SetRequestUri failed has no uri; Send still reaches the policy check
    // (HttpRelativeRequestPath.al:P168_Send_RelativeRequestUri `send-relative`), so it carries an empty url.
    let url = match uri {
        Some(uri) => resolve_request_path(&base, &uri)?,
        None => (String::new(), false),
    };
    send(c, &method, url, a.get(1))
}

// ------------------------------------------------------------------------------------------------ response

fn response_status(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_response(r, |s| Value::Integer(s.status))
}

fn response_reason(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    match with_response(r, |s| {
        (!s.reason_unpinned).then(|| Value::Text(s.reason.clone().into()))
    })? {
        Some(v) => Ok(v),
        None => unsupported(
            "ReasonPhrase of a response whose [HttpClientHandler] set a status and no ReasonPhrase: the phrase BC gives is pinned for 200 and 404 only (docs/probes-needed.md)",
        ),
    }
}

fn response_blocked(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_response(r, |s| Value::Boolean(s.blocked))
}

fn response_success(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_response(r, |s| Value::Boolean((200..300).contains(&s.status)))
}

fn response_content(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_response(r, |s| s.content.clone())
}

fn response_headers(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_response(r, |s| headers_value(&s.headers))
}

// ------------------------------------------------------------------------------------------------ test messages

fn test_request_type(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    match with_test_request(r, |s| s.request_type.clone())? {
        Value::Null => unsupported("TestHttpRequestMessage.RequestType of a request no client call made: not measured"),
        v => Ok(v),
    }
}

fn test_request_path(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_test_request(r, |s| Value::Text(s.path.clone().into()))
}

fn test_request_query(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_test_request(r, |s| s.query.clone())
}

fn test_request_secret(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_test_request(r, |s| Value::Boolean(s.has_secret_uri))
}

fn test_response_status(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    match a.first() {
        None => with_test_response(r, |s| Value::Integer(s.status)),
        Some(Value::Integer(n)) => {
            let n = *n;
            with_test_response(r, |s| s.status = n).map(|()| Value::Null)
        }
        Some(other) => unsupported(format!("HttpStatusCode set from {}", other.describe())),
    }
}

fn test_response_reason(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if a.is_empty() {
        return with_test_response(r, |s| Value::Text(s.reason.clone().into()));
    }
    let t = text(a, 0)?;
    with_test_response(r, |s| {
        s.reason = t;
        s.reason_set = true;
    })
    .map(|()| Value::Null)
}

fn test_response_blocked(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    match a.first() {
        None => with_test_response(r, |s| Value::Boolean(s.blocked)),
        Some(Value::Boolean(b)) => {
            let b = *b;
            with_test_response(r, |s| s.blocked = b).map(|()| Value::Null)
        }
        Some(other) => unsupported(format!("IsBlockedByEnvironment set from {}", other.describe())),
    }
}

fn test_response_success(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    match a.first() {
        None => with_test_response(r, |s| Value::Boolean(s.success)),
        Some(Value::Boolean(b)) => {
            let b = *b;
            with_test_response(r, |s| s.success = b).map(|()| Value::Null)
        }
        Some(other) => unsupported(format!("IsSuccessfulRequest set from {}", other.describe())),
    }
}

fn test_response_content(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_test_response(r, |s| s.content.clone())
}

fn test_response_headers(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_test_response(r, |s| headers_value(&s.headers))
}

// ------------------------------------------------------------------------------------------------ misc

/// A `FileUpload` no client filled: `FileName()` is a null string (assigned to a Text it is empty) and
/// `CreateInStream` opens an empty stream without an error
/// (types/TestHttpRequestFileUploadFieldErrorContracts.al:FileUpload_NoClientFilled_OpensAnEmptyStream).
/// A filled FileUpload cannot exist here: there is no client.
fn file_upload_name(_: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Out {
    Ok(Value::Null)
}

fn file_upload_create_in(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    crate::stream::create_empty_in_stream(a)?;
    Ok(Value::Null)
}

// ------------------------------------------------------------------------------------------------ cookies

/// The error of a Set-Cookie header the container refuses (P94Cookie.al:Response_AllAttributes).
fn cookie_parse_error() -> Signal {
    match raise::<()>(
        "An error occurred when parsing the Cookie header for Uri 'https://bccookie.microsoft.com/'.",
        "AL:Cookie",
    ) {
        Err(e) => e,
        Ok(()) => unreachable!("raise fails"),
    }
}

/// A `DateTime` value of an RFC 1123 date (`Wed, 21 Oct 2037 07:28:00 GMT`).
fn parse_cookie_date(text: &str) -> Option<i64> {
    let rest = text.split_once(',').map_or(text, |(_, r)| r).trim();
    let mut it = rest.split_whitespace();
    let day: u32 = it.next()?.parse().ok()?;
    let month = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ]
    .iter()
    .position(|m| it.clone().next().is_some_and(|x| x.eq_ignore_ascii_case(m)))?
        + 1;
    it.next();
    let year: i32 = it.next()?.parse().ok()?;
    let clock = it.next()?;
    let zone = it.next()?;
    if !zone.eq_ignore_ascii_case("GMT") {
        return None;
    }
    let mut c = clock.split(':');
    let (h, m, s): (i64, i64, i64) = (
        c.next()?.parse().ok()?,
        c.next()?.parse().ok()?,
        c.next()?.parse().ok()?,
    );
    let d = i64::from(crate::format::day_from_civil(year, month as u32, day));
    Some((d - 1) * 86_400_000 + ((h * 60 + m) * 60 + s) * 1000 + 1)
}

/// The cookies a response's Set-Cookie headers leave in a cookie container asked for `https://bccookie.microsoft.com/`, in the
/// order they were set. A Domain attribute is refused, a cookie whose Path is not `/` is not sent to that address and an expired
/// one is gone (platform/lt-xmlport-session-navapp-cookie/P94Cookie.al). SameSite and unknown attributes are ignored.
fn response_cookies(ctx: &dyn Ctx, headers: &HttpHeaderList) -> Result<Vec<CookieState>, Signal> {
    let values: Vec<String> = headers
        .borrow()
        .items
        .iter()
        .filter(|(k, _)| k.eq_ignore_ascii_case("Set-Cookie"))
        .flat_map(|(_, v)| v.clone())
        .collect();
    let mut out: Vec<CookieState> = Vec::new();
    for header in values {
        let mut parts = header.split(';');
        let first = parts.next().unwrap_or_default();
        let (name, value) = match first.split_once('=') {
            Some((n, v)) => (n.trim().to_string(), v.trim().to_string()),
            None => (first.trim().to_string(), String::new()),
        };
        let mut c = CookieState {
            name,
            value,
            path: "/".into(),
            ..CookieState::default()
        };
        let mut path_ok = true;
        let mut expired = false;
        for attr in parts {
            let (k, v) = attr
                .split_once('=')
                .map_or((attr.trim(), ""), |(k, v)| (k.trim(), v.trim()));
            match k.to_ascii_lowercase().as_str() {
                "domain" => return Err(cookie_parse_error()),
                "path" => path_ok = v == "/",
                "secure" => c.secure = true,
                "httponly" => c.http_only = true,
                "expires" => {
                    let Some(at) = parse_cookie_date(v) else {
                        return unsupported(format!("a cookie Expires of the form `{v}` (not pinned)"));
                    };
                    c.expires = at;
                }
                "max-age" => {
                    let Ok(secs) = v.parse::<i64>() else {
                        return unsupported(format!("a cookie Max-Age of `{v}` (not pinned)"));
                    };
                    let Some(now) = ctx.now_unix_millis() else {
                        return unsupported("Max-Age of a cookie without a clock");
                    };
                    c.expires = unix_to_datetime(now + secs * 1000);
                }
                _ => {}
            }
        }
        if let Some(now) = ctx.now_unix_millis() {
            if c.expires != 0 && c.expires <= unix_to_datetime(now) {
                expired = true;
            }
        }
        out.retain(|o| !(o.name == c.name && path_ok));
        if path_ok && !expired {
            out.push(c);
        }
    }
    Ok(out)
}

fn unix_to_datetime(ms: i64) -> i64 {
    (i64::from(crate::format::day_from_civil(1970, 1, 1)) - 1) * 86_400_000 + ms + 1
}

fn response_cookie_names(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let headers = with_response(r, |s| s.headers.clone())?;
    let cookies = response_cookies(c, &headers)?;
    let names: Vec<Value> = cookies.into_iter().map(|k| Value::Text(k.name.into())).collect();
    Ok(Value::List(Rc::new(RefCell::new(names.into()))))
}

fn response_get_cookie(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    let headers = with_response(r, |s| s.headers.clone())?;
    let cookies = response_cookies(c, &headers)?;
    match cookies.into_iter().find(|k| k.name == name) {
        Some(k) if !name.is_empty() => {
            a[1] = lib("Cookie", LibraryValue::Cookie(k));
            Ok(Value::Boolean(true))
        }
        _ => Ok(Value::Boolean(false)),
    }
}

fn with_cookie<R>(r: Option<&mut Value>, f: impl FnOnce(&mut CookieState) -> R) -> Result<R, Signal> {
    let cell = receiver_cell(r, "Cookie")?;
    let mut lib = cell.borrow_mut();
    if matches!(&*lib, LibraryValue::Empty) {
        *lib = LibraryValue::Cookie(CookieState::default());
    }
    match &mut *lib {
        LibraryValue::Cookie(s) => Ok(f(s)),
        other => unsupported(format!("Cookie method on {}", other.describe())),
    }
}

fn cookie_name(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let new = if a.is_empty() { None } else { Some(text(a, 0)?) };
    // What a setter answers is not pinned (assumption `cookie-setter-result`).
    if new.is_some() && c.ret_used() {
        crate::assumptions::assume(crate::assumptions::Assumption::CookieSetterResult);
    }
    // An empty name raises and keeps the old one (network/lt-ay-http/AyCookieSetterExpression.al:CookieSetter_EmptyName, BC 27-29).
    if new.as_deref() == Some("") {
        return raise("The 'Name'='' part of the cookie is invalid.", "AL:Cookie");
    }
    with_cookie(r, |s| {
        if let Some(n) = new {
            s.name = n;
        }
        Value::Text(s.name.clone().into())
    })
}

fn cookie_value(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let new = if a.is_empty() { None } else { Some(text(a, 0)?) };
    // What a setter answers is not pinned (assumption `cookie-setter-result`).
    if new.is_some() && c.ret_used() {
        crate::assumptions::assume(crate::assumptions::Assumption::CookieSetterResult);
    }
    with_cookie(r, |s| {
        if let Some(n) = new {
            s.value = n;
        }
        Value::Text(s.value.clone().into())
    })
}

fn cookie_domain(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_cookie(r, |s| Value::Text(s.domain.clone().into()))
}

fn cookie_path(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_cookie(r, |s| Value::Text(s.path.clone().into()))
}

fn cookie_secure(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_cookie(r, |s| Value::Boolean(s.secure))
}

fn cookie_http_only(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_cookie(r, |s| Value::Boolean(s.http_only))
}

fn cookie_expires(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_cookie(r, |s| Value::DateTime(s.expires))
}

pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    let ty = |i: usize| sig.params.get(i).map(|p| p.ty.as_str()).unwrap_or("");
    let n = sig.params.len();
    match (sig.receiver.as_str(), sig.name.as_str()) {
        ("HttpContent", "WriteFrom") if n == 1 && ty(0) == "Text" => Some(write_from),
        ("HttpContent", "WriteFrom") if n == 1 && ty(0) == "InStream" => Some(write_from_stream),
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
        ("HttpHeaders", "TryAddWithoutValidation") => Some(headers_try_add),
        ("HttpRequestMessage", "Content") => Some(request_content),
        ("HttpRequestMessage", "GetHeaders") => Some(request_get_headers),
        ("HttpRequestMessage", "Method") => Some(method),
        ("HttpRequestMessage", "SetRequestUri") if n == 1 && ty(0) == "Text" => Some(set_request_uri),
        ("HttpRequestMessage", "GetRequestUri") => Some(get_request_uri),
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

impl HttpContentState {
    /// A content with its own data and its own headers.
    pub fn deep_copy(&self) -> HttpContentState {
        let bag = self.headers.borrow();
        let copy = new_header_list(bag.kind);
        {
            let mut c = copy.borrow_mut();
            c.items = bag.items.clone();
            c.secret = bag.secret.clone();
        }
        HttpContentState {
            data: self.data.clone(),
            headers: copy,
            uninitialized: self.uninitialized,
            stream_taken: self.stream_taken,
            written: self.written,
            copied: true,
            from_stream: self.from_stream,
        }
    }

    /// `Clear`: the content gets no body and a header collection of its own that holds the Content-Type; a header
    /// collection handed out before keeps what it held (network/lt-http/HttpContent.al:ClearContent,
    /// lt-http-header-rules/P87Content.al:ClearAfterAdd, ClearBeforeWrite, ClearFresh).
    fn clear_body(&mut self) {
        let headers = new_header_list(HeaderKind::Content);
        headers
            .borrow_mut()
            .items
            .push(("Content-Type".into(), vec!["text/plain; charset=utf-8".into()]));
        self.data = Vec::new();
        self.headers = headers;
        self.uninitialized = false;
        self.stream_taken = false;
        self.written = false;
        self.from_stream = false;
    }

    /// `WriteFrom` of text or a stream: the body is replaced, the header collection stays the one every `HttpHeaders` handed out
    /// shows, with its Content-Type set to `text/plain; charset=utf-8` (put first when it was removed) and the other headers kept
    /// (lt-http-header-rules/P87Content.al:AddThenWrite_Text, WriteThenAdd_Text, RewriteKeepsHeader, P87ContentType.al:HeaderBeforeWrite,
    /// P87ContentStream.al, HttpWriteFromStream.al:P100_*). A Content-Length the headers already hold (see `read_as`) is not
    /// rewritten: it stays for an empty body (network/lt-ay-http/AyContentLength.al:ContentLength_SecondWriteFrom `empty`
    /// `Yes[13]`) and for a body of the same length (P100_SecondWrite_ReplacesBody, 5 bytes twice); after a body of another
    /// length the value is not measured: reading it is refused (`UNPINNED_LENGTH`).
    fn write_body(&mut self, data: Vec<u8>) {
        {
            let mut bag = self.headers.borrow_mut();
            match bag
                .items
                .iter_mut()
                .find(|(n, _)| n.eq_ignore_ascii_case("Content-Type"))
            {
                Some((_, v)) => *v = vec!["text/plain; charset=utf-8".into()],
                None => bag
                    .items
                    .insert(0, ("Content-Type".into(), vec!["text/plain; charset=utf-8".into()])),
            }
            if let Some((_, v)) = bag
                .items
                .iter_mut()
                .find(|(n, _)| n.eq_ignore_ascii_case("Content-Length"))
            {
                assume(crate::assumptions::Assumption::HttpContentLengthAfterRead);
                if !data.is_empty() && v.first() != Some(&data.len().to_string()) {
                    *v = vec![UNPINNED_LENGTH.into()];
                }
            }
        }
        self.data = data;
        self.uninitialized = false;
        self.stream_taken = false;
        self.written = true;
        self.from_stream = false;
    }
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
