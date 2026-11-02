//! `HttpClient.UseDefaultNetworkWindowsAuthentication` and `UseWindowsAuthentication` (Text and SecretText): configuration
//! calls that answer Yes, also when repeated or given blank texts (al-language-onprem/network/HttpClientAuthConfig.al:19-71).
//! A request after one that a `[HttpClientHandler]` answers is answered as before (:109-136); a request no handler answers is
//! refused, as is the call on a client with other settings (whether it rebuilds the client is not pinned).

use super::*;

pub(super) fn client_use_windows_auth(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    for v in a.iter() {
        if !matches!(
            v,
            Value::Text(_) | Value::Code(_) | Value::Library(crate::library_kind::LibraryKind::SecretText, _)
        ) {
            return unsupported(format!("HttpClient.UseWindowsAuthentication of {}", v.describe()));
        }
    }
    use_windows_auth(c, r)
}

pub(super) fn client_use_default_network_windows_auth(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    use_windows_auth(c, r)
}

fn use_windows_auth(c: &mut dyn Ctx, r: Option<&mut Value>) -> Out {
    let ret_used = c.ret_used();
    with_client(r, |s| {
        let fresh = HttpClientState::default();
        if !s.windows_auth_set
            && (s.rebuilt
                || s.cert_validation_set
                || s.cleared_after_rebuild
                || !s.base_address.is_empty()
                || s.timeout_ms != fresh.timeout_ms
                || !s.default_headers.borrow().items.is_empty()
                || !s.default_headers.borrow().secret.is_empty())
        {
            return unsupported(
                "Windows authentication on an HttpClient with other settings: whether it rebuilds the client is not pinned",
            );
        }
        s.windows_auth_set = true;
        Ok(if ret_used { Value::Boolean(true) } else { Value::Null })
    })?
}
