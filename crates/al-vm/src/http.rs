//! `HttpClient` calls answered by the running test's `[HttpClientHandler]` (ADR-027 section 5). Nothing here opens a
//! connection: a request is given to the handler, raises the policy error, or is refused.
//!
//! Pinned by the staging probes tests/al-language-onprem/session/PZHttpHandler.al (codeunit 61297) and
//! PZHttpBlock.al (61298), BC 29.0.54011.55816.

use crate::Session;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::describe::Describe;
use al_builtins::http::{fresh_test_response, split_url};
use al_builtins::native::{HttpOutgoing, Signal, raise};
use al_builtins::value::{LibraryValue, TestHttpRequestState, TestHttpResponseState, Value};
use std::cell::RefCell;
use std::rc::Rc;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// The policy a `TestHttpRequestPolicy` property names, in the casing BC prints it.
fn canonical_policy(written: &str) -> Option<&'static str> {
    [
        "AllowAllOutboundRequests",
        "BlockOutboundRequests",
        "AllowOutboundFromHandler",
    ]
    .into_iter()
    .find(|p| written.trim().trim_matches('\'').eq_ignore_ascii_case(p))
}

impl Session {
    /// The value of property `name` of object `object`, as written.
    pub fn object_property(&self, object: usize, name: &str) -> Option<String> {
        self.engine.program.objects[object]
            .properties
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim().to_string())
    }

    /// Sets the `TestHttpRequestPolicy` of the running test codeunit (its property value as written), or none.
    pub fn set_http_policy(&mut self, written: Option<&str>) {
        self.http_policy = written.and_then(canonical_policy);
    }

    /// The `HttpRequestType` value for `method`: a standard method by name, any other `Unknown`.
    fn http_request_type(&self, method: &str) -> Result<Value, Signal> {
        let members = al_builtins::sysenum::option_members("HttpRequestType").expect("known option type");
        let member = match method {
            "GET" => "Get",
            "POST" => "Post",
            "PUT" => "Put",
            "DELETE" => "Delete",
            "PATCH" => "Patch",
            "HEAD" => "Head",
            "OPTIONS" => "Options",
            "CONNECT" => "Connect",
            "TRACE" => "Trace",
            _ => "Unknown",
        };
        let ordinal = members.iter().position(|m| m == member).expect("a member") as i32;
        Ok(Value::Option(ordinal, members.into()))
    }

    pub(crate) fn http_send_impl(&mut self, request: HttpOutgoing) -> Result<TestHttpResponseState, Signal> {
        let policy = self.http_policy;
        // After UseServerCertificateValidation only a request a handler answers is pinned (al-language-onprem/network/
        // HttpClientAuthConfig.al:117-126): with no handler, or a relative path the handler never sees, it is refused.
        if request.cert_validation_set && (!self.has_http_handler() || !request.url.contains("://")) {
            return unsupported(
                "a request of an HttpClient after UseServerCertificateValidation that no [HttpClientHandler] answers (not pinned)",
            );
        }
        // A bound handler is called under every policy, on every major: BC 29 asserted (al-language-onprem/session/PZHttpBlock.al:50,
        // :106; network/lt-ay-http-handler/AyHttpHandler.al and PyHttpPolicy.al, the BC29PLUS legs). Windows BC 27.5 and 28.5 give the
        // same values (Windows check 2026-10-22); the Linux images of 27 and 28 never call the handler (bc-linux issue 192), so the
        // corpus asserts nothing on 27 and 28 (al-runner issue 5777).
        if !self.has_http_handler() {
            return match policy {
                Some(p @ ("BlockOutboundRequests" | "AllowOutboundFromHandler")) => raise(
                    format!(
                        "Unhandled outbound HTTP requests are not permitted when the TestHttpRequestPolicy property is set to '{p}'. Please ensure that an HTTP client handler is configured."
                    ),
                    "TestCodeunitUnhandledHttpRequest",
                ),
                // The request goes out for real, with no approval prompt (corpus bump 43: network/TestHttpClient.al and
                // lt-cz-http-leads/CzHttpOutbound.al bind no StrMenuHandler).
                _ => real_request(&request.url),
            };
        }
        if !request.url.contains("://") && !matches!(policy, Some("BlockOutboundRequests" | "AllowOutboundFromHandler"))
        {
            return unsupported(
                "a request to a relative path with no base address reaches a [HttpClientHandler] under the default policy or AllowAllOutboundRequests: not measured",
            );
        }
        if !request.url.contains("://") {
            // With no base address, `a/b`, `a.b` and `?q=1` raise the empty-uri error before the handler runs
            // (AyHttpHandler.al:NoBaseAddress_RelativePaths_1 and _2, BC 29). Other forms are not measured.
            // The handler is not called, and that does not fail the test (the tests pass on real BC 29).
            if al_builtins::http::is_empty_uri_form(&request.url) {
                self.http_handlers_count_as_run();
                return raise("Invalid URI: The URI is empty.", "AL:UriFormat");
            }
            return unsupported(
                "a request to a relative path with no base address, or with no uri, reaches a [HttpClientHandler]: what the handler sees is not measured",
            );
        }
        if request.unpinned_join {
            assume(Assumption::HttpRelativeRequestPath);
        }
        let (path, params) = split_url(&request.url);
        let query = Value::Dictionary(Rc::new(RefCell::new(
            params
                .into_iter()
                .map(|(k, v)| (Value::Text(k.into()), Value::Text(v.into())))
                .collect(),
        )));
        let req = Value::Library(
            "TestHttpRequestMessage".into(),
            Rc::new(RefCell::new(LibraryValue::TestHttpRequest(TestHttpRequestState {
                path,
                query,
                has_secret_uri: request.has_secret_uri,
                request_type: self.http_request_type(&request.method)?,
            }))),
        );
        let resp = Value::Library(
            "TestHttpResponseMessage".into(),
            Rc::new(RefCell::new(LibraryValue::TestHttpResponse(fresh_test_response()))),
        );
        let Some((answer, resp)) = self.dispatch_http_handler(req, resp)? else {
            return unsupported("the [HttpClientHandler] went away");
        };
        match answer {
            Value::Boolean(false) | Value::Null => {}
            // The handler lets the real request go, with no approval prompt: AyHttpHandler.al:Handler_FallThrough_ConnectionRefused
            // and the PY_HandlerTrue_* tests (BC 29, `menu=0`), PyHttpPolicy.al:Handler_AnswersTrue_* (corpus bump 43).
            // Under BlockOutboundRequests the request may not go out: PZHttpBlock.al:131-155
            // (RS_PassOnHandler_ReturnsTrue_UnderBlockOutboundRequests, BC 29; the handler ran once). BC 27 and 28 are not measured
            // (the corpus discards those legs); refusing there would lose that baseline case, so they raise the same error.
            Value::Boolean(true) if request.cert_validation_set => {
                return unsupported(
                    "an [HttpClientHandler] that lets the request of an HttpClient after UseServerCertificateValidation go out (not pinned)",
                );
            }
            Value::Boolean(true) if policy == Some("BlockOutboundRequests") => {
                return raise(
                    "Explicit HTTP client handler fallthrough is not allowed when the TestHttpRequestPolicy property is set to 'BlockOutboundRequests'.",
                    "NotAllowedHttpClientHandlerFallThrough",
                );
            }
            Value::Boolean(true) => return real_request(&request.url),
            other => return unsupported(format!("an [HttpClientHandler] that answers {}", other.describe())),
        }
        match resp {
            Value::Library(_, cell) => match &*cell.borrow() {
                LibraryValue::TestHttpResponse(s) => with_reason_phrase(s.clone()),
                LibraryValue::Empty => Ok(fresh_test_response()),
                _ => unsupported("an [HttpClientHandler] left something else than a TestHttpResponseMessage"),
            },
            _ => unsupported("an [HttpClientHandler] left something else than a TestHttpResponseMessage"),
        }
    }
}

/// A request that goes out for real: answered only where it fails before any connection
/// (`al_builtins::http::failure_without_connection`), else refused, since the runtime never opens a connection (ADR-027).
fn real_request(url: &str) -> Result<TestHttpResponseState, Signal> {
    match al_builtins::http::failure_without_connection(url) {
        Some((message, code)) => {
            let mut failed = fresh_test_response();
            failed.request_failed = true;
            failed.failure = Some((message.into(), code.into()));
            Ok(failed)
        }
        None => unsupported(
            "an outbound HTTP request that a test handler does not answer: the runtime never opens a connection (ADR-027). \
             To run the test, bind an [HttpClientHandler] that answers the request, or set TestHttpRequestPolicy = \
             BlockOutboundRequests on the test codeunit to get BC's error for it",
        ),
    }
}

/// A response whose handler set a status but no `ReasonPhrase` gets the status's reason phrase: 200 keeps "OK"
/// (PZHttpHandler.al:EmptyHandler_AnswersADefaultResponse), 404 gives "Not Found" (AyHttpHandler.al:Handler_OnlyStatus404), and
/// the statuses of AyHttpHandler.al:PY_ReasonPhrase_1 to _3 (:486-583, BC 29; Windows BC 27.5 and 28.5 give the same values,
/// Windows check 2026-10-22) give theirs. Another status's phrase is not measured: reading it is refused (`reason_unpinned`).
fn with_reason_phrase(mut s: TestHttpResponseState) -> Result<TestHttpResponseState, Signal> {
    if !s.reason_set {
        let reason = match s.status {
            100 => "Continue",
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            204 => "No Content",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            422 => "Unprocessable Entity",
            429 => "Too Many Requests",
            451 => "Unavailable For Legal Reasons",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "",
        };
        if reason.is_empty() {
            s.reason_unpinned = true;
        } else {
            s.reason = reason.into();
        }
    }
    Ok(s)
}
