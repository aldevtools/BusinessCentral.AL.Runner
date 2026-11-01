//! An AL `OutStream` or `InStream` assigned to a `DotNet` System.IO.Stream variable (Base Application's `DotNet_Stream`
//! wrapper, `FromOutStream` and `FromInStream`). BC hands .NET its own blob stream, a `NavBLOB+InternalStream`; the runtime
//! serves that object itself (ADR-014 section 1), so a run needs no .NET host for it.
//!
//! Pinned by tests/al-language-onprem-dotnet/lt-sg-dotnet/SgDotNetStreams.al (27, 28, 29):
//! - the type name, `CanRead`, `CanWrite`, `CanSeek` (all Yes), and right after the assignment `Length` and `Position` of
//!   either stream (:106-126, :369-397);
//! - over an `OutStream`: `WriteByte` and `Write(byte[], offset, count)` reach the Blob at once, `Flush` changes nothing
//!   (:129-151); AL and .NET writes keep their own positions, an AL write goes over a .NET byte (:154-172); after `Close` or
//!   `Dispose` an AL write goes over the .NET bytes from the AL position (:175-226) and a .NET `WriteByte` still succeeds
//!   (:192, what it wrote is never read there, so the Blob is marked: `DOTNET_WRITE_AFTER_CLOSE`);
//! - over an `InStream`: `ReadByte` reads from the start without moving the AL stream, and after AL read the stream to its end
//!   `ReadByte` answers -1 (:383-395);
//! - an `OutStream` assigned to a System.Object raises NotSupportedType "The type NavOutStream is unknown." (:247-264);
//! - Base Application's `DotNet_Stream.FromOutStream` then `WriteByte` (:431-446).
//!
//! `Position` after an AL read, a `Position` set and the stream assigned back to an `InStream`: blob_stream_position.rs.
//!
//! Not pinned, so refused: any other member (`Read`, `Seek`, `SetLength`, `CopyTo`, ...), any member but `Position` after a
//! `Position` set; `Length` and `Position` once anything read or wrote after the assignment (but see above); a read on the
//! `OutStream` side and a write, `Flush`, `Close` or `Dispose` on the `InStream` side; any member after `Close` or `Dispose`
//! other than `WriteByte`; a second `Close`; a stream taken from an `OutStream` whose position is not the end of the Blob or
//! from an `InStream` that was read from; an AL write that ends elsewhere than the .NET position and an AL read that stops
//! before the end (whether the two positions are one is not pinned then); the `OutStream` created again; a byte value out of
//! range; a `Write` range outside the array; an `InStream` assigned to a System.Object or any stream to another .NET type.
//! The object is never handed on to .NET code: `value_to_wire` refuses a runtime-provided object.

use super::registry::{MemberKind, Pin, RegistryMember, RegistryType};
use super::*;
use al_builtins::dotnet_sigs::{NAV_BLOB_STREAM, NAV_BLOB_STREAM_TYPE};
use al_builtins::library_kind::LibraryKind;
use al_builtins::stream::{BlobBuf, DOTNET_WRITE_AFTER_CLOSE};
use std::sync::atomic::{AtomicU32, Ordering};

const PIN: &str = "tests/al-language-onprem-dotnet/lt-sg-dotnet/SgDotNetStreams.al:106-446";

// The state of the object: which side it is, the Blob, the AL stream, the .NET position, the AL position seen at the last
// .NET call, whether it was closed, and (OutStream side) the id written into the Blob.
const S_OUT: usize = 0;
const S_BLOB: usize = 1;
pub(super) const S_AL: usize = 2;
const S_POS: usize = 3;
const S_SEEN: usize = 4;
const S_CLOSED: usize = 5;
const S_ID: usize = 6;
// The position and the Blob length when AL gave the stream to .NET: `Length` and `Position` are pinned only then.
const S_START: usize = 7;
const S_START_LEN: usize = 8;
// Whether a .NET read or write ran on the stream, even one that moved nothing (a `ReadByte` at the end).
pub(super) const S_TOUCHED: usize = 9;
/// The Position .NET code set on the InStream side (Null when none; blob_stream_position.rs).
pub(super) const S_SET: usize = 10;

static NEXT_VIEW: AtomicU32 = AtomicU32::new(1);

fn pin() -> Pin {
    Pin::Corpus(PIN.into())
}

fn int(v: &Value) -> usize {
    match v {
        Value::BigInteger(n) => *n as usize,
        _ => 0,
    }
}

fn big(n: usize) -> Value {
    Value::BigInteger(n as i64)
}

impl Session {
    /// `S := Stream` where `S` is the `DotNet` type `decl` (the bytecode's conversion of an AL stream to a `DotNet` slot).
    pub(crate) fn al_stream_into_dotnet(&mut self, v: Value, decl: u32) -> Result<Value, Signal> {
        let d = &self.engine.program.dotnet_types[decl as usize];
        let mscorlib = d.assembly.eq_ignore_ascii_case("mscorlib");
        let (kind, h) = match &v {
            Value::Library(kind, h) if *kind == LibraryKind::OutStream || *kind == LibraryKind::InStream => {
                (kind.clone(), h)
            }
            _ => return ops::convert(v, &RtType::DotNet(decl)),
        };
        let out = kind == LibraryKind::OutStream;
        if mscorlib && d.type_name == "System.Object" && out {
            // SgDotNetStreams.al:247-264 SG_OutStreamToObject: the variable stays null.
            return al_builtins::native::raise("The type NavOutStream is unknown.", "NotSupportedType");
        }
        // The corpus declares the type in mscorlib (SgDotNetStreams.al:20); Base Application's `DotNet Stream` alias names it
        // in netstandard, and SgDotNetStreams.al:431 SG_BaseAppDotNetStream_FromOutStream runs that path.
        let stream_assembly = mscorlib || d.assembly.eq_ignore_ascii_case("netstandard");
        if !(stream_assembly && d.type_name == "System.IO.Stream") {
            return unsupported(format!(
                "an AL {} assigned to a DotNet {} ({}): only System.IO.Stream (mscorlib, netstandard) is pinned",
                if out { "OutStream" } else { "InStream" },
                d.type_name,
                d.assembly
            ));
        }
        let state = match &*h.borrow() {
            LibraryValue::OutStream(blob, _) => {
                let mut buf = blob.borrow_mut();
                plain(&buf)?;
                if buf.write_pos != buf.bytes.len() {
                    return unsupported(
                        "an OutStream whose position is not the end of its Blob given to a DotNet Stream (the .NET position is not pinned)",
                    );
                }
                let id = NEXT_VIEW.fetch_add(1, Ordering::Relaxed);
                buf.dotnet_view = id;
                vec![
                    Value::Boolean(true),
                    Value::Blob(blob.clone()),
                    v.clone(),
                    big(buf.write_pos),
                    big(buf.write_pos),
                    Value::Boolean(false),
                    Value::BigInteger(i64::from(id)),
                    big(buf.write_pos),
                    big(buf.bytes.len()),
                    Value::Boolean(false),
                    Value::Null,
                ]
            }
            LibraryValue::InStream(st) => {
                let Some(blob) = st.live_blob() else {
                    return unsupported(
                        "an InStream that does not read a Blob given to a DotNet Stream (only a Blob's is pinned)",
                    );
                };
                plain(&blob.borrow())?;
                if st.pos != 0 || st.hit_end || st.handed_to_content || st.owner_gone || st.not_initialized {
                    return unsupported(
                        "an InStream that was read from given to a DotNet Stream (only one at its start is pinned)",
                    );
                }
                vec![
                    Value::Boolean(false),
                    Value::Blob(blob.clone()),
                    v.clone(),
                    big(0),
                    big(0),
                    Value::Boolean(false),
                    Value::BigInteger(0),
                    big(0),
                    big(st.len()),
                    Value::Boolean(false),
                    Value::Null,
                ]
            }
            other => {
                return unsupported(format!(
                    "an AL stream that was not created ({}) given to a DotNet Stream (not pinned)",
                    other.describe()
                ));
            }
        };
        Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
            type_name: NAV_BLOB_STREAM.into(),
            state: RefCell::new(state),
        }))))
    }

    /// The bytes of a .NET byte array the host holds, through `Convert.ToBase64String` (served in process when the array was
    /// made there, ADR-038).
    fn host_byte_array(&mut self, handle: DotNetHandle, clr_type: &str) -> Result<Vec<u8>, Signal> {
        const CONVERT: &str = "System.Convert";
        const MEMBER: &str = "ToBase64String";
        let engine = self.engine.clone();
        if let Err(why) = engine
            .dotnet
            .policy
            .check(Some("netstandard"), CONVERT, Access::Member(MEMBER))
        {
            return unsupported(format!("DotNet {CONVERT}.{MEMBER}: {why}"));
        }
        self.dotnet_ready(CONVERT, MEMBER)?;
        let session = self.dotnet.session.as_mut().expect("opened by dotnet_ready");
        let convert = DotNetType {
            assembly: "netstandard".into(),
            culture: Some("neutral".into()),
            public_key_token: Some("cc7b13ffcd2ddd51".into()),
            name: CONVERT.into(),
            ..Default::default()
        };
        let arg = DotNetArg {
            value: DotNetValue::Handle {
                handle,
                ty: clr_type.into(),
            },
            by_ref: false,
        };
        match session.invoke(DotNetTarget::Static(convert), MEMBER, vec![arg]) {
            Ok(DotNetReply {
                value: DotNetValue::Str(text),
                ..
            }) => match al_host::inproc::from_base64(&text) {
                Some(bytes) => Ok(bytes),
                None => unsupported("the bytes of a .NET byte array came back as text that is not Base64"),
            },
            Ok(_) => unsupported("the bytes of a .NET byte array came back as a value that is not text"),
            Err(DotNetFailure::Refused(why)) => unsupported(format!("DotNet {CONVERT}.{MEMBER}: {why}")),
            Err(DotNetFailure::Exception(e)) => unsupported(format!(
                "DotNet {CONVERT}.{MEMBER} of a byte array raised {}: {}",
                e.clr_type, e.message
            )),
        }
    }
}

/// A Blob with nothing special about it: loaded, no mark, not a file.
fn plain(buf: &BlobBuf) -> Result<(), Signal> {
    buf.observe()?;
    if !buf.loaded || buf.first_write_truncates || buf.file {
        return unsupported(
            "a stream over a Blob that is not loaded, or over a File, given to a DotNet Stream (not pinned)",
        );
    }
    Ok(())
}

pub(super) fn state_of(this: Option<&Rc<DotNetObject>>) -> Result<&RefCell<Vec<Value>>, Signal> {
    match this.map(|o| &o.kind) {
        Some(DotNetKind::Internal { state, .. }) => Ok(state),
        _ => unsupported("DotNet Stream member on something that is not an AL stream"),
    }
}

pub(super) fn blob_of(state: &RefCell<Vec<Value>>) -> Result<BlobRef, Signal> {
    match &state.borrow()[S_BLOB] {
        Value::Blob(b) => Ok(b.clone()),
        _ => unsupported("DotNet Stream without its Blob"),
    }
}

pub(super) fn is_out(state: &RefCell<Vec<Value>>) -> bool {
    matches!(state.borrow()[S_OUT], Value::Boolean(true))
}

fn is_closed(state: &RefCell<Vec<Value>>) -> bool {
    matches!(state.borrow()[S_CLOSED], Value::Boolean(true))
}

/// The state, refused after `Close` or `Dispose` (only `WriteByte` is pinned then) and brought up to date with what AL
/// did to its stream since the last .NET call.
fn open<'a>(this: Option<&'a Rc<DotNetObject>>, member: &str) -> Result<&'a RefCell<Vec<Value>>, Signal> {
    let state = state_of(this)?;
    if !matches!(state.borrow()[S_SET], Value::Null) {
        return unsupported(format!(
            "DotNet Stream.{member} over an AL stream after a Position set (not pinned)"
        ));
    }
    if is_closed(state) {
        return unsupported(format!(
            "DotNet Stream.{member} over an AL stream after Close or Dispose (only WriteByte is pinned there)"
        ));
    }
    sync(state)?;
    Ok(state)
}

/// Settles the .NET position after AL used its own stream. Pinned only where both readings of the two positions agree: an AL
/// write that ends at the .NET position (SgDotNetStreams.al:165, :188), an AL read to the end (:384-386).
fn sync(state: &RefCell<Vec<Value>>) -> Result<(), Signal> {
    let blob = blob_of(state)?;
    let (pos, seen) = {
        let s = state.borrow();
        (int(&s[S_POS]), int(&s[S_SEEN]))
    };
    if is_out(state) {
        let buf = blob.borrow();
        if buf.dotnet_view as usize != int(&state.borrow()[S_ID]) {
            return unsupported(
                "a DotNet Stream over an OutStream that was created again (what the .NET stream writes then is not pinned)",
            );
        }
        buf.observe()?;
        if buf.write_pos != seen {
            if buf.write_pos != pos {
                return unsupported(
                    "a DotNet Stream used after an AL write that ended elsewhere than its position (whether the two positions \
                     are one is not pinned)",
                );
            }
            state.borrow_mut()[S_SEEN] = big(buf.write_pos);
        }
        return Ok(());
    }
    let al = state.borrow()[S_AL].clone();
    let Value::Library(_, h) = &al else {
        return unsupported("DotNet Stream without its InStream");
    };
    let (al_pos, len) = match &*h.borrow() {
        LibraryValue::InStream(st) if st.live_blob().is_some_and(|b| Arc::ptr_eq(b, &blob)) => (st.pos, st.len()),
        _ => {
            return unsupported(
                "a DotNet Stream over an InStream that was created again (what the .NET stream reads then is not pinned)",
            );
        }
    };
    blob.borrow().observe()?;
    if al_pos != seen {
        if al_pos < len {
            return unsupported(
                "a DotNet Stream used after an AL read that stopped before the end (whether the two positions are one is not \
                 pinned)",
            );
        }
        let mut s = state.borrow_mut();
        s[S_POS] = big(len);
        s[S_SEEN] = big(al_pos);
    }
    Ok(())
}

/// Writes `bytes` at the .NET position: over what is there, then past the end (SgDotNetStreams.al:154-172).
fn put(state: &RefCell<Vec<Value>>, bytes: &[u8]) -> Result<(), Signal> {
    let blob = blob_of(state)?;
    let pos = int(&state.borrow()[S_POS]);
    state.borrow_mut()[S_TOUCHED] = Value::Boolean(true);
    {
        let mut buf = blob.borrow_mut();
        let at = pos.min(buf.bytes.len());
        let overlap = bytes.len().min(buf.bytes.len() - at);
        buf.bytes[at..at + overlap].copy_from_slice(&bytes[..overlap]);
        buf.bytes.extend_from_slice(&bytes[overlap..]);
        buf.loaded = true;
        buf.refetch = false;
        buf.persisted = false;
        buf.written = true;
        buf.inserted = false;
    }
    state.borrow_mut()[S_POS] = big(pos + bytes.len());
    Ok(())
}

fn yes(this: Option<&Rc<DotNetObject>>, member: &str) -> Result<Value, Signal> {
    open(this, member)?;
    Ok(Value::Boolean(true))
}

fn can_read(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    yes(this, "CanRead")
}

fn can_write(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    yes(this, "CanWrite")
}

fn can_seek(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    yes(this, "CanSeek")
}

fn get_type(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    open(this, "GetType")?;
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: NAV_BLOB_STREAM_TYPE.into(),
        state: RefCell::new(vec![Value::Text(NAV_BLOB_STREAM.into())]),
    }))))
}

fn full_name(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    Ok(state_of(this)?.borrow()[0].clone())
}

/// The state for `Length` or `Position`, refused once anything read or wrote since AL gave the stream to .NET
/// (SgDotNetStreams.al:122-123 and :391-392 read them only right after the assignment).
pub(super) fn untouched<'a>(
    this: Option<&'a Rc<DotNetObject>>,
    member: &str,
) -> Result<&'a RefCell<Vec<Value>>, Signal> {
    let state = open(this, member)?;
    let len = blob_of(state)?.borrow().bytes.len();
    let s = state.borrow();
    if matches!(s[S_TOUCHED], Value::Boolean(true))
        || int(&s[S_POS]) != int(&s[S_START])
        || int(&s[S_SEEN]) != int(&s[S_START])
        || len != int(&s[S_START_LEN])
    {
        return unsupported(format!(
            "DotNet Stream.{member} over an AL stream after a read or write (pinned only right after the assignment)"
        ));
    }
    drop(s);
    Ok(state)
}

fn length(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    let state = untouched(this, "Length")?;
    Ok(state.borrow()[S_START_LEN].clone())
}

pub(super) fn position(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    let state = untouched(this, "Position")?;
    Ok(state.borrow()[S_POS].clone())
}

fn read_byte(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    let state = open(this, "ReadByte")?;
    if is_out(state) {
        return unsupported("DotNet Stream.ReadByte over an OutStream (not pinned)");
    }
    let blob = blob_of(state)?;
    let pos = int(&state.borrow()[S_POS]);
    let next = blob.borrow().bytes.get(pos).copied();
    state.borrow_mut()[S_TOUCHED] = Value::Boolean(true);
    Ok(match next {
        Some(b) => {
            state.borrow_mut()[S_POS] = big(pos + 1);
            Value::Integer(i32::from(b))
        }
        // SgDotNetStreams.al:386 `read at end`.
        None => Value::Integer(-1),
    })
}

fn number(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(n) => Some(i64::from(*n)),
        Value::BigInteger(n) => Some(*n),
        Value::Byte(b) => Some(i64::from(*b)),
        _ => None,
    }
}

fn write_byte(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = state_of(this)?;
    if !is_out(state) {
        return unsupported("DotNet Stream.WriteByte over an InStream (not pinned)");
    }
    let byte = match args {
        [v] => match number(v).and_then(|n| u8::try_from(n).ok()) {
            Some(b) => b,
            None => {
                return unsupported(format!(
                    "DotNet Stream.WriteByte of {} (only a byte value is pinned)",
                    v.describe()
                ));
            }
        },
        _ => return unsupported("DotNet Stream.WriteByte takes one value"),
    };
    if is_closed(state) {
        // SgDotNetStreams.al:192: no error; what the Blob holds after it is not pinned.
        blob_of(state)?.borrow_mut().render_pending = Some(DOTNET_WRITE_AFTER_CLOSE.into());
        return Ok(Value::Null);
    }
    sync(state)?;
    put(state, &[byte])?;
    Ok(Value::Null)
}

fn write(s: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = open(this, "Write")?;
    if !is_out(state) {
        return unsupported("DotNet Stream.Write over an InStream (not pinned)");
    }
    let [Value::DotNet(array), offset, count] = args else {
        return unsupported("DotNet Stream.Write takes (byte[], offset, count) here");
    };
    let DotNetKind::Host { handle, clr_type } = &array.kind else {
        return unsupported("DotNet Stream.Write of something that is not a .NET byte array");
    };
    if clr_type != "System.Byte[]" {
        return unsupported(format!(
            "DotNet Stream.Write of a {clr_type} (only System.Byte[] is pinned)"
        ));
    }
    let (Some(offset), Some(count)) = (number(offset), number(count)) else {
        return unsupported("DotNet Stream.Write with an offset or count that is not a number");
    };
    let bytes = s.host_byte_array(*handle, clr_type)?;
    let (Ok(from), Ok(n)) = (usize::try_from(offset), usize::try_from(count)) else {
        return unsupported("DotNet Stream.Write with a negative offset or count (the .NET error is not pinned)");
    };
    let Some(slice) = from.checked_add(n).and_then(|end| bytes.get(from..end)) else {
        return unsupported("DotNet Stream.Write of a range outside the array (the .NET error is not pinned)");
    };
    put(state, slice)?;
    Ok(Value::Null)
}

fn flush(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    let state = open(this, "Flush")?;
    if !is_out(state) {
        return unsupported("DotNet Stream.Flush over an InStream (not pinned)");
    }
    // SgDotNetStreams.al:145-149: the Blob holds the bytes before and after Flush.
    Ok(Value::Null)
}

fn close(this: Option<&Rc<DotNetObject>>, member: &str) -> Result<Value, Signal> {
    let state = open(this, member)?;
    if !is_out(state) {
        return unsupported(format!("DotNet Stream.{member} over an InStream (not pinned)"));
    }
    state.borrow_mut()[S_CLOSED] = Value::Boolean(true);
    Ok(Value::Null)
}

fn close_call(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    close(this, "Close")
}

fn dispose_call(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    close(this, "Dispose")
}

pub(super) fn member(name: &str, kind: MemberKind, f: super::registry::MemberFn) -> RegistryMember {
    RegistryMember {
        name: name.into(),
        kind,
        pin: pin(),
        call: f,
        call_ref: None,
    }
}

pub fn types() -> Vec<RegistryType> {
    vec![
        RegistryType {
            clr_name: NAV_BLOB_STREAM.into(),
            members: vec![
                member("GetType", MemberKind::Call, get_type),
                member("CanRead", MemberKind::Get, can_read),
                member("CanWrite", MemberKind::Get, can_write),
                member("CanSeek", MemberKind::Get, can_seek),
                member("Length", MemberKind::Get, length),
                member("Position", MemberKind::Get, super::blob_stream_position::position_get),
                member("Position", MemberKind::Set, super::blob_stream_position::position_set),
                member("ReadByte", MemberKind::Call, read_byte),
                member("WriteByte", MemberKind::Call, write_byte),
                member("Write", MemberKind::Call, write),
                member("Flush", MemberKind::Call, flush),
                member("Close", MemberKind::Call, close_call),
                member("Dispose", MemberKind::Call, dispose_call),
            ],
        },
        RegistryType {
            clr_name: NAV_BLOB_STREAM_TYPE.into(),
            members: vec![member("FullName", MemberKind::Get, full_name)],
        },
    ]
}
