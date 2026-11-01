//! JsonObject, JsonArray, JsonToken and JsonValue.
//!
//! All four AL types are handles onto the same node tree (corpus
//! json/TestJsonToken.al: `JTok := JObj.AsToken(); JAsObj := JTok.AsObject()`
//! reach the object that was built). A node is an `Rc<RefCell<LibraryValue>>`;
//! containers hold their children as such handles, so a token obtained with
//! `Get` shares the child. Behaviour the corpus does not pin (error texts,
//! duplicate keys, out-of-range indexes) raises `Signal::Unsupported`.

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal};
use crate::signature::Signature;
use crate::value::{LibraryValue, Value};
use rust_decimal::Decimal;
use std::cell::RefCell;
use std::rc::{Rc, Weak};

mod filter;

pub type Handle = Rc<RefCell<LibraryValue>>;

/// A node plus the container it is attached to (for `Path`).
#[derive(Debug)]
pub struct JsonCell {
    pub node: JsonNode,
    pub parent: Option<Weak<RefCell<LibraryValue>>>,
    /// A value whose content is not pinned: any use of it is refused with this text.
    pub unpinned: Option<&'static str>,
}

#[derive(Debug)]
pub enum JsonNode {
    Object(Vec<(String, Handle)>),
    Array(Vec<Handle>),
    Scalar(Scalar),
}

#[derive(Debug, Clone)]
pub enum Scalar {
    Null,
    Undefined,
    /// A typed AL value: Boolean, Integer, BigInteger, Byte, Decimal, Text, Code,
    /// Char, Date, Time, DateTime, Duration or Option.
    Val(Value),
    /// A number the way .NET writes it, when no AL value holds it: an exponent form, `-0.0`, an integer beyond a
    /// Decimal (json/TestProbeJsonContracts.al:ExponentNumbers_ReadAndWrite). Written as it is; no AL read of it is pinned.
    Raw(String),
}

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn error(message: impl Into<String>, code: &str) -> Signal {
    Signal::Error(Box::new(crate::native::AlError {
        message: message.into(),
        code: code.into(),
        call_stack: String::new(),
        ..Default::default()
    }))
}

/// A failed conversion, as .NET words it (json/TestJsonRuntimeContracts.al:JsonObject_Errors_TextAndCode).
fn conversion(from: &str, to: &str) -> Signal {
    error(format!("Unable to convert from {from} to {to}."), "Conversion")
}

const NAV_JSON_VALUE: &str = "Microsoft.Dynamics.Nav.Runtime.NavJsonValue";

/// A JsonObject whose content is not pinned: every method on it, and its `Format`, is refused with `why`.
pub fn unpinned_object(why: &'static str) -> Value {
    let h = Rc::new(RefCell::new(LibraryValue::Json(JsonCell {
        node: JsonNode::Object(Vec::new()),
        parent: None,
        unpinned: Some(why),
    })));
    Value::Library("JsonObject".into(), h)
}

fn new_handle(node: JsonNode) -> Handle {
    Rc::new(RefCell::new(LibraryValue::Json(JsonCell {
        node,
        parent: None,
        unpinned: None,
    })))
}

/// The handle of a JSON-family value, giving a never-used JsonObject / JsonArray /
/// JsonValue variable its empty content on first use.
fn handle_of(v: &Value) -> Result<Handle, Signal> {
    let Value::Library(name, h) = v else {
        return unsupported(format!("{} used as a JSON handle", v.describe()));
    };
    if matches!(&*h.borrow(), LibraryValue::Empty) {
        let node = match name.to_ascii_lowercase().as_str() {
            "jsonobject" => JsonNode::Object(Vec::new()),
            "jsonarray" => JsonNode::Array(Vec::new()),
            // A cleared JsonValue is null, not undefined (types/ProbeTypeFormsAndFilters.al:
            // Clear_JsonTypes_GiveFreshUnassigned, BC 27.5); a never-used one is the same state (assumed).
            "jsonvalue" => JsonNode::Scalar(Scalar::Null),
            // An unassigned JsonToken acts as a JSON null (json/TestJsonTokenUnassignedEvaluateGuidContracts.al).
            "jsontoken" => JsonNode::Scalar(Scalar::Null),
            other => return unsupported(format!("use of an unassigned {other}")),
        };
        *h.borrow_mut() = LibraryValue::Json(JsonCell {
            node,
            parent: None,
            unpinned: None,
        });
    }
    match &*h.borrow() {
        LibraryValue::Json(JsonCell {
            unpinned: Some(why), ..
        }) => unsupported(*why),
        LibraryValue::Json(_) => Ok(h.clone()),
        other => unsupported(format!("{name} handle holding {}", other.describe())),
    }
}

fn recv(r: Option<&mut Value>) -> Result<Handle, Signal> {
    match r {
        Some(v) => handle_of(v),
        None => unsupported("JSON method without a receiver"),
    }
}

fn tagged(name: &str, h: Handle) -> Value {
    Value::Library(name.into(), h)
}

fn with_node<T>(h: &Handle, f: impl FnOnce(&mut JsonNode) -> T) -> T {
    match &mut *h.borrow_mut() {
        LibraryValue::Json(c) => f(&mut c.node),
        _ => unreachable!("checked by handle_of"),
    }
}

fn kind_name(h: &Handle) -> &'static str {
    with_node(h, |n| match n {
        JsonNode::Object(_) => "an object",
        JsonNode::Array(_) => "an array",
        JsonNode::Scalar(_) => "a value",
    })
}

// ------------------------------------------------------------------ tree

fn deep_clone(h: &Handle) -> Handle {
    let node = with_node(h, |n| match n {
        JsonNode::Scalar(s) => JsonNode::Scalar(s.clone()),
        JsonNode::Object(kids) => JsonNode::Object(kids.iter().map(|(k, c)| (k.clone(), deep_clone(c))).collect()),
        JsonNode::Array(kids) => JsonNode::Array(kids.iter().map(deep_clone).collect()),
    });
    let out = new_handle(node);
    adopt_children(&out);
    out
}

fn adopt_children(h: &Handle) {
    let kids: Vec<Handle> = with_node(h, |n| match n {
        JsonNode::Object(k) => k.iter().map(|(_, c)| c.clone()).collect(),
        JsonNode::Array(k) => k.clone(),
        JsonNode::Scalar(_) => Vec::new(),
    });
    for k in kids {
        set_parent(&k, Some(h));
    }
}

fn set_parent(child: &Handle, parent: Option<&Handle>) {
    if let LibraryValue::Json(c) = &mut *child.borrow_mut() {
        c.parent = parent.map(Rc::downgrade);
    }
}

fn parent_of(h: &Handle) -> Option<Handle> {
    match &*h.borrow() {
        LibraryValue::Json(c) => c.parent.as_ref().and_then(Weak::upgrade),
        _ => None,
    }
}

/// Whether `maybe_ancestor` is `h` or one of its ancestors.
fn is_ancestor_or_self(maybe_ancestor: &Handle, h: &Handle) -> bool {
    let mut cur = Some(h.clone());
    while let Some(c) = cur {
        if Rc::ptr_eq(&c, maybe_ancestor) {
            return true;
        }
        cur = parent_of(&c);
    }
    false
}

/// The handle to store when `child` is added to `container`: the child itself when it
/// is free, a deep copy when it already belongs to a tree (or would form a cycle).
/// Docs/probes-needed.md records that the corpus does not pin sharing versus copying.
fn attachable(container: &Handle, child: Handle) -> Handle {
    let attached = parent_of(&child).is_some();
    // A value token is copied whether or not it has a parent: a JsonValue set to another value afterwards leaves the
    // container as it was (json/TestProbeJsonContracts.al:ValueTypes_DurationTimeDateTime).
    let scalar = with_node(&child, |n| matches!(n, JsonNode::Scalar(_)));
    if attached || scalar || is_ancestor_or_self(&child, container) {
        deep_clone(&child)
    } else {
        child
    }
}

fn guid_text(g: &[u8; 16]) -> String {
    let h: Vec<String> = g.iter().map(|b| format!("{b:02X}")).collect();
    format!(
        "{{{}-{}-{}-{}-{}}}",
        h[0..4].concat(),
        h[4..6].concat(),
        h[6..8].concat(),
        h[8..10].concat(),
        h[10..16].concat()
    )
}

/// A node for an `Add`/`SetValue` argument.
fn node_for(v: &Value) -> Result<Handle, Signal> {
    match v {
        Value::Library(..) => handle_of(v),
        Value::Boolean(_)
        | Value::Integer(_)
        | Value::BigInteger(_)
        | Value::Byte(_)
        | Value::Decimal(_)
        | Value::Text(_)
        | Value::Code(_)
        | Value::Char(_)
        | Value::Date(..)
        | Value::Time(_)
        | Value::DateTime(_)
        | Value::Duration(_)
        | Value::Option(..) => Ok(new_handle(JsonNode::Scalar(Scalar::Val(v.clone())))),
        // A Guid is stored as its text, braced and in upper case
        // (json/TestJsonRuntimeContracts.al:JsonObject_WriteTo_GuidIsBracedUpperCase).
        Value::Guid(g) => Ok(new_handle(JsonNode::Scalar(Scalar::Val(Value::Text(
            guid_text(g).into(),
        ))))),
        other => unsupported(format!("{} as a JSON value", other.describe())),
    }
}

fn attach(container: &Handle, child: Handle) -> Handle {
    let child = attachable(container, child);
    set_parent(&child, Some(container));
    child
}

fn path_of(h: &Handle) -> String {
    let mut segs: Vec<String> = Vec::new();
    let mut cur = h.clone();
    while let Some(p) = parent_of(&cur) {
        let seg = with_node(&p, |n| match n {
            JsonNode::Object(kids) => kids.iter().find(|(_, c)| Rc::ptr_eq(c, &cur)).map(|(k, _)| {
                // An empty key adds nothing to the path (json/lt-json-overloads/P88ObjectPathYaml.al:Path_SpecialKeys).
                if k.is_empty() {
                    String::new()
                } else if k.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    format!(".{k}")
                } else {
                    format!("['{}']", k.replace('\'', "\\'"))
                }
            }),
            JsonNode::Array(kids) => kids.iter().position(|c| Rc::ptr_eq(c, &cur)).map(|i| format!("[{i}]")),
            JsonNode::Scalar(_) => None,
        });
        segs.push(seg.unwrap_or_default());
        cur = p;
    }
    let joined: String = segs.iter().rev().cloned().collect();
    joined.strip_prefix('.').map_or(joined.clone(), str::to_string)
}

// ---------------------------------------------------------------- parsing

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
    depth: usize,
    /// Set when the text holds a `//` comment: whether the reader takes it is not pinned.
    line_comment: bool,
}

type PResult<T> = Result<T, ParseError>;

#[derive(Debug)]
enum ParseError {
    /// Not valid JSON: ReadFrom reports failure.
    Invalid,
    /// Valid JSON the model does not represent (duplicate keys, huge nesting).
    Unsupported(&'static str),
}

impl Parser<'_> {
    /// Blanks and `/* */` comments (json/JsonReadFrom.al:ObjectReadFromErrors `comment`).
    fn ws(&mut self) {
        loop {
            while self.i < self.s.len() && matches!(self.s[self.i], b' ' | b'\t' | b'\n' | b'\r') {
                self.i += 1;
            }
            if self.s[self.i..].starts_with(b"/*") {
                match self.s[self.i + 2..].windows(2).position(|w| w == b"*/") {
                    Some(end) => {
                        self.i += 2 + end + 2;
                        continue;
                    }
                    None => return,
                }
            }
            if self.s[self.i..].starts_with(b"//") {
                self.line_comment = true;
            }
            return;
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        if self.s.get(self.i) == Some(&c) {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn lit(&mut self, word: &str) -> bool {
        if self.s[self.i..].starts_with(word.as_bytes()) {
            self.i += word.len();
            true
        } else {
            false
        }
    }

    fn value(&mut self) -> PResult<Handle> {
        self.ws();
        self.depth += 1;
        if self.depth > 256 {
            return Err(ParseError::Unsupported("JSON nested deeper than 256 levels"));
        }
        let h = match self.s.get(self.i) {
            Some(b'{') => self.object()?,
            Some(b'[') => self.array()?,
            Some(b'"' | b'\'') => {
                let t = self.string()?;
                new_handle(JsonNode::Scalar(Scalar::Val(Value::Text(t.into()))))
            }
            Some(b't') if self.lit("true") => new_handle(JsonNode::Scalar(Scalar::Val(Value::Boolean(true)))),
            Some(b'f') if self.lit("false") => new_handle(JsonNode::Scalar(Scalar::Val(Value::Boolean(false)))),
            Some(b'n') if self.lit("null") => new_handle(JsonNode::Scalar(Scalar::Null)),
            Some(b'-' | b'0'..=b'9') => self.number()?,
            // `NaN` reads as the text "NaN" (ReadFrom_Invalid `nan`); Infinity and the like are not pinned.
            Some(b'N') if self.lit("NaN") => new_handle(JsonNode::Scalar(Scalar::Val(Value::Text("NaN".into())))),
            Some(b'I') if self.s[self.i..].starts_with(b"Infinity") => {
                return Err(ParseError::Unsupported("JSON Infinity (not pinned)"));
            }
            _ => return Err(ParseError::Invalid),
        };
        self.depth -= 1;
        Ok(h)
    }

    fn object(&mut self) -> PResult<Handle> {
        self.i += 1;
        let mut kids: Vec<(String, Handle)> = Vec::new();
        self.ws();
        if !self.eat(b'}') {
            loop {
                self.ws();
                // A closing brace after a comma ends the object (`{"a":1,}`: json/JsonReadFrom.al:ObjectReadFromErrors `trailing_comma`).
                if !kids.is_empty() && self.eat(b'}') {
                    break;
                }
                let k = match self.s.get(self.i) {
                    Some(b'"' | b'\'') => self.string()?,
                    // An unquoted key (`{a:1}`: `unquoted_key`); only plain names are taken.
                    Some(c) if c.is_ascii_alphabetic() || *c == b'_' => {
                        let from = self.i;
                        while self
                            .s
                            .get(self.i)
                            .is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_')
                        {
                            self.i += 1;
                        }
                        String::from_utf8_lossy(&self.s[from..self.i]).into_owned()
                    }
                    _ => return Err(ParseError::Invalid),
                };
                self.ws();
                if !self.eat(b':') {
                    return Err(ParseError::Invalid);
                }
                let v = self.value()?;
                if kids.iter().any(|(e, _)| *e == k) {
                    return Err(ParseError::Unsupported("duplicate keys in a JSON object"));
                }
                kids.push((k, v));
                self.ws();
                if self.eat(b',') {
                    continue;
                }
                if self.eat(b'}') {
                    break;
                }
                return Err(ParseError::Invalid);
            }
        }
        let h = new_handle(JsonNode::Object(kids));
        adopt_children(&h);
        Ok(h)
    }

    fn array(&mut self) -> PResult<Handle> {
        self.i += 1;
        let mut kids = Vec::new();
        self.ws();
        if !self.eat(b']') {
            loop {
                kids.push(self.value()?);
                self.ws();
                if self.eat(b',') {
                    // `[1,]` is `[1]` (json/JsonReadFrom.al:ArrayReadFromErrors `trailing_comma`).
                    self.ws();
                    if self.eat(b']') {
                        break;
                    }
                    if self.s.get(self.i) == Some(&b',') {
                        return Err(ParseError::Unsupported("an empty element in a JSON array (not pinned)"));
                    }
                    continue;
                }
                if self.eat(b']') {
                    break;
                }
                return Err(ParseError::Invalid);
            }
        }
        let h = new_handle(JsonNode::Array(kids));
        adopt_children(&h);
        Ok(h)
    }

    fn hex4(&mut self) -> PResult<u16> {
        let h = self.s.get(self.i..self.i + 4).ok_or(ParseError::Invalid)?;
        let h = std::str::from_utf8(h).map_err(|_| ParseError::Invalid)?;
        let v = u16::from_str_radix(h, 16).map_err(|_| ParseError::Invalid)?;
        self.i += 4;
        Ok(v)
    }

    fn string(&mut self) -> PResult<String> {
        // Either quote opens a string and the same one closes it (`{'a':1}`, `single_quotes`).
        let quote = self.s[self.i];
        self.i += 1;
        let mut units: Vec<u16> = Vec::new();
        loop {
            let b = *self.s.get(self.i).ok_or(ParseError::Invalid)?;
            match b {
                q if q == quote => {
                    self.i += 1;
                    break;
                }
                b'\\' => {
                    self.i += 1;
                    let e = *self.s.get(self.i).ok_or(ParseError::Invalid)?;
                    self.i += 1;
                    let c = match e {
                        b'"' => '"' as u16,
                        b'\'' => '\'' as u16,
                        b'\\' => '\\' as u16,
                        b'/' => '/' as u16,
                        b'b' => 8,
                        b'f' => 12,
                        b'n' => 10,
                        b'r' => 13,
                        b't' => 9,
                        b'u' => self.hex4()?,
                        _ => return Err(ParseError::Invalid),
                    };
                    units.push(c);
                }
                0..=0x1f => return Err(ParseError::Invalid),
                _ => {
                    // A UTF-8 sequence copied through as UTF-16.
                    let len = match b {
                        0..=0x7f => 1,
                        0xc0..=0xdf => 2,
                        0xe0..=0xef => 3,
                        _ => 4,
                    };
                    let seq = self.s.get(self.i..self.i + len).ok_or(ParseError::Invalid)?;
                    let ch = std::str::from_utf8(seq)
                        .map_err(|_| ParseError::Invalid)?
                        .chars()
                        .next()
                        .ok_or(ParseError::Invalid)?;
                    let mut buf = [0u16; 2];
                    units.extend_from_slice(ch.encode_utf16(&mut buf));
                    self.i += len;
                }
            }
        }
        Ok(String::from_utf16_lossy(&units))
    }

    fn number(&mut self) -> PResult<Handle> {
        let start = self.i;
        self.eat(b'-');
        let int_start = self.i;
        while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
            self.i += 1;
        }
        let int_len = self.i - int_start;
        if int_len == 0 {
            return Err(ParseError::Invalid);
        }
        // `007` reads as 7 (json/lt-json-overloads/P88ValueReadFrom.al:ReadFrom_Invalid `leading_zero`); what a leading zero
        // does in front of a fraction or an exponent is not pinned.
        if int_len > 1 && self.s[int_start] == b'0' && matches!(self.s.get(self.i), Some(b'.' | b'e' | b'E')) {
            return Err(ParseError::Unsupported(
                "a JSON number with leading zeros and a fraction or exponent (not pinned)",
            ));
        }
        let mut is_int = true;
        if self.eat(b'.') {
            is_int = false;
            let f = self.i;
            while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                self.i += 1;
            }
            if self.i == f {
                return Err(ParseError::Invalid);
            }
        }
        if matches!(self.s.get(self.i), Some(b'e' | b'E')) {
            is_int = false;
            self.i += 1;
            if matches!(self.s.get(self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            let f = self.i;
            while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                self.i += 1;
            }
            if self.i == f {
                return Err(ParseError::Invalid);
            }
        }
        let text = std::str::from_utf8(&self.s[start..self.i]).map_err(|_| ParseError::Invalid)?;
        let v = if is_int {
            match text.parse::<i64>() {
                Ok(n) => Value::BigInteger(n),
                // An integer beyond a BigInteger keeps its digits when written back (json/TestJsonRuntimeContracts.al:
                // JsonObject_ReadFrom_NumbersKeepTheirWrittenForm, 12345678901234567890; ExponentNumbers_ReadAndWrite, `f`),
                // while a Decimal set from AL is cut at 18 significant digits (WriteTo_NumberForms). A read as a Decimal
                // parses the digits.
                Err(_) => return Ok(new_handle(JsonNode::Scalar(Scalar::Raw(text.to_string())))),
            }
        } else {
            // A number with a fraction or an exponent is read as a double and written back the
            // way .NET writes a double: `1.0`, `100.0` for `1e2`, `0.1`
            // (json/TestJsonRuntimeContracts.al:JsonObject_ReadFrom_NumbersKeepTheirWrittenForm).
            let f: f64 = text.parse().map_err(|_| ParseError::Invalid)?;
            if !f.is_finite() {
                return Err(ParseError::Unsupported("a JSON number outside the double range"));
            }
            // .NET writes a double in the exponent form `1E-05`, `1.5E+300` when its decimal exponent is below -4
            // or above 16, else in fixed notation with at least one decimal place; `-0.0` keeps its sign
            // (json/TestProbeJsonContracts.al:ExponentNumbers_ReadAndWrite).
            let exp10 = if f == 0.0 { 0 } else { f.abs().log10().floor() as i32 };
            if f.is_sign_negative() && f == 0.0 {
                return Ok(new_handle(JsonNode::Scalar(Scalar::Raw("-0.0".into()))));
            }
            if f != 0.0 && !(-4..=16).contains(&exp10) {
                let sci = format!("{f:e}");
                let (mant, ex) = sci.split_once('e').expect("exponent form");
                let ex: i32 = ex.parse().expect("exponent");
                let written = format!("{mant}E{}{:02}", if ex < 0 { '-' } else { '+' }, ex.abs());
                return Ok(new_handle(JsonNode::Scalar(Scalar::Raw(written))));
            }
            let mut written = format!("{f}");
            if !written.contains('.') {
                written.push_str(".0");
            }
            Value::Decimal(
                written
                    .parse::<Decimal>()
                    .map_err(|_| ParseError::Unsupported("number outside the Decimal range"))?,
            )
        };
        Ok(new_handle(JsonNode::Scalar(Scalar::Val(v))))
    }
}

/// `Ok(None)`: not valid JSON.
fn parse(text: &str) -> Result<Option<Handle>, Signal> {
    let mut p = Parser {
        s: text.as_bytes(),
        i: 0,
        depth: 0,
        line_comment: false,
    };
    match p.value() {
        Ok(h) => {
            p.ws();
            if p.line_comment {
                return unsupported("a // comment in JSON text (not pinned)");
            }
            // Text after the first value is ignored (json/TestProbeJsonContracts.al:ReadFrom_Invalid_Statement,
            // `trailing_garbage`: `{"a":1} trailing` raises nothing).
            Ok(Some(h))
        }
        Err(ParseError::Invalid) if p.line_comment => unsupported("a // comment in JSON text (not pinned)"),
        Err(ParseError::Invalid) => Ok(None),
        Err(ParseError::Unsupported(what)) => unsupported(what),
    }
}

/// A JsonObject value holding `text`, or `None` when the text is not a JSON object.
pub fn object_from_text(text: &str) -> Result<Option<Value>, Signal> {
    let Some(h) = parse(text)? else { return Ok(None) };
    if with_node(&h, |n| matches!(n, JsonNode::Object(_))) {
        Ok(Some(Value::Library("JsonObject".into(), h)))
    } else {
        Ok(None)
    }
}

// ---------------------------------------------------------------- writing

/// `s` as a JSON string literal, quoted and escaped the way `WriteTo` writes it.
pub fn string_literal(s: &str) -> String {
    let mut out = String::new();
    write_string(&mut out, s);
    out
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            // The line and paragraph separators are written as escapes (json/JsonWriteTo.al:UnicodeAndEscapes `line_sep`;
            // U+0085 and U+2029 follow Json.NET's list, not pinned).
            c @ ('\u{85}' | '\u{2028}' | '\u{2029}') => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Compact JSON (no whitespace); the corpus only pins `{}`, `[]` and that keys appear quoted
/// (json/TestJsonXmlDeepContracts.al, TestJsonObject.al JsonObject_WriteTo_SerializesJSON).
fn write(h: &Handle, out: &mut String) -> Result<(), Signal> {
    enum Step {
        Object(Vec<(String, Handle)>),
        Array(Vec<Handle>),
        Scalar(Scalar),
    }
    let step = with_node(h, |n| match n {
        JsonNode::Object(k) => Step::Object(k.clone()),
        JsonNode::Array(k) => Step::Array(k.clone()),
        JsonNode::Scalar(s) => Step::Scalar(s.clone()),
    });
    match step {
        Step::Object(kids) => {
            out.push('{');
            for (i, (k, c)) in kids.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(out, k);
                out.push(':');
                write(c, out)?;
            }
            out.push('}');
        }
        Step::Array(kids) => {
            out.push('[');
            for (i, c) in kids.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(c, out)?;
            }
            out.push(']');
        }
        Step::Scalar(Scalar::Null) => out.push_str("null"),
        Step::Scalar(Scalar::Undefined) => return unsupported("serialising an undefined JsonValue"),
        Step::Scalar(Scalar::Raw(t)) => out.push_str(&t),
        Step::Scalar(Scalar::Val(v)) => match v {
            Value::Boolean(b) => out.push_str(if b { "true" } else { "false" }),
            Value::Integer(n) => out.push_str(&n.to_string()),
            Value::BigInteger(n) => out.push_str(&n.to_string()),
            Value::Byte(n) => out.push_str(&n.to_string()),
            // A Duration is its milliseconds (json/TestProbeJsonContracts.al:ValueTypes_DurationTimeDateTime, `"dur":1500`).
            Value::Duration(n) => out.push_str(&n.to_string()),
            Value::Option(n, _) => out.push_str(&n.to_string()),
            // A decimal keeps its scale: 1.50 is written as 1.50
            // (json/TestJsonRuntimeContracts.al:JsonObject_WriteTo_IsCompactAndKeepsInsertionOrder).
            // A decimal beyond 18 significant digits is cut there (json/TestProbeJsonContracts.al:WriteTo_NumberForms:
            // 12345678901234567890.0 is written 12345678901234567900.0).
            Value::Decimal(d) => {
                let digits = d.abs().mantissa().to_string().len();
                let text = if digits > 18 {
                    // The zeros the cut leaves behind are not written (json/JsonWriteTo.al:NumberFormattingInWriteTo, 12345678901234567900.0).
                    let t = d.round_sf(18).unwrap_or(d).to_string();
                    if t.contains('.') {
                        t.trim_end_matches('0').trim_end_matches('.').to_string()
                    } else {
                        t
                    }
                } else {
                    d.to_string()
                };
                // A whole decimal is written with `.0` (json/lt-json-typed-overloads/P101ArrayAdd.al:P101_Add_Decimal: 0 is `0.0`).
                out.push_str(&text);
                if !text.contains('.') {
                    out.push_str(".0");
                }
            }
            // An undefined Date, Time and DateTime are written as the first day, midnight and the first day at midnight, the
            // date-time without the `Z` (json/lt-json-typed-overloads/P101ArrayAdd.al:P101_Add_Date `0001-01-01`, P101_Add_Time
            // `00:00:00`, P101_Add_DateTime `0001-01-01T00:00:00.0000000`).
            Value::Date(0, _) => write_string(out, "0001-01-01"),
            Value::Time(0) => write_string(out, "00:00:00"),
            Value::DateTime(0) => write_string(out, "0001-01-01T00:00:00.0000000"),
            Value::Date(d, _) if d > 0 => {
                let (y, m, day) = crate::format::civil_from_day(d);
                write_string(out, &format!("{y:04}-{m:02}-{day:02}"));
            }
            // A time with milliseconds adds them without trailing zeros, as `Format(T, 0, 9)`; a date-time adds seven digits
            // (json/TestProbeJsonContracts.al:ValueTypes_DurationTimeDateTime: `10:00:00.123`, `2024-01-15T10:20:30.0450000Z`;
            // misc/lt-outside-format-json-list/RwJsonTimeMs.al:116 RW_JsonTime_InMemory_Milliseconds: `.001`, `.01`, `.12`, `.5`).
            Value::Time(t) if t > 0 => {
                let ms = (t - 1) % 1000;
                let s = (t - 1) / 1000;
                let frac = if ms == 0 {
                    String::new()
                } else {
                    format!(".{}", format!("{ms:03}").trim_end_matches('0'))
                };
                write_string(out, &format!("{:02}:{:02}:{:02}{frac}", s / 3600, s / 60 % 60, s % 60));
            }
            Value::DateTime(ms) if ms > 0 => {
                let ms = ms - 1;
                let (y, m, d) = crate::format::civil_from_day(ms.div_euclid(86_400_000) as i32 + 1);
                let day_ms = ms.rem_euclid(86_400_000);
                let s = day_ms / 1000;
                write_string(
                    out,
                    &format!(
                        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:07}Z",
                        s / 3600,
                        s / 60 % 60,
                        s % 60,
                        (day_ms % 1000) * 10_000
                    ),
                );
            }
            Value::Text(s) | Value::Code(s) => write_string(out, &s),
            // A Char is its code (P101ArrayAdd.al:P101_Add_Char: `A` is 65).
            Value::Char(c) => out.push_str(&c.to_string()),
            other => {
                return unsupported(format!(
                    "serialising {} to JSON text (not pinned yet)",
                    other.describe()
                ));
            }
        },
    }
    Ok(())
}

// ---------------------------------------------------------------- scalars

fn scalar_of(h: &Handle) -> Result<Scalar, Signal> {
    with_node(h, |n| match n {
        JsonNode::Scalar(s) => Ok(s.clone()),
        JsonNode::Object(_) => unsupported("a value was read from a JSON object (error text not pinned yet)"),
        JsonNode::Array(_) => unsupported("a value was read from a JSON array (error text not pinned yet)"),
    })
}

fn as_i64(s: &Scalar) -> Result<i64, Signal> {
    match s {
        Scalar::Val(Value::Integer(n)) => Ok(i64::from(*n)),
        Scalar::Val(Value::BigInteger(n)) => Ok(*n),
        Scalar::Val(Value::Byte(n)) => Ok(i64::from(*n)),
        Scalar::Val(Value::Option(n, _)) => Ok(i64::from(*n)),
        Scalar::Val(Value::Duration(n)) => Ok(*n),
        Scalar::Val(Value::Decimal(d)) if d.fract().is_zero() => {
            i64::try_from(d.trunc().mantissa() / 10i128.pow(d.trunc().scale()))
                .or_else(|_| unsupported("number out of range"))
        }
        other => unsupported(format!(
            "reading {other} as a whole number (conversion not pinned yet)",
            other = other.describe()
        )),
    }
}

fn as_boolean(s: &Scalar) -> Result<Value, Signal> {
    match s {
        Scalar::Val(Value::Boolean(b)) => Ok(Value::Boolean(*b)),
        // json/JsonValueGetters.al:202 `truetext_asboolean`: the text "true" reads Yes.
        Scalar::Val(Value::Text(t) | Value::Code(t)) if &**t == "true" => Ok(Value::Boolean(true)),
        // json/JsonValueGetters.al UN_BooleanTextGetters* (writer UN's corpus branch, commit d3bcc580; assumption
        // `json-boolean-text-spellings`): "True", "TRUE", " true", "true " read Yes; "false", "False", "FALSE" read No.
        Scalar::Val(Value::Text(t) | Value::Code(t))
            if matches!(&**t, "True" | "TRUE" | " true" | "true " | "false" | "False" | "FALSE") =>
        {
            // Pinned by json/JsonValueGetters.al UN_BooleanTextGetters, UN_BooleanTextGetters_5 (was assumption `json-boolean-text-spellings`).
            Ok(Value::Boolean(t.trim().eq_ignore_ascii_case("true")))
        }
        // Other spellings of true and false (mixed case, blanks around false, other blanks) are not pinned.
        Scalar::Val(Value::Text(t) | Value::Code(t))
            if t.trim().eq_ignore_ascii_case("true") || t.trim().eq_ignore_ascii_case("false") =>
        {
            unsupported(format!("reading Text '{t}' as Boolean (conversion not pinned yet)"))
        }
        // json/JsonValueGetters.al:172 `text_asboolean`: a text of letters ("hello") raises the conversion error.
        Scalar::Val(Value::Text(t) | Value::Code(t)) if !t.is_empty() && t.bytes().all(|b| b.is_ascii_alphabetic()) => {
            Err(conversion(NAV_JSON_VALUE, "System.Boolean"))
        }
        // An empty text and a text of digits raise the same error (json/JsonValueGetters.al UN_BooleanTextGetters_2 "0",
        // UN_BooleanTextGetters_3 "1", UN_BooleanTextGetters_4 ""). Any other text (blanks inside, punctuation, a mix) is
        // assumed to (json-other-text-as-boolean-conversion-error).
        Scalar::Val(Value::Text(t) | Value::Code(t)) => {
            if !t.bytes().all(|b| b.is_ascii_digit()) {
                crate::assumptions::assume(crate::assumptions::Assumption::JsonOtherTextAsBooleanConversionError);
            }
            Err(conversion(NAV_JSON_VALUE, "System.Boolean"))
        }
        // A JSON number 0 reads No and 1 reads Yes (json/JsonValueGetters.al UN_BooleanTextGetters_5, same branch and
        // assumption). Other numbers are not pinned.
        Scalar::Val(Value::Integer(n @ (0 | 1))) => {
            // Pinned by json/JsonValueGetters.al UN_BooleanTextGetters, UN_BooleanTextGetters_5 (was assumption `json-boolean-text-spellings`).
            Ok(Value::Boolean(*n == 1))
        }
        Scalar::Val(Value::BigInteger(n @ (0 | 1))) => {
            // Pinned by json/JsonValueGetters.al UN_BooleanTextGetters, UN_BooleanTextGetters_5 (was assumption `json-boolean-text-spellings`).
            Ok(Value::Boolean(*n == 1))
        }
        other => unsupported(format!(
            "reading {} as Boolean (conversion not pinned yet)",
            other.describe()
        )),
    }
}

fn as_integer(s: &Scalar) -> Result<Value, Signal> {
    if matches!(s, Scalar::Null) {
        return Err(conversion(NAV_JSON_VALUE, "System.Int32"));
    }
    // A JSON `true` read as an Integer is 1 (misc/lt-probes-needed-1/PnAJsonGetters.al:191-210, ArrayWrongKindAtIndex
    // `int-of-bool`); `false` is not pinned.
    if let Scalar::Val(Value::Boolean(true)) = s {
        return Ok(Value::Integer(1));
    }
    if let Scalar::Val(Value::Text(t) | Value::Code(t)) = s {
        match t.trim().parse::<i32>() {
            Ok(n) => return Ok(Value::Integer(n)),
            Err(_) => return Err(conversion(NAV_JSON_VALUE, "System.Int32")),
        }
    }
    // A Char is a number in JSON: its code (json/lt-json-typed-readback/JsonTypedCharRead.al:48 `int` 65, :52 0, :55 65535).
    if let Scalar::Val(Value::Char(n)) = s {
        return Ok(Value::Integer(i32::from(*n)));
    }
    // A number with a fraction rounds to the nearest whole number, a tie to the even one (corpus master, corpus PR 871:
    // misc/lt-np4-probes/Np4FormatJson.al NP4_JsonValue_AsInteger_SetValueDecimal, _FromText: 1324.55 1325, 2.5 2, 1.5 2,
    // 0.5 0, -0.5 0, -1.5 -2, -2.5 -2, 3.4999 3).
    if let Scalar::Val(Value::Decimal(d)) = s
        && !d.fract().is_zero()
    {
        let r = d.round_dp_with_strategy(0, rust_decimal::RoundingStrategy::MidpointNearestEven);
        return i32::try_from(r.mantissa() / 10i128.pow(r.scale()))
            .map(Value::Integer)
            .or_else(|_| unsupported("number out of Integer range"));
    }
    i32::try_from(as_i64(s)?)
        .map(Value::Integer)
        .or_else(|_| unsupported("number out of Integer range"))
}

/// A JSON string read by AsBigInteger, AsByte or AsOption (json/JsonValueGetters.al Sq_DigitText_BigIntegerByteOption): digits with
/// an optional `-` and leading blanks are the number (`"42"`, `" 42"`, `"-5"`, `"3000000000"`), `Some(None)` for a number with a
/// fraction (`"4.5"`, the conversion error); `None` for any other text, which is not pinned.
fn digit_text(s: &Scalar) -> Option<Option<i64>> {
    let (Scalar::Val(Value::Text(t)) | Scalar::Val(Value::Code(t))) = s else {
        return None;
    };
    let t = t.trim_start_matches(' ');
    let digits = t.strip_prefix('-').unwrap_or(t);
    if all_digits(digits) {
        return t.parse::<i64>().ok().map(Some);
    }
    match t.split_once('.') {
        Some((a, b)) if all_digits(a) && all_digits(b) => Some(None),
        _ => None,
    }
}

fn as_biginteger(s: &Scalar) -> Result<Value, Signal> {
    match digit_text(s) {
        Some(Some(n)) => return Ok(Value::BigInteger(n)),
        Some(None) => {
            return Err(conversion(
                NAV_JSON_VALUE,
                "Microsoft.Dynamics.Nav.Runtime.NavBigInteger",
            ));
        }
        None => {}
    }
    as_i64(s).map(Value::BigInteger)
}

fn as_byte(s: &Scalar) -> Result<Value, Signal> {
    if let Some(n) = digit_text(s) {
        return n
            .and_then(|n| u8::try_from(n).ok())
            .map(Value::Byte)
            .ok_or_else(|| conversion(NAV_JSON_VALUE, "System.Byte"));
    }
    u8::try_from(as_i64(s)?)
        .map(Value::Byte)
        .or_else(|_| unsupported("number out of Byte range"))
}

fn as_option(s: &Scalar) -> Result<Value, Signal> {
    let n = match digit_text(s) {
        Some(n) => n
            .and_then(|n| i32::try_from(n).ok())
            .ok_or_else(|| conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.NavOption"))?,
        None => i32::try_from(as_i64(s)?).or_else(|_| unsupported("number out of Option range"))?,
    };
    Ok(Value::Option(n, std::sync::Arc::from(Vec::<String>::new())))
}

fn as_duration(s: &Scalar) -> Result<Value, Signal> {
    // A number or a numeric text is the milliseconds (json/TestProbeJsonContracts.al:IsoText_ReadAsDateTimeKinds,
    // `n_as_duration`, `nd_as_duration`).
    if let Scalar::Val(Value::Text(t) | Value::Code(t)) = s {
        if let Ok(n) = t.trim().parse::<i64>() {
            return Ok(Value::Duration(n));
        }
    }
    as_i64(s).map(Value::Duration)
}

fn all_digits(t: &str) -> bool {
    !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())
}

/// `-?digits(.digits)?` with no blanks (AjScalars.al: "100.00", "-3.5").
fn signed_point_number(t: &str) -> bool {
    let t = t.strip_prefix('-').unwrap_or(t);
    match t.split_once('.') {
        Some((a, b)) => all_digits(a) && all_digits(b),
        None => all_digits(t),
    }
}

/// Digits with blanks around them (AjScalars.al: " 7 ").
fn blank_padded_digits(t: &str) -> bool {
    all_digits(t.trim_matches(' '))
}

/// `digits e digits` (AjScalars.al: "1e2").
fn exponent_digits(t: &str) -> bool {
    t.split_once('e').is_some_and(|(a, b)| all_digits(a) && all_digits(b))
}

fn as_decimal(s: &Scalar) -> Result<Value, Signal> {
    match s {
        Scalar::Val(Value::Decimal(d)) => Ok(Value::Decimal(*d)),
        Scalar::Val(Value::Integer(n)) => Ok(Value::Decimal(Decimal::from(*n))),
        Scalar::Val(Value::BigInteger(n)) => Ok(Value::Decimal(Decimal::from(*n))),
        Scalar::Val(Value::Byte(n)) => Ok(Value::Decimal(Decimal::from(*n))),
        // JsonTypedCharRead.al:49 `dec` 65.
        Scalar::Val(Value::Char(n)) => Ok(Value::Decimal(Decimal::from(*n))),
        Scalar::Raw(t) if t.parse::<Decimal>().is_ok() && t.bytes().all(|b| b.is_ascii_digit() || b == b'-') => {
            Ok(Value::Decimal(t.parse::<Decimal>().expect("checked")))
        }
        // A text of digits reads as the number (`"42"`), a text of letters raises the conversion error
        // (json/JsonValueGetters.al:NumericTextGetters `numtext_asdecimal`, TextGetters `text_asdecimal`). A point, a sign or
        // blanks in the text are not pinned.
        Scalar::Val(Value::Text(t) | Value::Code(t)) if !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit()) => t
            .parse::<Decimal>()
            .map(Value::Decimal)
            .or_else(|_| unsupported("a text of digits beyond the Decimal range read as Decimal")),
        Scalar::Val(Value::Text(t) | Value::Code(t)) if !t.is_empty() && t.bytes().all(|b| b.is_ascii_alphabetic()) => {
            Err(conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.Decimal18"))
        }
        // types/lt-aj-probes/AjScalars.al:150-209: "100.00" reads 100, "-3.5" reads -3.5, " 7 " (blanks around digits) reads 7;
        // "" and "1e2" raise the conversion error. Other shapes (blanks around a sign or a point, a plus sign) are not pinned.
        Scalar::Val(Value::Text(t) | Value::Code(t)) if signed_point_number(t) || blank_padded_digits(t) => t
            .trim_matches(' ')
            .parse::<Decimal>()
            .map(Value::Decimal)
            .or_else(|_| unsupported("a numeric text beyond the Decimal range read as Decimal")),
        Scalar::Val(Value::Text(t) | Value::Code(t)) if t.is_empty() || exponent_digits(t) => {
            Err(conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.Decimal18"))
        }
        other => unsupported(format!(
            "reading {} as Decimal (conversion not pinned yet)",
            other.describe()
        )),
    }
}

fn as_text(s: &Scalar) -> Result<Value, Signal> {
    match s {
        Scalar::Val(Value::Text(t) | Value::Code(t)) => Ok(Value::Text(t.clone())),
        // A Char is a number in JSON, so its text is the code (P101ValueSetValue.al:P101_SetValue_Char `text0` = 65).
        Scalar::Val(Value::Char(c)) => Ok(Value::Text(c.to_string().into())),
        Scalar::Null => Err(conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.NavText")),
        // json/TestProbeJsonContracts.al:TypedGetters_WrongKinds: a number and a Boolean read as text (`True`).
        Scalar::Val(Value::Integer(n)) => Ok(Value::Text(n.to_string().into())),
        Scalar::Val(Value::BigInteger(n)) => Ok(Value::Text(n.to_string().into())),
        // A whole number read as a double keeps the one decimal place only in the written JSON, not in its text
        // (json/lt-json-overloads/P88ValueReadFrom.al:ReadFrom_Scalars `exp`: text 1000, json 1000.0).
        Scalar::Val(Value::Decimal(d)) if d.scale() == 1 && d.fract().is_zero() => {
            Ok(Value::Text(d.trunc().to_string().into()))
        }
        Scalar::Val(Value::Decimal(d)) => Ok(Value::Text(d.to_string().into())),
        Scalar::Val(Value::Boolean(b)) => Ok(Value::Text(if *b { "True" } else { "False" }.into())),
        Scalar::Raw(t) => Ok(Value::Text(t.clone().into())),
        // A Byte, an Option, a Duration, a Date, a Time and a DateTime read as the text they are written as, without the quotes
        // (json/lt-json-typed-overloads/P101ValueSetValue.al: P101_SetValue_Byte :120-125, _Option :268-271, _Date :299-304,
        // _Time :332-337, _DateTime :360-363, _Duration :391-396). A Time or DateTime with milliseconds reads as written too
        // (json/lt-json-overloads/P88DateTimeReadback.al:121-210: `14:30:15.123`, `2024-03-15T14:30:15.1230000Z`).
        Scalar::Val(
            v @ (Value::Byte(_)
            | Value::Option(..)
            | Value::Duration(_)
            | Value::Date(..)
            | Value::Time(_)
            | Value::DateTime(_)),
        ) => {
            let mut out = String::new();
            write(&new_handle(JsonNode::Scalar(Scalar::Val(v.clone()))), &mut out)?;
            let text = out.strip_prefix('"').and_then(|t| t.strip_suffix('"')).unwrap_or(&out);
            Ok(Value::Text(text.to_string().into()))
        }
        other => unsupported(format!(
            "reading {} as Text (conversion not pinned yet)",
            other.describe()
        )),
    }
}

fn as_code(s: &Scalar) -> Result<Value, Signal> {
    match as_text(s)? {
        Value::Text(t) => crate::ops::convert(Value::Text(t), &crate::value::RtType::Code(None)),
        other => Ok(other),
    }
}

fn as_char(s: &Scalar) -> Result<Value, Signal> {
    match s {
        Scalar::Val(Value::Char(c)) => Ok(Value::Char(*c)),
        Scalar::Val(Value::Text(t) | Value::Code(t)) => {
            let u: Vec<u16> = t.encode_utf16().collect();
            match u.as_slice() {
                [c] => Ok(Value::Char(*c)),
                // A text of three or four characters does not convert (json/lt-json-typed-overloads/P101PathValues.al:164
                // `text_as_char` 'xyz', :203 'text'), and neither do an empty text and texts of 5 and 36 characters
                // (json/JsonValueGetters.al:169 `text_aschar` 'hello', NumericTextGetters `empty_aschar`, `long_aschar`).
                _ if matches!(u.len(), 0 | 3 | 4 | 5 | 36) => Err(conversion(NAV_JSON_VALUE, "System.Char")),
                _ => unsupported(format!("reading text of length {} as Char", u.len())),
            }
        }
        // A whole number is the character of that code (P101PathValues.al:162 `int_as_char` 8364 is €, :170 0, :202 5;
        // json/lt-json-overloads/P88ObjectGetCharValues.al:28 7). Codes from the surrogates up are not pinned.
        Scalar::Val(Value::Integer(n)) if (0..0xD800).contains(n) => Ok(Value::Char(*n as u16)),
        Scalar::Val(Value::BigInteger(n)) if (0..0xD800).contains(n) => Ok(Value::Char(*n as u16)),
        // P101PathValues.al:167 `bool_as_char`.
        Scalar::Val(Value::Boolean(_)) => Err(conversion(NAV_JSON_VALUE, "System.Char")),
        other => unsupported(format!(
            "reading {} as Char (conversion not pinned yet)",
            other.describe()
        )),
    }
}

fn as_date(s: &Scalar) -> Result<Value, Signal> {
    match s {
        Scalar::Val(Value::Date(d, c)) => Ok(Value::Date(*d, *c)),
        // Only `yyyy-MM-dd` reads as a Date (json/TestProbeJsonContracts.al:IsoText_ReadAsDateTimeKinds: a date-time
        // text, `15.01.2024` and `01/15/2024` do not).
        Scalar::Val(Value::Text(t)) => match crate::iso::parse_date(t).filter(|_| t.len() == 10) {
            Some(d) => Ok(Value::Date(d, false)),
            None => Err(conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.NavDate")),
        },
        // A Time or DateTime held reads as no Date (json/lt-json-overloads/P88DateTimeReadback.al:121-210, Time_Readback and
        // DateTime_Readback `_asdate`).
        Scalar::Val(
            Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_) | Value::Time(_) | Value::DateTime(_),
        ) => Err(conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.NavDate")),
        other => unsupported(format!(
            "reading {} as Date (conversion not pinned yet)",
            other.describe()
        )),
    }
}

fn as_datetime(s: &Scalar) -> Result<Value, Signal> {
    match s {
        Scalar::Val(Value::DateTime(d)) => Ok(Value::DateTime(*d)),
        Scalar::Val(Value::Text(t)) => match crate::iso::parse_datetime(t) {
            Some(d) => Ok(Value::DateTime(d)),
            None => Err(conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.NavDateTime")),
        },
        // A Date or Time held reads as no DateTime (P88DateTimeReadback.al:78-163, Date_Readback and Time_Readback `_asdatetime`).
        Scalar::Val(
            Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_) | Value::Date(..) | Value::Time(_),
        ) => Err(conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.NavDateTime")),
        other => unsupported(format!(
            "reading {} as DateTime (conversion not pinned yet)",
            other.describe()
        )),
    }
}

fn as_time(s: &Scalar) -> Result<Value, Signal> {
    match s {
        // An undefined Time is written `"00:00:00"` and reads back as midnight (json/lt-json-overloads/P88DateTimeReadback.al:
        // 121-163 Time_Readback `time_blank_astime`).
        Scalar::Val(Value::Time(0)) => Ok(Value::Time(1)),
        Scalar::Val(Value::Time(t)) => Ok(Value::Time(*t)),
        Scalar::Val(Value::Text(t)) => match crate::iso::parse_time(t) {
            Some(t) => Ok(Value::Time(t)),
            None => Err(conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.NavTime")),
        },
        // A Date or DateTime held reads as no Time (P88DateTimeReadback.al:78-119, :166-210, `_astime`).
        Scalar::Val(
            Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_) | Value::Date(..) | Value::DateTime(_),
        ) => Err(conversion(NAV_JSON_VALUE, "Microsoft.Dynamics.Nav.Runtime.NavTime")),
        other => unsupported(format!(
            "reading {} as Time (conversion not pinned yet)",
            other.describe()
        )),
    }
}

type Coerce = fn(&Scalar) -> Result<Value, Signal>;

// ---------------------------------------------------------------- natives

fn key_text(v: &Value) -> Result<String, Signal> {
    crate::native::text_of(v)
        .map(String::from)
        .ok_or_else(|| Signal::Unsupported("JSON key that is not text".into()))
}

fn object_get_child(h: &Handle, key: &Value) -> Result<Option<Handle>, Signal> {
    let key = key_text(key)?;
    with_node(h, |n| match n {
        JsonNode::Object(kids) => Ok(kids.iter().find(|(k, _)| *k == key).map(|(_, c)| c.clone())),
        _ => unsupported("JsonObject method on a JSON value that is not an object"),
    })
}

fn array_get_child(h: &Handle, index: &Value) -> Result<Option<Handle>, Signal> {
    let i = match index {
        Value::Integer(i) => i64::from(*i),
        other => return unsupported(format!("JSON array index {}", other.describe())),
    };
    with_node(h, |n| match n {
        JsonNode::Array(kids) => Ok(usize::try_from(i).ok().and_then(|i| kids.get(i)).cloned()),
        _ => unsupported("JsonArray method on a JSON value that is not an array"),
    })
}

fn object_add(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let key = key_text(&a[0])?;
    if object_get_child(&h, &a[0])?.is_some() {
        // json/TestJsonRuntimeContracts.al:JsonObject_Errors_TextAndCode
        return Err(error(
            format!("The JSON object already contains the key '{key}'."),
            "DuplicateJsonProperty",
        ));
    }
    let child = attach(&h, node_for(&a[1])?);
    with_node(&h, |n| {
        if let JsonNode::Object(kids) = n {
            kids.push((key, child));
        }
    });
    Ok(Value::Boolean(true))
}

/// `Replace(Key, Value)`: removes the key and adds it again at the END; true when the key was there.
/// A missing key answers false when the result is used and raises "There is no property with the '<key>'
/// key on the JSON object." (JsonPropertyNotFound) as a statement (json/TestJsonObjectReplaceKeysContracts.al).
fn object_replace(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let key = key_text(&a[0])?;
    let Some(old) = object_get_child(&h, &a[0])? else {
        return if c.ret_used() {
            Ok(Value::Boolean(false))
        } else {
            Err(missing_key(&a[0]))
        };
    };
    let new = node_for(&a[1])?;
    with_node(&h, |n| {
        if let JsonNode::Object(kids) = n {
            kids.retain(|(k, _)| *k != key);
        }
    });
    set_parent(&old, None);
    let child = attach(&h, new);
    with_node(&h, |n| {
        if let JsonNode::Object(kids) = n {
            kids.push((key, child));
        }
    });
    Ok(Value::Boolean(true))
}

/// `Keys()`: the keys in insertion order, as a new list.
fn object_keys(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let keys = with_node(&h, |n| match n {
        JsonNode::Object(kids) => Ok(kids
            .iter()
            .map(|(k, _)| Value::Text(k.clone().into()))
            .collect::<Vec<_>>()),
        _ => unsupported("JsonObject.Keys on a JSON value that is not an object"),
    })?;
    Ok(Value::List(Rc::new(std::cell::RefCell::new(keys.into()))))
}

/// `Values()`: the values as tokens, in order, as the object was at the call (json/lt-json-typed-overloads/P101PathValues.al:
/// P101_Values `count` 6 and `count_after_add_snapshot`; json/lt-json-overloads/P88ObjectGetCharValues.al:50-58).
fn object_values(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let values = with_node(&h, |n| match n {
        JsonNode::Object(kids) => Ok(kids
            .iter()
            .map(|(_, c)| tagged("JsonToken", c.clone()))
            .collect::<Vec<_>>()),
        _ => unsupported("JsonObject.Values on a JSON value that is not an object"),
    })?;
    Ok(Value::List(Rc::new(std::cell::RefCell::new(values.into()))))
}

/// `RemoveAt(Index)`: the element goes, the rest moves up (json/JsonArrayBasics.al:182-193 `text` [1,3,2]; json/lt-json-overloads/
/// P88ArrayIndex.al:126 answers true). An index outside the array raises "Index out of bounds." (JsonArrayBasics.al:186-195),
/// pinned in statement form only.
fn array_remove_at(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let i = array_index(&a[0])?;
    let removed = with_node(&h, |n| match n {
        JsonNode::Array(kids) => Ok(usize::try_from(i)
            .ok()
            .filter(|i| *i < kids.len())
            .map(|i| kids.remove(i))),
        _ => unsupported("JsonArray.RemoveAt on a JSON value that is not an array"),
    })?;
    match removed {
        Some(child) => {
            set_parent(&child, None);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => unsupported("JsonArray.RemoveAt outside the array with the result used (not pinned)"),
        None => Err(index_out_of_bounds()),
    }
}

fn object_contains(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    Ok(Value::Boolean(object_get_child(&h, &a[0])?.is_some()))
}

fn object_get(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    match object_get_child(&h, &a[0])? {
        Some(child) => {
            a[1] = tagged("JsonToken", child);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        // As a statement a missing key raises JsonPropertyNotFound and leaves the token as it was
        // (misc/lt-y14-triage/Y14Small.al:389, corpus master).
        None => Err(missing_key(&a[0])),
    }
}

fn object_remove(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let key = key_text(&a[0])?;
    let removed = with_node(&h, |n| match n {
        JsonNode::Object(kids) => Ok(kids.iter().position(|(k, _)| *k == key).map(|i| kids.remove(i).1)),
        _ => unsupported("JsonObject.Remove on a JSON value that is not an object"),
    })?;
    match removed {
        Some(child) => {
            set_parent(&child, None);
            Ok(Value::Boolean(true))
        }
        // A missing key answers false and raises nothing, also in statement form, in a TryFunction and on an empty object
        // (json/JsonObjectBasics.al:397-425 QV_RemoveMissingKey_StatementForm; AL Runner issue 5680).
        None => Ok(Value::Boolean(false)),
    }
}

/// `GetX(Key [, DefaultIfNotFound])`. The corpus pins that a missing key throws even with
/// `DefaultIfNotFound = false` (json/TestJsonTypedGetters.al JsonObject_GetBoolean_MissingKey_Throws)
/// but not the text, nor the `true` case.
fn object_typed(r: Option<&mut Value>, a: &mut [Value], coerce: Coerce) -> Result<Value, Signal> {
    let h = recv(r)?;
    match object_get_child(&h, &a[0])? {
        Some(child) => {
            // GetText of a null, an object or an array raises nothing (json/TestProbeJsonContracts.al:
            // TypedGetters_WrongKinds `gettext_of_null`, `gettext_of_object`); the null reads as the empty text and the
            // other two as their compact JSON.
            if coerce as *const () == as_text as *const () {
                match with_node(&child, |n| match n {
                    JsonNode::Scalar(Scalar::Null) => Some(None),
                    JsonNode::Scalar(_) => None,
                    _ => Some(Some(())),
                }) {
                    Some(None) => return Ok(Value::Text(String::new().into())),
                    Some(Some(())) => {
                        let mut out = String::new();
                        write(&child, &mut out)?;
                        return Ok(Value::Text(out.into()));
                    }
                    None => {}
                }
            }
            coerce(&scalar_of(&child)?)
        }
        None if matches!(a.get(1), Some(Value::Boolean(true))) => {
            unsupported("JsonObject.GetX of a missing key with DefaultIfNotFound = true (not pinned)")
        }
        None => Err(missing_key(&a[0])),
    }
}

/// json/TestJsonRuntimeContracts.al:JsonObject_Errors_TextAndCode
fn missing_key(key: &Value) -> Signal {
    match crate::native::text_of(key) {
        Some(k) => error(
            format!("There is no property with the '{k}' key on the JSON object."),
            "JsonPropertyNotFound",
        ),
        None => Signal::Unsupported("JSON key that is not text".into()),
    }
}

/// The name .NET (Newtonsoft) gives the type of a node: `Object`, `Integer`, `String`, ...
fn token_type_name(h: &Handle) -> Result<&'static str, Signal> {
    with_node(h, |n| match n {
        JsonNode::Object(_) => Ok("Object"),
        JsonNode::Array(_) => Ok("Array"),
        JsonNode::Scalar(Scalar::Null) => Ok("Null"),
        JsonNode::Scalar(Scalar::Val(v)) => match v {
            Value::Integer(_) | Value::BigInteger(_) | Value::Byte(_) | Value::Option(..) => Ok("Integer"),
            Value::Decimal(_) => Ok("Float"),
            Value::Text(_) | Value::Code(_) | Value::Char(_) => Ok("String"),
            Value::Boolean(_) => Ok("Boolean"),
            other => unsupported(format!("the JSON type name of {}", other.describe())),
        },
        JsonNode::Scalar(Scalar::Undefined) => unsupported("the JSON type name of an undefined value"),
        JsonNode::Scalar(Scalar::Raw(_)) => Ok("Float"),
    })
}

/// json/TestProbeJsonContracts.al:TypedGetters_OutOfRangeAndMissing (index 5 and -1 of a one element array).
fn index_out_of_bounds() -> Signal {
    error("Index out of bounds.", "IndexOutOfBounds")
}

fn array_typed(r: Option<&mut Value>, a: &mut [Value], coerce: Coerce) -> Result<Value, Signal> {
    let h = recv(r)?;
    match array_get_child(&h, &a[0])? {
        Some(child) => coerce(&scalar_of(&child)?),
        None => Err(index_out_of_bounds()),
    }
}

/// A failed conversion of a typed getter in the language in force: the .NET text of [`conversion`] in 1033; in 1031 the
/// German text, pinned for a Date (misc/lt-system-functions/EvalLastError.al:463-481, BT_Evaluate_DottedDate_Language1031
/// `1031 json`). Any other conversion error in another language is not pinned and is refused.
fn in_language(language: i32, result: Result<Value, Signal>) -> Result<Value, Signal> {
    match result {
        Err(Signal::Error(e)) if e.code == "Conversion" && language != 1033 => {
            let date = format!("Unable to convert from {NAV_JSON_VALUE} to Microsoft.Dynamics.Nav.Runtime.NavDate.");
            if language == 1031 && e.message == date {
                return Err(error(
                    format!(
                        "Die Konvertierung von {NAV_JSON_VALUE} in Microsoft.Dynamics.Nav.Runtime.NavDate ist nicht m\u{f6}glich."
                    ),
                    "Conversion",
                ));
            }
            unsupported(format!(
                "the text of the JSON conversion error `{}` in the language {language} (not pinned)",
                e.message
            ))
        }
        other => other,
    }
}

macro_rules! typed_getters {
    ($($obj:ident, $arr:ident, $val:ident => $coerce:path;)*) => {$(
        fn $obj(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
            in_language(c.language(), object_typed(r, a, $coerce))
        }
        fn $arr(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
            in_language(c.language(), array_typed(r, a, $coerce))
        }
        #[allow(dead_code)]
        fn $val(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
            let h = recv(r)?;
            in_language(c.language(), $coerce(&scalar_of(&h)?))
        }
    )*};
}

typed_getters! {
    obj_boolean, arr_boolean, val_boolean => as_boolean;
    obj_integer, arr_integer, val_integer => as_integer;
    obj_biginteger, arr_biginteger, val_biginteger => as_biginteger;
    obj_byte, arr_byte, val_byte => as_byte;
    obj_option, arr_option, val_option => as_option;
    obj_duration, arr_duration, val_duration => as_duration;
    obj_decimal, arr_decimal, val_decimal => as_decimal;
    obj_text, arr_text, val_text => as_text;
    obj_char, arr_char, val_char => as_char;
    obj_date, arr_date, val_date => as_date;
    obj_datetime, arr_datetime, val_datetime => as_datetime;
    obj_time, arr_time, val_time => as_time;
}

/// `JsonValue.AsText` of a Boolean is `true` or `false`, where `GetText` of a property or an element says `True`
/// (json/lt-json-typed-overloads/P101ValueSetValue.al:P101_SetValue_Boolean against TestProbeJsonContracts.al:TypedGetters_WrongKinds).
fn val_text_json(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    match scalar_of(&recv(r)?)? {
        Scalar::Val(Value::Boolean(b)) => Ok(Value::Text((if b { "true" } else { "false" }).into())),
        s => as_text(&s),
    }
}

fn val_code(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    as_code(&scalar_of(&recv(r)?)?)
}

/// `GetObject` / `GetArray` / `AsObject` / `AsArray`: the node itself (a reference, like `Get`).
fn child_as(child: Handle, want: &str, tag: &str) -> Result<Value, Signal> {
    let ok = with_node(&child, |n| {
        matches!(
            (n, want),
            (JsonNode::Object(_), "object") | (JsonNode::Array(_), "array")
        )
    });
    if ok {
        Ok(tagged(tag, child))
    } else {
        unsupported(format!(
            "{tag} requested from {} (error text not pinned yet)",
            kind_name(&child)
        ))
    }
}

/// `GetObject` / `GetArray` of a key: a child of another kind is a conversion error between
/// .NET token types (json/TestJsonRuntimeContracts.al:JsonObject_Errors_TextAndCode).
fn obj_get_kind(r: Option<&mut Value>, a: &mut [Value], want: &str, tag: &str, to: &str) -> Result<Value, Signal> {
    let h = recv(r)?;
    match object_get_child(&h, &a[0])? {
        Some(c) => {
            let ok = with_node(&c, |n| {
                matches!(
                    (n, want),
                    (JsonNode::Object(_), "object") | (JsonNode::Array(_), "array")
                )
            });
            if ok {
                Ok(tagged(tag, c))
            } else {
                Err(conversion(token_type_name(&c)?, to))
            }
        }
        None if matches!(a.get(1), Some(Value::Boolean(true))) => {
            unsupported("JsonObject.GetObject or GetArray of a missing key with DefaultIfNotFound = true (not pinned)")
        }
        None => Err(missing_key(&a[0])),
    }
}

fn obj_get_object(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    obj_get_kind(r, a, "object", "JsonObject", "Object")
}

fn obj_get_array(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    obj_get_kind(r, a, "array", "JsonArray", "Array")
}

fn arr_get_object(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    match array_get_child(&h, &a[0])? {
        Some(c) => child_as(c, "object", "JsonObject"),
        None => Err(index_out_of_bounds()),
    }
}

fn arr_get_array(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    match array_get_child(&h, &a[0])? {
        Some(c) => child_as(c, "array", "JsonArray"),
        None => Err(index_out_of_bounds()),
    }
}

fn array_add(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let child = attach(&h, node_for(&a[0])?);
    with_node(&h, |n| match n {
        JsonNode::Array(kids) => {
            kids.push(child);
            Ok(Value::Boolean(true))
        }
        _ => unsupported("JsonArray.Add on a JSON value that is not an array"),
    })
}

/// The index a `JsonArray` method takes, as a number.
fn array_index(v: &Value) -> Result<i64, Signal> {
    match v {
        Value::Integer(i) => Ok(i64::from(*i)),
        other => unsupported(format!("JSON array index {}", other.describe())),
    }
}

/// `Insert(Index, Value)`: the value goes in before that index; an index from 0 to the count is allowed, another raises
/// `Index out of bounds.` / IndexOutOfBounds (json/lt-json-typed-overloads/P101ArrayInsert.al:P101_Insert_Boolean `ins_oob`, and the
/// same for every other type).
fn array_insert(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let i = array_index(&a[0])?;
    let count = with_node(&h, |n| match n {
        JsonNode::Array(kids) => Ok(kids.len() as i64),
        _ => unsupported("JsonArray.Insert on a JSON value that is not an array"),
    })?;
    if i < 0 || i > count {
        return Err(index_out_of_bounds());
    }
    let child = attach(&h, node_for(&a[1])?);
    with_node(&h, |n| {
        if let JsonNode::Array(kids) = n {
            kids.insert(i as usize, child);
        }
    });
    Ok(Value::Null)
}

/// `Set(Index, Value)`: the value replaces the one at that index; an index from 0 to the count minus one is allowed
/// (P101ArraySet.al:P101_Set_Boolean `set_oob` is the count itself).
fn array_set(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let i = array_index(&a[0])?;
    let count = with_node(&h, |n| match n {
        JsonNode::Array(kids) => Ok(kids.len() as i64),
        _ => unsupported("JsonArray.Set on a JSON value that is not an array"),
    })?;
    if i < 0 || i >= count {
        return Err(index_out_of_bounds());
    }
    let child = attach(&h, node_for(&a[1])?);
    let old = with_node(&h, |n| match n {
        JsonNode::Array(kids) => Some(std::mem::replace(&mut kids[i as usize], child)),
        _ => None,
    });
    if let Some(old) = old {
        set_parent(&old, None);
    }
    Ok(Value::Null)
}

/// `IndexOf(Value)`: the index of the first element that is the value, -1 when there is none. A typed value compares by value
/// (P101ArrayIndexOf.al:P101_IndexOf_Boolean `dup_first`, `empty`); a JsonObject or JsonArray is found only when the array holds that
/// very object, never an equal copy (:P101_IndexOf_JsonObject, :P101_IndexOf_JsonArray). A typed value against an element of
/// another type, and a JsonToken or JsonValue that holds a scalar (BC finds the token it was added as, which a copied scalar
/// cannot show), are refused.
fn array_index_of(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let kids: Vec<Handle> = with_node(&h, |n| match n {
        JsonNode::Array(kids) => Ok(kids.clone()),
        _ => unsupported("JsonArray.IndexOf on a JSON value that is not an array"),
    })?;
    let needle = &a[0];
    let found = if matches!(needle, Value::Library(..)) {
        let target = handle_of(needle)?;
        if with_node(&target, |n| matches!(n, JsonNode::Scalar(_))) {
            return unsupported(
                "JsonArray.IndexOf of a JsonToken or JsonValue holding a scalar (which element it names is not modelled)",
            );
        }
        kids.iter().position(|k| Rc::ptr_eq(k, &target))
    } else {
        let mut at = None;
        for (i, k) in kids.iter().enumerate() {
            let same = with_node(k, |n| match n {
                JsonNode::Scalar(Scalar::Val(v)) => {
                    // A Text against a Code element and the other way round: texts that differ also when the case is
                    // ignored do not match (json/lt-json-typed-overloads/P101CodeLabel.al:115-120, `indexof_label` 1 and
                    // `indexof_text10` 2 pass the Code "ABC" at 0). Texts that differ only in case, or are equal, are not pinned.
                    let strings = matches!(
                        (&*v, needle),
                        (Value::Text(_), Value::Code(_)) | (Value::Code(_), Value::Text(_))
                    );
                    if strings {
                        let (Some(x), Some(y)) = (crate::native::text_of(v), crate::native::text_of(needle)) else {
                            return None;
                        };
                        if x.to_lowercase() == y.to_lowercase() {
                            return None;
                        }
                        Some(false)
                    } else if std::mem::discriminant(v) == std::mem::discriminant(needle) {
                        Some(crate::ops::compare(v, needle).is_ok_and(|o| o == std::cmp::Ordering::Equal))
                    } else {
                        // json/lt-json-typed-readback/JsonTypedIndexOfCross.al: a stored Integer or Byte matches an Integer,
                        // Byte or BigInteger of the same number (:37-38 `byte_7`, `big_7`; :98-100 `int_255`, `byte_0`); an Integer
                        // never matches a Decimal or a Text (:36 `dec_7`, :39 `text_7`), nor a Decimal an Integer, Byte or
                        // BigInteger (:67-69). Other pairs are not pinned.
                        let whole = |x: &Value| match x {
                            Value::Integer(n) => Some(i64::from(*n)),
                            Value::Byte(n) => Some(i64::from(*n)),
                            Value::BigInteger(n) => Some(*n),
                            Value::Char(n) => Some(i64::from(*n)),
                            _ => None,
                        };
                        match (&*v, needle) {
                            // A stored Char and an Integer compare as numbers (json/lt-json-typed-readback/JsonTypedCharRead.al:
                            // 132 `idx_int_70` 0, :134 `idx_missing` passes the Integer 2 for Char 72).
                            (Value::Char(_), Value::Integer(_)) | (Value::Integer(_), Value::Char(_)) => {
                                Some(whole(v) == whole(needle))
                            }
                            (
                                Value::Integer(_) | Value::Byte(_),
                                Value::Integer(_) | Value::Byte(_) | Value::BigInteger(_),
                            ) => Some(whole(v) == whole(needle)),
                            (Value::Integer(_), Value::Decimal(_) | Value::Text(_))
                            | (Value::Decimal(_), Value::Integer(_) | Value::Byte(_) | Value::BigInteger(_)) => {
                                Some(false)
                            }
                            _ => None,
                        }
                    }
                }
                _ => Some(false),
            });
            match same {
                Some(true) => {
                    at = Some(i);
                    break;
                }
                Some(false) => {}
                None => {
                    return unsupported("JsonArray.IndexOf of a value against an element of another type (not pinned)");
                }
            }
        }
        at
    };
    Ok(Value::Integer(found.map_or(-1, |i| i as i32)))
}

/// The tokens of a JsonArray as `JsonToken` values, for `foreach`: in order, and as the array was when the
/// loop started (an element added inside is not visited; corpus types/PVLabelSplitForeach.al on the
/// staging branch). `None` for any other value.
pub fn array_items(v: &Value) -> Option<Vec<Value>> {
    let Value::Library(name, h) = v else { return None };
    // A never-used or cleared JsonArray has no items (json/JsonArrayBasics.al:274-281, ForeachAndPath).
    if matches!(&*h.borrow(), LibraryValue::Empty) && name.eq_ignore_ascii_case("jsonarray") {
        return Some(Vec::new());
    }
    if !matches!(&*h.borrow(), LibraryValue::Json(_)) {
        return None;
    }
    with_node(h, |n| match n {
        JsonNode::Array(kids) => Some(kids.iter().map(|k| tagged("JsonToken", k.clone())).collect()),
        _ => None,
    })
}

fn array_count(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    with_node(&h, |n| match n {
        JsonNode::Array(kids) => Ok(Value::Integer(kids.len() as i32)),
        _ => unsupported("JsonArray.Count on a JSON value that is not an array"),
    })
}

fn array_get(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    match array_get_child(&h, &a[0])? {
        Some(child) => {
            a[1] = tagged("JsonToken", child);
            Ok(Value::Boolean(true))
        }
        None if c.ret_used() => Ok(Value::Boolean(false)),
        // json/TestJsonRuntimeContracts.al:JsonObject_Errors_TextAndCode
        None => Err(error("Index out of bounds.", "IndexOutOfBounds")),
    }
}

/// Replaces the content of the receiver's node with a parsed tree of the wanted kind.
/// Invalid JSON is reported as `false` when the result is used (docs: "returns false if
/// the operation was not successful"; no corpus test feeds invalid JSON, see probes-needed.md).
fn read_from(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value], want: Option<&str>) -> Result<Value, Signal> {
    // ReadFrom(InStream) reads the rest of the stream and parses it like the text form.
    let text = if matches!(&a[0], Value::Library(_, h) if matches!(&*h.borrow(), LibraryValue::InStream(_))) {
        let text = crate::stream::read_serialised_text(&mut a[0])?;
        // A stream whose first token is a string (what is left after a partial read of `{"a":2}`)
        // is refused by the token reader (streams/ProbeJsonAndInStreamBytes.al:ReadFrom_InStream_AfterPartialRead_ReadsTheRest).
        // A JsonToken reads a stream that holds one whole string as that value (json/lt-json-typed-overloads/P101Streams.al:166-167,
        // `text_ok` Yes and `text_isvalue` Yes for `"text"`).
        let whole_string_into_token = want.is_none() && c.ret_used() && parse(&text)?.is_some();
        if text.trim_start().starts_with('"') && !whole_string_into_token {
            if c.ret_used() {
                return unsupported(
                    "the result of ReadFrom(InStream) on a stream that starts with a string token (not pinned)",
                );
            }
            return Err(error(
                "JSON tokens of type 'String' are not supported by this implementation.",
                "InvalidJsonType",
            ));
        }
        text
    } else {
        match (&a[0], crate::native::text_of(&a[0])) {
            (_, Some(t)) => t.to_string(),
            // A collected error's Callstack ends with the untracked mark (`collected_call_stack`), and Microsoft's Error
            // Message Management writes it into a JSON text and reads it back (ParseErrorText; Tests-General Journal, ERM
            // General Journal UT, GeneralJournalNumberOfJournalLines and 65 more). A mark inside a string value only hides
            // that string's text: the parse is exact and the value read keeps the mark. A mark anywhere else is refused.
            (Value::Text(t) | Value::Code(t), None) if marks_only_in_string_values(t.as_str()) => t.to_string(),
            _ => return unsupported("ReadFrom with an argument that is not text"),
        }
    };
    read_text_into(c.ret_used(), r, &text, want)
}

/// Whether every marker (`CALL_STACK_MARK`, `CARRIED_TEXT_MARK`) of `text` sits inside a JSON string that is a value, not
/// a key, and the text is JSON. A carried text's mark stands for content that is not known, so it is refused here too.
fn marks_only_in_string_values(text: &str) -> bool {
    if text.contains(crate::native::CARRIED_TEXT_MARK) {
        return false;
    }
    let mark = crate::native::CALL_STACK_MARK;
    let mut chars = text.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if c == mark {
            return false;
        }
        if c != '"' {
            continue;
        }
        let mut marked = false;
        let mut closed = false;
        while let Some((_, c)) = chars.next() {
            match c {
                '\\' => {
                    chars.next();
                }
                '"' => {
                    closed = true;
                    break;
                }
                c if c == mark => marked = true,
                _ => {}
            }
        }
        if !closed {
            return false;
        }
        // A string followed by `:` is a key.
        if marked {
            while chars.next_if(|(_, c)| c.is_whitespace()).is_some() {}
            if chars.peek().is_some_and(|(_, c)| *c == ':') {
                return false;
            }
        }
    }
    matches!(parse(text), Ok(Some(_)))
}

/// `Evaluate(JsonArray, Text)`: the text is read as `ReadFrom` reads it, and a text that is no array leaves the array as it was
/// and fails with ReadFrom's statement error, which the form with a used result leaves as the last error and answers false
/// (types/lt-uf-evaluate-json-rfc1123/UfEvaluateJsonRfc.al:88-176: an array, an empty one and one with blanks around it are
/// read; an object, a number, a string and null fail InvalidJsonType, `[1,`, the empty text and `not json` InvalidJsonData;
/// :178-194 the array is usable afterwards; al-runner issue 6175).
pub fn evaluate_array(c: &mut dyn Ctx, slot: &mut Value, text: &str) -> Result<Value, Signal> {
    // Pinned by types/lt-uf-evaluate-json-rfc1123/UfEvaluateJsonRfc.al JsonArray_Texts, JsonArray_UsedAfterEvaluate (was assumption `evaluate-json-array`).
    match read_text_into(false, Some(slot), text, Some("array")) {
        Err(Signal::Error(e)) if c.ret_used() => {
            c.set_last_error(*e);
            Ok(Value::Boolean(false))
        }
        other => other,
    }
}

/// Replaces the content of the receiver's node with `text` parsed as the wanted kind; a text that is not one answers false
/// when `ret_used`, else raises the statement form's error.
fn read_text_into(ret_used: bool, r: Option<&mut Value>, text: &str, want: Option<&str>) -> Result<Value, Signal> {
    let mut r = r;
    let mut h = recv(r.as_deref_mut())?;
    // A variable whose node sits inside a container (it was added) reads into a new node of its own: the container keeps what it
    // holds (json/lt-json-typed-overloads/P101ArrayAdd.al:P101_Add_JsonToken, `Tk.ReadFrom(..)` after `A.Add(Tk)` leaves `A` as it was).
    if parent_link(&h).and_then(|w| w.upgrade()).is_some() {
        if let Some(slot @ Value::Library(..)) = r {
            let Value::Library(name, _) = &*slot else {
                unreachable!()
            };
            let fresh = new_handle(JsonNode::Scalar(Scalar::Null));
            *slot = Value::Library(name.clone(), fresh.clone());
            h = fresh;
        }
    }
    let syntax = parse(text)?;
    let parsed = syntax.clone().filter(|p| {
        with_node(p, |n| {
            matches!(
                (n, want),
                (JsonNode::Object(_), Some("object"))
                    | (JsonNode::Array(_), Some("array"))
                    | (JsonNode::Scalar(_), Some("value"))
                    | (_, None)
            )
        })
    });
    let Some(parsed) = parsed else {
        return if ret_used {
            Ok(Value::Boolean(false))
        } else if syntax.is_none() {
            // json/TestJsonRuntimeContracts.al:JsonObject_ReadFrom_InvalidJson_ReturnsFalse_AndStatementFormThrows
            Err(error(
                "The data does not represent a valid JSON token.",
                "InvalidJsonData",
            ))
        } else {
            // json/TestProbeJsonContracts.al:ReadFrom_ArrayIntoObject_Statement, ReadFrom_ObjectIntoArray_Statement.
            // A scalar names its own type (misc/lt-probes-needed-1/PnAJsonReadKinds.al:StatementForm_WrongKind: Integer, String,
            // Null, Boolean); a fractional number is not pinned.
            let kind = syntax.as_ref().map(|p| {
                with_node(p, |n| match n {
                    JsonNode::Array(_) => Some("Array"),
                    JsonNode::Object(_) => Some("Object"),
                    JsonNode::Scalar(Scalar::Null) => Some("Null"),
                    JsonNode::Scalar(Scalar::Val(Value::Text(_))) => Some("String"),
                    JsonNode::Scalar(Scalar::Val(Value::Boolean(_))) => Some("Boolean"),
                    JsonNode::Scalar(Scalar::Val(Value::BigInteger(_) | Value::Integer(_))) => Some("Integer"),
                    _ => None,
                })
            });
            let Some(Some(kind)) = kind else {
                return unsupported(
                    "ReadFrom of a number with a fraction into an object or array (token type name not pinned)",
                );
            };
            Err(error(
                format!("JSON tokens of type '{kind}' are not supported by this implementation."),
                "InvalidJsonType",
            ))
        };
    };
    let LibraryValue::Json(JsonCell { node, .. }) = std::mem::take(&mut *parsed.borrow_mut()) else {
        unreachable!("parse builds JSON nodes")
    };
    let parent = parent_link(&h);
    *h.borrow_mut() = LibraryValue::Json(JsonCell {
        node,
        parent,
        unpinned: None,
    });
    // The children were adopted by the temporary root; re-point them at the receiver.
    adopt_children(&h);
    Ok(Value::Boolean(true))
}

fn parent_link(h: &Handle) -> Option<Weak<RefCell<LibraryValue>>> {
    match &*h.borrow() {
        LibraryValue::Json(c) => c.parent.clone(),
        _ => None,
    }
}

fn object_read_from(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    read_from(c, r, a, Some("object"))
}
fn array_read_from(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    read_from(c, r, a, Some("array"))
}
fn any_read_from(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    read_from(c, r, a, None)
}
/// A JsonValue takes scalars only: an object or an array is refused like a scalar into an object (misc/lt-probes-needed-1/
/// PnAJsonReadKinds.al:StatementForm_WrongKind `val<-object`, `val<-array`).
fn value_read_from(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    read_from(c, r, a, Some("value"))
}

fn write_to_text(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let mut out = String::new();
    write(&h, &mut out)?;
    a[0] = Value::Text(out.into());
    Ok(Value::Boolean(true))
}

/// `WriteTo(OutStream)`: the compact text of `WriteTo(Text)` goes to the stream
/// (json/TestJsonArray.al:90 pins the text; the bytes are an assumption, see
/// [`crate::stream::write_serialised_text`]).
fn write_to_stream(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let mut out = String::new();
    write(&h, &mut out)?;
    crate::stream::write_serialised_text(&mut a[0], &out)?;
    Ok(Value::Boolean(true))
}

// ------------------------------------------------------------------- YAML

/// The YAML forms the corpus pins (json/TestJsonRuntimeContracts.al:JsonObject_Yaml_RoundTripsOnThisTier,
/// json/TestProbeJsonContracts.al:Yaml_WriteNested, Yaml_Read): `key: value` lines, a nested object as `key:` and its
/// members indented by two spaces, an array as `key:` and `- item` lines at the key's own indent. A text is quoted with
/// single quotes (doubled inside) when it is empty, starts with `:` or holds `: ` or ` #`; `123` and `true` as texts stay
/// bare. Keys other than letters, digits and `_`, and an object inside an array, are refused.
fn plain_yaml_key(k: &str) -> bool {
    !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn yaml_text(t: &str) -> String {
    if t.is_empty() || t.starts_with(':') || t.contains(": ") || t.contains(" #") {
        format!("'{}'", t.replace('\'', "''"))
    } else {
        t.to_string()
    }
}

fn yaml_scalar(c: &Handle) -> Result<String, Signal> {
    let scalar = with_node(c, |n| match n {
        JsonNode::Scalar(Scalar::Val(v)) => Some(v.clone()),
        _ => None,
    });
    match scalar {
        Some(Value::Text(t) | Value::Code(t)) => Ok(yaml_text(&t)),
        Some(Value::Integer(_) | Value::BigInteger(_) | Value::Boolean(_) | Value::Decimal(_)) => {
            let mut t = String::new();
            write(c, &mut t)?;
            Ok(t)
        }
        _ => unsupported(
            "WriteToYaml of a value that is not a number, a Boolean or a text (the YAML form is not pinned)",
        ),
    }
}

fn yaml_members(kids: &[(String, Handle)], indent: usize, out: &mut String) -> Result<(), Signal> {
    let pad = " ".repeat(indent);
    for (k, c) in kids {
        if !plain_yaml_key(k) {
            return unsupported(format!("WriteToYaml with the key `{k}` (quoting is not pinned)"));
        }
        enum Kind {
            Obj(Vec<(String, Handle)>),
            Arr(Vec<Handle>),
            Other,
        }
        let kind = with_node(c, |n| match n {
            JsonNode::Object(k) => Kind::Obj(k.clone()),
            JsonNode::Array(k) => Kind::Arr(k.clone()),
            _ => Kind::Other,
        });
        match kind {
            Kind::Obj(inner) => {
                out.push_str(&format!("{pad}{k}:\r\n"));
                yaml_members(&inner, indent + 2, out)?;
            }
            Kind::Arr(items) => {
                out.push_str(&format!("{pad}{k}:\r\n"));
                for item in items {
                    if with_node(&item, |n| !matches!(n, JsonNode::Scalar(_))) {
                        return unsupported("WriteToYaml of an object or an array inside an array (not pinned)");
                    }
                    out.push_str(&format!("{pad}- {}\r\n", yaml_scalar(&item)?));
                }
            }
            Kind::Other => out.push_str(&format!("{pad}{k}: {}\r\n", yaml_scalar(c)?)),
        }
    }
    Ok(())
}

fn write_to_yaml(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let kids = with_node(&h, |n| match n {
        JsonNode::Object(k) => Ok(k.clone()),
        _ => unsupported("WriteToYaml of something that is not a JsonObject"),
    })?;
    let mut out = String::new();
    yaml_members(&kids, 0, &mut out)?;
    a[0] = Value::Text(out.into());
    Ok(Value::Boolean(true))
}

/// `ReadFromYaml`: `key: value` lines; every value reads as a text, and a value that holds `: ` again is no YAML
/// (json/TestProbeJsonContracts.al:Yaml_Read: `a: 1\nb: text` written on one line raises YamlBadFormat, and
/// `a:\n  b: 1` on one line reads as the key `a:\n  b` with the text `1`). Nested YAML on separate lines is not pinned.
fn read_from_yaml(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let Some(text) = crate::native::text_of(&a[0]) else {
        return unsupported("ReadFromYaml with an argument that is not text");
    };
    let mut members: Vec<String> = Vec::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        if line.starts_with(' ') || line.starts_with('-') {
            return unsupported("ReadFromYaml of a nested or list line (the YAML form is not pinned)");
        }
        let Some((k, v)) = line.split_once(": ") else {
            return unsupported("ReadFromYaml of a line that is not `key: value` (the YAML form is not pinned)");
        };
        if v.contains(": ") {
            return Err(error("The data does not represent valid YAML.", "YamlBadFormat"));
        }
        let mut key = String::new();
        write_string(&mut key, k);
        let mut value = String::new();
        write_string(&mut value, v);
        members.push(format!("{key}:{value}"));
    }
    let json = format!("{{{}}}", members.join(","));
    read_from(c, r, &mut [Value::Text(json.into())], Some("object"))
}

fn as_token(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    Ok(tagged("JsonToken", recv(r)?))
}

/// AsObject / AsArray of a token that holds a value raise a conversion error between the .NET token
/// types (json/TestJsonTokenUnassignedEvaluateGuidContracts.al:UnassignedJsonToken_ActsAsANull, a null).
fn token_as_object(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    // A value (a number, a null) is no object (json/TestProbeJsonContracts.al:TypedGetters_WrongKinds, `value_asobject`).
    if with_node(&h, |n| matches!(n, JsonNode::Scalar(_) | JsonNode::Array(_))) {
        return Err(conversion(
            "Microsoft.Dynamics.Nav.Runtime.NavJsonToken",
            "Microsoft.Dynamics.Nav.Runtime.NavJsonObject",
        ));
    }
    child_as(h, "object", "JsonObject")
}
fn token_as_array(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    if with_node(&h, |n| matches!(n, JsonNode::Scalar(_) | JsonNode::Object(_))) {
        return Err(conversion(
            "Microsoft.Dynamics.Nav.Runtime.NavJsonToken",
            "Microsoft.Dynamics.Nav.Runtime.NavJsonArray",
        ));
    }
    child_as(h, "array", "JsonArray")
}
fn token_as_value(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    if with_node(&h, |n| matches!(n, JsonNode::Scalar(_))) {
        Ok(tagged("JsonValue", h))
    } else {
        // json/TestJsonRuntimeContracts.al:JsonObject_Errors_TextAndCode
        Err(conversion(
            "Microsoft.Dynamics.Nav.Runtime.NavJsonToken",
            NAV_JSON_VALUE,
        ))
    }
}

fn token_is_object(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    Ok(Value::Boolean(with_node(&h, |n| matches!(n, JsonNode::Object(_)))))
}
fn token_is_array(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    Ok(Value::Boolean(with_node(&h, |n| matches!(n, JsonNode::Array(_)))))
}
fn token_is_value(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    Ok(Value::Boolean(with_node(&h, |n| matches!(n, JsonNode::Scalar(_)))))
}

fn clone_token(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    Ok(tagged("JsonToken", deep_clone(&recv(r)?)))
}

fn path(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    Ok(Value::Text(path_of(&recv(r)?).into()))
}

fn value_is_null(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    Ok(Value::Boolean(matches!(scalar_of(&h)?, Scalar::Null)))
}

fn value_is_undefined(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    Ok(Value::Boolean(matches!(scalar_of(&h)?, Scalar::Undefined)))
}

fn set_scalar(h: &Handle, s: Scalar) -> Result<Value, Signal> {
    with_node(h, |n| match n {
        JsonNode::Scalar(slot) => {
            *slot = s;
            Ok(Value::Null)
        }
        _ => unsupported("SetValue on a JSON node that is not a value"),
    })
}

/// `SetValue` on a value that sits in an object or array leaves that tree as it is: the variable takes a value of its own
/// (json/JsonWriteTo.al:ModifyNestedThroughToken, `after_value_setvalue`).
fn set_own_scalar(r: Option<&mut Value>, s: Scalar) -> Result<Value, Signal> {
    let Some(v) = r else {
        return unsupported("JSON method without a receiver");
    };
    let h = handle_of(v)?;
    let attached = matches!(&*h.borrow(), LibraryValue::Json(c) if c.parent.is_some());
    if attached {
        if let Value::Library(name, slot) = v {
            let fresh = new_handle(JsonNode::Scalar(s));
            let _ = slot;
            *v = Value::Library(name.clone(), fresh);
            return Ok(Value::Null);
        }
    }
    set_scalar(&h, s)
}

fn value_set(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let src = node_for(&a[0])?;
    let s = scalar_of(&src)?;
    set_own_scalar(r, s)
}
fn value_set_null(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    set_own_scalar(r, Scalar::Null)
}
fn value_set_undefined(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> Result<Value, Signal> {
    set_own_scalar(r, Scalar::Undefined)
}

// ------------------------------------------------------------ SelectToken

enum Seg {
    Key(String),
    /// `[n]`; a negative index matches nothing (json/JsonObjectBasics.al:323 `select_neg`).
    Index(i64),
    /// `[*]`.
    Wild,
    /// `[a:b]`.
    Slice(usize, usize),
    /// `..name`: every descendant named `name`.
    Descent(String),
    /// `[?(@.name == x)]` (`filter`).
    Filter(filter::Filter),
}

/// A path the platform cannot read: an indexer with no `]` (json/JsonObjectBasics.al:329 `select_bad`, :371
/// `select_token_unclosed`) or one that is neither a number, a quoted name, `*` nor a slice (:369 `select_token_bad_index`).
enum PathErr {
    Unclosed,
    BadIndex,
    /// A filter the platform cannot read (`filter::FilterErr::Invalid`).
    InvalidFilter,
    /// A name in double quotes, `["a"]`: false when the result is used (json/lt-dc-json-leads/DcJsonLeads.al:306, :340).
    DoubleQuoted,
    Refused(Signal),
}

/// Dotted names, `[n]` / `['name']` / `[*]` / `[a:b]` indexers and `..name`, with an optional leading `$`.
/// Filters as `filter` reads them.
fn parse_path(p: &str) -> Result<Vec<Seg>, PathErr> {
    let refuse = |what: &str| PathErr::Refused(Signal::Unsupported(what.into()));
    let b = p.as_bytes();
    let mut i = 0;
    let mut segs = Vec::new();
    if b.first() == Some(&b'$') {
        i = 1;
    }
    while i < b.len() {
        match b[i] {
            b'[' => {
                let close = p[i..].find(']').map(|c| c + i).ok_or(PathErr::Unclosed)?;
                let inner = &p[i + 1..close];
                if let Some(q) = inner.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
                    segs.push(Seg::Key(q.to_string()));
                } else if let Ok(n) = inner.parse::<i64>() {
                    segs.push(Seg::Index(n));
                } else if inner == "*" {
                    segs.push(Seg::Wild);
                } else if let Some((a, z)) = inner.split_once(':') {
                    match (a.parse::<usize>(), z.parse::<usize>()) {
                        (Ok(a), Ok(z)) => segs.push(Seg::Slice(a, z)),
                        _ => return Err(refuse("SelectToken slice form not pinned by the corpus")),
                    }
                } else if inner.len() >= 2 && inner.starts_with('"') && inner.ends_with('"') {
                    return Err(PathErr::DoubleQuoted);
                } else if inner.starts_with('?') {
                    match filter::parse(inner) {
                        Ok(f) => segs.push(Seg::Filter(f)),
                        Err(filter::FilterErr::Invalid) => return Err(PathErr::InvalidFilter),
                        Err(filter::FilterErr::Refused(why)) => return Err(refuse(why)),
                    }
                } else if inner.starts_with('(') || inner.contains(',') {
                    return Err(refuse("SelectToken indexer form not pinned by the corpus"));
                } else {
                    return Err(PathErr::BadIndex);
                }
                i = close + 1;
            }
            c => {
                let mut descent = false;
                if c == b'.' {
                    i += 1;
                    if b.get(i) == Some(&b'.') {
                        descent = true;
                        i += 1;
                    } else if b.get(i) == Some(&b'[') && b.get(i + 1) == Some(&b'?') {
                        // `$.[?(@.id==2)]` (al-runner issue 6588): the dot before a filter is skipped
                        // (json/lt-db-json-leads/DbJPathLeads.al:249 `dot` and :353 `tok_dot`).
                        continue;
                    }
                }
                let s = i;
                while i < b.len() && b[i] != b'.' && b[i] != b'[' {
                    i += 1;
                }
                let name = &p[s..i];
                if name.is_empty() || name.contains(['*', '?', '@']) {
                    return Err(refuse("SelectToken path form not pinned by the corpus"));
                }
                segs.push(match descent {
                    true => Seg::Descent(name.to_string()),
                    false => Seg::Key(name.to_string()),
                });
            }
        }
    }
    Ok(segs)
}

/// The descendants of `h` named `name`, in document order.
fn descendants_named(h: &Handle, name: &str, out: &mut Vec<Handle>) {
    let kids: Vec<(Option<String>, Handle)> = with_node(h, |n| match n {
        JsonNode::Object(kids) => kids.iter().map(|(k, c)| (Some(k.clone()), c.clone())).collect(),
        JsonNode::Array(kids) => kids.iter().map(|c| (None, c.clone())).collect(),
        _ => Vec::new(),
    });
    for (k, c) in kids {
        if k.as_deref() == Some(name) {
            out.push(c.clone());
        }
        descendants_named(&c, name, out);
    }
}

fn select_token(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let Some(p) = crate::native::text_of(&a[0]) else {
        return unsupported("SelectToken path that is not text");
    };
    let used = c.ret_used();
    // A path the platform cannot read answers false when the result is used (json/JsonObjectBasics.al:329 `select_bad`, an
    // unclosed indexer) and raises "The requested operation cannot be performed in this context." (InvalidOperation) as a
    // statement (:369-380). A bad index with the result used is not pinned.
    let segs = match parse_path(&p) {
        Ok(s) => s,
        Err(PathErr::Unclosed) if used => return Ok(Value::Boolean(false)),
        Err(PathErr::BadIndex) if used => return unsupported("SelectToken with a bad index, result used (not pinned)"),
        Err(PathErr::InvalidFilter) if used => return Ok(filter::invalid(c)),
        Err(PathErr::DoubleQuoted) if used => return Ok(Value::Boolean(false)),
        Err(PathErr::DoubleQuoted) => {
            return unsupported("SelectToken with a double-quoted name as a statement (not pinned)");
        }
        Err(PathErr::InvalidFilter) => {
            return unsupported("SelectToken with a double-quoted filter text as a statement (not pinned)");
        }
        Err(PathErr::Unclosed | PathErr::BadIndex) => {
            return Err(error(
                "The requested operation cannot be performed in this context.",
                "InvalidOperation",
            ));
        }
        Err(PathErr::Refused(e)) => return Err(e),
    };
    if segs.iter().any(|s| matches!(s, Seg::Filter(_))) {
        return filter::select(h, &segs, &p, used, &mut a[1]);
    }
    let last = segs.len().saturating_sub(1);
    let mut cur = h;
    for (n, seg) in segs.iter().enumerate() {
        let found: Vec<Handle> = match seg {
            Seg::Descent(name) => {
                let mut out = Vec::new();
                descendants_named(&cur, name, &mut out);
                out
            }
            _ => with_node(&cur, |node| match (node, seg) {
                (JsonNode::Object(kids), Seg::Key(k)) => {
                    kids.iter().filter(|(e, _)| e == k).map(|(_, c)| c.clone()).collect()
                }
                (JsonNode::Array(kids), Seg::Index(i)) => usize::try_from(*i)
                    .ok()
                    .and_then(|i| kids.get(i).cloned())
                    .into_iter()
                    .collect(),
                (JsonNode::Array(kids), Seg::Wild) => kids.clone(),
                (JsonNode::Array(kids), Seg::Slice(s, z)) => {
                    kids.iter().skip(*s).take(z.saturating_sub(*s)).cloned().collect()
                }
                _ => Vec::new(),
            }),
        };
        let plain = matches!(seg, Seg::Key(_) | Seg::Index(_));
        match found.len() {
            1 => cur = found.into_iter().next().expect("one"),
            // A name or an index that matches nothing (json/JsonObjectBasics.al:316 `select_missing`, :320 `select_arr_oob`,
            // :323 `select_neg`).
            0 if plain && used => return Ok(Value::Boolean(false)),
            0 if plain => {
                return unsupported("SelectToken without a match, result unused (error text not pinned yet)");
            }
            // `[*]` that ends the path and matches more than one token: false when the result is used (:324 `select_wild`),
            // else "The set of tokens matching the JSON path '<path>' contains more than one element."
            // (InvalidJsonPathSelection, :363 and :373-374).
            m if m > 1 && matches!(seg, Seg::Wild) && n == last => {
                if used {
                    return Ok(Value::Boolean(false));
                }
                return Err(error(
                    format!("The set of tokens matching the JSON path '{p}' contains more than one element."),
                    "InvalidJsonPathSelection",
                ));
            }
            _ => {
                return unsupported(
                    "SelectToken with a wildcard, a slice or `..` that matches no token, or more than one (not pinned)",
                );
            }
        }
    }
    a[1] = tagged("JsonToken", cur);
    Ok(Value::Boolean(true))
}

// ----------------------------------------------------------- SelectTokens

/// One step of a `SelectTokens` path. Only the forms json/lt-json-selecttokens/SelectTokensList.al,
/// json/lt-ay-json/AyJsonTokenSelectTokens.al and json/lt-dc-json-leads/DcJsonLeads.al:412-442 pin.
enum ManySeg {
    Key(String),
    Index(i64),
    /// `[*]` on an array.
    ArrayWild,
    /// `.*` on an object.
    ObjectWild,
    Slice(usize, usize),
    Descent(String),
    /// `[?(@.name > n)]` with a whole number n (SelectTokensList.al:265 `filter`, AyJsonTokenSelectTokens.al:62,
    /// DcJsonLeads.al:422).
    Greater(String, i64),
}

/// Why a `SelectTokens` path is not walked: the platform cannot read it (false and a last error), or it is not pinned.
enum ManyErr {
    Invalid,
    Refused(&'static str),
}

fn parse_many(p: &str) -> Result<Vec<ManySeg>, ManyErr> {
    // An indexer that is never closed is a path the platform cannot read: `$[`, `$.a[?(@.x`, `a..[`, `$.a.[`, `$[?(`
    // (SelectTokensList.al:124-206), `$.a[` (AyJsonTokenSelectTokens.al:136), `order_line[` (DcJsonLeads.al:427).
    if let Some(open) = p.rfind('[')
        && !p[open..].contains(']')
    {
        return Err(ManyErr::Invalid);
    }
    let b = p.as_bytes();
    let mut i = usize::from(b.first() == Some(&b'$'));
    let mut segs = Vec::new();
    while i < b.len() {
        if b[i] == b'[' {
            let close = p[i..].find(']').map(|c| c + i).ok_or(ManyErr::Invalid)?;
            let inner = &p[i + 1..close];
            if let Some(q) = inner.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')) {
                segs.push(ManySeg::Key(q.to_string()));
            } else if let Ok(n) = inner.parse::<i64>() {
                segs.push(ManySeg::Index(n));
            } else if inner == "*" {
                segs.push(ManySeg::ArrayWild);
            } else if let Some((a, z)) = inner.split_once(':')
                && let (Ok(a), Ok(z)) = (a.parse::<usize>(), z.parse::<usize>())
            {
                segs.push(ManySeg::Slice(a, z));
            } else if let Some(f) = inner.strip_prefix("?(").and_then(|s| s.strip_suffix(')')) {
                let Some((lhs, rhs)) = f.split_once('>') else {
                    return Err(ManyErr::Refused(
                        "SelectTokens filter that is not a > comparison (not pinned)",
                    ));
                };
                let Some(name) = lhs.trim().strip_prefix("@.") else {
                    return Err(ManyErr::Refused(
                        "SelectTokens filter on something else than @.name (not pinned)",
                    ));
                };
                if name.is_empty() || !name.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
                    return Err(ManyErr::Refused(
                        "SelectTokens filter on something else than @.name (not pinned)",
                    ));
                }
                let rhs = rhs.trim();
                // `$.a[?(@.p >)]`: no right operand (AyJsonTokenSelectTokens.al:137 `bad_filter`).
                if rhs.is_empty() {
                    return Err(ManyErr::Invalid);
                }
                let Ok(n) = rhs.parse::<i64>() else {
                    return Err(ManyErr::Refused(
                        "SelectTokens filter against something else than a whole number (not pinned)",
                    ));
                };
                segs.push(ManySeg::Greater(name.to_string(), n));
            } else {
                return Err(ManyErr::Refused("SelectTokens indexer form not pinned by the corpus"));
            }
            i = close + 1;
            continue;
        }
        let mut descent = false;
        if b[i] == b'.' {
            i += 1;
            if b.get(i) == Some(&b'.') {
                descent = true;
                i += 1;
            }
        }
        let s = i;
        while i < b.len() && b[i] != b'.' && b[i] != b'[' {
            i += 1;
        }
        let name = &p[s..i];
        if name == "*" && !descent {
            segs.push(ManySeg::ObjectWild);
            continue;
        }
        if name.is_empty() || name.contains(['*', '?', '@', '(', ')', '\'', ']']) {
            return Err(ManyErr::Refused("SelectTokens path form not pinned by the corpus"));
        }
        segs.push(match descent {
            true => ManySeg::Descent(name.to_string()),
            false => ManySeg::Key(name.to_string()),
        });
    }
    Ok(segs)
}

/// The tokens one step selects from `h`, in document order.
fn step_many(h: &Handle, seg: &ManySeg) -> Result<Vec<Handle>, Signal> {
    if let ManySeg::Descent(name) = seg {
        let mut out = Vec::new();
        descendants_named(h, name, &mut out);
        return Ok(out);
    }
    let refuse = |what: &str| unsupported::<Vec<Handle>>(format!("SelectTokens: {what} (not pinned)"));
    let filter_kids: Option<Vec<Handle>> = with_node(h, |n| match n {
        JsonNode::Array(kids) => Some(kids.clone()),
        _ => None,
    });
    match seg {
        ManySeg::Greater(name, limit) => {
            let Some(kids) = filter_kids else {
                return refuse("a filter on a token that is not an array");
            };
            let mut out = Vec::new();
            for k in kids {
                let field = with_node(&k, |n| match n {
                    JsonNode::Object(f) => Ok(f.iter().find(|(e, _)| e == name).map(|(_, c)| c.clone())),
                    _ => Err(()),
                });
                // An element without the name is left out (SelectTokensList.al:265 `{"x":30}`, AyJsonTokenSelectTokens.al:62
                // `{"n":3}`).
                let field = match field {
                    Ok(Some(f)) => f,
                    Ok(None) => continue,
                    Err(()) => return refuse("a filter over an element that is not an object"),
                };
                let n = with_node(&field, |n| match n {
                    JsonNode::Scalar(Scalar::Val(Value::Integer(v))) => Some(i64::from(*v)),
                    JsonNode::Scalar(Scalar::Val(Value::BigInteger(v))) => Some(*v),
                    _ => None,
                });
                match n {
                    Some(v) if v > *limit => out.push(k),
                    Some(_) => {}
                    None => return refuse("a filter that compares something else than a whole number"),
                }
            }
            Ok(out)
        }
        _ => with_node(h, |node| match (node, seg) {
            (JsonNode::Object(kids), ManySeg::Key(k)) => {
                Ok(kids.iter().filter(|(e, _)| e == k).map(|(_, c)| c.clone()).collect())
            }
            // A name on an array or a value selects nothing (SelectTokensList.al:46 `$[*].a` skips the array element
            // `[{"a":4}]`; AyJsonTokenSelectTokens.al:117 `$.a` on the value 7).
            (JsonNode::Array(_) | JsonNode::Scalar(_), ManySeg::Key(_)) => Ok(Vec::new()),
            // An index past the end selects nothing (SelectTokensList.al:44 `$[9]`).
            (JsonNode::Array(kids), ManySeg::Index(i)) if *i >= 0 => Ok(usize::try_from(*i)
                .ok()
                .and_then(|i| kids.get(i).cloned())
                .into_iter()
                .collect()),
            (JsonNode::Array(kids), ManySeg::ArrayWild) => Ok(kids.clone()),
            (JsonNode::Object(kids), ManySeg::ObjectWild) => Ok(kids.iter().map(|(_, c)| c.clone()).collect()),
            // `[1:3]` and `[0:2]` inside the array (SelectTokensList.al:52, AyJsonTokenSelectTokens.al:104).
            (JsonNode::Array(kids), ManySeg::Slice(a, z)) if a < z && *z <= kids.len() => Ok(kids[*a..*z].to_vec()),
            _ => Err(Signal::Unsupported(
                "SelectTokens: this step on this kind of token (not pinned)".into(),
            )),
        }),
    }
}

/// `SelectTokens(Path, var List of [JsonToken])`: true and the list replaced by the matches, none included
/// (SelectTokensList.al:61-72, the pre-filled list :92-95; AyJsonTokenSelectTokens.al:79-80). `''` and `$` select the token
/// itself (SelectTokensList.al:157 `bad3`, AyJsonTokenSelectTokens.al:138 `empty`, :116 `value_root`). A path the platform
/// cannot read answers false, leaves the list as it was and sets the last error "The requested operation cannot be performed
/// in this context." (InvalidOperation) without raising it (:144 `unclosed_prefilled`). Only the used result is pinned.
fn select_tokens(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let h = recv(r)?;
    let Some(p) = crate::native::text_of(&a[0]) else {
        return unsupported("SelectTokens path that is not text");
    };
    let Value::List(list) = &a[1] else {
        return unsupported("SelectTokens into something else than a List of [JsonToken]");
    };
    if !c.ret_used() {
        return unsupported("SelectTokens with the result unused (not pinned)");
    }
    let segs = match parse_many(&p) {
        Ok(s) => s,
        Err(ManyErr::Invalid) => {
            c.set_last_error(crate::native::AlError {
                message: "The requested operation cannot be performed in this context.".into(),
                code: "InvalidOperation".into(),
                ..Default::default()
            });
            return Ok(Value::Boolean(false));
        }
        Err(ManyErr::Refused(why)) => return unsupported(why),
    };
    let mut cur = vec![h];
    for seg in &segs {
        let mut next = Vec::new();
        for t in &cur {
            next.extend(step_many(t, seg)?);
        }
        cur = next;
    }
    let mut l = list.borrow_mut();
    if l.read_only {
        return unsupported("SelectTokens into a read-only list");
    }
    l.clear();
    l.extend(cur.into_iter().map(|t| tagged("JsonToken", t)));
    Ok(Value::Boolean(true))
}

// ---------------------------------------------------- WriteWithSecretsTo

/// The text of a SecretText argument: a cleared one is empty (json/lt-json-select-secrets/WriteWithSecrets.al:63
/// `empty_secret`).
fn secret_text(v: &Value) -> Result<String, Signal> {
    match v {
        Value::Library(_, h) => match &*h.borrow() {
            LibraryValue::Empty => Ok(String::new()),
            LibraryValue::Text(s) => Ok(s.clone()),
            other => unsupported(format!("WriteWithSecretsTo with a secret holding {}", other.describe())),
        },
        other => unsupported(format!("WriteWithSecretsTo with a secret that is {}", other.describe())),
    }
}

/// Puts `secret` at `path` in the copy `root`: `Ok(false)` when the platform answers false. Pinned by
/// json/lt-json-select-secrets/WriteWithSecrets.al: a dotted name with or without `$.` that names a string is replaced
/// (:40-45); a number, an object or an array element that is a number (:46-51, :126 `$.n`), a missing name (:52), an empty
/// path (:55), an unclosed indexer (:70) and `a..b` on a missing `a` (:73) answer false.
fn put_secret(root: &Handle, path: &str, secret: &str) -> Result<bool, Signal> {
    let segs = match parse_path(path) {
        Ok(s) => s,
        Err(PathErr::Unclosed) => return Ok(false),
        Err(PathErr::BadIndex) => return unsupported("WriteWithSecretsTo with a bad index in the path (not pinned)"),
        Err(PathErr::InvalidFilter | PathErr::DoubleQuoted) => {
            return unsupported("WriteWithSecretsTo with a filter or a double-quoted name in the path (not pinned)");
        }
        Err(PathErr::Refused(e)) => return Err(e),
    };
    if segs.is_empty() {
        return Ok(false);
    }
    let mut cur = root.clone();
    let mut indexed = false;
    for seg in &segs {
        let found: Vec<Handle> = match seg {
            Seg::Key(k) => with_node(&cur, |n| match n {
                JsonNode::Object(kids) => kids.iter().filter(|(e, _)| e == k).map(|(_, c)| c.clone()).collect(),
                _ => Vec::new(),
            }),
            Seg::Index(i) => {
                indexed = true;
                with_node(&cur, |n| match n {
                    JsonNode::Array(kids) => usize::try_from(*i)
                        .ok()
                        .and_then(|i| kids.get(i).cloned())
                        .into_iter()
                        .collect(),
                    _ => Vec::new(),
                })
            }
            _ => return unsupported("WriteWithSecretsTo with a wildcard, a slice or `..` in the path (not pinned)"),
        };
        match found.len() {
            0 if !indexed => return Ok(false),
            0 => return unsupported("WriteWithSecretsTo with an index that matches nothing (not pinned)"),
            1 => cur = found.into_iter().next().expect("one"),
            _ => return unsupported("WriteWithSecretsTo on a name that is in the object twice (not pinned)"),
        }
    }
    let kind = with_node(&cur, |n| match n {
        JsonNode::Scalar(Scalar::Val(Value::Text(_))) => Some(true),
        JsonNode::Scalar(
            Scalar::Val(Value::Integer(_) | Value::BigInteger(_) | Value::Decimal(_)) | Scalar::Raw(_),
        )
        | JsonNode::Object(_) => Some(false),
        _ => None,
    });
    match kind {
        Some(true) if indexed => unsupported("WriteWithSecretsTo on a string inside an array (not pinned)"),
        Some(true) => {
            with_node(&cur, |n| {
                *n = JsonNode::Scalar(Scalar::Val(Value::Text(secret.to_string().into())))
            });
            Ok(true)
        }
        Some(false) => Ok(false),
        None => unsupported("WriteWithSecretsTo on a value that is not a string, a number or an object (not pinned)"),
    }
}

/// Writes a copy of the object with every `(path, secret)` put in it to `a[out]` as a SecretText and answers true; the
/// object itself is not changed (WriteWithSecrets.al:67 and :128 `source_unchanged`). When one path fails the answer is
/// false and the result keeps what it held (:46-58 and :124-127; a cleared result stays empty, :59-62). Whether a false
/// as a statement raises an error is not pinned.
fn write_with_secrets(
    c: &mut dyn Ctx,
    r: Option<&mut Value>,
    a: &mut [Value],
    pairs: Vec<(String, String)>,
    out: usize,
) -> Result<Value, Signal> {
    let copy = deep_clone(&recv(r)?);
    for (path, secret) in &pairs {
        if !put_secret(&copy, path, secret)? {
            if !c.ret_used() {
                return unsupported("WriteWithSecretsTo that answers false, as a statement (not pinned)");
            }
            return Ok(Value::Boolean(false));
        }
    }
    let mut text = String::new();
    write(&copy, &mut text)?;
    a[out] = Value::Library("SecretText".into(), Rc::new(RefCell::new(LibraryValue::Text(text))));
    Ok(Value::Boolean(true))
}

/// `WriteWithSecretsTo(Path, Secret, var Result)` (json/lt-json-select-secrets/WriteWithSecrets.al:30-90).
fn write_with_secret_path(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let Some(path) = crate::native::text_of(&a[0]) else {
        return unsupported("WriteWithSecretsTo path that is not text");
    };
    let secret = secret_text(&a[1])?;
    write_with_secrets(c, r, a, vec![(path.to_string(), secret)], 2)
}

/// `WriteWithSecretsTo(Secrets: Dictionary of [Text, SecretText], var Result)`: every key is a path; an empty dictionary
/// writes the object as it is (json/lt-json-select-secrets/WriteWithSecrets.al:92-138). Microsoft's Shopify Connector
/// calls it with `$.name` keys (Shpfy Authentication Mgt. ExecuteTokenRequest), the form :40-43 and :112-115 pin.
fn write_with_secret_dictionary(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> Result<Value, Signal> {
    let Value::Dictionary(d) = &a[0] else {
        return unsupported(format!("WriteWithSecretsTo with {} for the secrets", a[0].describe()));
    };
    let mut pairs = Vec::new();
    for (k, v) in d.borrow().iter() {
        let Some(path) = crate::native::text_of(k) else {
            return unsupported("WriteWithSecretsTo with a dictionary key that is not text");
        };
        pairs.push((path.to_string(), secret_text(v)?));
    }
    write_with_secrets(c, r, a, pairs, 1)
}

// --------------------------------------------------------------- dispatch

/// The implementation of one catalog overload, if the corpus pins enough of it.
/// Overloads with stream parameters are not implemented here.
pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    let params: Vec<&str> = sig.params.iter().map(|p| p.ty.as_str()).collect();
    let first_by_var = sig.params.first().is_some_and(|p| p.by_var);
    let method = format!("{}.{}", sig.receiver, sig.name);
    let f: NativeFn = match (method.as_str(), params.as_slice()) {
        ("JsonObject.Add", [_, _]) => object_add,
        ("JsonObject.Contains", [_]) => object_contains,
        ("JsonObject.Replace", [_, _]) => object_replace,
        ("JsonObject.Keys", []) => object_keys,
        ("JsonObject.Values", []) => object_values,
        ("JsonArray.RemoveAt", [_]) => array_remove_at,
        ("JsonObject.Get", [_, _]) => object_get,
        ("JsonObject.Remove", [_]) => object_remove,
        ("JsonObject.GetBoolean", _) => obj_boolean,
        ("JsonObject.GetInteger", _) => obj_integer,
        ("JsonObject.GetBigInteger", _) => obj_biginteger,
        ("JsonObject.GetByte", _) => obj_byte,
        ("JsonObject.GetOption", _) => obj_option,
        ("JsonObject.GetDuration", _) => obj_duration,
        ("JsonObject.GetDecimal", _) => obj_decimal,
        ("JsonObject.GetText", _) => obj_text,
        ("JsonObject.GetChar", _) => obj_char,
        ("JsonObject.GetDate", _) => obj_date,
        ("JsonObject.GetDateTime", _) => obj_datetime,
        ("JsonObject.GetTime", _) => obj_time,
        ("JsonObject.GetObject", _) => obj_get_object,
        ("JsonObject.GetArray", _) => obj_get_array,
        ("JsonObject.ReadFrom", ["Text" | "InStream"]) => object_read_from,
        ("JsonObject.ReadFromYaml", ["Text"]) => read_from_yaml,
        ("JsonObject.WriteToYaml", ["Text"]) if first_by_var => write_to_yaml,
        ("JsonObject.WriteTo", ["Text"]) if first_by_var => write_to_text,
        ("JsonObject.WriteTo", ["OutStream"]) => write_to_stream,
        ("JsonObject.WriteWithSecretsTo", [_, _, _]) => write_with_secret_path,
        ("JsonObject.WriteWithSecretsTo", [_, _]) => write_with_secret_dictionary,
        ("JsonObject.AsToken" | "JsonArray.AsToken" | "JsonValue.AsToken", []) => as_token,
        ("JsonObject.Clone" | "JsonArray.Clone" | "JsonValue.Clone" | "JsonToken.Clone", []) => clone_token,
        ("JsonObject.Path" | "JsonArray.Path" | "JsonValue.Path" | "JsonToken.Path", []) => path,
        (
            "JsonObject.SelectToken" | "JsonArray.SelectToken" | "JsonValue.SelectToken" | "JsonToken.SelectToken",
            [_, _],
        ) => select_token,
        ("JsonObject.SelectTokens" | "JsonArray.SelectTokens" | "JsonToken.SelectTokens", [_, _]) => select_tokens,
        ("JsonArray.Add", [_]) => array_add,
        ("JsonArray.Insert", [_, _]) => array_insert,
        ("JsonArray.Set", [_, _]) => array_set,
        ("JsonArray.IndexOf", [_]) => array_index_of,
        ("JsonArray.Count", []) => array_count,
        ("JsonArray.Get", [_, _]) => array_get,
        ("JsonArray.GetBoolean", _) => arr_boolean,
        ("JsonArray.GetInteger", _) => arr_integer,
        ("JsonArray.GetBigInteger", _) => arr_biginteger,
        ("JsonArray.GetByte", _) => arr_byte,
        ("JsonArray.GetOption", _) => arr_option,
        ("JsonArray.GetDuration", _) => arr_duration,
        ("JsonArray.GetDecimal", _) => arr_decimal,
        ("JsonArray.GetText", _) => arr_text,
        ("JsonArray.GetChar", _) => arr_char,
        ("JsonArray.GetDate", _) => arr_date,
        ("JsonArray.GetDateTime", _) => arr_datetime,
        ("JsonArray.GetTime", _) => arr_time,
        ("JsonArray.GetObject", _) => arr_get_object,
        ("JsonArray.GetArray", _) => arr_get_array,
        ("JsonArray.ReadFrom", ["Text" | "InStream"]) => array_read_from,
        ("JsonArray.WriteTo", ["Text"]) if first_by_var => write_to_text,
        ("JsonArray.WriteTo", ["OutStream"]) => write_to_stream,
        ("JsonToken.ReadFrom", ["Text" | "InStream"]) => any_read_from,
        ("JsonValue.ReadFrom", ["Text" | "InStream"]) => value_read_from,
        ("JsonToken.WriteTo" | "JsonValue.WriteTo", ["Text"]) if first_by_var => write_to_text,
        ("JsonToken.WriteTo" | "JsonValue.WriteTo", ["OutStream"]) => write_to_stream,
        ("JsonToken.AsObject", []) => token_as_object,
        ("JsonToken.AsArray", []) => token_as_array,
        ("JsonToken.AsValue", []) => token_as_value,
        ("JsonToken.IsObject", []) => token_is_object,
        ("JsonToken.IsArray", []) => token_is_array,
        ("JsonToken.IsValue", []) => token_is_value,
        ("JsonValue.IsNull", []) => value_is_null,
        ("JsonValue.IsUndefined", []) => value_is_undefined,
        ("JsonValue.SetValue", [_]) => value_set,
        ("JsonValue.SetValueToNull", []) => value_set_null,
        ("JsonValue.SetValueToUndefined", []) => value_set_undefined,
        ("JsonValue.AsBoolean", []) => val_boolean,
        ("JsonValue.AsInteger", []) => val_integer,
        ("JsonValue.AsBigInteger", []) => val_biginteger,
        ("JsonValue.AsByte", []) => val_byte,
        ("JsonValue.AsOption", []) => val_option,
        ("JsonValue.AsDuration", []) => val_duration,
        ("JsonValue.AsDecimal", []) => val_decimal,
        ("JsonValue.AsText", []) => val_text_json,
        ("JsonValue.AsCode", []) => val_code,
        ("JsonValue.AsChar", []) => val_char,
        ("JsonValue.AsDate", []) => val_date,
        ("JsonValue.AsDateTime", []) => val_datetime,
        ("JsonValue.AsTime", []) => val_time,
        _ => return None,
    };
    Some(f)
}

/// The compact text of a JSON value, as `WriteTo(Text)` gives it (for `Format`).
pub fn write_text(h: &Handle, out: &mut String) -> Result<(), Signal> {
    write(h, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Np4FormatJson.al NP4_JsonValue_AsInteger_SetValueDecimal (corpus master, corpus PR 871).
    #[test]
    fn a_fraction_read_as_integer_rounds_ties_to_even() {
        for (v, want) in [
            ("1324.55", 1325),
            ("2.5", 2),
            ("1.5", 2),
            ("0.5", 0),
            ("0.4", 0),
            ("-0.5", 0),
            ("-1.5", -2),
            ("-2.5", -2),
            ("-1324.55", -1325),
            ("3.4999", 3),
        ] {
            let d: rust_decimal::Decimal = v.parse().unwrap();
            assert!(
                matches!(as_integer(&Scalar::Val(Value::Decimal(d))), Ok(Value::Integer(n)) if n == want),
                "{v}"
            );
        }
    }

    /// json/JsonValueGetters.al UN_BooleanTextGetters* (writer UN's corpus branch, commit d3bcc580).
    #[test]
    fn as_boolean_reads_the_pinned_spellings_and_numbers() {
        let text = |t: &str| as_boolean(&Scalar::Val(Value::Text(t.into())));
        let b = |r: Result<Value, Signal>| match r {
            Ok(Value::Boolean(b)) => Some(b),
            _ => None,
        };
        for t in ["true", "True", "TRUE", " true", "true "] {
            assert_eq!(b(text(t)), Some(true), "{t}");
        }
        for t in ["false", "False", "FALSE"] {
            assert_eq!(b(text(t)), Some(false), "{t}");
        }
        for t in ["0", "1", "yes", "Yes", "No", ""] {
            assert!(matches!(text(t), Err(Signal::Error(_))), "{t}");
        }
        assert!(matches!(text("tRue"), Err(Signal::Unsupported(_))));
        assert_eq!(b(as_boolean(&Scalar::Val(Value::Integer(0)))), Some(false));
        assert_eq!(b(as_boolean(&Scalar::Val(Value::Integer(1)))), Some(true));
        assert!(matches!(
            as_boolean(&Scalar::Val(Value::Integer(2))),
            Err(Signal::Unsupported(_))
        ));
    }
}
