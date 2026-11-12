//! The mark of a placeholder report output (ADR-040 amendments of 2026-11-08 and 2026-11-11).
//!
//! A rendering save of a report writes a small valid file of the asked kind (a PDF, a .docx, a .xlsx or an html page) in
//! place of BC's rendering. The buffer, and a file it is written to, carry this mark (`BlobBuf::render_pending`, the
//! session's pending files). Unlike the other marks of section 4 the bytes are there, and three things are not a look at
//! the content, because they answer as on BC whatever BC rendered:
//!
//! - presence and size: `Blob.HasValue`, `File.Exists` (F2, S2: it exists), and `Blob.Length` and `File.Len` as a size known only
//!   to be greater than 0 (F5; [`Value::PlaceholderSize`], below);
//! - a stream on it: `Blob.CreateInStream`, `File.Open` and `File.CreateInStream` (only a read from the stream looks);
//! - a byte copy: `CopyStream` into another blob or a file, a record copy and a stored Blob (the mark goes along).
//!
//! Every read of the bytes is refused with [`refusal`]: `Read`, `ReadText`, `EOS`, `InStream.Length`, hashes, Base64,
//! `XmlDocument.ReadFrom`, Excel Buffer, Data Compression. A test that reads the content tests the renderer, which this
//! runtime does not have; it is reported as not run, never as a wrong answer.

use std::cmp::Ordering;
use std::sync::Arc;

use crate::native::Signal;
use crate::value::{RtType, Value};

/// The start of every placeholder mark.
const PREFIX: &str = "a placeholder ";

/// The mark of the placeholder `format` (Pdf, Word, Excel, Html) a save of `report` wrote.
pub fn mark(format: &str, report: &str) -> Arc<str> {
    format!("{PREFIX}{format} of report {report}").into()
}

/// Whether a mark is a placeholder output's.
pub fn is_placeholder(mark: &str) -> bool {
    mark.starts_with(PREFIX)
}

/// The placeholder mark of a buffer, if it holds a placeholder output.
pub fn of(buf: &crate::stream::BlobBuf) -> Option<&Arc<str>> {
    buf.render_pending.as_ref().filter(|m| is_placeholder(m))
}

/// The placeholder mark of the buffer an InStream value reads, if it reads one.
pub fn of_in(v: &crate::value::Value) -> Option<Arc<str>> {
    let crate::value::Value::Library(_, h) = v else {
        return None;
    };
    match &*h.borrow() {
        crate::value::LibraryValue::InStream(s) => s.render_pending().filter(|m| is_placeholder(m)),
        _ => None,
    }
}

/// The refusal of a read of a placeholder output's content.
pub fn refusal(mark: &str) -> Signal {
    Signal::unsupported(format!(
        "a read of the content of {mark} (the runtime writes a placeholder, not BC's rendering: only that the output \
         exists and is not empty matches BC; a test that reads the content tests the layout renderer)"
    ))
}

/// The byte size of the placeholder `mark` (`Blob.Length`, `File.Len`): BC's rendering is not empty (reports/layouts-saveas/
/// AMReportOutputTests.al:146 and reports/CiReportRunLeads.al:72 `TempBlob.Length() > 0` is Yes), and nothing else about its size
/// is pinned. The placeholder's own byte count would be a wrong answer: `Length > 1000` is true on BC (ADR-040 amendment
/// 2026-11-12, A4). So the size is a value that answers only a comparison with 0 ([`size_order`]); it may be held in a number
/// variable or a Variant, passed and returned (System Application's Temp Blob.Length() returns Blob.Length), and every other use
/// is refused with [`size_refusal`].
pub fn size(mark: &Arc<str>) -> Value {
    Value::PlaceholderSize(mark.clone())
}

/// The refusal of a use of a placeholder's size other than a comparison with 0.
pub fn size_refusal(mark: &str, what: &str) -> Signal {
    Signal::unsupported(size_refusal_text(mark, what))
}

/// The text of [`size_refusal`].
pub fn size_refusal_text(mark: &str, what: &str) -> String {
    format!(
        "{what} of the byte size of {mark} (BC's rendering is only known not to be empty: only a comparison of the size with 0 \
         answers)"
    )
}

/// A size held in a slot of type `t`: an Integer, BigInteger, Decimal or Variant keeps it; any other type would read it.
pub fn size_convert(v: Value, t: &RtType) -> Result<Value, Signal> {
    match (&v, t) {
        (Value::PlaceholderSize(_), RtType::Integer | RtType::BigInteger | RtType::Decimal | RtType::Variant) => Ok(v),
        (Value::PlaceholderSize(m), _) => Err(size_refusal(m, "a conversion to another type")),
        _ => Ok(v),
    }
}

/// The order of `l` and `r` where either is a placeholder's size: greater than the number 0, and refused against anything else.
pub fn size_order(l: &Value, r: &Value) -> Result<Ordering, Signal> {
    let zero = |v: &Value| match v {
        Value::Integer(n) => *n == 0,
        Value::BigInteger(n) => *n == 0,
        Value::Decimal(d) => d.is_zero(),
        _ => false,
    };
    match (l, r) {
        (Value::PlaceholderSize(_), o) if zero(o) => Ok(Ordering::Greater),
        (o, Value::PlaceholderSize(_)) if zero(o) => Ok(Ordering::Less),
        (Value::PlaceholderSize(m), _) | (_, Value::PlaceholderSize(m)) => {
            Err(size_refusal(m, "a comparison other than with 0"))
        }
        _ => unreachable!("size_order is called with a placeholder size"),
    }
}
