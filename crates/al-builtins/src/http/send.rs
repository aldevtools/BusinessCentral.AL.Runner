//! `HttpClient`: settings, certificates and sending a request to the session.

use super::*;

// ------------------------------------------------------------------------------------------------ client

/// `HttpClient.Timeout` is 100 seconds until set; a value over 5 minutes raises
/// `HttpClientTimeoutTooLarge` naming the minutes of the value (network/lt-http/HttpClient.al:TimeoutValues).
pub(super) fn client_timeout(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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

pub(super) fn client_get_base(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_client(r, |s| Value::Text(s.base_address.clone().into()))
}

pub(super) fn client_set_base(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let u = text(a, 0)?;
    match checked_uri(c, &u)? {
        Some(n) => with_client(r, |s| s.base_address = n).map(|()| Value::Boolean(true)),
        None => Ok(Value::Boolean(false)),
    }
}

pub(super) fn client_default_headers(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_client(r, |s| headers_value(&s.default_headers))
}

/// `Clear` brings the client back to a fresh one: timeout, base address and default headers
/// (network/lt-http/HttpClient.al:ClientClear).
pub(super) fn client_clear(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_client(r, |s| {
        // Whether Clear also resets the certificate setting is not pinned: the mark stays, so a request no handler answers
        // stays refused.
        let cert_validation_set = s.cert_validation_set;
        let windows_auth_set = s.windows_auth_set;
        let cleared_after_rebuild = s.rebuilt || s.cleared_after_rebuild;
        *s = HttpClientState {
            cert_validation_set,
            windows_auth_set,
            cleared_after_rebuild,
            ..HttpClientState::default()
        };
    })
    .map(|()| Value::Null)
}

/// The first setting call that leaves the defaults (`UseServerCertificateValidation(false)`, `UseResponseCookies(true)`, call
/// or property-assign form) rebuilds the client: the default timeout (1 minute 40 seconds), no base address and new default
/// request headers, so the headers added before are gone and an HttpHeaders taken earlier no longer reaches the client. A
/// call that keeps a default (validation `true`, cookies `false`) on a client never rebuilt, and every call after the
/// rebuild, change nothing (network/lt-httpclient-property-assign/Y6HttpSettingChanges.al:30-132, :165-207, corpus master;
/// network/lt-cz-http-leads/CzHttpNoNetwork.al:98-143 for the headers reference, HttpClientPropertyAssign.al:73-77 for the
/// timeout). After a `Clear` of a rebuilt client whether a call rebuilds it again is not pinned: refused.
pub(super) fn client_settings_change(s: &mut HttpClientState, leaves_default: bool, what: &str) -> Result<(), Signal> {
    if !leaves_default || s.rebuilt {
        return Ok(());
    }
    if s.cleared_after_rebuild {
        return unsupported(format!(
            "HttpClient.{what} on a client cleared after a setting rebuilt it: whether it is rebuilt again is not pinned"
        ));
    }
    s.default_headers = crate::value::new_header_list(crate::value::HeaderKind::Default);
    s.timeout_ms = HttpClientState::default().timeout_ms;
    s.base_address.clear();
    s.rebuilt = true;
    Ok(())
}

/// `UseServerCertificateValidation` rebuilds the client as [`client_settings_change`] says. The call answers its argument
/// (Y6HttpSettingChanges.al:30-54: `true` on a fresh client Yes, `false` No, `false` again No, `true` after `false` Yes,
/// corpus master). A request of the client that no handler answers is refused outside BlockOutboundRequests
/// (`cert_validation_set`); with a handler it is answered as before
/// (al-language-onprem/network/HttpClientAuthConfig.al:117-126 `after_certificate_validation_off`).
pub(super) fn client_use_cert_validation(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let on = match a.first() {
        Some(Value::Boolean(b)) => *b,
        other => {
            return unsupported(format!(
                "HttpClient.UseServerCertificateValidation of {}",
                other.map_or("nothing".into(), |v| v.describe())
            ));
        }
    };
    let ret_used = c.ret_used();
    with_client(r, |s| {
        client_settings_change(s, !on, "UseServerCertificateValidation")?;
        s.cert_validation_set = true;
        s.cert_validation_last = Some(on);
        Ok(if ret_used { Value::Boolean(on) } else { Value::Null })
    })?
}

/// `UseResponseCookies` takes a Boolean; `true` rebuilds the client as [`client_settings_change`] says
/// (Y6HttpSettingChanges.al:83-110, :165-207, corpus master). Its result is not pinned.
pub(super) fn client_use_cookies(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let on = match a.first() {
        Some(Value::Boolean(b)) => *b,
        other => {
            return unsupported(format!(
                "HttpClient.UseResponseCookies of {}",
                other.map_or("nothing".into(), |v| v.describe())
            ));
        }
    };
    if c.ret_used() {
        return unsupported("the result of HttpClient.UseResponseCookies (not pinned)");
    }
    with_client(r, |s| client_settings_change(s, on, "UseResponseCookies"))?.map(|()| Value::Null)
}

/// `HttpClient.AddCertificate(Certificate[, Password])`, Text or SecretText, reads the certificate when it is called
/// (network/lt-httpclient-config-certificates/P108Cert1.al:21-40, Text, and P108Cert2.al:21-37, SecretText): an empty one
/// raises HttpRequest:Argument, text that is not Base64 HttpRequest:Format, and Base64 of a cut DER sequence (`MIIB`)
/// HttpRequest:Cryptographic, each with the generic request error text. Any other certificate (a complete DER structure,
/// which may be a real one, or bytes that do not start a DER sequence) is not pinned and is refused.
pub(super) fn client_add_certificate(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    const TEXT: &str =
        "There was an error while executing the HTTP request. Make sure you are connecting to a valid endpoint.";
    with_client(r, |_| ())?;
    let (cert, _) = text_or_secret(a, 0)?;
    if cert.is_empty() {
        return raise(TEXT, "HttpRequest:Argument");
    }
    let Ok(bytes) = crate::platform_native::b64_decode(&cert) else {
        return raise(TEXT, "HttpRequest:Format");
    };
    if bytes.first() == Some(&0x30) && !complete_der(&bytes) {
        return raise(TEXT, "HttpRequest:Cryptographic");
    }
    unsupported("HttpClient.AddCertificate of a certificate no test pins (whether BC takes it is not pinned)")
}

/// Whether `bytes` is one DER element with a definite length that spans all of them.
pub(super) fn complete_der(bytes: &[u8]) -> bool {
    let Some(&first) = bytes.get(1) else {
        return false;
    };
    let (len, header) = if first < 0x80 {
        (usize::from(first), 2)
    } else {
        let n = usize::from(first & 0x7f);
        if n == 0 || n > 4 || bytes.len() < 2 + n {
            return false;
        }
        let len = bytes[2..2 + n]
            .iter()
            .fold(0usize, |acc, &b| (acc << 8) | usize::from(b));
        (len, 2 + n)
    };
    header.checked_add(len) == Some(bytes.len())
}

/// Hands `(method, url)` to the session and writes the response it returns into the `var Response` argument.
pub(super) fn send(
    c: &mut dyn Ctx,
    method: &str,
    url: (String, bool),
    (cert_validation_set, cert_validation_off, windows_auth_set): (bool, bool, bool),
    response: Option<&Value>,
) -> Out {
    let state = c.http_send(HttpOutgoing {
        method: method.to_ascii_uppercase(),
        url: url.0,
        has_secret_uri: false,
        unpinned_join: url.1,
        cert_validation_set,
        cert_validation_off,
        windows_auth_set,
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
        pub(super) fn $name(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
            let (base, cert) = with_client(r, |s| {
                (
                    s.base_address.clone(),
                    (
                        s.cert_validation_set,
                        s.cert_validation_last == Some(false),
                        s.windows_auth_set,
                    ),
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

pub(super) fn client_send(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (base, cert) = with_client(r, |s| {
        (
            s.base_address.clone(),
            (
                s.cert_validation_set,
                s.cert_validation_last == Some(false),
                s.windows_auth_set,
            ),
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
        // What a handler sees of a request's cookies is not pinned (http::request_cookies).
        LibraryValue::HttpRequest(s) if s.headers.borrow().set_cookies => {
            return unsupported(
                "HttpClient.Send of a request with cookies set by SetCookie (what a handler sees is not pinned)",
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
