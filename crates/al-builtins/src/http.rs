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
        Some(n) => with_request(r, |s| {
            s.uri = Some(n);
            s.uri_secret = false;
        })
        .map(|()| Value::Boolean(true)),
        None => Ok(Value::Boolean(false)),
    }
}

/// `GetRequestUri` of a request whose uri is secret reads empty; a plain uri set over a secret one reads back
/// (xml/lt-xml-attr-doc-http-members/P97ReqSecretUri.al:79, :110, :113).
fn get_request_uri(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let uri = with_request(r, |s| if s.uri_secret { None } else { s.uri.clone() })?;
    Ok(Value::Text(uri.unwrap_or_default().into()))
}

/// `SetSecretRequestUri(SecretText)` answers whether the uri is valid and keeps a valid one as the request's secret uri. The forms
/// the corpus pins agree with `SetRequestUri`: empty, `not a uri` and a rooted path answer false; `ftp://`, `https://` with a
/// port, user info, a fragment or no path answer true; the method stays (xml/lt-xml-attr-doc-http-members/P97ReqSecretUri.al:
/// 118-158 SecretUri_InvalidValues, :160-192 SecretUri_Variants, :194-207 SecretUri_Method_Unchanged). A statement with an
/// invalid uri is not pinned (refused), nor is a form `SetRequestUri` itself has not measured.
fn set_secret_request_uri(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (u, _) = text_or_secret(a, 0)?;
    match normalize_uri(&u) {
        Ok(n) => with_request(r, |s| {
            s.uri = Some(n);
            s.uri_secret = true;
        })
        .map(|()| Value::Boolean(true)),
        Err(false) if c.ret_used() => Ok(Value::Boolean(false)),
        Err(false) => unsupported("SetSecretRequestUri with an invalid uri as a statement (its error is not pinned)"),
        Err(true) => unsupported("a secret request uri of a form SetRequestUri has not measured (not pinned)"),
    }
}

/// `GetSecretRequestUri` gives the uri as a SecretText, plain or secret; empty on a request with none
/// (xml/lt-xml-attr-doc-http-members/P97ReqSecretUri.al:75 `new_secret` Yes, :106 `after_plain_secret` No).
fn get_secret_request_uri(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let uri = with_request(r, |s| s.uri.clone())?.unwrap_or_default();
    Ok(lib("SecretText", LibraryValue::Text(uri)))
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

const WEEKDAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const WEEKDAYS_LONG: [&str; 7] = [
    "Sunday",
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Days since 1970-01-01 of a proleptic Gregorian date.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { yoe + era * 400 + 1 } else { yoe + era * 400 }, m, d)
}

fn digits_of(s: &str, len: std::ops::RangeInclusive<usize>) -> Option<i64> {
    (len.contains(&s.len()) && s.chars().all(|c| c.is_ascii_digit())).then(|| s.parse().ok())?
}

/// `HH:MM:SS` as seconds of the day.
fn time_of_day(s: &str) -> Option<i64> {
    let mut p = s.split(':');
    let (h, m, sec) = (p.next()?, p.next()?, p.next()?);
    if p.next().is_some() {
        return None;
    }
    let (h, m, sec) = (digits_of(h, 2..=2)?, digits_of(m, 2..=2)?, digits_of(sec, 2..=2)?);
    (h < 24 && m < 60 && sec < 60).then_some(h * 3600 + m * 60 + sec)
}

/// Seconds since 1970 of a valid date and time; the weekday given, when there is one, must be the date's.
fn instant(y: i64, mon: &str, d: i64, secs: i64, weekday: Option<usize>) -> Option<i64> {
    let m = MONTHS.iter().position(|x| *x == mon)? as i64 + 1;
    let days = days_from_civil(y, m, d);
    if civil_from_days(days) != (y, m, d) {
        return None;
    }
    if weekday.is_some_and(|w| w as i64 != (days + 4).rem_euclid(7)) {
        return None;
    }
    Some(days * 86_400 + secs)
}

/// An instant written the way .NET writes a date header: `Tue, 15 Nov 1994 08:12:31 GMT`.
fn rfc1123(t: i64) -> String {
    let days = t.div_euclid(86_400);
    let secs = t.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{}, {d:02} {} {y:04} {:02}:{:02}:{:02} GMT",
        WEEKDAYS[(days + 4).rem_euclid(7) as usize],
        MONTHS[(m - 1) as usize],
        secs / 3600,
        secs / 60 % 60,
        secs % 60
    )
}

/// The date forms network/lt-se-header-values/SeHeaderValues.al:SE_Shapes_Dates measured (corpus bump PR 737), all stored as
/// RFC 1123 GMT: `Tue, 15 Nov 1994 08:12:31 GMT` (also lt-http-header-rules/P87Values.al, AyHeaderAddResult.al:119), the same with
/// `UTC`, RFC 850 `Tuesday, 15-Nov-94 08:12:31 GMT`, asctime `Tue Nov 15 08:12:31 1994` and `15 Nov 1994 08:12:31 +0100` (the offset
/// is taken off). `None` for another text.
fn parse_date_forms(v: &str) -> Option<i64> {
    let p: Vec<&str> = v.split_whitespace().collect();
    let weekday = |w: &str, names: &[&str; 7]| names.iter().position(|x| *x == w);
    match p.as_slice() {
        [wd, d, mon, y, time, zone] if wd.ends_with(',') && matches!(*zone, "GMT" | "UTC") => {
            let w = weekday(&wd[..wd.len() - 1], &WEEKDAYS)?;
            instant(
                digits_of(y, 4..=4)?,
                mon,
                digits_of(d, 1..=2)?,
                time_of_day(time)?,
                Some(w),
            )
        }
        [wd, dmy, time, "GMT"] if wd.ends_with(',') => {
            let w = weekday(&wd[..wd.len() - 1], &WEEKDAYS_LONG)?;
            let mut parts = dmy.split('-');
            let (d, mon, yy) = (parts.next()?, parts.next()?, parts.next()?);
            if parts.next().is_some() {
                return None;
            }
            // A two-digit year of 50 or more is in the 1900s (94 is the measured one); a smaller one is not measured.
            let yy = digits_of(yy, 2..=2)?;
            if yy < 50 {
                return None;
            }
            instant(1900 + yy, mon, digits_of(d, 1..=2)?, time_of_day(time)?, Some(w))
        }
        [wd, mon, d, time, y] if WEEKDAYS.contains(wd) => {
            let w = weekday(wd, &WEEKDAYS)?;
            instant(
                digits_of(y, 4..=4)?,
                mon,
                digits_of(d, 1..=2)?,
                time_of_day(time)?,
                Some(w),
            )
        }
        [d, mon, y, time, zone] if zone.len() == 5 && zone.starts_with(['+', '-']) => {
            let hh = digits_of(&zone[1..3], 2..=2)?;
            let mm = digits_of(&zone[3..5], 2..=2)?;
            let offset = (hh * 3600 + mm * 60) * if zone.starts_with('-') { -1 } else { 1 };
            Some(
                instant(
                    digits_of(y, 4..=4)?,
                    mon,
                    digits_of(d, 1..=2)?,
                    time_of_day(time)?,
                    None,
                )? - offset,
            )
        }
        _ => None,
    }
}

/// A date header value: `Ok(Some(stored))`, `Ok(None)` for a refused one (InvalidHeader:Format), or a refusal of a text no probe
/// measured. Refused: a text without a digit or without a space (`not a date`, `yesterday`, `soon`, `0`, `2020-01-01`,
/// `1994-11-15T08:12:31Z`: lt-http/HttpHeaders.al:351, AyHeaderAddResult.al:131-132, :231, :315-328, P87Values.al:413-454) and a
/// date in lower case (SeHeaderValues.al:SE_Shapes_Dates `ifunmodifiedsince_4`).
fn http_date(v: &str) -> Result<Option<String>, Signal> {
    if let Some(t) = parse_date_forms(v) {
        let stored = rfc1123(t);
        return Ok(Some(stored));
    }
    if !v.contains(|c: char| c.is_ascii_digit()) || !v.contains(' ') {
        return Ok(None);
    }
    if !v.contains(|c: char| c.is_ascii_uppercase()) {
        let capitalized: String = v
            .split(' ')
            .map(|w| {
                let mut c = w.chars();
                c.next()
                    .map_or_else(String::new, |f| f.to_ascii_uppercase().to_string() + c.as_str())
            })
            .collect::<Vec<_>>()
            .join(" ")
            .replace("Gmt", "GMT");
        if parse_date_forms(&capitalized).is_some() {
            return Ok(None);
        }
    }
    unsupported(format!("HttpHeaders.Add of the date '{v}': its form is not measured"))
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

/// The tokens of a User-Agent: products `name/version` and comments `(...)`, each a value of its own. `Ok(None)` for an
/// unbalanced comment (`((((`: P87Values.al:330-336). A product that is neither `token` nor `token/token` (a lone `/`, spaces
/// around `/`, a second `/`) is not measured (only valid products are: SeHeaderValues.al:302-311), so it is refused.
fn user_agent_parts(v: &str) -> Result<Option<Vec<String>>, Signal> {
    let chars: Vec<char> = v.chars().collect();
    let (mut i, mut out) = (0, Vec::new());
    while i < chars.len() {
        if chars[i].is_whitespace() {
            i += 1;
        } else if chars[i] == '(' {
            let (mut depth, start) = (0, i);
            loop {
                let Some(c) = chars.get(i) else { return Ok(None) };
                match c {
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
                return unsupported(format!(
                    "User-Agent or Server value `{v}`: a product that is not `name` or `name/version` (not pinned)"
                ));
            }
            out.push(product);
        }
    }
    Ok((!out.is_empty()).then_some(out))
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
        "user-agent" => match user_agent_parts(value)? {
            Some(parts) => list(parts),
            None => fail(),
        },
        // `unit=first-last, first-last` and `unit=-suffix`, stored as written; a range without a digit is refused
        // (SeHeaderValues.al:SE_Shapes_Range_Host `bytes=0-9, 20-29`, `bytes=-5`, `items=1-2`, `bytes=x`; P87Values.al:349-350,
        // AyHeaderAddResult.al:201-203). Other range forms (`5-`, letters around digits) are not measured.
        "range" => match v.split_once('=') {
            Some((unit, rest)) if is_token(unit) && !rest.trim().is_empty() => {
                let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
                let items: Vec<&str> = rest.split(',').map(str::trim).collect();
                if items.iter().all(|i| match i.split_once('-') {
                    Some((a, b)) => (a.is_empty() || digits(a)) && digits(b),
                    None => false,
                }) {
                    single(vec![v.to_string()])
                } else if items.iter().any(|i| !i.contains(|c: char| c.is_ascii_digit())) {
                    fail()
                } else {
                    unsupported(format!("HttpHeaders.Add of the Range '{v}': not measured"))
                }
            }
            _ => fail(),
        },
        "host" => {
            let (host, port) = match v.rsplit_once(':') {
                Some((h, p)) => (h, Some(p)),
                None => (v, None),
            };
            if !host.is_empty()
                && !host.contains(char::is_whitespace)
                // An empty port is accepted as written (SeHeaderValues.al:SE_Shapes_Range_Host `a.example:`).
                && port.is_none_or(|p| p.chars().all(|c| c.is_ascii_digit()))
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
        // Stored as a number: ` 5` and `05` read `5` (SeHeaderValues.al:SE_Shapes_Content); `-1` is refused.
        "content-length" => match v.parse::<u64>() {
            Ok(n) if !v.is_empty() && v.chars().all(|c| c.is_ascii_digit()) => single(vec![n.to_string()]),
            _ => fail(),
        },
        "date" | "if-modified-since" | "if-unmodified-since" | "last-modified" | "expires" => match http_date(v)? {
            Some(stored) if existing.is_none() => Ok(Some(Typed {
                values: vec![stored],
                replace: false,
            })),
            _ => fail(),
        },
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
    // .NET stores the uri's normal form. Measured: one already in it, a scheme and host in upper case (`HTTPS://A.example/x` reads
    // `https://a.example/x`) and a host with no path (`https://a.example` reads `https://a.example/`): SeHeaderValues.al:
    // SE_Shapes_Numbers_Uris `location_4`, `referer_5`. Any other rewrite is not measured.
    if has_scheme {
        let simple = v.split_once("://").map(|(scheme, rest)| {
            let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
            let path = if end == rest.len() { "/" } else { &rest[end..] };
            format!(
                "{}://{}{path}",
                scheme.to_ascii_lowercase(),
                rest[..end].to_ascii_lowercase()
            )
        });
        return match normalize_uri(v) {
            Ok(n) if n == v => Ok(n),
            Ok(n) if simple.as_deref() == Some(n.as_str()) => Ok(n),
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
/// range `first-last` and `/length`. `abc` is refused (AyHeaderAddResult.al:297). `bytes */10` (no range, a length) is accepted
/// (SeHeaderValues.al:SE_Shapes_Numbers_Uris `contentrange_6`); other `*` forms are not measured.
fn content_range(v: &str) -> Result<bool, Signal> {
    let Some((unit, rest)) = v.split_once(' ') else {
        return Ok(false);
    };
    let digits = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_digit());
    if is_token(unit) && rest.strip_prefix("*/").is_some_and(digits) {
        return Ok(true);
    }
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
        // The client's default headers check like request headers (SeHeaderValues.al:SE_ClientHeaders, corpus bump PR 737).
        HeaderKind::Request | HeaderKind::Default => request.contains(&lower.as_str()),
        HeaderKind::Content => content.contains(&lower.as_str()),
        HeaderKind::Response => response.contains(&lower.as_str()),
    };
    // On another collection the header keeps the earlier rule, stored as written (the HeaderNameCase*.al probes add valid values of
    // these names to every collection).
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
        // `abc` refused (:241); `"a"` accepted (Hdr198Helper.al); a date is stored as a date header is (SeHeaderValues.al:
        // SE_Shapes_EntityTags `ifrange_5`).
        "if-range" => match () {
            _ if existing => single_unmeasured(),
            _ if is_entity_tag(v) && v != "*" => one(v.to_string()),
            _ => match http_date(v)? {
                Some(d) => one(d),
                None => fail(),
            },
        },
        // `5` accepted, `five` refused (:242-243); `007` is stored as `7` (SeHeaderValues.al:SE_Shapes_Numbers_Uris `maxforwards_0`).
        "max-forwards"
            if !existing
                && v.len() > 1
                && v.starts_with('0')
                && v.len() <= 9
                && v.chars().all(|c| c.is_ascii_digit()) =>
        {
            one(v.parse::<u32>().expect("checked").to_string())
        }
        "max-forwards" | "age" => match () {
            _ if existing => single_unmeasured(),
            _ if plain_number(v)? => one(v.to_string()),
            _ => fail(),
        },
        // `120` accepted, `later` refused (:367-368); a date is stored as a date header is (SeHeaderValues.al:SE_Shapes_Numbers_Uris
        // `retryafter_2`).
        "retry-after" => match () {
            _ if existing => single_unmeasured(),
            _ if plain_number(v)? => one(v.to_string()),
            _ => match http_date(v)? {
                Some(d) => one(d),
                None => fail(),
            },
        },
        "referer" | "location" | "content-location" => match () {
            _ if existing => single_unmeasured(),
            _ => one(uri_header(v)?),
        },
        // `=` refused (:257); `100-continue` accepted (Hdr198Helper.al); `100-continue; a=b` stored as written (SeHeaderValues.al:
        // SE_Shapes_Numbers_Uris `expect_7`). Other values or parameters are not measured.
        "expect" => {
            if let Some((head, param)) = v.split_once("; ") {
                if is_token(head)
                    && !v.contains(',')
                    && param.split_once('=').is_some_and(|(n, pv)| is_token(n) && is_token(pv))
                {
                    return one(v.to_string());
                }
            }
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

/// The names .NET keeps on request headers only, and on response headers only. On the other kind of message headers they are no
/// typed headers and are stored as written (SeHeaderValues.al:SE_RequestHeaders_OnResponse, SE_ResponseHeaders_OnRequest, corpus bump
/// PR 737).
const REQUEST_ONLY: &[&str] = &[
    "accept",
    "accept-charset",
    "accept-encoding",
    "accept-language",
    "authorization",
    "expect",
    "from",
    "host",
    "if-match",
    "if-modified-since",
    "if-none-match",
    "if-range",
    "if-unmodified-since",
    "max-forwards",
    "proxy-authorization",
    "range",
    "referer",
    "te",
    "user-agent",
];
const RESPONSE_ONLY: &[&str] = &[
    "accept-ranges",
    "age",
    "etag",
    "location",
    "proxy-authenticate",
    "retry-after",
    "server",
    "vary",
    "www-authenticate",
];
/// The request-only names SE_RequestHeaders_OnResponse added to response headers.
const REQUEST_ONLY_MEASURED_ON_RESPONSE: &[&str] = &[
    "range",
    "if-match",
    "accept",
    "authorization",
    "user-agent",
    "host",
    "referer",
    "max-forwards",
    "expect",
];
/// The names SE_ClientHeaders added to the client's default headers.
const MEASURED_ON_CLIENT: &[&str] = &[
    "range",
    "accept",
    "if-match",
    "etag",
    "age",
    "max-forwards",
    "referer",
    "via",
    "date",
];

/// A header a request or response collection would store as written: a name of the other kind of message.
fn of_the_other_message(kind: HeaderKind, lower: &str) -> bool {
    match kind {
        HeaderKind::Response => REQUEST_ONLY.contains(&lower),
        HeaderKind::Request | HeaderKind::Default => RESPONSE_ONLY.contains(&lower),
        HeaderKind::Content => false,
    }
}

/// `q=<number>` with spaces around the parts allowed.
fn is_quality(p: &str) -> bool {
    p.split_once('=').is_some_and(|(n, q)| {
        n.trim().eq_ignore_ascii_case("q")
            && !q.trim().is_empty()
            && q.trim().chars().all(|c| c.is_ascii_digit() || c == '.')
    })
}

/// The general and message headers that network/lt-se-header-values/SeHeaderValues.al (corpus bump PR 737) measured and the rules
/// above leave alone: Accept-Charset, Accept-Language and TE (request), Pragma, Trailer, Upgrade, Via and Warning (request and
/// response), Server and Proxy-Authenticate (response). `From` is stored as written (`a@b.example`, `not an email`). Value shapes
/// that file did not measure are refused.
fn typed_add_se(bag: &HeaderBag, lower: &str, value: &str) -> Result<Option<Typed>, Signal> {
    let v = value.trim();
    let list = |values: Vec<String>| Ok(Some(Typed { values, replace: false }));
    let fail = || header_error::<Option<Typed>>("Format");
    let unmeasured = || unsupported(format!("HttpHeaders.Add of the {lower} value '{value}': not measured"));
    let spaced_tokens = |s: &str| s.split_whitespace().count() > 1 && s.split_whitespace().all(is_token);
    match (bag.kind, lower) {
        // `utf-8`, `utf-8 ; q=0.5` (read `utf-8; q=0.5`), `en-US,de;q=0.8` (two values), `deflate;q=0.5`; `a b`, `e n` and `;` refused
        // (SE_Untyped_Request_1, _2).
        (HeaderKind::Request | HeaderKind::Default, "accept-charset" | "accept-language" | "te") => {
            let mut out = Vec::new();
            for item in split_list(value) {
                let mut parts = item.split(';');
                let head = parts.next().unwrap_or("").trim();
                if head.is_empty() || head.contains(char::is_whitespace) {
                    return fail();
                }
                if !is_token(head) || !parts.all(is_quality) {
                    return unmeasured();
                }
                out.push(join_params(&item));
            }
            if out.is_empty() {
                return unmeasured();
            }
            list(out)
        }
        // `no-cache`, `a=b` stored as written; `a b` refused (SE_Untyped_Request_2, SE_Untyped_Response).
        (_, "pragma") => match v.split_once('=') {
            _ if v.contains(',') => unmeasured(),
            None if is_token(v) => list(vec![v.to_string()]),
            Some((n, pv)) if is_token(n) && is_token(pv) => list(vec![v.to_string()]),
            None if spaced_tokens(v) => fail(),
            _ => unmeasured(),
        },
        // `X-A`, `websocket`, `HTTP/2.0` stored as written; `a b` refused (SE_Untyped_Request_2, _3).
        (_, "trailer" | "upgrade") => {
            let product = |s: &str| match s.split_once('/') {
                Some((n, ver)) => lower == "upgrade" && is_token(n) && is_token(ver),
                None => is_token(s),
            };
            match () {
                _ if v.contains(',') => unmeasured(),
                _ if product(v) => list(vec![v.to_string()]),
                _ if spaced_tokens(v) => fail(),
                _ => unmeasured(),
            }
        }
        // `1.1 proxy` and `1.0 fred, 1.1 p.example` (two values) stored as written; `bad` refused (SE_Untyped_Request_3,
        // SE_Untyped_Response, SE_ClientHeaders).
        (_, "via") => {
            let mut out = Vec::new();
            for item in split_list(value) {
                let parts: Vec<&str> = item.split(' ').collect();
                match parts.as_slice() {
                    [one] if is_token(one) => return fail(),
                    [version, host] if is_token(version) && is_token(host) => out.push(item.clone()),
                    _ => return unmeasured(),
                }
            }
            if out.is_empty() {
                return unmeasured();
            }
            list(out)
        }
        // `199 - "msg"` stored as written; `bad` refused (SE_Untyped_Request_3).
        (_, "warning") => {
            let parts: Vec<&str> = v.splitn(3, ' ').collect();
            match parts.as_slice() {
                [one] if is_token(one) => fail(),
                [code, agent, text]
                    if code.len() == 3
                        && code.chars().all(|c| c.is_ascii_digit())
                        && is_token(agent)
                        && text.len() >= 2
                        && text.starts_with('"')
                        && text.ends_with('"')
                        && !text[1..text.len() - 1].contains('"') =>
                {
                    list(vec![v.to_string()])
                }
                _ => unmeasured(),
            }
        }
        // Products and comments, each a value, as User-Agent: `a/1.0 (x)` reads `[a/1.0][(x)]`, `a b` reads `[a][b]`; `(` refused
        // (SE_Untyped_Response).
        (HeaderKind::Response, "server") => match user_agent_parts(value)? {
            Some(parts) => list(parts),
            None => fail(),
        },
        // A lone comma is an empty list (Add answers true, nothing stored); `Basic realm="x"` stored as written (SE_Untyped_Response).
        (HeaderKind::Response, "proxy-authenticate") => {
            if split_list(v).is_empty() && !v.is_empty() {
                return list(Vec::new());
            }
            match v.split_once(' ') {
                Some((scheme, param))
                    if is_token(scheme)
                        && param.split_once('=').is_some_and(|(n, pv)| {
                            is_token(n) && pv.len() >= 2 && pv.starts_with('"') && pv.ends_with('"')
                        })
                        && !param.contains(',') =>
                {
                    list(vec![v.to_string()])
                }
                _ => unmeasured(),
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
    let lower = name.to_ascii_lowercase();
    // A name of the other kind of message is stored as written (SeHeaderValues.al:SE_RequestHeaders_OnResponse,
    // SE_ResponseHeaders_OnRequest). Measured for every response-only
    // name on request headers, nine request-only names on response headers and ETag and Age on the client's headers.
    if of_the_other_message(bag.kind, &lower) {
        let measured = match bag.kind {
            HeaderKind::Response => REQUEST_ONLY_MEASURED_ON_RESPONSE.contains(&lower.as_str()),
            HeaderKind::Default => MEASURED_ON_CLIENT.contains(&lower.as_str()),
            _ => true,
        };
        if !measured {
            assume(crate::assumptions::Assumption::HttpTypedHeaderValueShapes);
        }
        push_value(&mut bag.items, &stored, value.to_string());
        return Ok(());
    }
    let se = typed_add_se(bag, &lower, value)?;
    let typed = match se {
        Some(t) => {
            // Measured by SeHeaderValues.al (corpus bump 72-78).
            Some(t)
        }
        None => match typed_add(bag, name, value)? {
            Some(t) => Some(t),
            None => typed_add_more(bag, name, value)?,
        },
    };
    // The client's default headers check like request headers (SeHeaderValues.al:SE_ClientHeaders, corpus bump PR 737); names that
    // file did not add to them follow that rule too.
    if bag.kind == HeaderKind::Default && !MEASURED_ON_CLIENT.contains(&lower.as_str()) {
        assume(crate::assumptions::Assumption::HttpTypedHeaderValueShapes);
    }
    // The value rules of a typed header other than Cache-Control (pinned by network/lt-http-header-names-cache-control/CacheControl*.al)
    // were measured for a few values each (AyHeaderAddResult.al, the P87 files, SeHeaderValues.al); a value of a shape they did not
    // measure follows the same rule (assumption http-typed-header-value-shapes).
    if typed.is_some() && lower != "cache-control" {
        assume(crate::assumptions::Assumption::HttpTypedHeaderValueShapes);
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
        // :383, InvalidOperation). lt-http-header-rules/P87Twice.al raises it for the statement. The false sets the last error to the
        // header error, text and code (:499-543 PY_AddFalse_LastError_*, on request, content and response headers); an Add that
        // answers true leaves it (`ok_after_prior`). For an empty name, on client headers and in the `if not H.Add(..)` form the pin
        // is :552-620 (SE_AddFalse_*).
        Err(Signal::Error(e))
            if ret_used
                && matches!(
                    e.code.as_str(),
                    "InvalidHeader:Format" | "InvalidHeader:Argument" | "InvalidOperation"
                ) =>
        {
            c.set_last_error(*e);
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
        unnamed_secret_header(b, Some(&name))?;
        Ok(Value::Boolean(find(&b.items, &name).is_some()))
    })?
}

/// Refuses a read of a secret content's headers that could meet the header no test names (`HeaderBag::unnamed_secret_header`):
/// the list of keys, and a header other than the Content-Type.
fn unnamed_secret_header(b: &HeaderBag, name: Option<&str>) -> Result<(), Signal> {
    if b.unnamed_secret_header && !name.is_some_and(|n| n.eq_ignore_ascii_case("Content-Type")) {
        return unsupported(
            "the headers of an HttpContent written from a SecretText: the header it lists besides the Content-Type is not pinned",
        );
    }
    Ok(())
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
        unnamed_secret_header(b, None)?;
        Ok::<_, Signal>(
            b.items
                .iter()
                .map(|(n, _)| Value::Text(n.clone().into()))
                .collect::<Vec<_>>(),
        )
    })??;
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
        unnamed_secret_header(b, Some(&name))?;
        if restricted_lookup(b, &name).is_err() {
            return Ok::<_, Signal>((None, b.kind));
        }
        Ok((find(&b.items, &name).map(|(_, v)| v.clone()), b.kind))
    })??;
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

/// `GetSecretValues(Name, var Values)` reads the secret values a name holds (xml/lt-xml-attr-doc-http-members/P97HdrSecretValues.al):
/// - the name is matched without regard to case (:106, :136); a name with only plain values, a missing name and an empty name
///   answer false (:103-105), and so does a collection no GetHeaders filled (:210-221 GetSecretValues_UninitialisedHeaders);
/// - an array is filled from the first element; when the values do not fit, false and the array is left as it was
///   (:102 `four_into_three`);
/// - a list gets the values appended; on false it is left as it was (:141-163 GetSecretValues_ListIsClearedOrAppended);
/// - Remove and Clear drop the secret values (:165-186), TryAddWithoutValidation stores one (:188-208).
///
/// Only the answer as a used result is pinned: a statement, and a name that is no token or a content header name, are refused.
/// Whether a false answer sets the last error is not measured: it is left as it was, as GetValues does on request headers.
fn headers_get_secret_values(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if !c.ret_used() {
        return unsupported("HttpHeaders.GetSecretValues as a statement (not pinned)");
    }
    let name = text(a, 0)?;
    let values = with_bag(r.as_deref(), |b| {
        if !name.is_empty() && restricted_lookup(b, &name).is_err() {
            return unsupported(format!(
                "HttpHeaders.GetSecretValues of `{name}`, a content header or no token (not pinned)"
            ));
        }
        Ok(find(&b.secret, &name).map(|(_, v)| v.clone()))
    })??;
    let Some(values) = values.filter(|_| !name.is_empty()) else {
        return Ok(Value::Boolean(false));
    };
    let secret = |v: String| lib("SecretText", LibraryValue::Text(v));
    match a.get(1) {
        Some(Value::Array(arr)) => {
            let mut arr = arr.borrow_mut();
            if values.len() > arr.len() {
                return Ok(Value::Boolean(false));
            }
            for (i, v) in values.into_iter().enumerate() {
                arr[i] = secret(v);
            }
        }
        Some(Value::List(list)) => list.borrow_mut().extend(values.into_iter().map(secret)),
        other => return unsupported(format!("HttpHeaders.GetSecretValues into {}", other.describe())),
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

/// `WriteFrom(SecretText)`: the body is the secret's text and the content is secret: `IsSecretContent` is true, `ReadAs(SecretText)`
/// gives it back, `ReadAs(Text)` answers false, and the headers no longer list the Content-Type
/// (xml/lt-xml-attr-doc-http-members/P97ContentSecret.al:83-101, :103-117, :119-150, :152-168; al-runner issue 6161). A later
/// WriteFrom of text or a stream, and Clear, make it plain again (:119-150, :170-190).
fn write_from_secret(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (t, _) = text_or_secret(a, 0)?;
    with_content(r, |s| {
        s.write_body(t.into_bytes());
        s.secret = true;
        let mut bag = s.headers.borrow_mut();
        bag.items.retain(|(n, _)| !n.eq_ignore_ascii_case("Content-Type"));
        bag.unnamed_secret_header = true;
    })
    .map(|()| Value::Null)
}

/// `IsSecretContent()`: the last WriteFrom took a SecretText (P97ContentSecret.al:83-150; network/lt-uf-http-clear/UfHttpClear.al:102
/// after a Clear and a WriteFrom of text).
fn is_secret_content(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_content(r, |s| Value::Boolean(s.secret))
}

/// `ReadAs(var SecretText)` with its result used answers true and gives the body as a secret, of a secret or a plain content, a
/// new one included (P97ContentSecret.al:21-28 with :83-150, :215-234). The statement form and the content of a request that has
/// none are not pinned.
fn read_as_secret(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    if !c.ret_used() {
        return unsupported("HttpContent.ReadAs(SecretText) as a statement (not pinned)");
    }
    let (data, uninit) = with_content(r, |s| (s.data.clone(), s.uninitialized))?;
    if uninit {
        return unsupported("HttpContent.ReadAs(SecretText) on the content of a request that has none (not pinned)");
    }
    let Ok(t) = String::from_utf8(data) else {
        return unsupported("HttpContent.ReadAs(SecretText) of a body that is not UTF-8 (not pinned)");
    };
    a[0] = lib("SecretText", LibraryValue::Text(t));
    Ok(Value::Boolean(true))
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
    let mut r = r;
    // A secret content does not read as text: with the result used the call answers false and leaves the text
    // (P97ContentSecret.al:30-38 with :83-101, :119-168). The statement form is not pinned.
    if with_content(r.as_deref_mut(), |s| s.secret)? {
        if c.ret_used() {
            return Ok(Value::Boolean(false));
        }
        return unsupported(
            "HttpContent.ReadAs(Text) as a statement on a content written from a SecretText (not pinned)",
        );
    }
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
        // Reading the body makes the content buffer it, and its headers then list a Content-Length of the body's length when they
        // hold none (misc/lt-probes-needed-5/PnE12HttpContent.al:WriteFromText_ThenReadAndHeaders reads `5` after WriteFrom('hello')
        // and ReadAs; network/lt-ay-http/AyContentLength.al:ContentLength_* and PY_*: none after ReadAs(InStream), an empty body reads
        // `0`). A Content-Length the headers already hold stays as it is: one an earlier read put there (PY_ContentLength_LongerBodyAfterRead
        // `second_read` Yes[5]) and one added by hand (AyContentLength.al:324-343 SE_ContentLength_AddedByHandBeforeRead `Yes[3]`).
        if !s.uninitialized {
            let mut bag = s.headers.borrow_mut();
            if find(&bag.items, "Content-Length").is_none() {
                bag.items
                    .push(("Content-Length".into(), vec![s.data.len().to_string()]));
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
    let mut r = r;
    // On a secret content the stream is left not initialized (P97ContentSecret.al:40-51: its EOS raises NotInitialized); what the
    // call answers is not pinned.
    if with_content(r.as_deref_mut(), |s| s.secret)? {
        return unsupported(
            "HttpContent.ReadAs(InStream) on a content written from a SecretText (its result is not pinned)",
        );
    }
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
    with_client(r, |s| {
        // Whether Clear also resets the certificate setting is not pinned: the mark stays, so a request no handler answers
        // stays refused.
        let cert_validation_set = s.cert_validation_set;
        *s = HttpClientState {
            cert_validation_set,
            ..HttpClientState::default()
        };
    })
    .map(|()| Value::Null)
}

/// `UseServerCertificateValidation(false)` gives the client new default request headers: an HttpHeaders taken earlier from
/// `DefaultRequestHeaders` no longer reaches the client, one taken after the call does; `true` keeps the link
/// (network/lt-cz-http-leads/CzHttpNoNetwork.al:98-121 DefaultRequestHeaders_ReferenceLost_AfterValidationOff and :123-143,
/// BC 27, 28, 29). Pinned only on a client with empty default headers, no base address and the default timeout: whether the
/// new client keeps them is not, so that detach is refused. The result is not pinned either: a used result is refused
/// (lowering drops the result of `if ... then;` with no branches). A request of the client that no handler answers is
/// refused (`cert_validation_set`); with a handler it is answered as before
/// (al-language-onprem/network/HttpClientAuthConfig.al:117-126 `after_certificate_validation_off`).
///
/// The call answers its argument when that repeats the setting made last (network/lt-httpclient-property-assign/
/// HttpClientPropertyAssign.al:24-27,36-37); any other used result stays refused. `false` on a client whose only change is its
/// timeout gives it the default timeout back (HttpClientPropertyAssign.al:73-77).
fn client_use_cert_validation(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let off = match a.first() {
        Some(Value::Boolean(b)) => !*b,
        other => {
            return unsupported(format!(
                "HttpClient.UseServerCertificateValidation of {}",
                other.map_or("nothing".into(), |v| v.describe())
            ));
        }
    };
    let ret_used = c.ret_used();
    with_client(r, |s| {
        if ret_used && s.cert_validation_last != Some(!off) {
            return unsupported("the result of HttpClient.UseServerCertificateValidation (not pinned)");
        }
        if off {
            let filled = {
                let b = s.default_headers.borrow();
                !b.items.is_empty() || !b.secret.is_empty()
            };
            if filled || !s.base_address.is_empty() {
                return unsupported(
                    "HttpClient.UseServerCertificateValidation(false) on a client with default request headers, a base address or a timeout: whether the client keeps them is not pinned",
                );
            }
            s.default_headers = crate::value::new_header_list(crate::value::HeaderKind::Default);
            s.timeout_ms = HttpClientState::default().timeout_ms;
        }
        s.cert_validation_set = true;
        s.cert_validation_last = Some(!off);
        Ok(if ret_used { Value::Boolean(!off) } else { Value::Null })
    })?
}

/// `UseResponseCookies` takes a Boolean and changes nothing a test can see (network/lt-http/HttpClient.al:BooleanSettings).
fn client_use_cookies(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_client(r, |_| ()).map(|()| Value::Null)
}

/// Hands `(method, url)` to the session and writes the response it returns into the `var Response` argument.
fn send(
    c: &mut dyn Ctx,
    method: &str,
    url: (String, bool),
    (cert_validation_set, cert_validation_off): (bool, bool),
    response: Option<&Value>,
) -> Out {
    let state = c.http_send(HttpOutgoing {
        method: method.to_ascii_uppercase(),
        url: url.0,
        has_secret_uri: false,
        unpinned_join: url.1,
        cert_validation_set,
        cert_validation_off,
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
            let (base, cert) = with_client(r, |s| {
                (
                    s.base_address.clone(),
                    (s.cert_validation_set, s.cert_validation_last == Some(false)),
                )
            })?;
            let path = text(a, 0)?;
            let url = resolve_request_path(&base, &path)?;
            let response = if $with_content { a.get(2) } else { a.get(1) };
            send(c, $method, url, cert, response)
        }
    };
}
verb!(client_get, "GET", false);
verb!(client_delete, "DELETE", false);
verb!(client_post, "POST", true);
verb!(client_put, "PUT", true);
verb!(client_patch, "PATCH", true);

fn client_send(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (base, cert) = with_client(r, |s| {
        (
            s.base_address.clone(),
            (s.cert_validation_set, s.cert_validation_last == Some(false)),
        )
    })?;
    let Some(Value::Library(_, req)) = a.first() else {
        return unsupported("HttpClient.Send without a request");
    };
    let (method, uri) = match &*req.borrow() {
        LibraryValue::HttpRequest(s) if s.uri_secret => {
            return unsupported(
                "HttpClient.Send of a request with a secret uri (what the request and a handler see is not pinned)",
            );
        }
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
    send(c, &method, url, cert, a.get(1))
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
            "ReasonPhrase of a response whose [HttpClientHandler] set a status and no ReasonPhrase: the phrase BC gives this status is not pinned (docs/probes-needed.md)",
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
            // AyHttpHandler.al:PY_ReasonPhrase_1 to _3 (al-language-onprem/network/lt-ay-http-handler, :486-583, BC 29; Windows
            // BC 27.5 and 28.5 give the same values): these values raise InvalidValue. The text of 301, 302 and 304 is pinned
            // only up to its 60th character, so reading it is refused. Every other value is kept as before.
            match n {
                418 | 299 | 599 | 0 | 999 | 1000 | -1 => {
                    return Err(Signal::Error(Box::new(crate::native::AlError {
                        message: format!("The value '{n}' is not a valid HTTP status code."),
                        code: "InvalidValue".into(),
                        ..Default::default()
                    })));
                }
                301 | 302 | 304 => {
                    return Err(Signal::Error(Box::new(crate::native::unpinned_text_error(
                        "InvalidValue",
                        "the full text of 'The value '301' is outside the allowed range for mocked HTTP' (AyHttpHandler.al:502 pins 60 characters)",
                    ))));
                }
                _ => {}
            }
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

/// What an Expires value reads as: a `DateTime` value, the parse error, or a form no corpus test pins.
enum CookieDate {
    At(i64),
    ParseError,
    NotPinned,
}

/// The Expires forms network/lt-http-cookies/HttpCookieForms.al:91 Response_ExpiresForms pins (and P94Cookie.al: RFC 1123 `Wed, 21
/// Oct 2037 07:28:00 GMT`): after a weekday and a comma (the weekday is not checked, `Mon, 21 Oct 2037` reads), `DD Mon YYYY`,
/// `DD-Mon-YYYY` or `DD-Mon-37` (`21-Oct-37` is 2037; other two-digit years are not pinned), the month in any case, the clock,
/// then `GMT`, no zone (read as GMT) or `+HHMM` (`+0200` is two hours earlier in UTC); and ISO `YYYY-MM-DDTHH:MM:SSZ`. The zone
/// `UTC`, the asctime form `Wed Oct 21 07:28:00 2037`, a text without a digit (`garbage`) and the empty text raise the parse error.
fn parse_cookie_date(text: &str) -> CookieDate {
    let text = text.trim();
    if text.is_empty() || !text.bytes().any(|b| b.is_ascii_digit()) {
        return CookieDate::ParseError;
    }
    let clock = |c: &str| -> Option<i64> {
        let mut it = c.split(':');
        let h: i64 = it.next()?.parse().ok()?;
        let m: i64 = it.next()?.parse().ok()?;
        let s: i64 = it.next()?.parse().ok()?;
        (it.next().is_none() && h < 24 && m < 60 && s < 60 && c.len() == 8).then_some((h * 60 + m) * 60 + s)
    };
    let at = |y: i32, mo: u32, d: u32, secs: i64| -> CookieDate {
        if !(1..=12).contains(&mo) || !(1..=31).contains(&d) {
            return CookieDate::NotPinned;
        }
        let day = i64::from(crate::format::day_from_civil(y, mo, d));
        CookieDate::At((day - 1) * 86_400_000 + secs * 1000 + 1)
    };
    // ISO 8601 with a `Z` (`iso`).
    if let Some(iso) = text.strip_suffix('Z') {
        if let Some((date, time)) = iso.split_once('T') {
            let p: Vec<&str> = date.split('-').collect();
            if let ([y, mo, d], Some(secs)) = (p.as_slice(), clock(time)) {
                if let (4, Ok(y), Ok(mo), Ok(d)) = (y.len(), y.parse::<i32>(), mo.parse::<u32>(), d.parse::<u32>()) {
                    return at(y, mo, d, secs);
                }
            }
        }
        return CookieDate::NotPinned;
    }
    let months = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let month = |m: &str| {
        months
            .iter()
            .position(|x| x.eq_ignore_ascii_case(m))
            .map(|i| i as u32 + 1)
    };
    let words: Vec<&str> = text.split_whitespace().collect();
    // asctime: `Wed Oct 21 07:28:00 2037` (`asctime`).
    if let [w, mo, d, c, y] = words.as_slice() {
        if !w.ends_with(',')
            && month(mo).is_some()
            && d.parse::<u32>().is_ok()
            && clock(c).is_some()
            && y.parse::<i32>().is_ok()
        {
            return CookieDate::ParseError;
        }
    }
    let Some((_, rest)) = text.split_once(',') else {
        return CookieDate::NotPinned;
    };
    let words: Vec<&str> = rest.split_whitespace().collect();
    let (date, c, zone) = match words.as_slice() {
        [d, mo, y, c, rest @ ..] if rest.len() <= 1 => (vec![*d, *mo, *y], *c, rest.first().copied()),
        [dmy, c, rest @ ..] if rest.len() <= 1 => (dmy.split('-').collect(), *c, rest.first().copied()),
        _ => return CookieDate::NotPinned,
    };
    let ([d, mo, y], Some(secs)) = (date.as_slice(), clock(c)) else {
        return CookieDate::NotPinned;
    };
    let (Ok(d), Some(mo)) = (d.parse::<u32>(), month(mo)) else {
        return CookieDate::NotPinned;
    };
    let y = match (y.len(), y.parse::<i32>()) {
        (4, Ok(y)) => y,
        // `Wednesday, 21-Oct-37 07:28:00 GMT` (`rfc850`).
        (2, Ok(37)) => 2037,
        _ => return CookieDate::NotPinned,
    };
    let shift = match zone {
        None => 0,
        Some(z) if z.eq_ignore_ascii_case("GMT") => 0,
        Some(z) if z.eq_ignore_ascii_case("UTC") => return CookieDate::ParseError,
        Some(z) => match z
            .strip_prefix('+')
            .filter(|o| o.len() == 4 && o.bytes().all(|b| b.is_ascii_digit()))
        {
            Some(o) => {
                let (h, m): (i64, i64) = (o[..2].parse().unwrap_or(0), o[2..].parse().unwrap_or(0));
                if h > 23 || m > 59 {
                    return CookieDate::NotPinned;
                }
                (h * 60 + m) * 60
            }
            None => return CookieDate::NotPinned,
        },
    };
    at(y, mo, d, secs - shift)
}

/// The cookies of one Set-Cookie value: commas separate cookies (`a=1, b=2`, `a=1,b=2`, `a=x,y` is `a=x` and `y`), except the
/// comma after the weekday of an Expires date (`a=1; Expires=Wed, 21 Oct 2037 07:28:00 GMT, b=2` is two cookies)
/// (HttpCookieForms.al:149 Response_TwoCookiesInOneValue). A comma inside double quotes is not pinned.
fn split_set_cookie(header: &str) -> Result<Vec<String>, Signal> {
    let mut out: Vec<String> = Vec::new();
    let mut joins_next = false;
    let segments: Vec<&str> = header.split(',').collect();
    for (i, seg) in segments.iter().enumerate() {
        match out.last_mut() {
            Some(last) if joins_next => {
                last.push(',');
                last.push_str(seg);
            }
            _ => out.push(seg.to_string()),
        }
        let last = out.last().expect("pushed");
        // A lone double quote is part of the value (`sid=a"b`, :168 Response_NamesThatAreNotTokens).
        if last.matches('"').count() % 2 == 1 && i + 1 < segments.len() {
            return unsupported("a Set-Cookie value with a comma inside double quotes (not pinned)");
        }
        let tail = last.rsplit(';').next().unwrap_or_default().trim();
        joins_next = tail.split_once('=').is_some_and(|(k, v)| {
            k.trim().eq_ignore_ascii_case("expires")
                && !v.trim().is_empty()
                && v.trim().chars().all(|c| c.is_ascii_alphabetic())
        });
    }
    Ok(out)
}

/// The cookies a response's Set-Cookie headers leave in a cookie container asked for `https://bccookie.microsoft.com/`, in the
/// order they were set (platform/lt-xmlport-session-navapp-cookie/P94Cookie.al, network/lt-http-cookies/HttpCookieForms.al):
/// - A Domain attribute, an empty name (`=abc`) and a name that starts with `$` raise the parse error; `a b` and `café` are
///   names (:168 Response_NamesThatAreNotTokens).
/// - A cookie whose Path is not `/` or empty is not sent to that address; a repeated Path whose first is not `/` drops it; `Path=`
///   gives the path `` (:193 Response_OtherAttributes). `Version=1` and `Port="443"` make the default path empty; `Secure=false`
///   and `HttpOnly=0` set neither; Comment, Discard, Priority, Partitioned, SameSite and unknown attributes are ignored.
/// - Max-Age is an Integer (a sign `+` allowed) of seconds from now; 0 and -1 drop the cookie; `abc`, `1.5`, empty and
///   `99999999999` raise the parse error; another whole number past the Integer range is not pinned and is refused. Of Max-Age and Expires the first one decides (`Max-Age=3600; Expires=<1970>` stays,
///   `Expires=<1970>; Max-Age=3600` and `Max-Age=0; Expires=<2037>` are gone) (:122 Response_MaxAgeForms). Two of the same
///   kind are not pinned.
/// - A later cookie of the same name in any case takes the place of the earlier one (`SID` then `sid` leaves `sid`, :222
///   Response_NamesDifferInCase); an expired one removes it.
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
        for one in split_set_cookie(&header)? {
            let mut parts = one.split(';');
            let first = parts.next().unwrap_or_default();
            let (name, value) = match first.split_once('=') {
                Some((n, v)) => (n.trim().to_string(), v.trim().to_string()),
                None => (first.trim().to_string(), String::new()),
            };
            if name.is_empty() || name.starts_with('$') {
                return Err(cookie_parse_error());
            }
            let mut c = CookieState {
                name,
                value,
                ..CookieState::default()
            };
            let mut path: Option<String> = None;
            let mut dropped = false;
            let mut empty_default_path = false;
            let mut expires: Option<i64> = None;
            let mut max_age: Option<i64> = None;
            // Which of Expires and Max-Age came first.
            let mut max_age_first: Option<bool> = None;
            for attr in parts {
                let (k, v) = match attr.split_once('=') {
                    Some((k, v)) => (k.trim(), Some(v.trim())),
                    None => (attr.trim(), None),
                };
                match (k.to_ascii_lowercase().as_str(), v) {
                    ("", None) => {}
                    ("domain", _) => return Err(cookie_parse_error()),
                    ("path", Some(p)) => match &path {
                        None => path = Some(p.to_string()),
                        Some(first) if first != "/" => dropped = true,
                        Some(_) => {
                            return unsupported("a Set-Cookie value with a second Path after `Path=/` (not pinned)");
                        }
                    },
                    ("secure", None) => c.secure = true,
                    ("secure", Some("false")) => {}
                    ("httponly", None) => c.http_only = true,
                    ("httponly", Some("0")) => {}
                    // `Expires` with no value is ignored (`noValue`).
                    ("expires", None) => {}
                    ("expires", Some(v)) => match parse_cookie_date(v) {
                        CookieDate::At(at) => {
                            if expires.replace(at).is_some() {
                                return unsupported("a cookie with two Expires attributes (not pinned)");
                            }
                            max_age_first.get_or_insert(false);
                        }
                        CookieDate::ParseError => return Err(cookie_parse_error()),
                        CookieDate::NotPinned => {
                            return unsupported(format!("a cookie Expires of the form `{v}` (not pinned)"));
                        }
                    },
                    ("max-age", Some(v)) => {
                        let digits = v.strip_prefix('+').unwrap_or(v);
                        let Ok(secs) = digits.parse::<i32>() else {
                            // Only `99999999999` is pinned past the Integer range (:122 Response_MaxAgeForms `huge`); other
                            // whole numbers out of range are not.
                            let whole = digits.strip_prefix('-').unwrap_or(digits);
                            if v != "99999999999" && !whole.is_empty() && whole.bytes().all(|b| b.is_ascii_digit()) {
                                return unsupported(format!(
                                    "a cookie Max-Age of `{v}`, a whole number past the Integer range (only 99999999999 is pinned)"
                                ));
                            }
                            return Err(cookie_parse_error());
                        };
                        if max_age.replace(i64::from(secs)).is_some() {
                            return unsupported("a cookie with two Max-Age attributes (not pinned)");
                        }
                        max_age_first.get_or_insert(true);
                    }
                    ("version", Some("1")) | ("port", Some("\"443\"")) => empty_default_path = true,
                    ("comment" | "priority" | "samesite", Some(_)) | ("discard" | "partitioned", None) => {}
                    ("path" | "secure" | "httponly" | "max-age" | "version" | "port", _) => {
                        return unsupported(format!("the cookie attribute `{}` (not pinned)", attr.trim()));
                    }
                    _ => {}
                }
            }
            if empty_default_path && path.is_some() {
                return unsupported("a cookie with Version or Port and a Path (not pinned)");
            }
            let path_ok = match path.as_deref() {
                None => {
                    c.path = if empty_default_path { String::new() } else { "/".into() };
                    true
                }
                Some(p @ ("/" | "")) => {
                    c.path = p.to_string();
                    true
                }
                Some(_) => false,
            } && !dropped;
            match (max_age_first, max_age, expires) {
                (Some(true), Some(secs), _) => {
                    let Some(now) = ctx.now_unix_millis() else {
                        return unsupported("Max-Age of a cookie without a clock");
                    };
                    c.expires = unix_to_datetime(now + secs * 1000);
                }
                (_, _, Some(at)) => c.expires = at,
                _ => {}
            }
            let mut expired = false;
            if let Some(now) = ctx.now_unix_millis() {
                if c.expires != 0 && c.expires <= unix_to_datetime(now) {
                    expired = true;
                }
            }
            out.retain(|o| !(o.name.eq_ignore_ascii_case(&c.name) && path_ok));
            if path_ok && !expired {
                out.push(c);
            }
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

fn cookie_name(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let new = if a.is_empty() { None } else { Some(text(a, 0)?) };
    // A setter whose result is used answers the new text (network/lt-ay-http/AyCookieSetterExpression.al:28, `[n] get=n` and
    // `[v] get=v`, on a fresh cookie and on one with other values).
    // An empty name raises and keeps the old one (network/lt-ay-http/AyCookieSetterExpression.al:CookieSetter_EmptyName, BC 27-29).
    // A `;`, `,` or `=` in it, a `$` first, only blanks, or blanks on both sides raise the same error with the name
    // (network/lt-http-cookies/HttpCookieForms.al:286 CookieSetter_InvalidCharacters); `a b` is a name. A blank on one side only
    // and the other control characters are not pinned.
    if let Some(n) = new.as_deref() {
        let blanks_around = n.len() > 1 && n.starts_with(' ') && n.ends_with(' ');
        if n.trim().is_empty() || n.contains([';', ',', '=']) || n.starts_with('$') || blanks_around {
            return raise(format!("The 'Name'='{n}' part of the cookie is invalid."), "AL:Cookie");
        }
        if n.starts_with(char::is_whitespace) || n.ends_with(char::is_whitespace) || n.contains(char::is_control) {
            return unsupported(format!(
                "a cookie Name `{n}` with a blank on one side or a control character (not pinned)"
            ));
        }
    }
    with_cookie(r, |s| {
        if let Some(n) = new {
            s.name = n;
        }
        Value::Text(s.name.clone().into())
    })
}

fn cookie_value(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let new = if a.is_empty() { None } else { Some(text(a, 0)?) };
    // A setter whose result is used answers the new text (network/lt-ay-http/AyCookieSetterExpression.al:28, `[n] get=n` and
    // `[v] get=v`, on a fresh cookie and on one with other values).
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
            c.unnamed_secret_header = bag.unnamed_secret_header;
        }
        HttpContentState {
            data: self.data.clone(),
            headers: copy,
            uninitialized: self.uninitialized,
            stream_taken: self.stream_taken,
            written: self.written,
            copied: true,
            from_stream: self.from_stream,
            secret: self.secret,
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
        self.secret = false;
    }

    /// `WriteFrom` of text or a stream: the body is replaced, the header collection stays the one every `HttpHeaders` handed out
    /// shows, with its Content-Type set to `text/plain; charset=utf-8` (put first when it was removed) and the other headers kept
    /// (lt-http-header-rules/P87Content.al:AddThenWrite_Text, WriteThenAdd_Text, RewriteKeepsHeader, P87ContentType.al:HeaderBeforeWrite,
    /// P87ContentStream.al, HttpWriteFromStream.al:P100_*). A Content-Length the headers already hold (see `read_as`) is not
    /// rewritten: it stays for an empty body (network/lt-ay-http/AyContentLength.al:ContentLength_SecondWriteFrom `empty`
    /// `Yes[13]`), for a body of the same length (P100_SecondWrite_ReplacesBody, 5 bytes twice) and for a longer, a shorter or a
    /// stream body (AyContentLength.al:PY_ContentLength_LongerBodyAfterRead, _ShorterBodyAfterRead, _InStreamAfterRead `Yes[5]`).
    /// Pinned for a value a read put there (AyContentLength.al PY_ContentLength_*); for one added by hand it is assumption
    /// http-content-length-after-read.
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
            if find(&bag.items, "Content-Length").is_some() {
                assume(crate::assumptions::Assumption::HttpContentLengthAfterRead);
            }
        }
        self.data = data;
        self.uninitialized = false;
        self.stream_taken = false;
        self.written = true;
        self.from_stream = false;
        self.secret = false;
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
    fn a_user_agent_product_outside_the_pinned_shapes_is_refused() {
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
        for v in ["Shop App / Product 365", "a /1", "a/ 1", "/", "a/b/c"] {
            let mut b = HeaderBag::default();
            assert!(
                matches!(add_header(&mut b, "User-Agent", v, false), Err(Signal::Unsupported(_))),
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
