//! Parsing XML text into a node tree.

use super::*;

// ------------------------------------------------------------ parsing

#[derive(Debug)]
pub(super) enum ParseError {
    Invalid,
    /// Not well-formed, with the text and position of the .NET exception BC reports
    /// (xml/TestXmlRuntimeContracts.al:XmlDocument_ReadFrom_Invalid_ReturnsFalse_AndStatementFormThrows).
    Failed {
        message: String,
        line: usize,
        pos: usize,
        /// Whether `XmlDocument.ReadFrom` is known to word its error with this message; an XmlPort
        /// import words its errors with the .NET messages in every case.
        dom: bool,
    },
    Unsupported(&'static str),
}

pub(super) struct Parser<'a> {
    s: &'a str,
    i: usize,
    /// The names of the open elements, outermost first.
    open: Vec<String>,
    /// In-scope namespace declarations, innermost last.
    scope: Vec<(String, String)>,
    depth: usize,
    /// Parsing for `XmlDocument.ReadFrom`, whose errors are worded differently from an XmlPort import's.
    dom: bool,
    /// Where the root element starts.
    root_at: usize,
}

pub(super) type PResult<T> = Result<T, ParseError>;

impl Parser<'_> {
    fn rest(&self) -> &str {
        &self.s[self.i..]
    }
    /// A well-formedness error of `XmlDocument.ReadFrom`, worded as .NET words it, at byte offset `at`
    /// (xml/TestProbeXmlContracts.al:ReadFrom_Invalid_Statement).
    fn fail_at<T>(&self, what: &str, at: usize) -> PResult<T> {
        let (line, pos) = self.line_pos(at);
        Err(ParseError::Failed {
            message: format!("{what} Line {line}, position {pos}."),
            line,
            pos,
            dom: true,
        })
    }
    /// Line and position (both 1-based, in UTF-16 units) of byte offset `at`.
    fn line_pos(&self, at: usize) -> (usize, usize) {
        let before = &self.s[..at];
        let line = before.matches('\n').count() + 1;
        let start = before.rfind('\n').map_or(0, |n| n + 1);
        let pos = before[start..].encode_utf16().count() + 1;
        (line, pos)
    }
    fn starts(&self, p: &str) -> bool {
        self.rest().starts_with(p)
    }
    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }
    fn bump(&mut self, n: usize) {
        self.i += n;
    }
    fn skip_ws(&mut self) -> bool {
        let before = self.i;
        while self.peek().is_some_and(|c| matches!(c, ' ' | '\t' | '\n' | '\r')) {
            self.i += 1;
        }
        self.i > before
    }
    fn expect(&mut self, p: &str) -> PResult<()> {
        if self.starts(p) {
            self.bump(p.len());
            Ok(())
        } else {
            Err(ParseError::Invalid)
        }
    }

    fn name(&mut self) -> PResult<String> {
        let start = self.i;
        let mut first = true;
        while let Some(c) = self.peek() {
            let ok = if first {
                is_name_start(c) || c == ':'
            } else {
                is_name_char(c) || c == ':'
            };
            if !ok {
                break;
            }
            first = false;
            self.i += c.len_utf8();
        }
        if self.i == start {
            match self.peek() {
                // .NET names the character that cannot start a name and where it stands
                // (xml/XmlDocReadWrite.al:ReadFrom_Invalid, badname and lt_in_text).
                Some(c) => self.fail_at(
                    &format!(
                        "Name cannot begin with the '{c}' character, hexadecimal value 0x{:02X}.",
                        u32::from(c)
                    ),
                    start,
                ),
                None => Err(ParseError::Invalid),
            }
        } else {
            Ok(self.s[start..self.i].to_string())
        }
    }

    /// Decodes entity references in `raw`; `attr` also normalises white space.
    /// The text of `raw` with its references replaced; `base` is the byte offset of `raw` in the document, for the
    /// position of an undeclared entity.
    fn decode(&self, raw: &str, attr: bool, base: usize) -> PResult<String> {
        let mut out = String::with_capacity(raw.len());
        let mut i = 0usize;
        while let Some(c) = raw[i..].chars().next() {
            let here = i;
            i += c.len_utf8();
            match c {
                '&' => i = self.reference(raw, i, base, &mut out)?,
                '<' if attr => return Err(ParseError::Invalid),
                '\r' => {
                    if raw[i..].starts_with('\n') {
                        i += 1;
                    }
                    out.push(if attr { ' ' } else { '\n' });
                }
                '\n' | '\t' if attr => out.push(' '),
                c => out.push(c),
            }
            let _ = here;
        }
        Ok(out)
    }

    /// The reference whose `&` precedes byte `i` of `raw`; returns the offset after its `;`. The error texts and places
    /// are .NET's (xml/XmlEntitiesMisc.al:Entities_CharRefs).
    fn reference(&self, raw: &str, i: usize, base: usize, out: &mut String) -> PResult<usize> {
        let unexpected = |at: usize| match self.s[base + at..].chars().next() {
            Some(x) => self.fail_at(
                &format!("'{x}' is an unexpected token. The expected token is ';'."),
                base + at,
            ),
            None => Err(ParseError::Invalid),
        };
        if raw[i..].starts_with('#') {
            let hex = raw[i + 1..].starts_with('x');
            let d = i + 1 + usize::from(hex);
            let digits: String = raw[d..]
                .chars()
                .take_while(|x| if hex { x.is_ascii_hexdigit() } else { x.is_ascii_digit() })
                .collect();
            if digits.is_empty() {
                let kind = if hex { "hexadecimal" } else { "decimal" };
                return self.fail_at(
                    &format!("Invalid syntax for a {kind} numeric entity reference."),
                    base + d,
                );
            }
            let end = d + digits.len();
            if !raw[end..].starts_with(';') {
                return unexpected(end);
            }
            let v = u32::from_str_radix(&digits, if hex { 16 } else { 10 }).unwrap_or(u32::MAX);
            let valid = matches!(v, 0x9 | 0xA | 0xD | 0x20..=0xD7FF | 0xE000..=0xFFFD | 0x10000..=0x10FFFF);
            // An XmlPort import does not check characters (xmlport/lt-xmlport-query-datatransfer/XpTEncoding.al:
            // Encoding_ImportControlCharacterEntity: `&#1;` reads).
            if !valid && self.dom {
                // A value past the last code point is reported as the high surrogate .NET computes for it.
                let shown = if v > 0x10FFFF && v < 0x10000 + (0x10_0000 << 1) {
                    0xD800 + ((v - 0x10000) >> 10)
                } else {
                    v
                };
                let text = if shown == 0 {
                    ".".to_string()
                } else if shown < 0x20 || (0xD800..=0xDFFF).contains(&shown) || shown > 0xFFFF || shown >= 0xFFFE {
                    format!("\\u{shown:04X}")
                } else {
                    char::from_u32(shown).map(String::from).unwrap_or_default()
                };
                return self.fail_at(
                    &format!("'{text}', hexadecimal value 0x{shown:02X}, is an invalid character."),
                    base + d,
                );
            }
            out.push(char::from_u32(v).unwrap_or('\u{fffd}'));
            return Ok(end + 1);
        }
        let name: String = raw[i..]
            .chars()
            .enumerate()
            .take_while(|(n, x)| if *n == 0 { is_name_start(*x) } else { is_name_char(*x) })
            .map(|(_, x)| x)
            .collect();
        if name.is_empty() {
            return self.fail_at("An error occurred while parsing EntityName.", base + i);
        }
        let end = i + name.len();
        if !raw[end..].starts_with(';') {
            return unexpected(end);
        }
        match name.as_str() {
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "amp" => out.push('&'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            e => return self.fail_at(&format!("Reference to undeclared entity '{e}'."), base + i),
        }
        Ok(end + 1)
    }

    fn quoted(&mut self) -> PResult<String> {
        let q = self.peek().ok_or(ParseError::Invalid)?;
        if q != '"' && q != '\'' {
            return self.fail_at(
                &format!("'{q}' is an unexpected token. The expected token is '\"' or '''."),
                self.i,
            );
        }
        self.bump(1);
        let end = self.rest().find(q).ok_or(ParseError::Invalid)?;
        let raw = self.s[self.i..self.i + end].to_string();
        let base = self.i;
        self.bump(end + 1);
        self.decode(&raw, true, base)
    }

    fn declaration(&mut self) -> PResult<Handle> {
        self.expect("<?xml")?;
        let (mut version, mut encoding, mut standalone) = (String::new(), String::new(), String::new());
        loop {
            let had_ws = self.skip_ws();
            if self.starts("?>") {
                self.bump(2);
                break;
            }
            if !had_ws {
                return Err(ParseError::Invalid);
            }
            let n = self.name()?;
            self.skip_ws();
            self.expect("=")?;
            self.skip_ws();
            let value_at = self.i + 1;
            let v = self.quoted()?;
            match n.as_str() {
                "version" => {
                    // xml/XmlDocReadWrite.al:ReadFrom_Invalid_Leading (bad_version): the place is the value's first character.
                    if v != "1.0" {
                        return self.fail_at(&format!("Version number '{v}' is invalid."), value_at);
                    }
                    version = v;
                }
                "encoding" => encoding = v,
                "standalone" => standalone = v,
                _ => return Err(ParseError::Invalid),
            }
        }
        // Only version 1.0 reads (xml/TestProbeXmlContracts.al:ReadFrom_Invalid_ResultUsed: 9.9 is refused).
        if version != "1.0" {
            return Err(ParseError::Invalid);
        }
        Ok(new_node(XData::Declaration {
            version,
            encoding,
            standalone,
            explicit: false,
        }))
    }

    fn comment(&mut self) -> PResult<Handle> {
        self.expect("<!--")?;
        let Some(end) = self.rest().find("-->") else {
            return self.fail_at(
                "Unexpected end of file while parsing Comment has occurred.",
                self.s.len(),
            );
        };
        let body = self.s[self.i..self.i + end].to_string();
        if let Some(n) = body.find("--") {
            // xml/XmlEntitiesMisc.al:Comments_And_PI_Roundtrip (bad_comment): the place is the first of the two hyphens.
            return self.fail_at(
                "An XML comment cannot contain '--', and '-' cannot be the last character.",
                self.i + n,
            );
        }
        self.bump(end + 3);
        Ok(new_node(XData::Comment(body)))
    }

    fn pi(&mut self) -> PResult<Handle> {
        self.expect("<?")?;
        let target_at = self.i;
        let target = self.name()?;
        if target == "xml" {
            // xml/XmlDocReadWrite.al:ReadFrom_Invalid_Leading (leading_space_decl): the place is the target's.
            return self.fail_at(
                "Unexpected XML declaration. The XML declaration must be the first node in the document, and no whitespace characters are allowed to appear before it.",
                target_at,
            );
        }
        if target.eq_ignore_ascii_case("xml") {
            return Err(ParseError::Invalid);
        }
        let had_ws = self.skip_ws();
        let end = self.rest().find("?>").ok_or(ParseError::Invalid)?;
        let data = self.s[self.i..self.i + end].to_string();
        if !had_ws && !data.is_empty() {
            return Err(ParseError::Invalid);
        }
        self.bump(end + 2);
        Ok(new_node(XData::Pi { target, data }))
    }

    fn resolve(&self, prefix: &str, is_attr: bool, at: usize) -> PResult<String> {
        let declared = |p: &str| self.scope.iter().rev().find(|(sp, _)| sp == p).map(|(_, u)| u.clone());
        match prefix {
            // An unprefixed attribute is in no namespace; an unprefixed element is in the default one.
            "" if is_attr => Ok(String::new()),
            "" => Ok(declared("").unwrap_or_default()),
            "xml" => Ok(XML_URI.to_string()),
            "xmlns" => Ok(XMLNS_URI.to_string()),
            p => match declared(p) {
                Some(u) => Ok(u),
                None => self.fail_at(&format!("'{p}' is an undeclared prefix."), at),
            },
        }
    }

    fn element(&mut self) -> PResult<Handle> {
        self.depth += 1;
        if self.depth > 200 {
            return Err(ParseError::Unsupported("XML nested deeper than 200 levels"));
        }
        self.expect("<")?;
        let name_at = self.i;
        let qn = self.name()?;
        self.open.push(qn.clone());
        let mut raw_attrs: Vec<(String, String, usize)> = Vec::new();
        let self_closed = loop {
            let had_ws = self.skip_ws();
            if self.starts("/>") {
                self.bump(2);
                break true;
            }
            if self.starts(">") {
                self.bump(1);
                break false;
            }
            if !had_ws {
                return Err(ParseError::Invalid);
            }
            let an_at = self.i;
            let an = self.name()?;
            self.skip_ws();
            self.expect("=")?;
            self.skip_ws();
            let av = self.quoted()?;
            if raw_attrs.iter().any(|(n, _, _)| *n == an) {
                return self.fail_at(&format!("'{an}' is a duplicate attribute name."), an_at);
            }
            raw_attrs.push((an, av, an_at));
        };
        // Namespace declarations first, so the element's own name sees them.
        let scope_mark = self.scope.len();
        for (n, v, _) in &raw_attrs {
            if n == "xmlns" {
                self.scope.push((String::new(), v.clone()));
            } else if let Some(p) = n.strip_prefix("xmlns:") {
                self.scope.push((p.to_string(), v.clone()));
            }
        }
        let split = |n: &str| match n.split_once(':') {
            Some((p, l)) => (p.to_string(), l.to_string()),
            None => (String::new(), n.to_string()),
        };
        let (prefix, local) = split(&qn);
        if local.contains(':') || !is_ncname(&local) || (!prefix.is_empty() && !is_ncname(&prefix)) {
            return Err(ParseError::Invalid);
        }
        let ns = self.resolve(&prefix, false, name_at)?;
        let mut attrs = Vec::new();
        let mut seen: Vec<(String, String)> = Vec::new();
        for (n, v, n_at) in raw_attrs {
            let (ap, al, ans) = if n == "xmlns" {
                (String::new(), "xmlns".to_string(), XMLNS_URI.to_string())
            } else if let Some(p) = n.strip_prefix("xmlns:") {
                ("xmlns".to_string(), p.to_string(), XMLNS_URI.to_string())
            } else {
                let (p, l) = split(&n);
                if l.contains(':') || !is_ncname(&l) || (!p.is_empty() && !is_ncname(&p)) {
                    return Err(ParseError::Invalid);
                }
                let u = self.resolve(&p, true, n_at)?;
                (p, l, u)
            };
            if seen.contains(&(ans.clone(), al.clone())) {
                return Err(ParseError::Invalid);
            }
            seen.push((ans.clone(), al.clone()));
            attrs.push(new_node(XData::Attribute {
                prefix: ap,
                local: al,
                ns: ans,
                value: v,
            }));
        }
        let mut children = Vec::new();
        if !self_closed {
            loop {
                if self.starts("</") {
                    self.bump(2);
                    let close_at = self.i;
                    let close = self.name()?;
                    self.skip_ws();
                    self.expect(">")?;
                    if close != qn {
                        let (sl, sp) = self.line_pos(name_at);
                        let (line, pos) = self.line_pos(close_at);
                        return Err(ParseError::Failed {
                            message: format!(
                                "The '{qn}' start tag on line {sl} position {sp} does not match the end tag of '{close}'. Line {line}, position {pos}."
                            ),
                            line,
                            pos,
                            dom: true,
                        });
                    }
                    break;
                } else if self.starts("<!--") {
                    children.push(self.comment()?);
                } else if self.starts("<![CDATA[") {
                    self.bump(9);
                    let end = self.rest().find("]]>").ok_or(ParseError::Invalid)?;
                    let body = self.s[self.i..self.i + end].replace("\r\n", "\n").replace('\r', "\n");
                    self.bump(end + 3);
                    children.push(new_node(XData::CData(body)));
                } else if self.starts("<?") {
                    children.push(self.pi()?);
                } else if self.starts("<!") {
                    return Err(ParseError::Invalid);
                } else if self.starts("<") {
                    children.push(self.element()?);
                } else if self.peek().is_none() && self.dom {
                    // xml/TestProbeXmlContracts.al:ReadFrom_Invalid_Statement (`<a>`): worded as an error at the root.
                    return self.fail_at("Data at the root level is invalid.", self.root_at);
                } else if self.peek().is_none() {
                    // Pinned for an XmlPort import (xmlport/TestXmlPortExportImportContracts.al:
                    // XmlPortImport_MalformedXml_Throws).
                    let (line, pos) = self.line_pos(self.s.len());
                    let names: Vec<&str> = self.open.iter().rev().map(String::as_str).collect();
                    return Err(ParseError::Failed {
                        message: format!(
                            "Unexpected end of file has occurred. The following elements are not closed: {}. Line {line}, position {pos}.",
                            names.join(", ")
                        ),
                        line,
                        pos,
                        dom: false,
                    });
                } else {
                    let end = self.rest().find('<').unwrap_or(self.rest().len());
                    let raw = self.s[self.i..self.i + end].to_string();
                    self.bump(end);
                    if raw.contains("]]>") {
                        return Err(ParseError::Invalid);
                    }
                    // White space is kept, inside and outside the root
                    // (xml/TestXmlRuntimeContracts.al:XmlDocument_ReadFrom_KeepsWhiteSpaceInsideAndOutsideTheRoot).
                    let text = self.decode(&raw, false, self.i - end)?;
                    children.push(new_node(XData::Text(text)));
                }
            }
        }
        self.scope.truncate(scope_mark);
        self.open.pop();
        self.depth -= 1;
        let open = !self_closed && children.is_empty();
        Ok(new_node(XData::Element {
            prefix,
            local,
            ns,
            attrs,
            children,
            open,
        }))
    }

    /// `<!DOCTYPE name ... [ internal subset ] >`.
    fn skip_doctype(&mut self) -> PResult<()> {
        // `<!DOCTYPE name`, then white space and an external id, a `[` or a `>`: the errors are .NET's, with their places
        // (xml/lt-xml-doctype-declaration/DocTypeRead.al:Read_Malformed).
        let start = self.i;
        let after_keyword = start + "<!DOCTYPE".len();
        let mut at = after_keyword;
        let bytes = self.s.as_bytes();
        while at < bytes.len() && matches!(bytes[at], b' ' | b'\t' | b'\n' | b'\r') {
            at += 1;
        }
        if let Some(c) = self.s[at..].chars().next() {
            if !is_name_start(c) {
                return self.fail_at(
                    &format!(
                        "Name cannot begin with the '{c}' character, hexadecimal value 0x{:02X}.",
                        u32::from(c)
                    ),
                    at,
                );
            }
            let mut end = at;
            for (n, ch) in self.s[at..].char_indices() {
                if !(is_name_start(ch) || ch.is_alphanumeric() || matches!(ch, '-' | '.')) {
                    break;
                }
                end = at + n + ch.len_utf8();
            }
            let mut next = end;
            while next < bytes.len() && matches!(bytes[next], b' ' | b'\t' | b'\n' | b'\r') {
                next += 1;
            }
            let c = self.s[next..].chars().next();
            let named_external = self.s[next..].starts_with("SYSTEM") || self.s[next..].starts_with("PUBLIC");
            if !(matches!(c, Some('>' | '[')) || named_external) {
                return self.fail_at("Expecting external ID, '[' or '>'.", next);
            }
        }
        let mut depth = 0i32;
        let mut quote: Option<char> = None;
        let rest = self.rest().to_string();
        for (n, c) in rest.char_indices() {
            match (quote, c) {
                (Some(q), c) if c == q => quote = None,
                (Some(_), _) => {}
                (None, '"' | '\'') => quote = Some(c),
                (None, '[') => depth += 1,
                (None, ']') => depth -= 1,
                (None, '>') if depth <= 0 => {
                    self.bump(n + 1);
                    return Ok(());
                }
                _ => {}
            }
        }
        Err(ParseError::Invalid)
    }

    fn document(&mut self) -> PResult<Handle> {
        let mut decl = None;
        if self.starts("<?xml") && self.rest()[5..].starts_with([' ', '\t', '\r', '\n']) {
            decl = Some(self.declaration()?);
        }
        let mut children = Vec::new();
        let mut root_seen = false;
        loop {
            let ws_start = self.i;
            self.skip_ws();
            if self.i > ws_start {
                let ws = self.s[ws_start..self.i].replace("\r\n", "\n").replace('\r', "\n");
                children.push(new_node(XData::Text(ws)));
            }
            if self.peek().is_none() {
                break;
            }
            if self.starts("<!--") {
                children.push(self.comment()?);
            } else if self.starts("<?") {
                children.push(self.pi()?);
            } else if self.starts("<!")
                && self.rest()[2..].len() >= 7
                && self.rest()[2..9].eq_ignore_ascii_case("DOCTYPE")
                && !self.starts("<!DOCTYPE")
            {
                return self.fail_at(
                    &format!(
                        "'{}' is an unexpected token. The expected token is 'DOCTYPE'.",
                        &self.rest()[2..9]
                    ),
                    self.i + 2,
                );
            } else if self.starts("<!DOCTYPE") && root_seen {
                return self.fail_at("DTD must be defined before the document root element.", self.i);
            } else if self.starts("<!DOCTYPE") {
                // A document type is read and dropped: GetDocumentType answers false and WriteTo writes none
                // (xml/TestProbeXmlContracts.al:DocumentType_ReadWithDoctype).
                self.skip_doctype()?;
            } else if self.starts("<") && !root_seen {
                match self.rest()[1..].chars().next() {
                    Some(c) if !is_name_start(c) => {
                        return self.fail_at(
                            &format!(
                                "Name cannot begin with the '{c}' character, hexadecimal value 0x{:02X}.",
                                u32::from(c)
                            ),
                            self.i + 1,
                        );
                    }
                    None => return self.fail_at("Data at the root level is invalid.", self.i),
                    Some(_) => {}
                }
                self.root_at = self.i;
                children.push(self.element()?);
                root_seen = true;
            } else if !self.starts("<") && root_seen && self.peek() == Some('\0') {
                // The zero byte `OutStream.Write(Text)` adds after the text, read back after the root element: .NET names the
                // character `.` (xml/lt-de-xml-leads/DeXmlWriteLeads.al:85-89, positions 155 and 32).
                return self.fail_at("'.', hexadecimal value 0x00, is an invalid character.", self.i);
            } else if !self.starts("<") {
                return self.fail_at("Data at the root level is invalid.", self.i);
            } else {
                return self.fail_at("There are multiple root elements.", self.i + 1);
            }
        }
        if !root_seen {
            return Err(ParseError::Failed {
                message: "Root element is missing.".into(),
                line: 0,
                pos: 0,
                dom: true,
            });
        }
        Ok(new_node(XData::Document { decl, children }))
    }
}

/// A .NET XML exception: its message and the position it names.
pub(super) struct Failure {
    pub(super) message: String,
    pub(super) line: usize,
    pub(super) pos: usize,
    pub(super) dom: bool,
}

/// `Ok(Err(detail))`: not well-formed XML; `detail` is BC's exception text with its position
/// where the runtime knows it.
pub(super) fn parse_document_detail(text: &str, dom: bool) -> Result<Result<Handle, Option<Failure>>, Signal> {
    let mut p = Parser {
        s: text,
        i: 0,
        open: Vec::new(),
        scope: Vec::new(),
        depth: 0,
        dom,
        root_at: 0,
    };
    match p.document() {
        Ok(d) => Ok(Ok(d)),
        Err(ParseError::Invalid) => Ok(Err(None)),
        Err(ParseError::Failed {
            message,
            line,
            pos,
            dom,
        }) => Ok(Err(Some(Failure {
            message,
            line,
            pos,
            dom,
        }))),
        Err(ParseError::Unsupported(w)) => unsupported(w),
    }
}
