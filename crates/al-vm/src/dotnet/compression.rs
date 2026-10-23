//! `System.IO.Compression.GZipStream` and `CompressionMode`, the DotNet types behind System
//! Application's "Data Compression" codeunit (`GZipCompress`, `GZipDecompress`). The runtime provides
//! them itself (ADR-014 section 1), so a run needs no .NET host.
//!
//! Pinned by the corpus staging branch, tests/al-language/types/PZGzip.al (BC 29.0, staging 2c09ecd):
//! text and all 256 byte values round trip; the packed bytes start with 31 and 139; empty input
//! compresses to no bytes and decompressing an empty stream gives an empty result without an error;
//! text that is not gzip raises "A call to System.IO.Compression.GZipStream.CopyTo failed with this
//! message: The archive entry was compressed using an unsupported compression method."
//! (`DotNetInvoke:InvalidData`); a truncated stream decompresses without an error to a shorter result.
//!
//! Not pinned, so refused: a stream shorter than the 10 byte gzip header, corrupt compressed data, a
//! byte count in `CopyStream`, reading from a compressing stream or writing to a decompressing one.
//! The bytes of the gzip header after the two magic numbers (modification time, operating system) are
//! whatever the encoder writes; the probe observed only the first two.

use super::registry::{MemberKind, Pin, RegistryMember, RegistryType};
use crate::Session;
use al_builtins::dotnet_sigs::{COMPRESSION_MODE, GZIP_STREAM};
use al_builtins::native::Signal;
use al_builtins::stream::{drain_in, new_blob, write_bytes, write_raw_bytes};
use al_builtins::value::{DotNetKind, DotNetObject, LibraryValue, Value};
use flate2::Compression;
use flate2::write::GzEncoder;
use flate2::{Decompress, FlushDecompress, Status};
use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

const PIN: &str = "tests/al-language/types/PZGzip.al:13 (staging 2c09ecd)";

// State of a GZipStream: the stream it wraps, the mode (`CompressionMode` ordinal), the bytes written
// so far (compressing), and whether it was disposed.
const S_STREAM: usize = 0;
const S_MODE: usize = 1;
const S_BUFFER: usize = 2;
const S_DISPOSED: usize = 3;

const DECOMPRESS: i64 = 0;
const COMPRESS: i64 = 1;

fn unsupported<T>(why: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(why.into()))
}

fn pin() -> Pin {
    Pin::Corpus(PIN.into())
}

fn mode_value(name: &str, ordinal: i64) -> Value {
    Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Enum {
        clr_type: COMPRESSION_MODE.into(),
        name: name.into(),
        ordinal,
    })))
}

fn mode_compress(_: &mut Session, _: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    Ok(mode_value("Compress", COMPRESS))
}

fn mode_decompress(_: &mut Session, _: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    Ok(mode_value("Decompress", DECOMPRESS))
}

fn gzip_new(_: &mut Session, _: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let [stream, Value::DotNet(mode)] = args else {
        return unsupported("DotNet GZipStream takes (stream, CompressionMode) here");
    };
    let DotNetKind::Enum { ordinal, clr_type, .. } = &mode.kind else {
        return unsupported("DotNet GZipStream with a mode that is not a CompressionMode");
    };
    if clr_type != COMPRESSION_MODE || (*ordinal != COMPRESS && *ordinal != DECOMPRESS) {
        return unsupported("DotNet GZipStream with a mode that is not a CompressionMode");
    }
    let expected = if *ordinal == COMPRESS { "OutStream" } else { "InStream" };
    match stream {
        Value::Library(kind, _) if kind == expected => {}
        other => {
            return unsupported(format!(
                "DotNet GZipStream({}) over {}: only an {expected} is supported",
                if *ordinal == COMPRESS { "Compress" } else { "Decompress" },
                al_builtins::describe::Describe::describe(other)
            ));
        }
    }
    Ok(Value::DotNet(Rc::new(DotNetObject::new(DotNetKind::Internal {
        type_name: GZIP_STREAM.into(),
        state: RefCell::new(vec![
            stream.clone(),
            Value::BigInteger(*ordinal),
            Value::Blob(new_blob()),
            Value::Boolean(false),
        ]),
    }))))
}

fn state_of(this: Option<&Rc<DotNetObject>>) -> Result<&RefCell<Vec<Value>>, Signal> {
    match this.map(|o| &o.kind) {
        Some(DotNetKind::Internal { state, .. }) => Ok(state),
        _ => unsupported("DotNet GZipStream member on something that is not a GZipStream"),
    }
}

fn mode_of(state: &RefCell<Vec<Value>>) -> i64 {
    match state.borrow()[S_MODE] {
        Value::BigInteger(n) => n,
        _ => -1,
    }
}

/// The bytes of a gzip stream as `GZipStream.CopyTo` gives them.
fn inflate(bytes: &[u8]) -> Result<Vec<u8>, Signal> {
    if bytes.is_empty() {
        return Ok(Vec::new());
    }
    if bytes.len() < 10 {
        return unsupported(
            "GZipStream decompression of fewer than 10 bytes (the header is 10): BC's answer is not pinned",
        );
    }
    if bytes[0] != 0x1f || bytes[1] != 0x8b {
        return al_builtins::native::raise(
            "A call to System.IO.Compression.GZipStream.CopyTo failed with this message: The archive entry was compressed using an unsupported compression method.",
            "DotNetInvoke:InvalidData",
        );
    }
    let mut out = Vec::new();
    let mut rest = bytes;
    // One member after the other (a stream may hold several). The decoder works on the raw deflate data so
    // that a truncated stream gives everything that could be decoded before it ended (PZGzip.al).
    while rest.len() >= 10 && rest[0] == 0x1f && rest[1] == 0x8b {
        let Some(data) = member_data(rest) else { return Ok(out) };
        let mut d = Decompress::new(false);
        loop {
            out.reserve(8192);
            let (in_before, out_before) = (d.total_in(), d.total_out());
            let status = d
                .decompress_vec(&data[d.total_in() as usize..], &mut out, FlushDecompress::None)
                .map_err(|e| {
                    Signal::Unsupported(format!(
                        "GZipStream decompression of corrupt data ({e}): the error BC raises is not pinned"
                    ))
                })?;
            if status == Status::StreamEnd {
                break;
            }
            if d.total_in() == in_before && d.total_out() == out_before {
                // Input used up before the end of the member: the stream was cut.
                return Ok(out);
            }
        }
        // After the deflate data: CRC32 and size (8 bytes), then maybe another member.
        let used = data.len() - (data.len() - d.total_in() as usize);
        let tail = &data[used..];
        rest = if tail.len() >= 8 { &tail[8..] } else { &[] };
    }
    Ok(out)
}

/// The deflate data of the gzip member at the start of `bytes` (after the header and its optional fields).
fn member_data(bytes: &[u8]) -> Option<&[u8]> {
    let flags = bytes[3];
    let mut at = 10;
    if flags & 0x04 != 0 {
        let n = u16::from_le_bytes([*bytes.get(at)?, *bytes.get(at + 1)?]) as usize;
        at += 2 + n;
    }
    for bit in [0x08, 0x10] {
        if flags & bit != 0 {
            at += bytes.get(at..)?.iter().position(|b| *b == 0)? + 1;
        }
    }
    if flags & 0x02 != 0 {
        at += 2;
    }
    bytes.get(at..)
}

fn gzip_copy_to(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let state = state_of(this)?;
    if mode_of(state) != DECOMPRESS {
        return unsupported("GZipStream.CopyTo on a stream that compresses (not pinned)");
    }
    let [Value::Library(kind, _)] = args else {
        return unsupported("DotNet GZipStream.CopyTo takes one OutStream here");
    };
    if kind != "OutStream" {
        return unsupported("DotNet GZipStream.CopyTo into something that is not an OutStream");
    }
    let mut source = state.borrow()[S_STREAM].clone();
    let bytes = drain_in(&mut source)?;
    let out = inflate(&bytes)?;
    write_raw_bytes(&mut args[0].clone(), &out)?;
    Ok(Value::Null)
}

fn gzip_dispose(_: &mut Session, this: Option<&Rc<DotNetObject>>, _: &[Value]) -> Result<Value, Signal> {
    let state = state_of(this)?;
    if matches!(state.borrow()[S_DISPOSED], Value::Boolean(true)) {
        return Ok(Value::Null);
    }
    state.borrow_mut()[S_DISPOSED] = Value::Boolean(true);
    if mode_of(state) != COMPRESS {
        return Ok(Value::Null);
    }
    let Value::Blob(buffer) = state.borrow()[S_BUFFER].clone() else {
        return unsupported("GZipStream without its buffer");
    };
    // An opaque copy (`write_into`) packs to an opaque package in the wrapped stream.
    if let Some(mark) = buffer.borrow().render_pending.clone() {
        let target = state.borrow()[S_STREAM].clone();
        let Value::Library(_, h) = &target else {
            return unsupported("GZipStream over a stream that is not an OutStream");
        };
        let LibraryValue::OutStream(blob, _) = &*h.borrow() else {
            return unsupported("GZipStream over a stream that is not an OutStream");
        };
        al_builtins::stream::mark_carried(blob, mark);
        return Ok(Value::Null);
    }
    let raw = buffer.borrow().bytes.clone();
    // Empty input gives no bytes at all, not even a header (PZGzip.al: GZip_EmptyInput).
    if raw.is_empty() {
        return Ok(Value::Null);
    }
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(&raw)
        .and_then(|()| enc.try_finish())
        .map_err(|e| Signal::Unsupported(format!("GZipStream compression failed: {e}")))?;
    let packed = enc
        .finish()
        .map_err(|e| Signal::Unsupported(format!("GZipStream compression failed: {e}")))?;
    let mut target = state.borrow()[S_STREAM].clone();
    write_raw_bytes(&mut target, &packed)?;
    Ok(Value::Null)
}

/// `Close`: as `Dispose` for a compressing stream. Measured on Windows BC 29.0 (probe cu 61289
/// SC_GZipStream_Close_ConfigPackageShape, the shape of Config. Pckg. Compression Mgt.ProcessGZip,
/// ConfigPckgCompressionMgt.Codeunit.al:80: no error, the bytes start 1F 8B 08 00, GZipDecompress gives the source back); no
/// corpus test pins it yet, so it marks `gzipstream-close-is-dispose`. Close of a decompressing stream is not measured.
fn gzip_close(s: &mut Session, this: Option<&Rc<DotNetObject>>, a: &[Value]) -> Result<Value, Signal> {
    if mode_of(state_of(this)?) != COMPRESS {
        return unsupported("GZipStream.Close of a decompressing stream (not pinned)");
    }
    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::GZipStreamCloseIsDispose);
    gzip_dispose(s, this, a)
}

/// `CopyStream(GZipStream, InStream)`: the bytes wait in the stream until `Dispose` packs them.
pub(super) fn write_into(out: &Value, src: &Value, count: Option<i64>, carry: bool) -> Option<Result<(), Signal>> {
    let Value::DotNet(o) = out else { return None };
    let DotNetKind::Internal { type_name, state } = &o.kind else {
        return None;
    };
    if type_name != GZIP_STREAM {
        return None;
    }
    Some((|| {
        if count.is_some() {
            return unsupported("CopyStream with a byte count into a GZipStream (not pinned)");
        }
        if mode_of(state) != COMPRESS {
            return unsupported("CopyStream into a GZipStream that decompresses (not pinned)");
        }
        if !matches!(src, Value::Library(al_builtins::library_kind::LibraryKind::InStream, _)) {
            return unsupported("CopyStream into a GZipStream from something that is not an InStream");
        }
        let Value::Blob(buffer) = state.borrow()[S_BUFFER].clone() else {
            return unsupported("GZipStream without its buffer");
        };
        // Content carried unread in the company build (owner decision 2026-10-23 "Carry bytes unread"): the stream holds an
        // opaque copy, and Dispose writes an opaque package.
        if carry && let Some(mark) = al_builtins::stream::carried_mark_of_in(src) {
            al_builtins::stream::mark_carried(&buffer, mark);
            return Ok(());
        }
        let bytes = drain_in(&mut src.clone())?;
        write_bytes(&buffer, &bytes);
        Ok(())
    })())
}

fn getter(name: &str, f: super::registry::MemberFn) -> RegistryMember {
    RegistryMember {
        name: name.into(),
        kind: MemberKind::Get,
        pin: pin(),
        call: f,
        call_ref: None,
    }
}

fn call(name: &str, f: super::registry::MemberFn) -> RegistryMember {
    RegistryMember {
        kind: MemberKind::Call,
        ..getter(name, f)
    }
}

pub fn types() -> Vec<RegistryType> {
    vec![
        RegistryType {
            clr_name: COMPRESSION_MODE.into(),
            members: vec![getter("Compress", mode_compress), getter("Decompress", mode_decompress)],
        },
        RegistryType {
            clr_name: GZIP_STREAM.into(),
            members: vec![
                RegistryMember {
                    kind: MemberKind::Construct,
                    ..call("GZipStream", gzip_new)
                },
                call("CopyTo", gzip_copy_to),
                call("Dispose", gzip_dispose),
                call("Close", gzip_close),
            ],
        },
    ]
}
