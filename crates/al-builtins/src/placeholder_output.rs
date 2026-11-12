//! The mark of a placeholder report output (ADR-040 amendments of 2026-11-08 and 2026-11-11).
//!
//! A rendering save of a report writes a small valid file of the asked kind (a PDF, a .docx, a .xlsx or an html page) in
//! place of BC's rendering. The buffer, and a file it is written to, carry this mark (`BlobBuf::render_pending`, the
//! session's pending files). Unlike the other marks of section 4 the bytes are there, and three things are not a look at
//! the content, because they answer as on BC whatever BC rendered:
//!
//! - presence and size: `Blob.HasValue`, `Blob.Length`, `File.Exists`, `File.Len` (F2, F5, S2: it exists and is not empty);
//! - a stream on it: `Blob.CreateInStream`, `File.Open` and `File.CreateInStream` (only a read from the stream looks);
//! - a byte copy: `CopyStream` into another blob or a file, a record copy and a stored Blob (the mark goes along).
//!
//! Every read of the bytes is refused with [`refusal`]: `Read`, `ReadText`, `EOS`, `InStream.Length`, hashes, Base64,
//! `XmlDocument.ReadFrom`, Excel Buffer, Data Compression. A test that reads the content tests the renderer, which this
//! runtime does not have; it is reported as not run, never as a wrong answer.

use std::sync::Arc;

use crate::native::Signal;

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
