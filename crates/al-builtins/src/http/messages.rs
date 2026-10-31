//! `HttpResponseMessage`, `TestHttpRequestMessage`, `TestHttpResponseMessage` and `FileUpload`.

use super::*;

// ------------------------------------------------------------------------------------------------ response

pub(super) fn response_status(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_response(r, |s| Value::Integer(s.status))
}

pub(super) fn response_reason(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    match with_response(r, |s| {
        (!s.reason_unpinned).then(|| Value::Text(s.reason.clone().into()))
    })? {
        Some(v) => Ok(v),
        None => unsupported(
            "ReasonPhrase of a response whose [HttpClientHandler] set a status and no ReasonPhrase: the phrase BC gives this status is not pinned (docs/probes-needed.md)",
        ),
    }
}

pub(super) fn response_blocked(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_response(r, |s| Value::Boolean(s.blocked))
}

pub(super) fn response_success(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_response(r, |s| Value::Boolean((200..300).contains(&s.status)))
}

pub(super) fn response_content(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_response(r, |s| s.content.clone())
}

pub(super) fn response_headers(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_response(r, |s| headers_value(&s.headers))
}

// ------------------------------------------------------------------------------------------------ test messages

pub(super) fn test_request_type(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    match with_test_request(r, |s| s.request_type.clone())? {
        Value::Null => unsupported("TestHttpRequestMessage.RequestType of a request no client call made: not measured"),
        v => Ok(v),
    }
}

pub(super) fn test_request_path(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_test_request(r, |s| Value::Text(s.path.clone().into()))
}

pub(super) fn test_request_query(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_test_request(r, |s| s.query.clone())
}

pub(super) fn test_request_secret(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_test_request(r, |s| Value::Boolean(s.has_secret_uri))
}

pub(super) fn test_response_status(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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

pub(super) fn test_response_reason(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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

pub(super) fn test_response_blocked(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    match a.first() {
        None => with_test_response(r, |s| Value::Boolean(s.blocked)),
        Some(Value::Boolean(b)) => {
            let b = *b;
            with_test_response(r, |s| s.blocked = b).map(|()| Value::Null)
        }
        Some(other) => unsupported(format!("IsBlockedByEnvironment set from {}", other.describe())),
    }
}

pub(super) fn test_response_success(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    match a.first() {
        None => with_test_response(r, |s| Value::Boolean(s.success)),
        Some(Value::Boolean(b)) => {
            let b = *b;
            with_test_response(r, |s| s.success = b).map(|()| Value::Null)
        }
        Some(other) => unsupported(format!("IsSuccessfulRequest set from {}", other.describe())),
    }
}

pub(super) fn test_response_content(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_test_response(r, |s| s.content.clone())
}

pub(super) fn test_response_headers(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_test_response(r, |s| headers_value(&s.headers))
}

// ------------------------------------------------------------------------------------------------ misc

/// A `FileUpload` no client filled: `FileName()` is a null string (assigned to a Text it is empty) and
/// `CreateInStream` opens an empty stream without an error
/// (types/TestHttpRequestFileUploadFieldErrorContracts.al:FileUpload_NoClientFilled_OpensAnEmptyStream).
/// A filled FileUpload cannot exist here: there is no client.
pub(super) fn file_upload_name(_: &mut dyn Ctx, _: Option<&mut Value>, _: &mut [Value]) -> Out {
    Ok(Value::Null)
}

pub(super) fn file_upload_create_in(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> Out {
    crate::stream::create_empty_in_stream(a)?;
    Ok(Value::Null)
}
