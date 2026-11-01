//! The per-header rules of `HttpHeaders.Add`: known names, value forms, and which headers belong to which message.

use super::*;

pub(super) const KNOWN_NAMES: &[&str] = &[
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
pub(super) fn canonical_name(name: &str) -> Option<&'static str> {
    KNOWN_NAMES.iter().copied().find(|k| k.eq_ignore_ascii_case(name))
}

/// The name a header is stored under. A known header written in another case than .NET's canonical form is stored canonically, in
/// all 39 known names and on request, content and response headers (network/lt-http-header-names-cache-control/HeaderNameCaseRequest.al,
/// HeaderNameCaseContent.al, HeaderNameCaseResponse.al: `keys=1:Accept-Charset`); a custom name keeps its first case (CustomNameCase.al).
pub(super) fn stored_name(items: &[(String, Vec<String>)], name: &str) -> Result<String, Signal> {
    if let Some((n, _)) = find(items, name) {
        return Ok(n.clone());
    }
    Ok(canonical_name(name).map_or_else(|| name.to_string(), str::to_string))
}

/// `a; b=c` with one "; " between the parts (.NET writes a parameter list that way).
pub(super) fn join_params(v: &str) -> String {
    v.split(';').map(str::trim).collect::<Vec<_>>().join("; ")
}

pub(super) fn is_token(s: &str) -> bool {
    !s.is_empty() && s.chars().all(is_token_char)
}

/// Splits a list header value at the commas outside quotes.
pub(super) fn split_list(v: &str) -> Vec<String> {
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
pub(super) fn token_with_params(item: &str) -> Option<String> {
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
pub(super) fn is_entity_tag(item: &str) -> bool {
    let t = item.strip_prefix("W/").unwrap_or(item);
    item == "*" || (t.len() >= 2 && t.starts_with('"') && t.ends_with('"') && !t[1..t.len() - 1].contains('"'))
}

/// The probe that would pin the error a User-Agent with a product that is not `name` or `name/version` raises as a statement, and
/// the last error an Add of one sets when its result is used (it answers false: UserAgentProductForms.al:110-121).
pub(super) const UA_BAD_PRODUCT_PROBE: &str = "http-user-agent-bad-product-error";

/// The tokens of a User-Agent (`ua`) or Server: products `name/version` and comments `(...)`, each a value of its own. `Ok(None)`
/// for an unbalanced comment (`((((`: P87Values.al:330-336). Valid products: SeHeaderValues.al:302-311.
/// User-Agent: blanks around a `/` are dropped (`a / b`, `a /1`, `a/ 1` read `[a/b]`, `[a/1]`; `A B / C D 365 E` reads five values
/// `[A][B/C][D][365][E]`), and a product that is still not `token` or `token/token` (`/`, `a/b/c`, `a@b`) makes Add answer false
/// with nothing stored (network/lt-se-header-values/UserAgentProductForms.al:110-121, :132-143, :154-165): an error marked with
/// [`UA_BAD_PRODUCT_PROBE`], whose text and code are not pinned. Server: neither is measured, so such a product is refused.
pub(super) fn user_agent_parts(v: &str, ua: bool) -> Result<Option<Vec<String>>, Signal> {
    let mut chars: Vec<char> = v.chars().collect();
    if ua {
        // Drop the blanks next to a `/` outside comments.
        let (mut out, mut depth) = (Vec::with_capacity(chars.len()), 0i32);
        for (i, &c) in chars.iter().enumerate() {
            match c {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            if depth == 0 && c.is_whitespace() {
                let next = chars[i + 1..].iter().find(|c| !c.is_whitespace());
                if out.last() == Some(&'/') || next == Some(&'/') {
                    continue;
                }
            }
            out.push(c);
        }
        chars = out;
    }
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
            if !ok && ua {
                return Err(Signal::Error(Box::new(crate::native::unpinned_error_for(
                    UA_BAD_PRODUCT_PROBE,
                ))));
            }
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
pub(super) const CACHE_CONTROL_ORDER: &[&str] = &[
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
pub(super) fn cache_control_item(item: &str) -> Result<Option<(String, String)>, Signal> {
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
pub(super) fn cache_control(existing: &[String], added: &str) -> Result<Option<String>, Signal> {
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
pub(super) struct Typed {
    pub(super) values: Vec<String>,
    /// The collection holds these values instead of what it held (Cache-Control is one merged value).
    pub(super) replace: bool,
}

pub(super) fn typed_add(bag: &HeaderBag, name: &str, value: &str) -> Result<Option<Typed>, Signal> {
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
        "user-agent" => match user_agent_parts(value, true)? {
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
pub(super) fn plain_number(v: &str) -> Result<bool, Signal> {
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
pub(super) fn uri_header(v: &str) -> Result<String, Signal> {
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
pub(super) fn content_range(v: &str) -> Result<bool, Signal> {
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
pub(super) fn typed_add_more(bag: &HeaderBag, name: &str, value: &str) -> Result<Option<Typed>, Signal> {
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
pub(super) const REQUEST_ONLY: &[&str] = &[
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
pub(super) const RESPONSE_ONLY: &[&str] = &[
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
pub(super) const REQUEST_ONLY_MEASURED_ON_RESPONSE: &[&str] = &[
    "range",
    "if-match",
    "accept",
    "authorization",
    "user-agent",
    "host",
    "referer",
    "max-forwards",
    "expect",
    // SeHeaderValues.al:454-505 Y2_RequestOnlyNames_OnResponse and _OnClient.
    "accept-charset",
    "accept-encoding",
    "accept-language",
    "from",
    "if-modified-since",
    "if-none-match",
    "if-range",
    "if-unmodified-since",
    "proxy-authorization",
    "te",
];
/// The names SE_ClientHeaders added to the client's default headers.
pub(super) const MEASURED_ON_CLIENT: &[&str] = &[
    "range",
    "accept",
    "if-match",
    "etag",
    "age",
    "max-forwards",
    "referer",
    "via",
    "date",
    // SeHeaderValues.al:454-505 Y2_RequestOnlyNames_OnResponse and _OnClient.
    "accept-charset",
    "accept-encoding",
    "accept-language",
    "from",
    "if-modified-since",
    "if-none-match",
    "if-range",
    "if-unmodified-since",
    "proxy-authorization",
    "te",
];

/// A header a request or response collection would store as written: a name of the other kind of message.
pub(super) fn of_the_other_message(kind: HeaderKind, lower: &str) -> bool {
    match kind {
        HeaderKind::Response => REQUEST_ONLY.contains(&lower),
        HeaderKind::Request | HeaderKind::Default => RESPONSE_ONLY.contains(&lower),
        HeaderKind::Content => false,
    }
}

/// `q=<number>` with spaces around the parts allowed.
pub(super) fn is_quality(p: &str) -> bool {
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
pub(super) fn typed_add_se(bag: &HeaderBag, lower: &str, value: &str) -> Result<Option<Typed>, Signal> {
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
                // A quality that is no number is refused (SeHeaderValues.al:507-528 Y2_Shapes_RequestOnlyValues,
                // `utf-8;q=x`).
                let params: Vec<&str> = parts.collect();
                if params.iter().any(|p| {
                    p.split_once('=').is_some_and(|(n, q)| {
                        n.trim().eq_ignore_ascii_case("q") && is_token(q.trim()) && !is_quality(p)
                    })
                }) {
                    return fail();
                }
                if !is_token(head) || !params.iter().all(|p| is_quality(p)) {
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
        (HeaderKind::Response, "server") => match user_agent_parts(value, false)? {
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
