//! In-process stand-ins for a closed set of pure .NET framework members (ADR-038).
//!
//! `InprocHost` wraps the real `DotNetHost` (the sidecar). Its sessions answer the members of `inproc.txt` themselves,
//! in Rust, and send everything else to the wrapped session. The sidecar stays the oracle: a call the stand-in does not
//! take for sure (an out-of-range index, invalid Base64, text a codec would repair) goes to the sidecar as if no stand-in
//! existed, so its answer, error included, is .NET's own.
//!
//! Objects made here have handles from `LOCAL_BASE` up, which the sidecar never hands out. When AL code passes such an
//! object to a member the set does not serve, the object *migrates*: its state is rebuilt in the sidecar once, and every
//! later call on it goes there (ADR-038 section 2). One side owns an object at a time, so identity and mutation behave as
//! in .NET.
//!
//! `ALRUNNER_NO_INPROC=1` leaves the sidecar alone: `wrap` returns the host it was given.

use crate::dotnet::*;
use md5::Md5;

mod math;
mod regex;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

/// Handles of objects made in process start here; the sidecar counts from 1.
const LOCAL_BASE: u64 = 1 << 48;

/// The members this module serves, one `Type.Member` per line (the list test keeps it in step with the code and with
/// the differential tests).
pub const MEMBERS: &str = include_str!("inproc.txt");

/// The host to give the engine: `host` wrapped with the stand-ins, or `host` itself when `ALRUNNER_NO_INPROC` is set.
pub fn wrap(host: Arc<dyn DotNetHost>) -> Arc<dyn DotNetHost> {
    if std::env::var_os("ALRUNNER_NO_INPROC").is_some() {
        host
    } else {
        Arc::new(InprocHost::new(host))
    }
}

pub struct InprocHost {
    inner: Arc<dyn DotNetHost>,
}

impl InprocHost {
    /// The stand-ins in front of `inner`, whatever the environment says (the differential tests).
    pub fn new(inner: Arc<dyn DotNetHost>) -> InprocHost {
        InprocHost { inner }
    }
}

impl DotNetHost for InprocHost {
    fn availability(&self) -> Result<(), DotNetUnavailable> {
        self.inner.availability()
    }
    fn identity(&self) -> Option<String> {
        self.inner.identity().map(|i| format!("{i} in-process stand-ins"))
    }
    fn open_session(&self, cfg: &DotNetSessionConfig) -> Box<dyn DotNetSession> {
        Box::new(InprocSession::over(self.inner.open_session(cfg)))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Enc {
    Utf8,
    Ascii,
    Unicode,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Algo {
    Md5,
    Sha1,
    Sha256,
    Sha384,
    Sha512,
}

impl Algo {
    fn of(name: &str) -> Option<Algo> {
        Some(match name {
            "MD5" => Algo::Md5,
            "SHA1" => Algo::Sha1,
            "SHA256" => Algo::Sha256,
            "SHA384" => Algo::Sha384,
            "SHA512" => Algo::Sha512,
            _ => return None,
        })
    }
    fn name(self) -> &'static str {
        match self {
            Algo::Md5 => "MD5",
            Algo::Sha1 => "SHA1",
            Algo::Sha256 => "SHA256",
            Algo::Sha384 => "SHA384",
            Algo::Sha512 => "SHA512",
        }
    }
    fn digest(self, data: &[u8]) -> Vec<u8> {
        match self {
            Algo::Md5 => Md5::digest(data).to_vec(),
            Algo::Sha1 => Sha1::digest(data).to_vec(),
            Algo::Sha256 => Sha256::digest(data).to_vec(),
            Algo::Sha384 => Sha384::digest(data).to_vec(),
            Algo::Sha512 => Sha512::digest(data).to_vec(),
        }
    }
    /// The CLR type `HashAlgorithm.Create(name)` answers.
    fn clr_type(self) -> &'static str {
        match self {
            Algo::Md5 => "System.Security.Cryptography.MD5CryptoServiceProvider",
            Algo::Sha1 => "System.Security.Cryptography.SHA1CryptoServiceProvider",
            Algo::Sha256 => "System.Security.Cryptography.SHA256Managed",
            Algo::Sha384 => "System.Security.Cryptography.SHA384Managed",
            Algo::Sha512 => "System.Security.Cryptography.SHA512Managed",
        }
    }
}

impl Enc {
    fn clr_type(self) -> &'static str {
        match self {
            Enc::Utf8 => "System.Text.UTF8Encoding+UTF8EncodingSealed",
            Enc::Ascii => "System.Text.ASCIIEncoding+ASCIIEncodingSealed",
            Enc::Unicode => "System.Text.UnicodeEncoding",
        }
    }
    fn member(self) -> &'static str {
        match self {
            Enc::Utf8 => "UTF8",
            Enc::Ascii => "ASCII",
            Enc::Unicode => "Unicode",
        }
    }
    /// The bytes of `s`, or `None` when the codec's replacement of what it cannot encode is left to .NET (ASCII with a
    /// character beyond the Basic Multilingual Plane: how many `?` a surrogate pair gets is the sidecar's to say).
    fn bytes(self, s: &str) -> Option<Vec<u8>> {
        Some(match self {
            Enc::Utf8 => s.as_bytes().to_vec(),
            Enc::Unicode => s.encode_utf16().flat_map(|u| u.to_le_bytes()).collect(),
            Enc::Ascii => {
                if s.chars().any(|c| u32::from(c) > 0xFFFF) {
                    return None;
                }
                s.chars().map(|c| if c.is_ascii() { c as u8 } else { b'?' }).collect()
            }
        })
    }
    /// The text of `b`, or `None` when the bytes need the codec's repair of invalid input (the sidecar answers then).
    fn string(self, b: &[u8]) -> Option<String> {
        match self {
            Enc::Utf8 => String::from_utf8(b.to_vec()).ok(),
            Enc::Ascii => b.is_ascii().then(|| b.iter().map(|&c| c as char).collect()),
            Enc::Unicode => {
                if b.len() % 2 != 0 {
                    return None;
                }
                let units: Vec<u16> = b.chunks(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect();
                String::from_utf16(&units).ok()
            }
        }
    }
}

enum Local {
    Builder(String),
    Bytes(Vec<u8>),
    Encoding(Enc),
    Hash {
        algo: Algo,
        disposed: bool,
    },
    /// `TimeZoneInfo` from `FindSystemTimeZoneById`: the id as asked and the base offset in ticks.
    Zone {
        id: String,
        base: i64,
    },
    /// `DateTimeOffset`: the clock time in ticks (as `.DateTime`) and the offset in ticks.
    Offset {
        clock: i64,
        offset: i64,
    },
    /// A `Regex` made from constructor arguments the sidecar already accepted (`regex.rs`): the arguments, and the
    /// canonical sidecar object its matching members go to.
    Regex {
        args: Vec<DotNetValue>,
        canonical: u64,
    },
    /// The object moved to the sidecar: its handle there.
    Forward(u64),
}

type Cell = Rc<RefCell<Local>>;

pub struct InprocSession {
    inner: Box<dyn DotNetSession>,
    locals: HashMap<u64, Cell>,
    next: u64,
    /// One `TimeZoneInfo` object per id (upper-cased key), as .NET caches them (reference equality is observable).
    zones: HashMap<String, Cell>,
    /// The canonical sidecar `Regex` per constructor key (`regex::key`), made by the first construction that succeeded.
    regexes: HashMap<String, u64>,
}

/// `DateTime.MaxValue` in ticks.
const MAX_TICKS: i64 = 3_155_378_975_999_999_999;
const TICKS_PER_MINUTE: i64 = 600_000_000;
const MAX_OFFSET_TICKS: i64 = 14 * 60 * TICKS_PER_MINUTE;

/// A `DateTimeOffset` when .NET would accept the arguments; `None` leaves the call (and its exception) to the sidecar.
fn offset_of(clock: i64, offset: i64) -> Option<Local> {
    let utc = clock.checked_sub(offset)?;
    let ok = (0..=MAX_TICKS).contains(&clock)
        && (0..=MAX_TICKS).contains(&utc)
        && offset % TICKS_PER_MINUTE == 0
        && offset.abs() <= MAX_OFFSET_TICKS;
    ok.then_some(Local::Offset { clock, offset })
}

type Reply = Result<DotNetReply, DotNetFailure>;

fn net_type(name: &str) -> DotNetType {
    DotNetType {
        assembly: "netstandard".into(),
        culture: Some("neutral".into()),
        public_key_token: Some("cc7b13ffcd2ddd51".into()),
        name: name.into(),
        ..Default::default()
    }
}

fn arg(v: DotNetValue) -> DotNetArg {
    DotNetArg {
        value: v,
        by_ref: false,
    }
}

fn is_static(t: &DotNetTarget, name: &str) -> bool {
    matches!(t, DotNetTarget::Static(ty) if ty.name == name)
}

fn utf16(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn newline() -> &'static str {
    if cfg!(windows) { "\r\n" } else { "\n" }
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn to_base64(b: &[u8]) -> String {
    let mut out = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n =
            (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 {
            B64[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if c.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

/// The bytes of a canonical Base64 text (length a multiple of 4, `=` only as the last one or two characters); anything
/// else, white space included, is left to .NET.
pub fn from_base64(s: &str) -> Option<Vec<u8>> {
    let b = s.as_bytes();
    if b.len() % 4 != 0 {
        return None;
    }
    let pad = b.iter().rev().take_while(|&&c| c == b'=').count();
    if pad > 2 {
        return None;
    }
    let body = &b[..b.len() - pad];
    let val = |c: u8| B64.iter().position(|&x| x == c).map(|p| p as u32);
    let mut out = Vec::with_capacity(b.len() / 4 * 3);
    for chunk in body.chunks(4) {
        let mut n = 0u32;
        for &c in chunk {
            n = (n << 6) | val(c)?;
        }
        n <<= 6 * (4 - chunk.len() as u32);
        let bytes = [(n >> 16) as u8, (n >> 8) as u8, n as u8];
        out.extend_from_slice(&bytes[..chunk.len() * 6 / 8]);
    }
    Some(out)
}

/// `Byte.ToString(format)` and `Int32.ToString(format)` for the formats D, d, X, x with an optional precision of up to
/// two digits and no format at all. Anything else is left to .NET.
fn format_integer(v: i64, unsigned_bits: Option<u32>, format: Option<&str>) -> Option<String> {
    let Some(f) = format else {
        return Some(v.to_string());
    };
    let mut chars = f.chars();
    let kind = chars.next()?;
    let digits: String = chars.collect();
    if digits.len() > 2 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let precision: usize = if digits.is_empty() { 0 } else { digits.parse().ok()? };
    match kind {
        'D' | 'd' => {
            let body = v.unsigned_abs().to_string();
            let padded = format!("{body:0>precision$}");
            Some(if v < 0 { format!("-{padded}") } else { padded })
        }
        'X' | 'x' => {
            let bits = if v < 0 { v as u32 as u64 } else { v as u64 };
            let _ = unsigned_bits;
            let s = if kind == 'X' {
                format!("{bits:X}")
            } else {
                format!("{bits:x}")
            };
            Some(format!("{s:0>precision$}"))
        }
        _ => None,
    }
}

impl InprocSession {
    fn over(inner: Box<dyn DotNetSession>) -> InprocSession {
        InprocSession {
            inner,
            locals: HashMap::new(),
            next: LOCAL_BASE,
            zones: HashMap::new(),
            regexes: HashMap::new(),
        }
    }

    fn local(&self, h: u64) -> Option<Cell> {
        self.locals.get(&h).cloned()
    }

    fn make(&mut self, state: Local, ty: &str) -> Reply {
        let h = self.next;
        self.next += 1;
        self.locals.insert(h, Rc::new(RefCell::new(state)));
        Ok(DotNetReply::value(DotNetValue::Handle {
            handle: h,
            ty: ty.into(),
        }))
    }

    /// Another handle on the object of `cell` (what `Append` answers: the same builder, a new handle).
    fn alias(&mut self, cell: &Cell, ty: &str) -> Reply {
        let h = self.next;
        self.next += 1;
        self.locals.insert(h, cell.clone());
        Ok(DotNetReply::value(DotNetValue::Handle {
            handle: h,
            ty: ty.into(),
        }))
    }

    fn bytes_reply(&mut self, b: Vec<u8>) -> Reply {
        self.make(Local::Bytes(b), "System.Byte[]")
    }

    /// The bytes behind a `byte[]` argument: an array made here, or one the sidecar holds (read once, by value, through
    /// Base64: `MemoryStream.ToArray()` feeds `ComputeHash` this way). `None` for anything else.
    fn local_bytes(&mut self, v: &DotNetValue) -> Option<Vec<u8>> {
        let DotNetValue::Handle { handle, ty } = v else {
            return None;
        };
        if let Some(cell) = self.local(*handle) {
            return match &*cell.borrow() {
                Local::Bytes(b) => Some(b.clone()),
                _ => None,
            };
        }
        if ty != "System.Byte[]" {
            return None;
        }
        let r = self
            .inner
            .invoke(
                DotNetTarget::Static(net_type("System.Convert")),
                "ToBase64String",
                vec![arg(DotNetValue::Handle {
                    handle: *handle,
                    ty: ty.clone(),
                })],
            )
            .ok()?;
        match r.value {
            DotNetValue::Str(t) => from_base64(&t),
            _ => None,
        }
    }

    /// The members of the set on a target; `None` when the stand-in does not take the call.
    fn try_local(&mut self, target: &DotNetTarget, member: &str, args: &[DotNetArg]) -> Option<Reply> {
        let vals: Vec<&DotNetValue> = args.iter().map(|a| &a.value).collect();
        match target {
            DotNetTarget::Static(_) => self.try_static(target, member, &vals),
            DotNetTarget::Instance(h) => {
                let cell = self.local(*h)?;
                self.try_instance(&cell, member, &vals)
            }
            DotNetTarget::Value(v) => {
                if member != "ToString" || vals.len() > 1 {
                    return None;
                }
                let format = match vals.first() {
                    None => None,
                    Some(DotNetValue::Str(s)) => Some(s.as_str()),
                    Some(_) => return None,
                };
                let text = match v {
                    DotNetValue::Byte(b) => format_integer(i64::from(*b), Some(8), format)?,
                    DotNetValue::Int32(i) => format_integer(i64::from(*i), None, format)?,
                    _ => return None,
                };
                Some(Ok(DotNetReply::value(DotNetValue::Str(text))))
            }
        }
    }

    fn try_static(&mut self, target: &DotNetTarget, member: &str, vals: &[&DotNetValue]) -> Option<Reply> {
        if is_static(target, "System.Math") {
            return math::math(member, vals).map(|v| Ok(DotNetReply::value(v)));
        }
        if is_static(target, "System.TimeSpan") {
            return math::time_span(member, vals).map(|v| Ok(DotNetReply::value(v)));
        }
        if is_static(target, "System.Text.Encoding") && vals.is_empty() {
            let enc = match member {
                "UTF8" => Enc::Utf8,
                "ASCII" => Enc::Ascii,
                "Unicode" => Enc::Unicode,
                _ => return None,
            };
            return Some(self.make(Local::Encoding(enc), enc.clr_type()));
        }
        if is_static(target, "System.Convert") && vals.len() == 1 {
            return match member {
                "ToBase64String" => {
                    let b = self.local_bytes(vals[0])?;
                    Some(Ok(DotNetReply::value(DotNetValue::Str(to_base64(&b)))))
                }
                "FromBase64String" => {
                    let DotNetValue::Str(s) = vals[0] else { return None };
                    let b = from_base64(s)?;
                    Some(self.bytes_reply(b))
                }
                _ => None,
            };
        }
        if is_static(target, "System.TimeZoneInfo") && member == "FindSystemTimeZoneById" && vals.len() == 1 {
            let DotNetValue::Str(id) = vals[0] else { return None };
            // .NET caches by id, ignoring case: a second lookup in another case finds the same object.
            if let Some(cell) = self.zones.get(&id.to_ascii_uppercase()).cloned() {
                return Some(self.alias(&cell, "System.TimeZoneInfo"));
            }
            let base = crate::tz::base_utc_offset_ticks(id)?;
            let reply = self.make(Local::Zone { id: id.clone(), base }, "System.TimeZoneInfo");
            if let Ok(DotNetReply {
                value: DotNetValue::Handle { handle, .. },
                ..
            }) = &reply
            {
                let cell = self.locals[handle].clone();
                self.zones.insert(id.to_ascii_uppercase(), cell);
            }
            return Some(reply);
        }
        if is_static(target, "System.Security.Cryptography.HashAlgorithm") && member == "Create" && vals.len() == 1 {
            let DotNetValue::Str(name) = vals[0] else { return None };
            let algo = Algo::of(name)?;
            return Some(self.make(Local::Hash { algo, disposed: false }, algo.clr_type()));
        }
        None
    }

    fn try_instance(&mut self, cell: &Cell, member: &str, vals: &[&DotNetValue]) -> Option<Reply> {
        enum Act {
            Reply(DotNetValue),
            Alias(&'static str),
            Fresh(Local, &'static str),
            Bytes(Vec<u8>),
            Void,
        }
        let act = {
            let mut st = cell.borrow_mut();
            match &mut *st {
                Local::Builder(s) => match (member, vals) {
                    ("Append", [DotNetValue::Str(t)]) => {
                        s.push_str(t);
                        Act::Alias("System.Text.StringBuilder")
                    }
                    ("Append", [DotNetValue::Char(c)]) => {
                        s.push(char::from_u32(u32::from(*c))?);
                        Act::Alias("System.Text.StringBuilder")
                    }
                    ("AppendLine", []) => {
                        s.push_str(newline());
                        Act::Alias("System.Text.StringBuilder")
                    }
                    ("AppendLine", [DotNetValue::Str(t)]) => {
                        s.push_str(t);
                        s.push_str(newline());
                        Act::Alias("System.Text.StringBuilder")
                    }
                    ("Clear", []) => {
                        s.clear();
                        Act::Alias("System.Text.StringBuilder")
                    }
                    ("Length", []) => Act::Reply(DotNetValue::Int32(i32::try_from(s.encode_utf16().count()).ok()?)),
                    ("ToString", []) => Act::Reply(DotNetValue::Str(s.clone())),
                    ("ToString", [DotNetValue::Int32(start), DotNetValue::Int32(len)]) => {
                        let u = utf16(s);
                        let (a, n) = (usize::try_from(*start).ok()?, usize::try_from(*len).ok()?);
                        let part = u.get(a..a.checked_add(n)?)?;
                        Act::Reply(DotNetValue::Str(String::from_utf16(part).ok()?))
                    }
                    _ => return None,
                },
                Local::Bytes(b) => match (member, vals) {
                    ("Length", []) => Act::Reply(DotNetValue::Int32(i32::try_from(b.len()).ok()?)),
                    ("GetValue", [DotNetValue::Int32(i)]) => {
                        Act::Reply(DotNetValue::Byte(*b.get(usize::try_from(*i).ok()?)?))
                    }
                    ("SetValue", [DotNetValue::Byte(v), DotNetValue::Int32(i)]) => {
                        *b.get_mut(usize::try_from(*i).ok()?)? = *v;
                        Act::Void
                    }
                    _ => return None,
                },
                Local::Encoding(e) => match (member, vals) {
                    ("GetBytes", [DotNetValue::Str(t)]) => Act::Bytes(e.bytes(t)?),
                    ("GetByteCount", [DotNetValue::Str(t)]) => {
                        Act::Reply(DotNetValue::Int32(i32::try_from(e.bytes(t)?.len()).ok()?))
                    }
                    ("GetString", [v]) => {
                        let e = *e;
                        drop(st);
                        let b = self.local_bytes(v)?;
                        let text = e.string(&b)?;
                        return Some(Ok(DotNetReply::value(DotNetValue::Str(text))));
                    }
                    _ => return None,
                },
                Local::Hash { algo, disposed } => match (member, vals) {
                    ("ComputeHash", [v]) if !*disposed => {
                        let algo = *algo;
                        drop(st);
                        let b = self.local_bytes(v)?;
                        return Some(self.bytes_reply(algo.digest(&b)));
                    }
                    ("Dispose", []) => {
                        *disposed = true;
                        Act::Void
                    }
                    _ => return None,
                },
                Local::Zone { id, base } => match (member, vals) {
                    ("BaseUtcOffset", []) => Act::Reply(DotNetValue::TimeSpan(*base)),
                    // ADR-038 amendment 2026-10-27: the UTC zone has no offset at any instant, whatever the kind of
                    // the DateTime (the session's local zone is UTC too, ADR-014 section 9). Every other zone goes to
                    // the sidecar, which has the host's own rules. Pinned by the differential test against the
                    // sidecar (`al-test-cli/tests/dotnet_diff.rs`, "GetUtcOffset(DateTime)").
                    ("GetUtcOffset", [DotNetValue::DateTime(_)]) if id.eq_ignore_ascii_case("UTC") => {
                        Act::Reply(DotNetValue::TimeSpan(0))
                    }
                    _ => return None,
                },
                Local::Offset { clock, offset } => match (member, vals) {
                    ("Offset", []) => Act::Reply(DotNetValue::TimeSpan(*offset)),
                    ("DateTime", []) => Act::Reply(DotNetValue::DateTime(*clock)),
                    // The session's local time zone is UTC (ADR-014 section 9): the local time is the UTC time.
                    ("LocalDateTime", []) => Act::Reply(DotNetValue::DateTime(*clock - *offset)),
                    ("ToLocalTime", []) => Act::Fresh(offset_of(*clock - *offset, 0)?, "System.DateTimeOffset"),
                    _ => return None,
                },
                Local::Forward(_) | Local::Regex { .. } => return None,
            }
        };
        Some(match act {
            Act::Reply(v) => Ok(DotNetReply::value(v)),
            Act::Alias(ty) => self.alias(cell, ty),
            Act::Bytes(b) => self.bytes_reply(b),
            Act::Fresh(state, ty) => self.make(state, ty),
            Act::Void => Ok(DotNetReply::value(DotNetValue::Null)),
        })
    }

    /// The sidecar handle of local object `h`, rebuilding the object there on first need.
    fn remote_of(&mut self, h: u64) -> Result<u64, DotNetFailure> {
        let Some(cell) = self.local(h) else { return Ok(h) };
        let made = match &*cell.borrow() {
            Local::Forward(r) => return Ok(*r),
            Local::Builder(s) => self.inner.construct(
                &net_type("System.Text.StringBuilder"),
                vec![arg(DotNetValue::Str(s.clone()))],
            ),
            Local::Bytes(b) => self.inner.invoke(
                DotNetTarget::Static(net_type("System.Convert")),
                "FromBase64String",
                vec![arg(DotNetValue::Str(to_base64(b)))],
            ),
            Local::Encoding(e) => self.inner.invoke(
                DotNetTarget::Static(net_type("System.Text.Encoding")),
                e.member(),
                vec![],
            ),
            Local::Zone { id, .. } => self.inner.invoke(
                DotNetTarget::Static(net_type("System.TimeZoneInfo")),
                "FindSystemTimeZoneById",
                vec![arg(DotNetValue::Str(id.clone()))],
            ),
            Local::Offset { clock, offset } => self.inner.construct(
                &net_type("System.DateTimeOffset"),
                vec![arg(DotNetValue::Int64(*clock)), arg(DotNetValue::TimeSpan(*offset))],
            ),
            Local::Hash { algo, .. } => self.inner.invoke(
                DotNetTarget::Static(net_type("System.Security.Cryptography.HashAlgorithm")),
                "Create",
                vec![arg(DotNetValue::Str(algo.name().into()))],
            ),
            Local::Regex { args, .. } => self
                .inner
                .construct(&net_type(regex::REGEX), args.iter().cloned().map(arg).collect()),
        }?;
        let DotNetValue::Handle { handle: r, .. } = made.value else {
            return Err(DotNetFailure::Refused(
                "the sidecar did not answer a handle for a migrated object".into(),
            ));
        };
        if matches!(&*cell.borrow(), Local::Hash { disposed: true, .. }) {
            self.inner.invoke(DotNetTarget::Instance(r), "Dispose", vec![])?;
        }
        *cell.borrow_mut() = Local::Forward(r);
        Ok(r)
    }

    /// `new Regex(...)` (`regex.rs`): a local object when these arguments were accepted before, else the sidecar's
    /// construction, whose object becomes the canonical one when it succeeds.
    fn construct_regex(&mut self, ty: &DotNetType, args: Vec<DotNetArg>) -> Reply {
        let vals: Vec<&DotNetValue> = args.iter().map(|a| &a.value).collect();
        let Some(key) = regex::key(&vals) else {
            let args = self.args_out(args)?;
            return self.inner.construct(ty, args);
        };
        let values: Vec<DotNetValue> = args.iter().map(|a| a.value.clone()).collect();
        let canonical = match self.regexes.get(&key) {
            Some(&c) => c,
            None => {
                let made = self.inner.construct(ty, args)?;
                let DotNetValue::Handle { handle, ty: made_ty } = made.value else {
                    return Ok(made);
                };
                if made_ty != regex::REGEX || !made.by_ref.is_empty() || made.streams != StreamEffects::default() {
                    return Ok(DotNetReply {
                        value: DotNetValue::Handle { handle, ty: made_ty },
                        ..made
                    });
                }
                self.regexes.insert(key, handle);
                handle
            }
        };
        self.make(
            Local::Regex {
                args: values,
                canonical,
            },
            regex::REGEX,
        )
    }

    /// The canonical sidecar object a matching member of a local `Regex` goes to.
    fn canonical_regex(&self, target: &DotNetTarget, member: &str) -> Option<u64> {
        let DotNetTarget::Instance(h) = target else { return None };
        let cell = self.locals.get(h)?;
        match &*cell.borrow() {
            Local::Regex { canonical, .. } if regex::is_matching(member) => Some(*canonical),
            _ => None,
        }
    }

    fn value_out(&mut self, v: DotNetValue) -> Result<DotNetValue, DotNetFailure> {
        Ok(match v {
            DotNetValue::Handle { handle, ty } => DotNetValue::Handle {
                handle: self.remote_of(handle)?,
                ty,
            },
            other => other,
        })
    }

    fn args_out(&mut self, args: Vec<DotNetArg>) -> Result<Vec<DotNetArg>, DotNetFailure> {
        args.into_iter()
            .map(|a| {
                Ok(DotNetArg {
                    value: self.value_out(a.value)?,
                    by_ref: a.by_ref,
                })
            })
            .collect()
    }

    fn target_out(&mut self, t: DotNetTarget) -> Result<DotNetTarget, DotNetFailure> {
        Ok(match t {
            DotNetTarget::Instance(h) => DotNetTarget::Instance(self.remote_of(h)?),
            DotNetTarget::Value(v) => DotNetTarget::Value(self.value_out(v)?),
            s => s,
        })
    }
}

impl DotNetSession for InprocSession {
    fn is_in_process_utc_zone(&self, handle: DotNetHandle) -> bool {
        self.local(handle)
            .is_some_and(|c| matches!(&*c.borrow(), Local::Zone { id, .. } if id.eq_ignore_ascii_case("UTC")))
    }
    fn construct(&mut self, ty: &DotNetType, args: Vec<DotNetArg>) -> Reply {
        if ty.name == "System.Text.StringBuilder" {
            let vals: Vec<&DotNetValue> = args.iter().map(|a| &a.value).collect();
            match vals.as_slice() {
                [] => return self.make(Local::Builder(String::new()), "System.Text.StringBuilder"),
                [DotNetValue::Str(s)] => return self.make(Local::Builder(s.clone()), "System.Text.StringBuilder"),
                [DotNetValue::Int32(n)] if *n >= 0 => {
                    return self.make(Local::Builder(String::new()), "System.Text.StringBuilder");
                }
                _ => {}
            }
        }
        if ty.name == "System.DateTimeOffset" {
            let state = match args.iter().map(|a| &a.value).collect::<Vec<_>>().as_slice() {
                // `DateTimeOffset(DateTime)`: a clock time of unspecified kind is local time, and local is UTC.
                [DotNetValue::DateTime(t)] => offset_of(*t, 0),
                [DotNetValue::Int64(t), DotNetValue::TimeSpan(o)] => offset_of(*t, *o),
                _ => None,
            };
            if let Some(state) = state {
                return self.make(state, "System.DateTimeOffset");
            }
        }
        if ty.name == regex::REGEX {
            return self.construct_regex(ty, args);
        }
        let args = self.args_out(args)?;
        self.inner.construct(ty, args)
    }

    fn invoke(&mut self, target: DotNetTarget, member: &str, args: Vec<DotNetArg>) -> Reply {
        if let Some(r) = self.try_local(&target, member, &args) {
            return r;
        }
        let target = match self.canonical_regex(&target, member) {
            Some(c) => DotNetTarget::Instance(c),
            None => self.target_out(target)?,
        };
        let args = self.args_out(args)?;
        self.inner.invoke(target, member, args)
    }

    fn invoke_generic(
        &mut self,
        target: DotNetTarget,
        member: &str,
        type_arg: &DotNetType,
        args: Vec<DotNetArg>,
    ) -> Reply {
        let target = self.target_out(target)?;
        let args = self.args_out(args)?;
        self.inner.invoke_generic(target, member, type_arg, args)
    }

    fn get_property(&mut self, target: DotNetTarget, name: &str) -> Reply {
        if let Some(r) = self.try_local(&target, name, &[]) {
            return r;
        }
        let target = self.target_out(target)?;
        self.inner.get_property(target, name)
    }

    fn set_property(&mut self, target: DotNetTarget, name: &str, value: DotNetValue) -> Reply {
        let target = self.target_out(target)?;
        let value = self.value_out(value)?;
        self.inner.set_property(target, name, value)
    }

    fn stream_write(&mut self, handle: DotNetHandle, bytes: &[u8]) -> Result<(), DotNetFailure> {
        let h = self.remote_of(handle)?;
        self.inner.stream_write(h, bytes)
    }

    fn stream_read(&mut self, handle: DotNetHandle, max: Option<usize>) -> Result<Vec<u8>, DotNetFailure> {
        let h = self.remote_of(handle)?;
        self.inner.stream_read(h, max)
    }

    fn release(&mut self, handles: &[DotNetHandle]) {
        let mut remote = Vec::new();
        for &h in handles {
            match self.locals.remove(&h) {
                None => remote.push(h),
                Some(cell) => {
                    // The last handle on an object that moved to the sidecar frees it there.
                    if Rc::strong_count(&cell) == 1 {
                        if let Local::Forward(r) = &*cell.borrow() {
                            remote.push(*r);
                        }
                    }
                }
            }
        }
        if !remote.is_empty() {
            self.inner.release(&remote);
        }
    }

    fn close(mut self: Box<Self>) {
        let mut canonical: Vec<u64> = self.regexes.values().copied().collect();
        if !canonical.is_empty() {
            canonical.sort_unstable();
            self.inner.release(&canonical);
        }
        self.inner.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// A sidecar that answers nothing and writes down what reached it.
    struct Stub(Arc<Mutex<Vec<String>>>);

    impl DotNetSession for Stub {
        fn construct(&mut self, ty: &DotNetType, _: Vec<DotNetArg>) -> Reply {
            self.0.lock().unwrap().push(format!("new {}", ty.name));
            Err(DotNetFailure::Refused("stub".into()))
        }
        fn invoke(&mut self, _: DotNetTarget, member: &str, _: Vec<DotNetArg>) -> Reply {
            self.0.lock().unwrap().push(member.to_string());
            Err(DotNetFailure::Refused("stub".into()))
        }
        fn invoke_generic(&mut self, _: DotNetTarget, member: &str, g: &DotNetType, _: Vec<DotNetArg>) -> Reply {
            self.0.lock().unwrap().push(format!("{member}<{}>", g.name));
            Err(DotNetFailure::Refused("stub".into()))
        }
        fn get_property(&mut self, _: DotNetTarget, name: &str) -> Reply {
            self.0.lock().unwrap().push(name.to_string());
            Err(DotNetFailure::Refused("stub".into()))
        }
        fn set_property(&mut self, _: DotNetTarget, name: &str, _: DotNetValue) -> Reply {
            self.0.lock().unwrap().push(name.to_string());
            Err(DotNetFailure::Refused("stub".into()))
        }
        fn release(&mut self, _: &[DotNetHandle]) {}
        fn close(self: Box<Self>) {}
    }

    fn session() -> (InprocSession, Arc<Mutex<Vec<String>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s = InprocSession::over(Box::new(Stub(seen.clone())));
        (s, seen)
    }

    fn handle_of(r: Reply) -> u64 {
        match r.unwrap().value {
            DotNetValue::Handle { handle, .. } => handle,
            other => panic!("{other:?}"),
        }
    }

    // A generic-only method closed over a type argument (OpenXml's GetWorksheetByName<T>) reaches the sidecar with its type.
    #[test]
    fn a_generic_call_reaches_the_sidecar_with_its_type_argument() {
        let (mut s, seen) = session();
        let _ = s.invoke_generic(
            DotNetTarget::Static(net_type("System.Runtime.CompilerServices.Unsafe")),
            "As",
            &net_type("System.Text.StringBuilder"),
            vec![],
        );
        assert_eq!(*seen.lock().unwrap(), vec!["As<System.Text.StringBuilder>".to_string()]);
    }

    fn listed() -> Vec<String> {
        MEMBERS
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .map(String::from)
            .collect()
    }

    #[test]
    fn every_listed_member_is_answered_without_the_sidecar() {
        let (mut s, seen) = session();
        let sb = handle_of(s.construct(&net_type("System.Text.StringBuilder"), vec![]));
        let bytes = handle_of(s.invoke(
            DotNetTarget::Static(net_type("System.Convert")),
            "FromBase64String",
            vec![arg(DotNetValue::Str("AQID".into()))],
        ));
        let enc = handle_of(s.invoke(DotNetTarget::Static(net_type("System.Text.Encoding")), "UTF8", vec![]));
        let hash = handle_of(s.invoke(
            DotNetTarget::Static(net_type("System.Security.Cryptography.HashAlgorithm")),
            "Create",
            vec![arg(DotNetValue::Str("SHA256".into()))],
        ));
        let me = |h: u64| DotNetTarget::Instance(h);
        let b = |h: u64| {
            arg(DotNetValue::Handle {
                handle: h,
                ty: "x".into(),
            })
        };
        let calls: Vec<Reply> = vec![
            s.invoke(me(sb), "Append", vec![arg(DotNetValue::Str("a".into()))]),
            s.invoke(me(sb), "AppendLine", vec![]),
            s.invoke(me(sb), "Length", vec![]),
            s.invoke(me(sb), "ToString", vec![]),
            s.invoke(me(sb), "Clear", vec![]),
            s.invoke(me(bytes), "GetValue", vec![arg(DotNetValue::Int32(0))]),
            s.invoke(
                me(bytes),
                "SetValue",
                vec![arg(DotNetValue::Byte(1)), arg(DotNetValue::Int32(0))],
            ),
            s.invoke(me(bytes), "Length", vec![]),
            s.invoke(
                DotNetTarget::Value(DotNetValue::Byte(1)),
                "ToString",
                vec![arg(DotNetValue::Str("X2".into()))],
            ),
            s.invoke(DotNetTarget::Value(DotNetValue::Int32(1)), "ToString", vec![]),
            s.invoke(me(enc), "GetBytes", vec![arg(DotNetValue::Str("a".into()))]),
            s.invoke(me(enc), "GetByteCount", vec![arg(DotNetValue::Str("a".into()))]),
            s.invoke(me(enc), "GetString", vec![b(bytes)]),
            s.invoke(me(hash), "ComputeHash", vec![b(bytes)]),
            s.invoke(me(hash), "Dispose", vec![]),
            s.invoke(
                DotNetTarget::Static(net_type("System.Convert")),
                "ToBase64String",
                vec![b(bytes)],
            ),
        ];
        for (k, c) in calls.iter().enumerate() {
            assert!(c.is_ok(), "call {k} reached the sidecar: {c:?}");
        }
        assert!(seen.lock().unwrap().is_empty(), "forwarded: {:?}", seen.lock().unwrap());
        // The list names every one of them (and the properties and constructors the calls above stand for).
        assert!(listed().len() >= 20, "{:?}", listed());
    }

    #[test]
    fn time_zone_and_offset_members_are_answered_without_the_sidecar() {
        let (mut s, seen) = session();
        let tz = handle_of(s.invoke(
            DotNetTarget::Static(net_type("System.TimeZoneInfo")),
            "FindSystemTimeZoneById",
            vec![arg(DotNetValue::Str("W. Europe Standard Time".into()))],
        ));
        let base = s.get_property(DotNetTarget::Instance(tz), "BaseUtcOffset").unwrap();
        assert_eq!(base.value, DotNetValue::TimeSpan(36_000_000_000));
        let dto = handle_of(s.construct(
            &net_type("System.DateTimeOffset"),
            vec![arg(DotNetValue::DateTime(1_000_000))],
        ));
        let me = DotNetTarget::Instance(dto);
        assert_eq!(
            s.get_property(me.clone(), "Offset").unwrap().value,
            DotNetValue::TimeSpan(0)
        );
        assert_eq!(
            s.get_property(me.clone(), "LocalDateTime").unwrap().value,
            DotNetValue::DateTime(1_000_000)
        );
        assert_eq!(
            s.get_property(me.clone(), "DateTime").unwrap().value,
            DotNetValue::DateTime(1_000_000)
        );
        assert!(s.invoke(me, "ToLocalTime", vec![]).is_ok());
        let two = handle_of(s.construct(
            &net_type("System.DateTimeOffset"),
            vec![
                arg(DotNetValue::Int64(900_000_000_000)),
                arg(DotNetValue::TimeSpan(36_000_000_000)),
            ],
        ));
        assert_eq!(
            s.get_property(DotNetTarget::Instance(two), "LocalDateTime")
                .unwrap()
                .value,
            DotNetValue::DateTime(864_000_000_000)
        );
        assert!(seen.lock().unwrap().is_empty(), "forwarded: {:?}", seen.lock().unwrap());
    }

    #[test]
    fn utc_offset_of_the_utc_zone_is_answered_without_the_sidecar() {
        let (mut s, seen) = session();
        for id in ["UTC", "utc"] {
            let tz = handle_of(s.invoke(
                DotNetTarget::Static(net_type("System.TimeZoneInfo")),
                "FindSystemTimeZoneById",
                vec![arg(DotNetValue::Str(id.into()))],
            ));
            for t in [0, 638_000_000_000_000_000, MAX_TICKS] {
                let r = s.invoke(
                    DotNetTarget::Instance(tz),
                    "GetUtcOffset",
                    vec![arg(DotNetValue::DateTime(t))],
                );
                assert_eq!(r.unwrap().value, DotNetValue::TimeSpan(0));
            }
        }
        assert!(seen.lock().unwrap().is_empty(), "forwarded: {:?}", seen.lock().unwrap());
    }

    /// The fact the VM's off-Windows gate of `GetUtcOffset` asks (al-vm `dotnet/zone_rules.rs`): only a zone made here
    /// for the id `UTC` answers yes; another zone made here and a handle the stand-in does not hold answer no.
    #[test]
    fn only_the_utc_zone_made_in_process_is_reported_as_utc() {
        let (mut s, _) = session();
        let zone = |s: &mut InprocSession, id: &str| {
            handle_of(s.invoke(
                DotNetTarget::Static(net_type("System.TimeZoneInfo")),
                "FindSystemTimeZoneById",
                vec![arg(DotNetValue::Str(id.into()))],
            ))
        };
        for id in ["UTC", "utc"] {
            let h = zone(&mut s, id);
            assert!(s.is_in_process_utc_zone(h), "{id}");
        }
        let other = zone(&mut s, "Greenwich Standard Time");
        assert!(!s.is_in_process_utc_zone(other));
        assert!(!s.is_in_process_utc_zone(7));
    }

    #[test]
    fn utc_offset_of_another_zone_goes_to_the_sidecar() {
        let (mut s, seen) = session();
        let tz = handle_of(s.invoke(
            DotNetTarget::Static(net_type("System.TimeZoneInfo")),
            "FindSystemTimeZoneById",
            vec![arg(DotNetValue::Str("Greenwich Standard Time".into()))],
        ));
        let _ = s.invoke(
            DotNetTarget::Instance(tz),
            "GetUtcOffset",
            vec![arg(DotNetValue::DateTime(0))],
        );
        // The zone moves to the sidecar first (rebuilt there by its id); the stub refuses that, so the call stops there.
        let seen = seen.lock().unwrap().clone();
        assert_eq!(seen, ["FindSystemTimeZoneById"], "{seen:?}");
    }

    #[test]
    fn what_the_stand_in_cannot_know_goes_to_the_sidecar() {
        let (mut s, seen) = session();
        // An id outside the table; an offset that is not whole minutes; ticks that leave the DateTime range.
        let _ = s.invoke(
            DotNetTarget::Static(net_type("System.TimeZoneInfo")),
            "FindSystemTimeZoneById",
            vec![arg(DotNetValue::Str("Nowhere Standard Time".into()))],
        );
        let _ = s.construct(
            &net_type("System.DateTimeOffset"),
            vec![arg(DotNetValue::Int64(5)), arg(DotNetValue::TimeSpan(10))],
        );
        let _ = s.construct(
            &net_type("System.DateTimeOffset"),
            vec![arg(DotNetValue::Int64(5)), arg(DotNetValue::TimeSpan(36_000_000_000))],
        );
        assert_eq!(seen.lock().unwrap().len(), 3, "{:?}", seen.lock().unwrap());
    }

    #[test]
    fn members_outside_the_list_go_to_the_sidecar() {
        let (mut s, seen) = session();
        let sb = handle_of(s.construct(&net_type("System.Text.StringBuilder"), vec![]));
        let _ = s.construct(&net_type("System.IO.MemoryStream"), vec![]);
        for m in ["Insert", "Replace", "Remove", "Equals", "GetHashCode", "AppendFormat"] {
            let _ = s.invoke(DotNetTarget::Instance(sb), m, vec![]);
        }
        let _ = s.invoke(
            DotNetTarget::Static(net_type("System.Convert")),
            "ToString",
            vec![arg(DotNetValue::Int32(1))],
        );
        let seen = seen.lock().unwrap().clone();
        // The first thing the sidecar sees for a moved builder is its rebuilding.
        assert!(seen.iter().any(|m| m == "new System.IO.MemoryStream"), "{seen:?}");
        assert!(seen.iter().any(|m| m == "new System.Text.StringBuilder"), "{seen:?}");
        assert!(seen.iter().any(|m| m == "ToString"), "{seen:?}");
    }

    /// A sidecar that answers every construction with a new handle and every call with `true`, and writes down what
    /// reached it.
    struct Answering(Arc<Mutex<Vec<String>>>, u64);

    impl DotNetSession for Answering {
        fn construct(&mut self, ty: &DotNetType, args: Vec<DotNetArg>) -> Reply {
            self.0.lock().unwrap().push(format!("new {}", ty.name));
            if matches!(args.first().map(|a| &a.value), Some(DotNetValue::Str(p)) if p == "(") {
                return Err(DotNetFailure::Refused("invalid pattern".into()));
            }
            self.1 += 1;
            Ok(DotNetReply::value(DotNetValue::Handle {
                handle: self.1,
                ty: ty.name.clone(),
            }))
        }
        fn invoke(&mut self, t: DotNetTarget, member: &str, _: Vec<DotNetArg>) -> Reply {
            self.0.lock().unwrap().push(format!("{member} on {t:?}"));
            Ok(DotNetReply::value(DotNetValue::Bool(true)))
        }
        fn invoke_generic(&mut self, _: DotNetTarget, member: &str, _: &DotNetType, _: Vec<DotNetArg>) -> Reply {
            self.0.lock().unwrap().push(member.to_string());
            Ok(DotNetReply::value(DotNetValue::Null))
        }
        fn get_property(&mut self, t: DotNetTarget, name: &str) -> Reply {
            self.0.lock().unwrap().push(format!("get {name} on {t:?}"));
            Ok(DotNetReply::value(DotNetValue::Int32(0)))
        }
        fn set_property(&mut self, _: DotNetTarget, name: &str, _: DotNetValue) -> Reply {
            self.0.lock().unwrap().push(name.to_string());
            Ok(DotNetReply::value(DotNetValue::Null))
        }
        fn release(&mut self, h: &[DotNetHandle]) {
            self.0.lock().unwrap().push(format!("release {h:?}"));
        }
        fn close(self: Box<Self>) {
            self.0.lock().unwrap().push("close".into());
        }
    }

    fn answering() -> (InprocSession, Arc<Mutex<Vec<String>>>) {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s = InprocSession::over(Box::new(Answering(seen.clone(), 0)));
        (s, seen)
    }

    fn dec(t: &str) -> DotNetArg {
        arg(DotNetValue::Decimal(t.into()))
    }

    #[test]
    fn exact_math_members_and_from_ticks_are_answered_without_the_sidecar() {
        let (mut s, seen) = session();
        let math = || DotNetTarget::Static(net_type("System.Math"));
        let cases: Vec<(&str, Vec<DotNetArg>, DotNetValue)> = vec![
            ("Abs", vec![dec("-1.50")], DotNetValue::Decimal("1.50".into())),
            ("Floor", vec![dec("-0.5")], DotNetValue::Decimal("-1".into())),
            ("Ceiling", vec![dec("2.5")], DotNetValue::Decimal("3".into())),
            ("Truncate", vec![dec("-2.5")], DotNetValue::Decimal("-2".into())),
            ("Sign", vec![dec("-0.1")], DotNetValue::Int32(-1)),
            ("Max", vec![dec("1.0"), dec("1.00")], DotNetValue::Decimal("1.0".into())),
            (
                "Min",
                vec![dec("1.0"), dec("1.00")],
                DotNetValue::Decimal("1.00".into()),
            ),
        ];
        for (m, args, want) in cases {
            assert_eq!(s.invoke(math(), m, args).unwrap().value, want, "{m}");
        }
        let ts = s.invoke(
            DotNetTarget::Static(net_type("System.TimeSpan")),
            "FromTicks",
            vec![arg(DotNetValue::Int64(10_000_000))],
        );
        assert_eq!(ts.unwrap().value, DotNetValue::TimeSpan(10_000_000));
        assert!(seen.lock().unwrap().is_empty(), "forwarded: {:?}", seen.lock().unwrap());
    }

    #[test]
    fn math_through_a_double_goes_to_the_sidecar() {
        let (mut s, seen) = session();
        for m in ["Sqrt", "Sin", "Log", "Exp", "Pow"] {
            let _ = s.invoke(DotNetTarget::Static(net_type("System.Math")), m, vec![dec("2")]);
        }
        let _ = s.invoke(
            DotNetTarget::Static(net_type("System.Math")),
            "Abs",
            vec![arg(DotNetValue::Int32(i32::MIN))],
        );
        assert_eq!(seen.lock().unwrap().len(), 6, "{:?}", seen.lock().unwrap());
    }

    fn regex_args(p: &str) -> Vec<DotNetArg> {
        vec![
            arg(DotNetValue::Str(p.into())),
            arg(DotNetValue::Int32(0)),
            arg(DotNetValue::TimeSpan(10_000_000)),
        ]
    }

    #[test]
    fn a_regex_built_again_from_the_same_arguments_costs_no_request() {
        let (mut s, seen) = answering();
        let ty = net_type(regex::REGEX);
        let first = handle_of(s.construct(&ty, regex_args("^a$")));
        let _ = s.invoke(
            DotNetTarget::Instance(first),
            "IsMatch",
            vec![arg(DotNetValue::Str("a".into()))],
        );
        s.release(&[first]);
        for _ in 0..3 {
            let again = handle_of(s.construct(&ty, regex_args("^a$")));
            assert!(again >= LOCAL_BASE);
            let r = s.invoke(
                DotNetTarget::Instance(again),
                "IsMatch",
                vec![arg(DotNetValue::Str("a".into()))],
            );
            assert_eq!(r.unwrap().value, DotNetValue::Bool(true));
            s.release(&[again]);
        }
        let seen = seen.lock().unwrap().clone();
        // One construction in the sidecar, then only the matches, all on that one object.
        assert_eq!(
            seen,
            [
                "new System.Text.RegularExpressions.Regex",
                "IsMatch on Instance(1)",
                "IsMatch on Instance(1)",
                "IsMatch on Instance(1)",
                "IsMatch on Instance(1)",
            ],
            "{seen:?}"
        );
    }

    #[test]
    fn a_regex_member_that_can_see_identity_builds_the_object_for_real() {
        let (mut s, seen) = answering();
        let ty = net_type(regex::REGEX);
        let _ = handle_of(s.construct(&ty, regex_args("^a$")));
        let two = handle_of(s.construct(&ty, regex_args("^a$")));
        let _ = s.invoke(DotNetTarget::Instance(two), "GetHashCode", vec![]);
        let _ = s.get_property(DotNetTarget::Instance(two), "Options");
        s.release(&[two]);
        Box::new(s).close();
        let seen = seen.lock().unwrap().clone();
        assert_eq!(
            seen,
            [
                "new System.Text.RegularExpressions.Regex",
                "new System.Text.RegularExpressions.Regex",
                "GetHashCode on Instance(2)",
                "get Options on Instance(2)",
                "release [2]",
                "release [1]",
                "close",
            ],
            "{seen:?}"
        );
    }

    #[test]
    fn an_invalid_pattern_is_sent_every_time() {
        let (mut s, seen) = answering();
        let ty = net_type(regex::REGEX);
        for _ in 0..2 {
            assert!(s.construct(&ty, regex_args("(")).is_err());
        }
        // A case-insensitive pattern depends on the culture current at construction: never proxied.
        for _ in 0..2 {
            let _ = s.construct(&ty, vec![arg(DotNetValue::Str("a".into())), arg(DotNetValue::Int32(1))]);
        }
        assert_eq!(seen.lock().unwrap().len(), 4, "{:?}", seen.lock().unwrap());
    }

    #[test]
    fn the_wrap_leaves_the_host_alone_when_asked() {
        // Only the shape: a wrapped host and a bare one are both hosts; the switch is read at wrap time.
        struct H;
        impl DotNetHost for H {
            fn availability(&self) -> Result<(), DotNetUnavailable> {
                Ok(())
            }
            fn open_session(&self, _: &DotNetSessionConfig) -> Box<dyn DotNetSession> {
                Box::new(Stub(Arc::new(Mutex::new(Vec::new()))))
            }
        }
        let wrapped = wrap(Arc::new(H));
        assert!(wrapped.availability().is_ok());
    }
}
