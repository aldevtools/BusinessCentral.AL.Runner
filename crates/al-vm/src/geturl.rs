//! `GetUrl` with a company, an object, a record and its filters, for `ClientType::Web`. Only what the corpus pins is built;
//! every other form is refused (ADR-042). The host part is the server address of the host (assumption
//! `geturl-web-base-address`); the query after it is pinned by:
//! - platform/lt-system-database-members/GetUrlCompany.al:67-77 and session/lt-geturl-shape/GetUrlShape.al:121-130 (company),
//! - GetUrlObjects.al:62-91 and GetUrlShape.al:152-163 (object type and id),
//! - GetUrlBookmarkFilter.al (bookmark of the key, filter text, UseFilters), GetUrlRecord.al, GetUrlRecordLayout.al.

use crate::Session;
use crate::records::Rec;
use al_builtins::describe::Describe;
use al_builtins::native::Signal;
use al_builtins::value::{LibraryValue, RtType, Value};

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

/// `http://host[:port]/path` with the port set to `port`.
fn with_port(base: &str, port: u16) -> String {
    let rest = &base["http://".len()..];
    let (authority, path) = rest.split_at(rest.find('/').unwrap_or(rest.len()));
    let host = match authority.rsplit_once(':') {
        Some((h, p)) if !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) => h,
        _ => authority,
    };
    format!("http://{host}:{port}{path}")
}

/// The option member name of a system option value.
fn member(v: Option<&Value>) -> Option<String> {
    match v {
        Some(Value::Option(o, members)) => members.get(*o as usize).cloned(),
        _ => None,
    }
}

/// Percent-encodes `s` for a query value. Letters, digits, `-`, `.`, `_` and `~` stay; every other printable ASCII
/// character but the backtick, and non-ASCII characters (UTF-8), are encoded with capital hex digits
/// (GetUrlCompany.al:67-77, GetUrlBookmarkFilter.al: filter_*, platform/lt-t-geturl/TGetUrl.al:310-329
/// GetUrl_Web_CompanyCharacters and TGetUrl.al:356-364 GetUrl_Web_FilterCharacters). The backtick and control characters
/// are not pinned and are refused.
fn encode(s: &str) -> Result<String, Signal> {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || "-._~".contains(c) {
            out.push(c);
        } else if c == '`' || c.is_ascii_control() {
            return unsupported(format!(
                "GetUrl with the character `{}` in a company or filter text: only letters, digits, . - and the characters of the pinned probes are built",
                c.escape_default()
            ));
        } else {
            let mut b = [0u8; 4];
            for byte in c.encode_utf8(&mut b).bytes() {
                out.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    Ok(out)
}

fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for i in 0..=chunk.len() {
            out.push(T[((n >> (18 - 6 * i)) & 63) as usize] as char);
        }
        for _ in chunk.len()..3 {
            out.push('=');
        }
    }
    out
}

/// The record argument of `GetUrl`: a Record or an open RecordRef.
fn record_arg(v: Option<&Value>) -> Result<Rec, Signal> {
    match v {
        Some(Value::Record(r)) => Ok(r.clone()),
        Some(Value::Library(_, l)) => match &*l.borrow() {
            LibraryValue::RecordRef(r) => Ok(r.clone()),
            // PGetUrl.al:P109_GetUrl_Overload3_ClosedRecordRef
            _ => Err(Signal::Error(Box::new(al_builtins::native::AlError {
                message: "The record is not open.".into(),
                code: "RecordNotOpened".into(),
                call_stack: String::new(),
                ..Default::default()
            }))),
        },
        other => unsupported(format!("GetUrl with record {}", other.describe())),
    }
}

/// The company of a Phone, Tablet, Desktop, Background, Management or NAS address: letters, digits, space and `.` are
/// pinned (`My Company`, `LTP109CO` and the corpus company `CRONUS International Ltd.`, see `get_url_mobile`).
fn mobile_company(v: &Value) -> Result<(), Signal> {
    match v {
        Value::Text(t) | Value::Code(t)
            if !t.has_marker()
                && t.as_str()
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '.') =>
        {
            Ok(())
        }
        other => unsupported(format!(
            "GetUrl of this client type with company {} (only letters, digits, space and . are pinned)",
            other.describe()
        )),
    }
}

impl Session {
    /// `GetUrl(ClientType::Default, ...)`: the web address with the scheme `dynamicsnav`, then `/<company>/` and for an
    /// object `runpage?page=N` or `runreport?report=N`, for a page with a record `&bookmark=...`. Pinned for an http address
    /// by session/lt-geturl-shape/GetUrlShape.al:270 (`default alone`) and platform/lt-t-geturl/TGetUrl.al:203-210
    /// (GetUrl_ClientDefault: a company with a space, the empty company, page 21, report 69367, a page with a record and
    /// UseFilters false). Every other form is refused.
    fn get_url_default(&mut self, args: &[Value]) -> Result<Value, Signal> {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::GetUrlWebBaseAddress);
        let base = self.host.identity.server_base_url();
        let Some(rest) = base.strip_prefix("http://") else {
            return unsupported(format!(
                "GetUrl(ClientType::Default) with the server address {base}: only an http address is pinned"
            ));
        };
        let company = match args.get(1) {
            None => String::new(),
            Some(Value::Text(t)) | Some(Value::Code(t)) if !t.has_marker() => {
                let t = t.as_str();
                if !t.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ') {
                    return unsupported(
                        "GetUrl(ClientType::Default) with a company name of other characters than letters, digits and space (not pinned)",
                    );
                }
                t.replace(' ', "%20")
            }
            other => return unsupported(format!("GetUrl with company {}", other.describe())),
        };
        let mut url = format!("dynamicsnav://{}/{company}/", rest.trim_end_matches('/'));
        if args.len() > 2 {
            let id = match args.get(3) {
                Some(Value::Integer(n)) if *n > 0 => *n,
                other => {
                    return unsupported(format!(
                        "GetUrl(ClientType::Default) with object id {} (only a positive id is pinned)",
                        other.describe()
                    ));
                }
            };
            match (member(args.get(2)).as_deref(), args.len()) {
                (Some("Page"), 4) => url.push_str(&format!("runpage?page={id}")),
                (Some("Report"), 4) => url.push_str(&format!("runreport?report={id}")),
                (Some("Page"), 6) if matches!(args.get(5), Some(Value::Boolean(false))) => {
                    let rec = record_arg(args.get(4))?;
                    url.push_str(&format!("runpage?page={id}"));
                    if let Some(part) = self.url_record_part(&rec, false)? {
                        url.push('&');
                        url.push_str(&part);
                    }
                }
                (kind, n) => {
                    return unsupported(format!(
                        "GetUrl(ClientType::Default) of object type {} with {n} arguments (only a page or a report, and a page with a record and UseFilters false, are pinned)",
                        kind.unwrap_or("?")
                    ));
                }
            }
        }
        Ok(Value::Text(url.into()))
    }
    /// `GetUrl(ClientType, Company, ObjectType, ObjectId, Record|RecordRef, UseFilters, Layout)`: `args` is the call's arguments.
    pub(crate) fn get_url(&mut self, args: &[Value]) -> Result<Value, Signal> {
        match member(args.first()).as_deref() {
            // Windows gives the same text as Default for every form TGetUrl.al pins (GetUrl_ClientWindows,
            // TGetUrl.al:233-240, against GetUrl_ClientDefault, TGetUrl.al:203-210; also
            // lt-system-database-onprem/PGetUrl.al:71,75).
            Some("Default") | Some("Windows") => return self.get_url_default(args),
            Some("Phone") | Some("Tablet") | Some("Desktop") => return self.get_url_mobile(args),
            // Background, Management and NAS have no address: the empty text, alone and with a company
            // (GetUrlCtBackground.al:69-73, GetUrlCtManagement.al:69-73, GetUrlCtNAS.al:69-73, all with
            // CompanyName() or the empty company). With an object the value is not pinned (PGetUrl.al:102-103 pins
            // only that there is no error).
            Some("Background") | Some("Management") | Some("NAS") if args.len() <= 2 => {
                if let Some(company) = args.get(1) {
                    mobile_company(company)?;
                }
                return Ok(Value::Text("".into()));
            }
            Some("Web") => {}
            Some(kind @ ("SOAP" | "OData" | "ODataV4" | "Api")) => {
                let kind = kind.to_string();
                return self.get_url_service(&kind, args);
            }
            _ => {
                return unsupported(format!(
                    "GetUrl({}; the call had {} arguments): only ClientType::Web, Default, Windows, Phone, Tablet, Desktop, SOAP, OData, ODataV4 and Api are built, and Background, Management and NAS without an object (the other forms are not pinned)",
                    args.first().describe(),
                    args.len()
                ));
            }
        }
        self.get_url_web(args)
    }

    /// `GetUrl(ClientType::SOAP | OData | ODataV4 | Api, ...)`: the web address (its host and instance, masked by the pins,
    /// come from the host's server address: assumption `geturl-web-base-address`), then `/WS/Services`, `/ODataV4` or
    /// `/api/`; with a company `/WS/<company>/Services` or `/ODataV4/Company('<company>')`, the empty company as none;
    /// with an object or a record the empty text. Pinned by platform/lt-t-geturl/TGetUrl.al:481-559 on corpus master
    /// (TV_GetUrl_ClientSOAP, _ClientOData, _ClientODataV4, _ClientApi): the company `My Company`, page 21, report 69367,
    /// a page with a record and UseFilters false, a codeunit (SOAP, ODataV4) and a query (ODataV4). Refused: Api with a
    /// company (the id in `companies(...)` is not pinned), other company characters (`.` only with an object) and every other
    /// form.
    fn get_url_service(&mut self, kind: &str, args: &[Value]) -> Result<Value, Signal> {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::GetUrlWebBaseAddress);
        let refuse = |why: &str| {
            unsupported(format!(
                "GetUrl(ClientType::{kind}; the call had {} arguments): {why} (not pinned)",
                args.len()
            ))
        };
        let company = match args.get(1) {
            None => String::new(),
            // `.` too with an object, whose answer is the empty text whatever the company: the corpus company
            // `CRONUS International Ltd.` is accepted for these client types (GetUrlCtService.al, CompanyName()).
            Some(Value::Text(t)) | Some(Value::Code(t))
                if !t.has_marker()
                    && t.as_str()
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == ' ' || (c == '.' && args.len() > 2)) =>
            {
                t.as_str().to_string()
            }
            Some(_) => return refuse("a company of other characters than letters, digits and space"),
        };
        if args.len() > 2 {
            let object = member(args.get(2));
            if !matches!(args.get(3), Some(Value::Integer(n)) if *n > 0) {
                return refuse("an object id that is not positive");
            }
            let pinned = match (object.as_deref(), args.len()) {
                (Some("Page") | Some("Report"), 4) => true,
                (Some("Codeunit"), 4) => kind == "SOAP" || kind == "ODataV4",
                (Some("Query"), 4) => kind == "ODataV4",
                (Some("Page"), 6) => {
                    record_arg(args.get(4))?;
                    matches!(args.get(5), Some(Value::Boolean(false)))
                }
                _ => false,
            };
            if !pinned {
                return refuse("this object type or argument list");
            }
            return Ok(Value::Text("".into()));
        }
        let base = self.host.identity.server_base_url();
        if !base.starts_with("http://") {
            return refuse("a server address other than http");
        }
        // The service address is not the web address: TGetUrl.al:95-102 masks the web address first, and the service
        // texts still show http://<host>/<instance>/... (TGetUrl.al:487, 504, 521, 538). Its host part comes from the web
        // address with BC's default service port, 7047 for SOAP and 7048 for OData, ODataV4 and Api (assumption
        // `geturl-web-base-address`).
        let base = with_port(base.trim_end_matches('/'), if kind == "SOAP" { 7047 } else { 7048 });
        let company = company.replace(' ', "%20");
        let url = match (kind, company.is_empty()) {
            ("SOAP", true) => format!("{base}/WS/Services"),
            ("SOAP", false) => format!("{base}/WS/{company}/Services"),
            ("Api", true) => format!("{base}/api/"),
            ("Api", false) => return refuse("a company (the id in companies(...) is not pinned)"),
            (_, true) => format!("{base}/ODataV4"),
            (_, false) => format!("{base}/ODataV4/Company('{company}')"),
        };
        Ok(Value::Text(url.into()))
    }

    /// `GetUrl(ClientType::Phone | Tablet | Desktop, ...)`: the scheme `ms-dynamicsnav`, the web address after `http://`
    /// without a trailing slash, then the query of the web form directly after it (no `/` before the `?`). Pinned by
    /// platform/lt-t-geturl/TGetUrl.al:248-255, 263-270, 278-285 (alone, a company, the empty company, page 21, report
    /// 69367, a page with a record and UseFilters false), lt-system-database-members/GetUrlCtMain.al:111-115, 153-157,
    /// 195-199 (CompanyName()) and lt-system-database-onprem/PGetUrl.al:68-73 (a company and page 99999). Every other form
    /// is refused.
    fn get_url_mobile(&mut self, args: &[Value]) -> Result<Value, Signal> {
        if let Some(company) = args.get(1) {
            mobile_company(company)?;
        }
        let pinned_form = match args.len() {
            1 | 2 => true,
            4 => {
                matches!(member(args.get(2)).as_deref(), Some("Page") | Some("Report"))
                    && matches!(args.get(3), Some(Value::Integer(n)) if *n > 0)
            }
            6 => {
                member(args.get(2)).as_deref() == Some("Page")
                    && matches!(args.get(3), Some(Value::Integer(n)) if *n > 0)
                    && matches!(args.get(5), Some(Value::Boolean(false)))
            }
            _ => false,
        };
        if !pinned_form {
            return unsupported(format!(
                "GetUrl({}; the call had {} arguments): for Phone, Tablet and Desktop only a company, a page or report with a positive id, and a page with a record and UseFilters false are pinned",
                args.first().describe(),
                args.len()
            ));
        }
        let base = self.host.identity.server_base_url();
        let Some(rest) = base.strip_prefix("http://") else {
            return unsupported(format!(
                "GetUrl(ClientType::Phone, Tablet or Desktop) with the server address {base}: only an http address is pinned"
            ));
        };
        let rest = rest.trim_end_matches('/').to_string();
        let web = match self.get_url_web(args)? {
            Value::Text(t) => t.as_str().to_string(),
            other => return unsupported(format!("GetUrl result {}", other.describe())),
        };
        let query = match web.find("/?") {
            Some(at) => web[at + 1..].to_string(),
            None => String::new(),
        };
        Ok(Value::Text(format!("ms-dynamicsnav://{rest}{query}").into()))
    }

    /// The `ClientType::Web` form.
    fn get_url_web(&mut self, args: &[Value]) -> Result<Value, Signal> {
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::GetUrlWebBaseAddress);
        let base = self.host.identity.server_base_url();
        let company = match args.get(1) {
            None => String::new(),
            Some(Value::Text(t)) | Some(Value::Code(t)) if !t.has_marker() => t.as_str().to_string(),
            other => return unsupported(format!("GetUrl with company {}", other.describe())),
        };
        let mut query: Vec<String> = Vec::new();
        if !company.is_empty() {
            query.push(format!("company={}", encode(&company)?));
        }
        if args.len() > 2 {
            let kind = member(args.get(2));
            let id = match args.get(3) {
                Some(Value::Integer(n)) => *n,
                other => return unsupported(format!("GetUrl with object id {}", other.describe())),
            };
            let key = match kind.as_deref() {
                Some("Page") => "page",
                Some("Report") => "report",
                Some("Table") => "table",
                Some("Query") => "query",
                // codeunits and XMLports have no web client address (GetUrlObjects.al:64,67); a codeunit with a record
                // and UseFilters false neither (TGetUrl.al:385,388,397).
                Some("Codeunit") | Some("XMLport") | Some("XmlPort") if args.len() == 4 => {
                    return Ok(Value::Text("".into()));
                }
                Some("Codeunit") if args.len() == 6 && matches!(args.get(5), Some(Value::Boolean(false))) => {
                    record_arg(args.get(4))?;
                    return Ok(Value::Text("".into()));
                }
                other => {
                    return unsupported(format!(
                        "GetUrl of object type {} with a record (only Page and Table are pinned; Report and Query pin only \"no query part\")",
                        other.unwrap_or("?")
                    ));
                }
            };
            // An id of zero or less has no address (GetUrlObjects.al:84-85,89-90).
            if id <= 0 && args.len() == 4 {
                return Ok(Value::Text("".into()));
            }
            if args.len() > 4 {
                // A record of another object kind than a page: no address (GetUrlShape.al:152-163 `table_record`;
                // the report and query forms are pinned only to have no '?', GetUrlBookmarkFilter.al:`report`, `query`).
                let rec = record_arg(args.get(4))?;
                if key == "report" || key == "query" {
                    // A report or a query with a record has no address: the empty text (TGetUrl.al:383-384,386-387,
                    // 389-394 GetUrl_Web_ReportQueryCodeunitWithRecord: a report with UseFilters false and true, with
                    // and without filters on the record; a query with UseFilters false). Other forms are not pinned.
                    let pinned = matches!(
                        (key, args.len(), args.get(5)),
                        ("report", 6, Some(Value::Boolean(_))) | ("query", 6, Some(Value::Boolean(false)))
                    );
                    if !pinned {
                        return unsupported(format!(
                            "GetUrl of a {key} with a record: only \"no query part\" is pinned, not the value"
                        ));
                    }
                    return Ok(Value::Text("".into()));
                }
                if key != "page" {
                    return Ok(Value::Text("".into()));
                }
                if id <= 0 {
                    return unsupported("GetUrl of page 0 or a negative page with a record (not pinned)");
                }
                let use_filters = match args.get(5) {
                    None => false,
                    Some(Value::Boolean(b)) => *b,
                    other => return unsupported(format!("GetUrl with UseFilters {}", other.describe())),
                };
                // The layout name changes nothing, for a name that does not exist and for the empty text
                // (GetUrlRecordLayout.al:GetUrl_Record_Layout).
                match args.get(6) {
                    None | Some(Value::Text(_)) => {}
                    other => return unsupported(format!("GetUrl with layout {}", other.describe())),
                }
                query.push(format!("page={id}"));
                if let Some(part) = self.url_record_part(&rec, use_filters)? {
                    query.push(part);
                }
            } else {
                if id <= 0 {
                    return Ok(Value::Text("".into()));
                }
                query.push(format!("{key}={id}"));
            }
        }
        if query.is_empty() {
            return Ok(Value::Text(base.into()));
        }
        Ok(Value::Text(
            format!("{}/?{}", base.trim_end_matches('/'), query.join("&")).into(),
        ))
    }

    /// The `bookmark=` or `filter=` part for a record: the filters when `use_filters` and there are some, nothing when
    /// `use_filters` and there are none, else the bookmark of the key (GetUrlBookmarkFilter.al:GetUrl_Filter_Text,
    /// GetUrl_Bookmark_TwoFieldKey `usefilters_true_not_inserted`).
    fn url_record_part(&mut self, rec: &Rec, use_filters: bool) -> Result<Option<String>, Signal> {
        let t = self.table(rec)?;
        {
            let r = rec.borrow();
            if r.temporary {
                return unsupported("GetUrl with a temporary record (not pinned)");
            }
            if r.marked_only || !r.marks.is_empty() && use_filters {
                return unsupported("GetUrl with marked records (not pinned)");
            }
        }
        if use_filters {
            let (groups_ok, mut fields) = {
                let r = rec.borrow();
                (
                    r.filter_group == 0 && r.filters.iter().all(|f| f.group == 0),
                    r.filters.iter().map(|f| f.field).collect::<Vec<_>>(),
                )
            };
            if !groups_ok {
                return unsupported("GetUrl with filters in a filter group other than 0 (not pinned)");
            }
            if fields.is_empty() {
                return Ok(None);
            }
            fields.sort_unstable();
            fields.dedup();
            let mut parts = Vec::new();
            for pos in fields {
                let f = &t.fields[pos];
                if !f.stored || !matches!(f.ty, RtType::Code(_) | RtType::Text(_) | RtType::Integer) {
                    return unsupported(format!(
                        "GetUrl filter on field {} (only Code, Text and Integer fields of the table are pinned)",
                        f.name
                    ));
                }
                if self.field_caption(&t, pos) != f.name {
                    return unsupported("GetUrl filter on a field whose caption differs from its name (not pinned)");
                }
                let text = self.field_filter_text(rec, pos)?;
                parts.push(format!("'{}' IS '{}'", f.name, text.replace('\'', "''")));
            }
            return Ok(Some(format!("filter={}", encode(&parts.join(" AND "))?)));
        }
        // The bookmark: the table number, then the primary key fields. Only a key of one Code[10], Code[20] or Code[50]
        // field, one Integer field, a Code[20] and an Integer field (in that order), or two Code[20] fields is pinned
        // (TGetUrl.al:439-447 GetUrl_Web_BookmarkOfCodeKeys for Code[10], Code[50] and two Code[20]).
        let table_id = match self.engine.program.objects[t.object as usize].id {
            Some(id) => id,
            None => return unsupported("GetUrl bookmark of a table without an id"),
        };
        let r = rec.borrow();
        let mut data: Vec<u8> = table_id.to_le_bytes().to_vec();
        let mut size = 6usize;
        let key = &t.keys[0];
        let shape: Vec<&RtType> = key.iter().map(|&p| &t.fields[p].ty).collect();
        let ok = matches!(
            shape.as_slice(),
            [RtType::Code(Some(10 | 20 | 50))]
                | [RtType::Integer]
                | [RtType::Code(Some(20)), RtType::Integer]
                | [RtType::Code(Some(20)), RtType::Code(Some(20))]
        );
        if !ok {
            return unsupported(
                "GetUrl bookmark of a primary key other than Code[10], Code[20], Code[50], Integer, Code[20] and Integer, or two Code[20] (not pinned)",
            );
        }
        let two_codes = key.len() == 2 && matches!(shape[1], RtType::Code(_));
        for &p in key.iter() {
            match &r.fields[p] {
                Value::Code(c) => {
                    let s = c.as_str();
                    if !s.is_ascii() {
                        return unsupported("GetUrl bookmark of a Code value with a non-ASCII character (not pinned)");
                    }
                    if s.is_empty() && key.len() > 1 && !two_codes {
                        return unsupported("GetUrl bookmark of an empty Code in a two-field key (not pinned)");
                    }
                    // A non-empty Code is marked 0xff, an empty one 0x00 (TGetUrl.al:446-447 `two second empty`,
                    // `two first empty`).
                    data.extend_from_slice(&[0x02, 0x7b, if s.is_empty() { 0x00 } else { 0xff }]);
                    for b in s.bytes() {
                        data.extend_from_slice(&[b, 0]);
                    }
                    data.extend_from_slice(&[0, 0]);
                    size += 5 + 2 * s.len();
                }
                Value::Integer(n) => {
                    data.extend_from_slice(&[0x00, 0x87]);
                    data.extend_from_slice(&n.to_le_bytes());
                    size += 6;
                }
                other => return unsupported(format!("GetUrl bookmark of a key value {}", other.describe())),
            }
        }
        while data.last() == Some(&0) {
            data.pop();
        }
        let b64 = base64(&data)
            .replace('+', "%2b")
            .replace('/', "%2f")
            .replace('=', "%3d");
        Ok(Some(format!("bookmark={size}%3b{b64}")))
    }
}
