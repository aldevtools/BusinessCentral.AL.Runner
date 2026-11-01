//! The `Position` of the runtime's blob stream over an AL `InStream` (blob_stream.rs) after AL read from it, a `Position` set,
//! and the stream assigned back to an `InStream` (System Application's Data Compression IsGZip and IsZip). Pinned by
//! tests/al-language-onprem-dotnet/lt-y51-blob-stream-position/Y51BlobStreamPosition.al (BC 27.5, 28.5, 29.0):
//! - `Position` follows what AL read (2 after two `InStream.Read` of a byte, BlobStream_PositionSet_AfterALRead);
//! - a `Position` set takes any number, -1 and past the end too, reads back as set, and does not move the AL stream
//!   (`rest_after_set` `cdef`, BlobStream_PositionSet_OutOfRange);
//! - `InS := S` gives an InStream over the Blob from its start, whatever either position is (BlobStream_AssignBack_SameProcedure;
//!   BlobStream_AssignBackToInStream_InCallee, where the caller's InStream, which the callee's by-value parameter shares, reads
//!   all of it afterwards and can be given to a DotNet Stream again): the InStream the stream was made from, rewound.
//! Refused: these on the OutStream side, after a .NET read (`ReadByte`), and any other member after a Position set.

use super::blob_stream::{S_AL, S_SET, S_TOUCHED, blob_of, is_out, state_of};
use super::unsupported;
use crate::Session;
use al_builtins::dotnet_sigs::NAV_BLOB_STREAM;
use al_builtins::native::Signal;
use al_builtins::value::{DotNetKind, DotNetObject, LibraryValue, RtType, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// The state of an InStream-side stream no .NET read touched.
fn in_side(this: Option<&Rc<DotNetObject>>) -> Result<Option<&RefCell<Vec<Value>>>, Signal> {
    let state = state_of(this)?;
    if is_out(state) || matches!(state.borrow()[S_TOUCHED], Value::Boolean(true)) {
        return Ok(None);
    }
    Ok(Some(state))
}

pub(super) fn position_get(s: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let Some(state) = in_side(this)? else {
        return super::blob_stream::position(s, this, args);
    };
    let set = state.borrow()[S_SET].clone();
    if !matches!(set, Value::Null) {
        return Ok(set);
    }
    let al = state.borrow()[S_AL].clone();
    match &al {
        Value::Library(_, h) => match &*h.borrow() {
            LibraryValue::InStream(st) if st.live_blob().is_some() => Ok(Value::BigInteger(st.pos as i64)),
            _ => unsupported("DotNet Stream.Position over an InStream that was created again (not pinned)"),
        },
        _ => unsupported("DotNet Stream without its InStream"),
    }
}

pub(super) fn position_set(_: &mut Session, this: Option<&Rc<DotNetObject>>, args: &[Value]) -> Result<Value, Signal> {
    let Some(state) = in_side(this)? else {
        return unsupported("DotNet Stream.Position set over an OutStream or after a .NET read (not pinned)");
    };
    let to = match args {
        [Value::Integer(n)] => i64::from(*n),
        [Value::BigInteger(n)] => *n,
        _ => return unsupported("DotNet Stream.Position set to something that is not a number"),
    };
    state.borrow_mut()[S_SET] = Value::BigInteger(to);
    Ok(Value::Null)
}

impl Session {
    /// The bytecode's conversion of `v` for a slot of type `ty`. An AL stream into a DotNet variable becomes a runtime-provided
    /// System.IO.Stream (blob_stream.rs); that stream into an InStream, see `blob_stream_into_instream`.
    pub(crate) fn convert_op(&mut self, v: Value, ty: &RtType) -> Result<Value, Signal> {
        match (ty, &v) {
            (RtType::DotNet(i), Value::Library(..)) => self.al_stream_into_dotnet(v, *i),
            _ => match self.blob_stream_into_instream(&v, ty) {
                Some(r) => r,
                None => al_builtins::ops::convert_plain(v, ty),
            },
        }
    }

    /// `InS := S` where `S` holds the runtime's blob stream over an InStream: a new InStream over the Blob from its start (see
    /// the module comment). `None` when `v` is no such stream or `ty` no InStream.
    pub(crate) fn blob_stream_into_instream(&self, v: &Value, ty: &RtType) -> Option<Result<Value, Signal>> {
        let (RtType::Library(al_builtins::library_kind::LibraryKind::InStream), Value::DotNet(o)) = (ty, v) else {
            return None;
        };
        let DotNetKind::Internal { type_name, .. } = &o.kind else {
            return None;
        };
        if type_name != NAV_BLOB_STREAM {
            return None;
        }
        let this = Some(o);
        Some((|| {
            let state = state_of(this)?;
            if is_out(state) {
                return unsupported("a DotNet Stream over an OutStream assigned to an InStream (not pinned)");
            }
            let blob = blob_of(state)?;
            let al = state.borrow()[S_AL].clone();
            let Value::Library(_, h) = &al else {
                return unsupported("DotNet Stream without its InStream");
            };
            match &mut *h.borrow_mut() {
                LibraryValue::InStream(st)
                    if st.live_blob().is_some_and(|b| std::sync::Arc::ptr_eq(b, &blob))
                        && !(st.handed_to_content || st.owner_gone || st.not_initialized) =>
                {
                    st.pos = 0;
                    st.hit_end = false;
                }
                _ => {
                    return unsupported(
                        "a DotNet Stream assigned to an InStream after its InStream was created again or handed on (not pinned)",
                    );
                }
            }
            Ok(al)
        })())
    }
}
