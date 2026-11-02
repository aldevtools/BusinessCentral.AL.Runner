//! The cookies of an `HttpRequestMessage`: `SetCookie(Name, Value)`, `SetCookie(Cookie)`, `GetCookie`, `GetCookieNames` and
//! `RemoveCookie` (xml/lt-xml-attr-doc-http-members/P97ReqCookie.al, P97ReqCookieObj.al; BC 29, with and without a uri).
//!
//! They live in the request's `Cookie` header, one `name=value` value per cookie: the header is there after a SetCookie
//! (P97ReqCookie.al:264-284 `header_cookie` Yes, `keys` 1), and a `Cookie` value added through the headers is one more cookie
//! (`names_after_manual` = `a;manual`). What the header's values read as, and every other change of the header, is not pinned:
//! once SetCookie wrote it, those are refused ([`cookie_header_guard`]), as is a request sent with cookies.

use super::*;

const COOKIE: &str = "Cookie";

/// Refuses a read or change of the `Cookie` header (or of every header: `name` None) of a request whose cookies SetCookie
/// wrote: what it holds then is not pinned.
pub(super) fn cookie_header_guard(b: &HeaderBag, name: Option<&str>) -> Result<(), Signal> {
    if b.set_cookies && name.is_none_or(|n| n.eq_ignore_ascii_case(COOKIE)) {
        return unsupported(
            "the Cookie header of a request whose cookies SetCookie wrote (what it holds is not pinned)",
        );
    }
    Ok(())
}

/// The request's cookies in order: (index of the header value, name, value). A value that is no single `name=value`
/// pair (several pairs, or one without `=`) is not pinned.
fn cookies(b: &HeaderBag) -> Result<Vec<(usize, String, String)>, Signal> {
    let Some((_, values)) = find(&b.items, COOKIE) else {
        return Ok(Vec::new());
    };
    if !b.set_cookies {
        return unsupported("the cookies of a request whose Cookie header was added without SetCookie (not pinned)");
    }
    let mut out = Vec::new();
    for (i, v) in values.iter().enumerate() {
        match v.split_once('=') {
            Some((n, val)) if !v.contains(';') && !n.trim().is_empty() => {
                out.push((i, n.trim().to_string(), val.to_string()))
            }
            _ => {
                return unsupported(format!(
                    "a Cookie header value `{v}` that is no single name=value pair (not pinned)"
                ));
            }
        }
    }
    Ok(out)
}

fn request_headers(r: Option<&mut Value>) -> Result<HttpHeaderList, Signal> {
    with_request(r, |s| s.headers.clone())
}

/// A name SetCookie takes: not empty, no `;`, `,` or `=`, no `$` first (P97ReqCookie.al:SetCookie_InvalidNamesAndValues: No,
/// `a b` and `café` Yes). Blanks around it, a blank on one side or a control character are not pinned.
fn name_ok(n: &str) -> Result<bool, Signal> {
    if n.is_empty() || n.contains([';', ',', '=']) || n.starts_with('$') {
        return Ok(false);
    }
    if n.starts_with(char::is_whitespace) || n.ends_with(char::is_whitespace) || n.contains(char::is_control) {
        return unsupported(format!(
            "SetCookie with the name `{n}` (a blank on one side or a control character: not pinned)"
        ));
    }
    Ok(true)
}

/// A value SetCookie takes: no `;` (No); `=`, blanks, quotes and other letters are kept (Yes). A `,` or a control character
/// is not pinned.
fn value_ok(v: &str) -> Result<bool, Signal> {
    if v.contains(';') {
        return Ok(false);
    }
    if v.contains(',') || v.contains(char::is_control) {
        return unsupported(format!(
            "SetCookie with the value `{v}` (a comma or a control character: not pinned)"
        ));
    }
    Ok(true)
}

/// Sets cookie `name`: a cookie of that name in another case is replaced where it stands, name and value
/// (P97ReqCookie.al:SetCookie_NoUri `names_after_case` = `A;b`); a new one goes last.
fn set(b: &mut HeaderBag, name: &str, value: &str) -> Result<(), Signal> {
    let pair = format!("{name}={value}");
    match cookies(b)?.into_iter().find(|(_, n, _)| n.eq_ignore_ascii_case(name)) {
        Some((i, _, _)) => {
            let at = b
                .items
                .iter()
                .position(|(n, _)| n.eq_ignore_ascii_case(COOKIE))
                .expect("the header holds it");
            b.items[at].1[i] = pair;
        }
        None => push_value(&mut b.items, COOKIE, pair),
    }
    b.set_cookies = true;
    Ok(())
}

/// The answer of a check that failed: No when the result is used; as a statement it is not pinned.
fn no(c: &dyn Ctx, what: &str) -> Out {
    if c.ret_used() {
        Ok(Value::Boolean(false))
    } else {
        unsupported(format!(
            "{what} as a statement with an argument it refuses (not pinned)"
        ))
    }
}

pub(super) fn request_set_cookie(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let (name, value) = match a.first() {
        // SetCookie(Cookie): its name and value; an uninitialised cookie is refused with No, one with only a name is set
        // with an empty value (P97ReqCookieObj.al:SetCookie_Object_Uninitialised). The cookie is copied
        // (GetCookie_Cookie_IsCopy).
        Some(Value::Library(_, cell)) if a.len() == 1 => match &*cell.borrow() {
            LibraryValue::Empty => return no(c, "HttpRequestMessage.SetCookie"),
            LibraryValue::Cookie(k) => {
                if !k.domain.is_empty() || !k.path.is_empty() || k.secure || k.http_only || k.expires != 0 {
                    return unsupported(
                        "HttpRequestMessage.SetCookie of a cookie with a domain, path, Secure, HttpOnly or Expires (not pinned)",
                    );
                }
                (k.name.clone(), k.value.clone())
            }
            other => return unsupported(format!("HttpRequestMessage.SetCookie of {}", other.describe())),
        },
        _ => (text(a, 0)?, text(a, 1)?),
    };
    if name.is_empty() {
        return no(c, "HttpRequestMessage.SetCookie");
    }
    if !name_ok(&name)? || !value_ok(&value)? {
        return no(c, "HttpRequestMessage.SetCookie");
    }
    let headers = request_headers(r)?;
    set(&mut headers.borrow_mut(), &name, &value)?;
    Ok(if c.ret_used() {
        Value::Boolean(true)
    } else {
        Value::Null
    })
}

/// `GetCookie(Name, var Cookie)`: the cookie of exactly that name (`abc` does not find `Abc`), as a new Cookie with an empty
/// domain, path `/` and neither Secure nor HttpOnly (P97ReqCookieObj.al `Yes|obj|v1||/|No|No`); no cookie for a missing or
/// empty name (P97ReqCookie.al:GetCookie_Missing_And_Case).
pub(super) fn request_get_cookie(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    let headers = request_headers(r)?;
    let found = cookies(&headers.borrow())?
        .into_iter()
        .find(|(_, n, _)| *n == name && !name.is_empty());
    match found {
        Some((_, n, v)) => {
            if a.len() > 1 {
                a[1] = lib(
                    "Cookie",
                    LibraryValue::Cookie(CookieState {
                        name: n,
                        value: v,
                        path: "/".into(),
                        ..CookieState::default()
                    }),
                );
            }
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        None => unsupported("HttpRequestMessage.GetCookie of a missing cookie as a statement (not pinned)"),
    }
}

pub(super) fn request_cookie_names(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let headers = request_headers(r)?;
    let names: Vec<Value> = cookies(&headers.borrow())?
        .into_iter()
        .map(|(_, n, _)| Value::Text(n.into()))
        .collect();
    Ok(Value::List(Rc::new(RefCell::new(names.into()))))
}

/// `RemoveCookie(Name)`: Yes when a cookie of exactly that name was there; No for one already removed, a missing or empty
/// name and a name in another case (P97ReqCookie.al:RemoveCookie_Results).
pub(super) fn request_remove_cookie(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
    let name = text(a, 0)?;
    let headers = request_headers(r)?;
    let mut b = headers.borrow_mut();
    let found = cookies(&b)?
        .into_iter()
        .find(|(_, n, _)| *n == name && !name.is_empty());
    let removed = match found {
        Some((i, _, _)) => {
            let at = b
                .items
                .iter()
                .position(|(n, _)| n.eq_ignore_ascii_case(COOKIE))
                .expect("the header holds it");
            b.items[at].1.remove(i);
            if b.items[at].1.is_empty() {
                return unsupported(
                    "RemoveCookie of the last cookie of a request (what its Cookie header is then is not pinned)",
                );
            }
            true
        }
        None => false,
    };
    if c.ret_used() {
        Ok(Value::Boolean(removed))
    } else if removed {
        Ok(Value::Null)
    } else {
        unsupported("HttpRequestMessage.RemoveCookie of a missing cookie as a statement (not pinned)")
    }
}
