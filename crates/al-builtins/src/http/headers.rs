//! The `HttpHeaders` methods: Add, Remove, Contains, Keys, GetValues, TryAddWithoutValidation.

use super::*;

// ------------------------------------------------------------------------------------------------ headers

pub(super) const INVALID_HEADER: &str = "Invalid HTTP header. Please, make sure the format of the header is correct.";
pub(super) const GET_VALUES_MISSING: &str =
    "Exception of type 'Microsoft.Dynamics.Nav.Types.Exceptions.NavNclInvalidHeaderException' was thrown.";
pub(super) const INVALID_OPERATION: &str = "The requested operation cannot be performed in this context.";

pub(super) fn is_token_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || "!#$%&'*+-.^_`|~".contains(c)
}

/// A name the collection of `kind` refuses: the content headers on request, response or client headers, and on the headers of a content
/// every known name that is no content header (network/lt-http-header-names-cache-control/HeaderNameCaseContent.al: all 39 known names;
/// lt-http-header-rules/P87Matrix.al for the first eleven).
pub(super) fn not_here(kind: HeaderKind, name: &str) -> bool {
    if kind != HeaderKind::Content {
        return is_content_header(name);
    }
    canonical_name(name).is_some() && !is_content_header(name)
}

/// The headers `HttpContentHeaders` owns: a request, response or client header collection refuses them.
pub(super) fn is_content_header(name: &str) -> bool {
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

pub(super) fn header_error<T>(code: &str) -> Result<T, Signal> {
    raise(INVALID_HEADER, &format!("InvalidHeader:{code}"))
}

/// Whether `v` is `type/subtype` with optional `;name=value` parameters.
pub(super) fn is_media_type(v: &str) -> bool {
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
pub(super) fn param_value(v: &str) -> bool {
    !v.is_empty() && (!v.starts_with('"') || (v.len() >= 2 && v.ends_with('"')))
}

pub(super) fn find<'a>(items: &'a [(String, Vec<String>)], name: &str) -> Option<&'a (String, Vec<String>)> {
    items.iter().find(|(n, _)| n.eq_ignore_ascii_case(name))
}

pub(super) fn push_value(items: &mut Vec<(String, Vec<String>)>, name: &str, value: String) {
    match items.iter_mut().find(|(n, _)| n.eq_ignore_ascii_case(name)) {
        Some((_, v)) => v.push(value),
        None => items.push((name.to_string(), vec![value])),
    }
}

/// `HttpHeaders.Add` (network/lt-http/HttpHeaders.al, HttpClient.al, HttpContent.al, HttpResponse.al).
pub(super) fn add_header(bag: &mut HeaderBag, name: &str, value: &str, secret: bool) -> Result<(), Signal> {
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

pub(super) fn with_bag<R>(r: Option<&Value>, f: impl FnOnce(&mut HeaderBag) -> R) -> Result<R, Signal> {
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

pub(super) fn headers_add(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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
        // A User-Agent product that is not `name` or `name/version`: false, nothing stored (UserAgentProductForms.al:110-121); the
        // last error it leaves is not pinned, so reading it refuses.
        Err(Signal::Error(e)) if ret_used && e.message.ends_with(UA_BAD_PRODUCT_PROBE) => {
            c.set_last_error(*e);
            Ok(Value::Boolean(false))
        }
        Err(e) => Err(e),
    }
}

/// `Remove(Name)` drops every value of the name and answers true, whether or not it was there.
pub(super) fn headers_remove(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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
pub(super) fn restricted_lookup(b: &HeaderBag, name: &str) -> Result<(), Signal> {
    if not_here(b.kind, name) {
        return header_error("InvalidOperation");
    }
    if name.chars().any(|c| !is_token_char(c)) {
        return header_error("Format");
    }
    Ok(())
}

pub(super) fn headers_contains(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    with_bag(r.as_deref(), |b| {
        restricted_lookup(b, &name)?;
        unnamed_secret_header(b, Some(&name))?;
        Ok(Value::Boolean(find(&b.items, &name).is_some()))
    })?
}

/// Refuses a read of a secret content's headers that could meet the header no test names (`HeaderBag::unnamed_secret_header`):
/// the list of keys, and a header other than the Content-Type.
pub(super) fn unnamed_secret_header(b: &HeaderBag, name: Option<&str>) -> Result<(), Signal> {
    if b.unnamed_secret_header && !name.is_some_and(|n| n.eq_ignore_ascii_case("Content-Type")) {
        return unsupported(
            "the headers of an HttpContent written from a SecretText: the header it lists besides the Content-Type is not pinned",
        );
    }
    Ok(())
}

pub(super) fn headers_contains_secret(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    with_bag(r.as_deref(), |b| Value::Boolean(find(&b.secret, &name).is_some()))
}

pub(super) fn headers_clear(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_bag(r.as_deref(), |b| {
        b.items.clear();
        b.secret.clear();
    })?;
    Ok(Value::Null)
}

pub(super) fn headers_keys(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
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
pub(super) fn headers_get_values(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    // A content header name asked of request, response or client headers is no header there: GetValues answers false
    // (lt-http-header-rules/P87Matrix.al `get=No/n=0`), where Contains raises.
    let (values, kind, restricted) = with_bag(r.as_deref(), |b| {
        unnamed_secret_header(b, Some(&name))?;
        if restricted_lookup(b, &name).is_err() {
            return Ok::<_, Signal>((None, b.kind, true));
        }
        Ok((find(&b.items, &name).map(|(_, v)| v.clone()), b.kind, false))
    })??;
    let Some(values) = values else {
        let error = crate::native::AlError {
            message: GET_VALUES_MISSING.into(),
            code: "InvalidHeader".into(),
            ..Default::default()
        };
        // A statement on request and client headers for a token name that is not there raises the same text and code as on content
        // headers: its first 80 characters and the code are pinned by network/lt-se-header-values/UserAgentProductForms.al:116-118,
        // :138-140 (`get=E[Exception of type '...NavNclInvalidHeaderEx|InvalidHeader]` after an Add answered false).
        let pinned_missing = matches!(kind, HeaderKind::Request | HeaderKind::Default) && !restricted && !c.ret_used();
        if kind != HeaderKind::Content && !pinned_missing {
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
pub(super) fn headers_get_secret_values(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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
pub(super) fn headers_try_add(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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
