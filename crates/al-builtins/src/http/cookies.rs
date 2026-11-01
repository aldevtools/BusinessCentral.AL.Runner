//! `HttpCookie` and the Set-Cookie headers of a response.

use super::*;

// ------------------------------------------------------------------------------------------------ cookies

/// The error of a Set-Cookie header the container refuses (P94Cookie.al:Response_AllAttributes).
pub(super) fn cookie_parse_error() -> Signal {
    match raise::<()>(
        "An error occurred when parsing the Cookie header for Uri 'https://bccookie.microsoft.com/'.",
        "AL:Cookie",
    ) {
        Err(e) => e,
        Ok(()) => unreachable!("raise fails"),
    }
}

/// What an Expires value reads as: a `DateTime` value, the parse error, or a form no corpus test pins.
pub(super) enum CookieDate {
    At(i64),
    ParseError,
    NotPinned,
}

/// The Expires forms network/lt-http-cookies/HttpCookieForms.al:91 Response_ExpiresForms pins (and P94Cookie.al: RFC 1123 `Wed, 21
/// Oct 2037 07:28:00 GMT`): after a weekday and a comma (the weekday is not checked, `Mon, 21 Oct 2037` reads), `DD Mon YYYY`,
/// `DD-Mon-YYYY` or `DD-Mon-37` (`21-Oct-37` is 2037; other two-digit years are not pinned), the month in any case, the clock,
/// then `GMT`, no zone (read as GMT) or `+HHMM` (`+0200` is two hours earlier in UTC); and ISO `YYYY-MM-DDTHH:MM:SSZ`. The zone
/// `UTC`, the asctime form `Wed Oct 21 07:28:00 2037`, a text without a digit (`garbage`) and the empty text raise the parse error.
pub(super) fn parse_cookie_date(text: &str) -> CookieDate {
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
pub(super) fn split_set_cookie(header: &str) -> Result<Vec<String>, Signal> {
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
pub(super) fn response_cookies(ctx: &dyn Ctx, headers: &HttpHeaderList) -> Result<Vec<CookieState>, Signal> {
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

pub(super) fn unix_to_datetime(ms: i64) -> i64 {
    (i64::from(crate::format::day_from_civil(1970, 1, 1)) - 1) * 86_400_000 + ms + 1
}

pub(super) fn response_cookie_names(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    let headers = with_response(r, |s| s.headers.clone())?;
    let cookies = response_cookies(c, &headers)?;
    let names: Vec<Value> = cookies.into_iter().map(|k| Value::Text(k.name.into())).collect();
    Ok(Value::List(Rc::new(RefCell::new(names.into()))))
}

pub(super) fn response_get_cookie(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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

pub(super) fn with_cookie<R>(r: Option<&mut Value>, f: impl FnOnce(&mut CookieState) -> R) -> Result<R, Signal> {
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

pub(super) fn cookie_name(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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

pub(super) fn cookie_value(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Out {
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

pub(super) fn cookie_domain(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_cookie(r, |s| Value::Text(s.domain.clone().into()))
}

pub(super) fn cookie_path(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_cookie(r, |s| Value::Text(s.path.clone().into()))
}

pub(super) fn cookie_secure(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_cookie(r, |s| Value::Boolean(s.secure))
}

pub(super) fn cookie_http_only(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_cookie(r, |s| Value::Boolean(s.http_only))
}

pub(super) fn cookie_expires(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Out {
    with_cookie(r, |s| Value::DateTime(s.expires))
}
