//! The mark of a placeholder report output (ADR-040 amendments of 2026-11-08, 2026-11-11 and 2026-11-12).
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
//!
//! One exception, for the PDF only (owner decision 2026-11-12): a read that stays inside the bytes the corpus pins at the start of
//! BC's PDF answers ([`pdf_read`], [`PDF_PREFIX`]).
//!
//! the mark of a placeholder report output (ADR-040 amendment 2026-11-11): presence and byte copies
//! answer, every read of the content is refused; the size is `Value::PlaceholderSize`, which answers
//! only a comparison with 0; a record field may hold it, with the number type of its slot, and
//! `size_read` refuses a platform read of such a field (amendment 2026-11-12b; the store refuses a
//! filter or sort on it in al-data `MemStore::unknown_read`); a comparison `size_order` refused that
//! a caller turned into an answer (a page's row compare) is refused at the next builtin call or the
//! end of the procedure (`UNANSWERED`). A file of such bytes that the reading session holds no mark
//! for gets `outside_mark` when it is opened (al-vm `outside_output_mark`). Entry points: `mark`, `outside_mark`, `is_outside`, `is_placeholder`, `refusal`, `size`,
//! `size_order`, `size_read`, `size_guard`, `refuse_unanswered`.

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

/// The mark of a file that holds the placeholder `kind` (a format name, or `rendered_text::WRITTEN_KIND`) and that the session
/// reading it holds no mark for: an earlier session or another process wrote it (al-vm `Session::outside_output_mark`). It
/// names no report and no render, so no read of the content answers but the PDF's pinned start ([`pdf_read`]).
pub fn outside_mark(kind: &str) -> Arc<str> {
    format!("{PREFIX}{kind}{OUTSIDE}").into()
}

/// The end of an [`outside_mark`].
const OUTSIDE: &str = " written outside this session";

/// Whether `mark` is an [`outside_mark`].
pub fn is_outside(mark: &str) -> bool {
    mark.starts_with(PREFIX) && mark.ends_with(OUTSIDE)
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
    let pinned = if is_pdf(mark) {
        ", and that a PDF starts with the bytes '%PDF-'"
    } else {
        ""
    };
    let mark = crate::rendered_text::shown(mark);
    Signal::unsupported(format!(
        "a read of the content of {mark} (the runtime writes a placeholder, not BC's rendering: only that the output \
         exists and is not empty{pinned} matches BC; a test that reads the content tests the layout renderer)"
    ))
}

/// The bytes the corpus pins at the start of BC's PDF: the five-byte signature `%PDF-`. ReadText(Text, 5) of a Windows stream
/// gives '%PDF-' (reports/layouts-saveas/AMReportOutputTests.al:42-45, reports/layouts-saveas/AMRdlcOutput.al:177-180,
/// al-language-onprem/reports/layouts-saveas/AMRdlcSaveAs.al:199-202), four Byte reads, each after EOS answered No, give
/// 37,80,68,70 (streams/CjFix.al:128-147 through streams/CjWordSections.al:125 SectionBreaks_WordMergeDataItem_Pdf `Pdf head`).
/// The bytes after it (the version: '%PDF-1.3' in events/platform-triggers/ReportEventsMoreCallForms.al:93, the placeholder's
/// '1.4') are not BC's and stay refused.
pub const PDF_PREFIX: &[u8] = b"%PDF-";

/// The Html placeholder's bytes: a minimal valid html page (reports/layouts-saveas/AMReportOutputTests.al:205 `kind` markup: it
/// starts with `<`). A whole text read of a buffer that holds exactly these bytes gives the rendered content's mark
/// (`rendered_text::read_whole`).
pub const HTML: &[u8] =
    b"<!DOCTYPE html>\n<html><head><meta charset=\"utf-8\"><title></title></head><body></body></html>\n";

/// Whether a mark is the PDF placeholder's.
pub fn is_pdf(mark: &str) -> bool {
    mark.strip_prefix(PREFIX).is_some_and(|m| m.starts_with("Pdf "))
}

/// Whether `mark` is the mark of a placeholder Html.
pub fn is_html(mark: &str) -> bool {
    mark.strip_prefix(PREFIX).is_some_and(|m| m.starts_with("Html "))
}

/// A read of a stream that [`pdf_read`] can answer inside [`PDF_PREFIX`].
#[derive(Debug, Clone, Copy)]
pub enum PdfRead {
    /// `EOS`: No while the next byte is inside the prefix.
    Eos,
    /// `Read` into a Byte: one byte, whatever the encoding.
    Byte,
    /// `Read` into a Char on a single-byte encoding: one byte (the prefix is ASCII, so every single-byte encoding reads it as
    /// itself).
    Char,
    /// `ReadText(Text, Length)` on a single-byte encoding: Length bytes, one character each.
    Text(usize),
}

/// What [`pdf_read`] answers.
#[derive(Debug)]
pub enum PdfAnswer {
    Eos(bool),
    Byte(u8),
    Text(Vec<u8>),
}

/// The answer of `read` on an InStream value that reads the PDF placeholder, or its refusal; `None` for any other stream. It
/// answers only when the buffer starts with [`PDF_PREFIX`] (a copy taken part way, or bytes AL wrote over it, do not) and the
/// read ends inside it; a UTF-8 or UTF-16 stream may decode ahead of what it returns and is refused. The position moves as the
/// read would move it.
pub fn pdf_read(v: &Value, read: PdfRead) -> Option<Result<PdfAnswer, Signal>> {
    let Value::Library(_, h) = v else {
        return None;
    };
    let mut g = h.borrow_mut();
    let crate::value::LibraryValue::InStream(s) = &mut *g else {
        return None;
    };
    // Most streams hold no mark: they are answered before the buffer handle is cloned (every Read of a byte passes here).
    if !of(&s.live_blob()?.borrow()).is_some_and(|m| is_pdf(m)) {
        return None;
    }
    let blob = s.live_blob()?.clone();
    let buf = blob.borrow();
    let mark = of(&buf).filter(|m| is_pdf(m))?.clone();
    let single_byte = matches!(s.enc, crate::stream::Encoding::Windows | crate::stream::Encoding::MsDos);
    let (start, end) = match read {
        PdfRead::Eos | PdfRead::Byte | PdfRead::Char => (s.pos, s.pos + 1),
        PdfRead::Text(n) => (s.pos, s.pos.saturating_add(n)),
    };
    let decodes_ahead = matches!(read, PdfRead::Text(_) | PdfRead::Char) && !single_byte;
    if !buf.bytes.starts_with(PDF_PREFIX) || end > PDF_PREFIX.len() || decodes_ahead {
        return Some(Err(refusal(&mark)));
    }
    Some(Ok(match read {
        PdfRead::Eos => PdfAnswer::Eos(false),
        PdfRead::Byte | PdfRead::Char => {
            s.pos = end;
            PdfAnswer::Byte(PDF_PREFIX[start])
        }
        PdfRead::Text(_) => {
            s.pos = end;
            PdfAnswer::Text(PDF_PREFIX[start..end].to_vec())
        }
    }))
}

/// The byte size of the placeholder `mark` (`Blob.Length`, `File.Len`): BC's rendering is not empty (reports/layouts-saveas/
/// AMReportOutputTests.al:146 and reports/CiReportRunLeads.al:72 `TempBlob.Length() > 0` is Yes), and nothing else about its size
/// is pinned. The placeholder's own byte count would be a wrong answer: `Length > 1000` is true on BC (ADR-040 amendment
/// 2026-11-12, A4). So the size is a value that answers only a comparison with 0 ([`size_order`]); it may be held in a number
/// variable or a Variant, passed and returned (System Application's Temp Blob.Length() returns Blob.Length), and every other use
/// is refused with [`size_refusal`].
pub fn size(mark: &Arc<str>) -> Value {
    SIZE_MADE.store(true, std::sync::atomic::Ordering::Relaxed);
    Value::PlaceholderSize(mark.clone(), crate::value::SqlType::Integer)
}

/// A BigInteger whose number nothing measures and that is known only to be greater than 0, marked `why` (no placeholder
/// mark): it follows the rules of a placeholder's size ([`size_order`], [`size_convert`], [`size_guard`]), and its refusals
/// name `why` ([`size_refusal_text`]). Made by al-vm `restamped_value` (a SystemRowVersion the runtime does not give as BC).
pub fn positive(why: String) -> Value {
    SIZE_MADE.store(true, std::sync::atomic::Ordering::Relaxed);
    Value::PlaceholderSize(why.into(), crate::value::SqlType::BigInteger)
}

/// What a size-like value (`Value::PlaceholderSize`) of mark `mark` is, for a refusal text: the byte size of a placeholder
/// output, or the value [`positive`] marked.
pub fn size_shown(mark: &str) -> String {
    if is_placeholder(mark) {
        format!("the byte size of {}", crate::rendered_text::shown(mark))
    } else {
        mark.to_string()
    }
}

/// Set by [`size`] and never cleared: whether a placeholder's size may exist in this process. A size can be held in a
/// SingleInstance global or a Variant beyond the session that made it, so the flag is not per session. It only spares the VM
/// the scan of [`size_guard`] on every builtin call until the first size is made; it decides no answer.
static SIZE_MADE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Whether [`size_guard`] can find a size (see [`SIZE_MADE`]).
#[inline]
pub fn sizes_made() -> bool {
    SIZE_MADE.load(std::sync::atomic::Ordering::Relaxed)
}

std::thread_local! {
    /// The mark of a size [`size_order`] could not order (a comparison with anything but 0), until [`size_guard`] or
    /// [`refuse_unanswered`] takes it. A caller that turns that refusal into an answer made the answer up: a page counts a
    /// value it cannot compare with xRec or the stored row as changed (al-vm `pages/draft.rs`, `same_values`). So the next
    /// builtin call, or the end of the running procedure, refuses in its place.
    static UNANSWERED: std::cell::RefCell<Option<Arc<str>>> = const { std::cell::RefCell::new(None) };
}

/// The refusal of the comparison [`UNANSWERED`] holds, taken.
fn take_unanswered() -> Option<Signal> {
    let m = UNANSWERED.with(|u| u.borrow_mut().take())?;
    Some(size_refusal(&m, "a comparison other than with 0"))
}

/// `outcome` of a procedure that just ended, or the refusal of a comparison of a size made while it ran and turned into an
/// answer by its caller ([`UNANSWERED`]). A refusal the procedure already ended with stays.
pub fn refuse_unanswered(outcome: Result<(), Signal>) -> Result<(), Signal> {
    if !sizes_made() {
        return outcome;
    }
    match take_unanswered() {
        Some(refusal) if !matches!(outcome, Err(Signal::Unsupported(_))) => Err(refusal),
        _ => outcome,
    }
}

/// Refuses a builtin or a .NET member (`name`, a builtin id name or `DotNet Type.Member`) given a placeholder's size as its
/// receiver or an argument. The callee would compare the size with its own code and not through [`size_order`] (a record
/// filter counts a value it cannot compare as not passing, a List or Dictionary as not equal) or read its number, so the
/// answer would be made up. The one allowed use: `Variant.Is*` on a Variant that holds it, which asks only the type of the slot
/// that held it (`Value::is_kind`); Library Assert's `AreEqual`/`AreNotEqual` ask it.
pub fn size_guard(name: &str, recv: Option<&Value>, args: &[Value]) -> Result<(), Signal> {
    if let Some(refusal) = take_unanswered() {
        return Err(refusal);
    }
    let held = |v: Option<&Value>| match v {
        Some(Value::PlaceholderSize(m, _)) => Some(m.clone()),
        _ => None,
    };
    if let Some(m) = args.iter().find_map(|v| held(Some(v))) {
        return Err(size_refusal(&m, &format!("{name} given the value")));
    }
    match held(recv) {
        Some(m) if !name.starts_with("Variant.Is") => Err(size_refusal(&m, &format!("{name} on the value"))),
        _ => Ok(()),
    }
}

/// The refusal of a use of a placeholder's size other than a comparison with 0.
pub fn size_refusal(mark: &str, what: &str) -> Signal {
    Signal::unsupported(size_refusal_text(mark, what))
}

/// The text of [`size_refusal`].
pub fn size_refusal_text(mark: &str, what: &str) -> String {
    if !is_placeholder(mark) {
        return format!("{what} of {mark} (only known to be greater than 0: only a comparison with 0 answers)");
    }
    let mark = crate::rendered_text::shown(mark);
    format!(
        "{what} of the byte size of {mark} (BC's rendering is only known not to be empty: only a comparison of the size with 0 \
         answers)"
    )
}

/// Refuses a platform read of a record field's value `v` (`what`) when the field holds a placeholder's size: a FlowField or
/// `CalcSums` over it, `Validate` of it, a page control that shows it (ADR-040 amendment 2026-11-12b, point 4). The store
/// refuses a filter or a sort on such a field itself (al-data `MemStore::unknown_read`), and `format` refuses `Format`.
pub fn size_read(v: &Value, what: &str) -> Result<(), Signal> {
    match v {
        Value::PlaceholderSize(m, _) => Err(size_refusal(m, what)),
        _ => Ok(()),
    }
}

/// A size held in a slot of type `t`: an Integer, BigInteger or Decimal keeps it with the slot's type, a Variant as it is
/// (`Value::is_kind` answers that type); any other type would read it.
pub fn size_convert(v: Value, t: &RtType) -> Result<Value, Signal> {
    use crate::value::SqlType;
    let Value::PlaceholderSize(m, _) = &v else {
        return Ok(v);
    };
    let ty = match t {
        RtType::Variant => return Ok(v),
        // A value `positive` marked is a BigInteger whose size nothing measures: an Integer or a Decimal may raise on BC.
        RtType::BigInteger if !is_placeholder(m) => SqlType::BigInteger,
        _ if !is_placeholder(m) => return Err(size_refusal(m, "a conversion to another type")),
        RtType::Integer => SqlType::Integer,
        RtType::BigInteger => SqlType::BigInteger,
        RtType::Decimal => SqlType::Decimal,
        _ => return Err(size_refusal(m, "a conversion to another type")),
    };
    Ok(Value::PlaceholderSize(m.clone(), ty))
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
        (Value::PlaceholderSize(..), o) if zero(o) => Ok(Ordering::Greater),
        (o, Value::PlaceholderSize(..)) if zero(o) => Ok(Ordering::Less),
        (Value::PlaceholderSize(m, _), _) | (_, Value::PlaceholderSize(m, _)) => {
            UNANSWERED.with(|u| *u.borrow_mut() = Some(m.clone()));
            Err(size_refusal(m, "a comparison other than with 0"))
        }
        _ => unreachable!("size_order is called with a placeholder size"),
    }
}

/// `InStream.EOS`, `Read` into a Byte or a Char, and `ReadText(Text, Length)` (`length`) of the PDF placeholder, as AL values:
/// the result and the variable `a[0]` takes. `None` for any other read or stream; the caller's path refuses a placeholder's.
pub fn pdf_in(r: Option<&Value>, a: &mut [Value], what: PdfRead) -> Option<Result<Value, Signal>> {
    let read = match (what, a.first()) {
        (PdfRead::Byte, Some(Value::Byte(_))) => PdfRead::Byte,
        (PdfRead::Byte, Some(Value::Char(_))) => PdfRead::Char,
        (PdfRead::Byte, _) => return None,
        (other, _) => other,
    };
    Some(pdf_read(r?, read)?.map(|answer| match answer {
        PdfAnswer::Eos(at_end) => Value::Boolean(at_end),
        PdfAnswer::Byte(b) => {
            a[0] = match read {
                PdfRead::Char => Value::Char(u16::from(b)),
                _ => Value::Byte(b),
            };
            Value::Integer(1)
        }
        PdfAnswer::Text(bytes) => {
            let n = bytes.len() as i32;
            a[0] = Value::Text(String::from_utf8_lossy(&bytes).into_owned().into());
            Value::Integer(n)
        }
    }))
}
