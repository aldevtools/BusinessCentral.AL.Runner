//! URI handling: `System.Uri` normal forms, request-path resolution and the request-side methods.

use super::*;

// ------------------------------------------------------------------------------------------------ uri

pub(super) const INVALID_URL: &str = "Failed to create the URL for the request. Please, make sure to set a valid URL.";

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
pub(super) fn escape(s: &str) -> String {
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

pub(super) fn percent_decode(s: &str) -> String {
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
pub(super) fn resolve_request_path(base: &str, path: &str) -> Result<(String, bool), Signal> {
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
pub(super) fn is_pinned_join(base_path: &str, path: &str) -> bool {
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
pub(super) fn remove_dot_segments(path: &str) -> String {
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
pub(super) fn is_pinned_relative(path: &str) -> bool {
    let word = path.strip_prefix('/').unwrap_or(path);
    (!word.is_empty() && word.chars().all(|c| c.is_ascii_alphanumeric())) || is_empty_uri_form(path)
}

/// The no-base-address forms that raise "Invalid URI: The URI is empty." when they reach a handler on BC 29, exactly as pinned
/// (AyHttpHandler.al:NoBaseAddress_RelativePaths_1 `a/b`, `a.b`; _2 `?q=1`). Any other form is not measured.
pub fn is_empty_uri_form(path: &str) -> bool {
    matches!(path, "a/b" | "a.b" | "?q=1")
}

pub(super) fn invalid_url<T>() -> Result<T, Signal> {
    raise(INVALID_URL, "CreateUrl:UriFormat")
}

/// `SetRequestUri` and `SetBaseAddress`: a valid uri is kept in its normal form; an invalid one answers false when
/// the result is used and raises otherwise (handlers/PWRefusedBuiltins.al:HttpRequestMessage_SetRequestUri_ValidAndInvalid).
pub(super) fn checked_uri(c: &dyn Ctx, u: &str) -> Result<Option<String>, Signal> {
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
pub(super) fn method(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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

pub(super) fn set_request_uri(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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
pub(super) fn get_request_uri(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let uri = with_request(r, |s| if s.uri_secret { None } else { s.uri.clone() })?;
    Ok(Value::Text(uri.unwrap_or_default().into()))
}

/// `SetSecretRequestUri(SecretText)` answers whether the uri is valid and keeps a valid one as the request's secret uri. The forms
/// the corpus pins agree with `SetRequestUri`: empty, `not a uri` and a rooted path answer false; `ftp://`, `https://` with a
/// port, user info, a fragment or no path answer true; the method stays (xml/lt-xml-attr-doc-http-members/P97ReqSecretUri.al:
/// 118-158 SecretUri_InvalidValues, :160-192 SecretUri_Variants, :194-207 SecretUri_Method_Unchanged). A statement with an
/// invalid uri is not pinned (refused), nor is a form `SetRequestUri` itself has not measured.
pub(super) fn set_secret_request_uri(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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
pub(super) fn get_secret_request_uri(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let uri = with_request(r, |s| s.uri.clone())?.unwrap_or_default();
    Ok(lib("SecretText", LibraryValue::Text(uri)))
}

pub(super) fn request_get_headers(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let list = with_request(r, |s| s.headers.clone())?;
    hand_out_headers(a.first(), &list)?;
    Ok(Value::Boolean(true))
}

/// `Content := C` stores a copy of the content (misc/W4XmlJsonHttp.al:RequestContent_IsCopyOrReference: writing
/// the original afterwards does not change the request's), `Content` reads the request's own content. A request
/// nobody set a content on has an uninitialized one (network/lt-http/HttpRequest.al:FreshRequestContent).
pub(super) fn request_content(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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
