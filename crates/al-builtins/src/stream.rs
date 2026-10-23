//! In-memory streams over Blob fields, plus the array helpers that share their
//! corpus folder (`streams/`).
//!
//! A Blob field holds a shared buffer (`BlobRef`); `CreateOutStream` returns a
//! stream that appends to that same buffer, so a write is visible through the
//! field at once. Whether a table keeps the record's own buffer (temporary
//! tables) or a copy (database tables) is decided by the VM at the store
//! boundary (streams/TestBlobUncommittedWriteIsolation.al). An `InStream`
//! is a live view of that buffer (streams/TestStreamsRuntimeContracts.al:InStream_CreatedBeforeAWrite_SeesTheWrite).
//!
//! `CreateOutStream` does not empty the buffer: a stream writes from the start and overwrites what
//! was there (:BlobField_CreateOutStream_DoesNotEmptyExistingData). Text is written in
//! Windows-1252 unless an encoding says otherwise (:WriteText_DefaultEncodingIsWindows1252_NotUtf8).

use crate::describe::Describe;
use crate::native::{Ctx, NativeFn, Signal, raise};
use crate::signature::Signature;
use crate::value::{LibraryValue, Value};
use std::cell::RefCell;
use std::rc::Rc;

/// What a Blob field holds.
#[derive(Debug, Default)]
pub struct BlobBuf {
    pub bytes: Vec<u8>,
    /// Where the most recent `OutStream` on this buffer writes next. `CreateOutStream` sets it to 0.
    pub write_pos: usize,
    /// False after the record was read from a table: the content then has to be
    /// fetched with `CalcFields` before a stream reads it
    /// (streams/TestBlob.al Blob_Get_WithoutCalcFields_InStreamReadsEmpty).
    pub loaded: bool,
    /// The content is what the database holds or was saved there, or the buffer never came from a row: a stream write does not
    /// change it. `CalcFields` fetches a Blob that is not (a `Get` row written in memory is read back from the database:
    /// record/lt-blob-calcfields-length/BlobCalcFieldsLength.al:BlobCalcFields_UnsavedContent_Result) and leaves one that is
    /// (streams/TestBlobCalcFieldsUncommittedWrite.al:Blob_CalcFields_AfterUncommittedWriteWithoutModify_KeepsWrite).
    pub fetched: bool,
    /// The buffer is the copy a record assignment made: its bytes show, but `CalcFields` fetches the Blob from the stored
    /// row (types/ProbeTypeFormsAndFilters.al:FormatRecord_AssignmentCopiesBlobBytes) unless a write came first.
    pub refetch: bool,
    /// A `Modify` of a temporary record wrote this content to the row; a write through a stream clears it.
    pub persisted: bool,
    /// The buffer is the unloaded copy a Get of a temporary record handed out: its `Length` is the content's, not the
    /// stored size of a database row (misc/lt-probes-needed-1/PnABlobLength.al:Length_InMemoryAndFindVariants `temporary-regot`).
    pub temp_row: bool,
    /// A stream wrote to the buffer since it was fetched or saved: `Modify` counts the Blob as changed whatever it holds now
    /// (misc/lt-probes-needed-1/PnASystemFields.al:Modify_StampsOnlyWhenStoredFieldDiffers `same-blob-content-keeps-stamp`).
    pub written: bool,
    /// ADR-040: a report on the allow-list was saved to this buffer in a rendered format and nothing was written:
    /// the name of the report. Any look at the buffer is refused (`observe`); a copy of the buffer carries the mark
    /// (`copy_blob`, a record assignment, `CalcFields`); creating the stream again clears it.
    pub render_pending: Option<std::sync::Arc<str>>,
    /// The content is what an `Insert` stored, and no `Modify` or stream write came after. `Media.ImportStream` from a stream on
    /// such a Blob of the same record is refused (misc/lt-probes-needed-5/PnE06Media.al:66-84 against streams/
    /// lt-stream-read-write-media/P102Media.al:24-41, see `refuse_media_import_from_own_blob` in al-vm).
    pub inserted: bool,
    /// The buffer is the unloaded copy a read of a database row handed out (`copy_blob_db_row`): the first stream write
    /// after `CreateOutStream` replaces the stored content (record/lt-blob-calcfields-length/BlobCalcFieldsLength.al:222-350).
    pub db_row: bool,
    /// `CreateOutStream` was called on an unloaded database-row Blob and nothing was written yet: the next write empties
    /// the buffer first (BlobCalcFieldsLength.al:222 'new' over 'a long stored value' is 'new'); until then the stored value
    /// stays (:350 RF_GetWithoutCalcFields_OutStreamNotWritten_Modify).
    pub first_write_truncates: bool,
}

thread_local! {
    /// The report of a pending buffer that AL looked at through a path that cannot return the error itself (a read
    /// of a stream by a parser, a hash, a .NET call, `Format`). The caller of the builtin turns it into the refusal
    /// (`take_pending_observation`), so such a read never gives a result.
    static OBSERVED: std::cell::RefCell<Option<std::sync::Arc<str>>> = const { std::cell::RefCell::new(None) };
    static OBSERVED_ANY: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The mark `Media.ExportStream` puts on a buffer before BC 29: BC re-encodes an imported PNG into bytes the corpus does not pin
/// (types/W7Media.al:141-151), and other content is not probed on those majors.
pub const MEDIA_REENCODED: &str = "Media.ExportStream";

/// The mark a .docx `Media.ImportFile` puts on its media from BC 29, whose Linux tier stores a re-zipped package the corpus
/// does not pin (`Gate::DocxMediaRezippedFrom29`).
pub const MEDIA_DOCX_REZIPPED: &str = "Media.ImportFile of a .docx";

/// The mark on the Tenant Media Content of a .jpg file holding a PNG (`media::jpg_holding_png`): BC re-encodes it.
pub const MEDIA_JPG_HOLDING_PNG: &str = "Media.ImportFile of a .jpg holding a PNG";

/// The mark a stream write puts on a Blob of a record read without `CalcFields` while the stored content is longer than what
/// was written: what the Blob holds then is not pinned (AL Runner issue 5681). A write that reaches the stored content's end
/// clears it (`write_bytes`): both readings agree then (record/lt-blob-calcfields-length/BlobCalcFieldsLength.al:
/// BlobCalcFields_UnsavedContent_Result, BlobCalcFields_AfterModifyAndClear_Result).
pub const UNLOADED_WRITE: &str = "Blob.CreateOutStream without CalcFields";

/// The mark a stream write puts on a Blob when `CalcFields` (or another load) came between `CreateOutStream` on the unloaded
/// database-row Blob and the first write: whether the write still replaces the content is not pinned.
pub const UNLOADED_THEN_LOADED: &str = "Blob.CreateOutStream without CalcFields, loaded before the write";

/// The mark on the Metadata Blob of a query's Application Object Metadata row: the bytes hold only the query's id, for
/// Microsoft's `QueryMetadataReader.FromStream` (al-vm `dotnet/query_metadata.rs`). What BC stores there is not pinned, so
/// every other look at the Blob is refused; `HasValue` is Yes (ODataUtility.GetTenantWebServiceMetadata reads it before
/// FromStream, and the reader is not null: QxDotNetDemoShapes.al:174-181).
pub const QUERY_METADATA: &str = "Application Object Metadata.Metadata of a query";

/// Whether a mark is one of the two unpinned Blob writes above (they refuse any look, not only a report column).
pub fn is_unloaded_write_mark(what: &str) -> bool {
    what == UNLOADED_WRITE || what == UNLOADED_THEN_LOADED
}

/// The refusal for a look at the output of a report that was run for its effects only (ADR-040).
pub fn render_pending_error(report: &str) -> Signal {
    if report == UNLOADED_WRITE {
        return Signal::Unsupported(
            "a Blob written through a stream after the record was read without CalcFields, shorter than its stored content \
             (whether the stored bytes stay after the new ones is not pinned)"
                .into(),
        );
    }
    if report == UNLOADED_THEN_LOADED {
        return Signal::Unsupported(
            "a Blob written through a stream created before CalcFields loaded it, after a read without CalcFields \
             (whether the write replaces the content is not pinned)"
                .into(),
        );
    }
    if report == MEDIA_REENCODED {
        return Signal::Unsupported(
            "the bytes Media.ExportStream gives on a major that re-encodes an imported PNG (not pinned)".into(),
        );
    }
    if report == MEDIA_JPG_HOLDING_PNG {
        return Signal::Unsupported(
            "the stored content of a .jpg file holding a PNG that Media.ImportFile read (BC re-encodes it; ReMediaJpgHoldingPng.al:108 \
             does not pin the bytes)"
                .into(),
        );
    }
    if report == QUERY_METADATA {
        return Signal::Unsupported(
            "the bytes of the Metadata Blob of Application Object Metadata (not pinned; only QueryMetadataReader.FromStream \
             reads it)"
                .into(),
        );
    }
    if report == MEDIA_DOCX_REZIPPED {
        return Signal::Unsupported(
            "the Tenant Media row and the exported bytes of a .docx Media.ImportFile on this major (a re-zipped package on the Linux \
             tier, not pinned: MediaFiles.al:505-509)"
                .into(),
        );
    }
    // ADR-044 uses the same mark for the picture a `System.Drawing.Bitmap` was asked to save: the name is the .NET type.
    if report.starts_with("System.Drawing.") {
        return Signal::Unsupported(format!("picture drawn by {report} is not produced"));
    }
    Signal::Unsupported(format!("rendered output of report {report} is not produced"))
}

/// Records that AL looked at a pending buffer.
pub fn note_pending_observation(report: &std::sync::Arc<str>) {
    OBSERVED.with(|o| *o.borrow_mut() = Some(report.clone()));
    OBSERVED_ANY.with(|c| c.set(true));
}

/// The refusal for a look at a pending buffer that happened since the last call, if one did. One branch on a
/// thread-local flag in the usual case.
pub fn take_pending_observation() -> Option<Signal> {
    if !OBSERVED_ANY.with(|c| c.get()) {
        return None;
    }
    OBSERVED_ANY.with(|c| c.set(false));
    OBSERVED
        .with(|o| o.borrow_mut().take())
        .map(|r| render_pending_error(&r))
}

impl BlobBuf {
    /// Refuses a look at the buffer when it is pending (ADR-040).
    pub fn observe(&self) -> Result<(), Signal> {
        match &self.render_pending {
            Some(report) => Err(render_pending_error(report)),
            None => Ok(()),
        }
    }
}

/// A buffer that follows the borrowing rules of a `RefCell` (any number of readers or one writer,
/// else a panic) and can be shared between threads: the platform listings hold blob columns, and
/// the test threads read them.
#[derive(Debug, Default)]
pub struct BlobCell(std::sync::RwLock<BlobBuf>);

impl BlobCell {
    pub fn new(buf: BlobBuf) -> BlobCell {
        BlobCell(std::sync::RwLock::new(buf))
    }

    /// Shared access; panics while the buffer is borrowed mutably, as `RefCell::borrow` does.
    pub fn borrow(&self) -> std::sync::RwLockReadGuard<'_, BlobBuf> {
        match self.0.try_read() {
            Ok(g) => g,
            Err(std::sync::TryLockError::Poisoned(p)) => p.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => panic!("already mutably borrowed"),
        }
    }

    /// Exclusive access; panics while the buffer is borrowed, as `RefCell::borrow_mut` does.
    pub fn borrow_mut(&self) -> std::sync::RwLockWriteGuard<'_, BlobBuf> {
        match self.0.try_write() {
            Ok(g) => g,
            Err(std::sync::TryLockError::Poisoned(p)) => p.into_inner(),
            Err(std::sync::TryLockError::WouldBlock) => panic!("already borrowed"),
        }
    }
}

pub type BlobRef = std::sync::Arc<BlobCell>;

pub fn new_blob() -> BlobRef {
    std::sync::Arc::new(BlobCell::new(BlobBuf {
        bytes: Vec::new(),
        write_pos: 0,
        loaded: true,
        fetched: true,
        refetch: false,
        persisted: false,
        temp_row: false,
        written: false,
        render_pending: None,
        inserted: false,
        db_row: false,
        first_write_truncates: false,
    }))
}

/// A copy with its own buffer (what a database table stores and returns).
/// The unloaded copy a Get of a temporary record hands out for a persisted Blob.
pub fn copy_blob_temp_row(b: &BlobRef) -> BlobRef {
    let c = copy_blob(b, false);
    c.borrow_mut().temp_row = true;
    c
}

pub fn copy_blob(b: &BlobRef, loaded: bool) -> BlobRef {
    let b = b.borrow();
    std::sync::Arc::new(BlobCell::new(BlobBuf {
        bytes: b.bytes.clone(),
        write_pos: 0,
        loaded,
        fetched: loaded,
        refetch: false,
        persisted: false,
        temp_row: false,
        written: false,
        render_pending: b.render_pending.clone(),
        inserted: b.inserted,
        db_row: false,
        first_write_truncates: false,
    }))
}

/// The unloaded copy a read of a database row hands out (`BlobBuf::db_row`).
pub fn copy_blob_db_row(b: &BlobRef) -> BlobRef {
    let c = copy_blob(b, false);
    c.borrow_mut().db_row = true;
    c
}

/// Writes `bytes` at the buffer's write position (over what is there, then past the end) and
/// moves the position on.
pub fn write_bytes(b: &BlobRef, bytes: &[u8]) {
    let mut buf = b.borrow_mut();
    if buf.first_write_truncates {
        buf.first_write_truncates = false;
        if buf.loaded {
            buf.render_pending = Some(UNLOADED_THEN_LOADED.into());
        } else {
            // BlobCalcFieldsLength.al:222, :250, :270: the record held none of the stored content, the write is all of it.
            buf.bytes.clear();
            buf.write_pos = 0;
            buf.render_pending = None;
        }
    }
    let at = buf.write_pos.min(buf.bytes.len());
    let overlap = bytes.len().min(buf.bytes.len() - at);
    buf.bytes[at..at + overlap].copy_from_slice(&bytes[..overlap]);
    buf.bytes.extend_from_slice(&bytes[overlap..]);
    buf.write_pos = at + bytes.len();
    buf.loaded = true;
    buf.refetch = false;
    buf.persisted = false;
    buf.written = true;
    buf.inserted = false;
    if buf.write_pos == buf.bytes.len() && buf.render_pending.as_deref() == Some(UNLOADED_WRITE) {
        buf.render_pending = None;
    }
}

/// How text becomes bytes in a stream. MSDos is code page 437 and the default (streams/TestStreamsRuntimeContracts.al:
/// WriteText_DefaultEncodingIsWindows1252_NotUtf8: 'é' is 82, WriteText_ExplicitEncodings_Bytes); Windows is cp1252;
/// UTF16 is little endian without a byte order mark.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Encoding {
    Utf8,
    Windows,
    #[default]
    MsDos,
    Utf16,
}

/// Unicode of the code page 437 bytes 0x80..=0xFF.
const CP437_HIGH: [char; 128] = [
    'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å', 'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û',
    'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ', 'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡',
    '«', '»', '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐', '└', '┴', '┬', '├', '─',
    '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧', '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█',
    '▄', '▌', '▐', '▀', 'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩', '≡', '±', '≥',
    '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{a0}',
];

/// What the Windows tier reads for the code page 437 bytes whose character code page 1252 does not have (corpus
/// streams/StTEncodings.al:CrossEncodingUtf8ToOthers and CrossEncodingWindowsToOthers, streams/W7StreamRead.al:
/// ReadTextWithLength_Utf8Sequences, types/PEOptionEnumBigText.al): and al-language-onprem/handlers/TestRequestPageSaveAsXmlContracts.al:FileText (EF BB BF): and streams/StTEncodings.al:RoundTripSameEncoding (B1 is the euro sign): XmlPort byte order mark FF FE (xmlport/lt-xmlport-query-datatransfer, misc/lt-probes-needed-3/PnC19XmlPort.al): only these fifteen bytes are pinned.
const CP437_AS_READ: [(u8, char); 18] = [
    (0xFE, '\u{FE}'),
    (0xB1, '\u{20AC}'),
    (0xBB, '\u{2039}'),
    (0xBF, '\u{8F}'),
    (0xEF, '\u{D9}'),
    (0xA9, '\u{14}'),
    (0xB6, '\u{2020}'),
    (0xBC, '\u{152}'),
    (0xC3, '\u{201C}'),
    (0xDF, '\u{C8}'),
    (0xE2, '\u{CB}'),
    (0xE4, '\u{CD}'),
    (0xE9, '\u{D2}'),
    (0xFC, '\u{FD}'),
    // streams/lt-bigtext-write/BigTextWriteEdges.al:Write_NonAscii_Utf8Stream (D0 reads as 168).
    (0xD0, '\u{A8}'),
    // report/lt-y-report-props/YReportPropsTests.al:96, the UTF-8 bytes of `年` (E5 B9 B4) read as `\u00CE\u2030\u201E`.
    (0xE5, '\u{CE}'),
    (0xB9, '\u{2030}'),
    (0xB4, '\u{201E}'),
];

/// Unicode of cp1252 bytes 0x80..=0x9F (the undefined ones map to themselves, as .NET does).
const CP1252_HIGH: [u16; 32] = [
    0x20AC, 0x0081, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0x008D,
    0x017D, 0x008F, 0x0090, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A,
    0x0153, 0x009D, 0x017E, 0x0178,
];

impl Encoding {
    pub fn encode(self, s: &str) -> Vec<u8> {
        match self {
            Encoding::Utf8 => s.as_bytes().to_vec(),
            Encoding::Utf16 => s.encode_utf16().flat_map(u16::to_le_bytes).collect(),
            Encoding::MsDos => s
                .chars()
                .map(|c| match c {
                    c if (c as u32) < 0x80 => c as u8,
                    // The euro sign is written as 0xB1 (streams/TestStreamsRuntimeContracts.al:
                    // WriteText_ExplicitEncodings_Bytes, streams/TestProbeStreamContracts.al `61 84 94 B1 00`).
                    '€' => 0xB1,
                    c => CP437_HIGH.iter().position(|u| *u == c).map_or(b'?', |i| 0x80 + i as u8),
                })
                .collect(),
            Encoding::Windows => s
                .chars()
                .map(|c| {
                    let cp = c as u32;
                    if cp < 0x80 || (0xA0..=0xFF).contains(&cp) {
                        cp as u8
                    } else {
                        CP1252_HIGH
                            .iter()
                            .position(|u| u32::from(*u) == cp)
                            .map_or(b'?', |i| 0x80 + i as u8)
                    }
                })
                .collect(),
        }
    }

    pub fn decode(self, b: &[u8]) -> String {
        match self {
            Encoding::Utf8 => String::from_utf8_lossy(b).into_owned(),
            Encoding::Utf16 => {
                let units: Vec<u16> = b.chunks_exact(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect();
                String::from_utf16_lossy(&units)
            }
            Encoding::MsDos => b
                .iter()
                .map(|&x| match x {
                    0..=0x7F => char::from(x),
                    x => CP437_AS_READ
                        .iter()
                        .find(|(b, _)| *b == x)
                        .map_or(CP437_HIGH[usize::from(x - 0x80)], |(_, c)| *c),
                })
                .collect(),
            Encoding::Windows => b
                .iter()
                .map(|&x| match x {
                    0x80..=0x9F => char::from_u32(u32::from(CP1252_HIGH[usize::from(x - 0x80)])).unwrap_or('?'),
                    x => char::from(x),
                })
                .collect(),
        }
    }

    /// Whether `decode` of `b` gives what the Windows tier gives: every code page 437 byte whose character is also in
    /// code page 1252 reads as that character, and the bytes of `CP437_AS_READ` as pinned; the rest is not pinned.
    pub fn decode_pinned(self, b: &[u8]) -> bool {
        self != Encoding::MsDos
            || b.iter().all(|&x| {
                x < 0x80 || CP437_AS_READ.iter().any(|(p, _)| *p == x) || {
                    let c = CP437_HIGH[usize::from(x - 0x80)] as u32;
                    c <= 0xFF || u16::try_from(c).is_ok_and(|u| CP1252_HIGH.contains(&u))
                }
            })
    }

    /// Whether `encode` of `t` is pinned: ASCII, the characters code page 1252 has for a Windows stream, and for an MS-DOS
    /// stream the characters of code page 437 and the euro sign.
    pub fn encode_pinned(self, t: &str) -> bool {
        t.is_ascii()
            || match self {
                Encoding::Windows => self.decode(&self.encode(t)) == *t,
                Encoding::MsDos => t.chars().all(|c| c.is_ascii() || c == '€' || CP437_HIGH.contains(&c)),
                _ => false,
            }
    }

    /// Bytes per code unit of the line terminator search.
    fn unit(self) -> usize {
        if self == Encoding::Utf16 { 2 } else { 1 }
    }

    /// From the `Encoding` argument of `CreateOutStream` / `CreateInStream` (an option value).
    pub(crate) fn from_arg(a: &[Value]) -> Result<Encoding, Signal> {
        match a.get(1) {
            None => Ok(Encoding::default()),
            // Ordinals: MSDos 0, UTF8 1, UTF16 2, Windows 3 (:TextEncoding_Ordinals).
            Some(Value::Option(0, _)) => Ok(Encoding::MsDos),
            Some(Value::Option(1, _)) => Ok(Encoding::Utf8),
            Some(Value::Option(2, _)) => Ok(Encoding::Utf16),
            Some(Value::Option(3, _)) => Ok(Encoding::Windows),
            Some(other) => unsupported(format!("text encoding {}", other.describe())),
        }
    }
}

/// Where an `InStream` reads from.
#[derive(Debug)]
enum Source {
    /// Bytes taken once (a resource, a download).
    Snapshot(Vec<u8>),
    /// A Blob buffer: later writes show (the content of a buffer that was not loaded is empty).
    Live(BlobRef),
}

/// Read position over a source.
#[derive(Debug)]
pub struct InState {
    source: Source,
    pub pos: usize,
    pub enc: Encoding,
    /// A read already found the stream at its end: the next typed read there raises (streams/StTStreams.al:ReadPastEnd against
    /// streams/lt-stream-read-write-media/P102ReadTyped.al `pastend`: the first one answers 0).
    pub hit_end: bool,
    /// `HttpContent.WriteFrom` took the stream: only `EOS` is pinned afterwards (see [`hand_to_content`]).
    pub handed_to_content: bool,
    /// The local that owned the stream's bytes went out of scope: only `Length`, `EOS` and `ReadText` are pinned afterwards
    /// (see [`owner_out_of_scope`]).
    pub owner_gone: bool,
}

impl InState {
    pub fn snapshot(data: Vec<u8>, enc: Encoding) -> InState {
        InState {
            source: Source::Snapshot(data),
            pos: 0,
            enc,
            hit_end: false,
            handed_to_content: false,
            owner_gone: false,
        }
    }

    pub fn live(blob: BlobRef, enc: Encoding) -> InState {
        InState {
            source: Source::Live(blob),
            pos: 0,
            enc,
            hit_end: false,
            handed_to_content: false,
            owner_gone: false,
        }
    }

    /// Marks the buffer a live stream reads as holding output that was never produced (ADR-040, ADR-044) and clears
    /// it of bytes. Answers false for a stream that does not read a live buffer.
    pub fn mark_render_pending(&self, what: std::sync::Arc<str>) -> bool {
        match &self.source {
            Source::Live(b) => {
                let mut buf = b.borrow_mut();
                buf.bytes.clear();
                buf.loaded = true;
                buf.render_pending = Some(what);
                true
            }
            Source::Snapshot(_) => false,
        }
    }

    /// The Blob buffer the stream reads, when it reads one.
    pub fn live_blob(&self) -> Option<&BlobRef> {
        match &self.source {
            Source::Live(b) => Some(b),
            Source::Snapshot(_) => None,
        }
    }

    /// The report whose unproduced output the stream looks at (ADR-040), if it does.
    pub fn render_pending(&self) -> Option<std::sync::Arc<str>> {
        match &self.source {
            Source::Live(b) => b.borrow().render_pending.clone(),
            Source::Snapshot(_) => None,
        }
    }

    /// Runs `f` on the bytes the stream sees now.
    pub fn with_data<T>(&self, f: impl FnOnce(&[u8]) -> T) -> T {
        match &self.source {
            Source::Snapshot(d) => f(d),
            Source::Live(b) => {
                let buf = b.borrow();
                if let Some(report) = &buf.render_pending {
                    // The stream sees nothing; the builtin that asked is refused when it returns.
                    note_pending_observation(report);
                    return f(&[]);
                }
                f(if buf.loaded { &buf.bytes } else { &[] })
            }
        }
    }

    pub fn len(&self) -> usize {
        self.with_data(<[u8]>::len)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The bytes from the read position on.
    pub fn rest(&self) -> Vec<u8> {
        let pos = self.pos;
        self.with_data(|d| d[pos.min(d.len())..].to_vec())
    }

    /// Takes the bytes from the read position on and moves to the end.
    pub fn drain(&mut self) -> Vec<u8> {
        let rest = self.rest();
        self.pos = self.len();
        rest
    }
}

type NResult = Result<Value, Signal>;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

fn blob_of(r: Option<&mut Value>) -> Result<BlobRef, Signal> {
    match r {
        Some(Value::Blob(b)) => Ok(b.clone()),
        other => unsupported(format!("Blob method on {}", other.describe())),
    }
}

pub(crate) fn set_stream(slot: &Value, content: LibraryValue) -> Result<(), Signal> {
    match slot {
        Value::Library(_, h) => {
            *h.borrow_mut() = content;
            Ok(())
        }
        other => unsupported(format!("a stream variable was expected, got {}", other.describe())),
    }
}

fn blob_create_out(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let enc = Encoding::from_arg(a)?;
    let b = blob_of(r)?;
    {
        // The content stays (:BlobField_CreateOutStream_DoesNotEmptyExistingData).
        let mut buf = b.borrow_mut();
        if !buf.loaded && buf.db_row {
            // A record read from the database without CalcFields: the stream replaces the stored content at its first
            // write, and leaves it while nothing is written (BlobCalcFieldsLength.al:222, :350). The Blob stays unloaded
            // until then (:350 'after create' is the stored size).
            buf.first_write_truncates = true;
            buf.write_pos = 0;
            buf.refetch = false;
            drop(buf);
            set_stream(&a[0], LibraryValue::OutStream(b, enc))?;
            return Ok(Value::Null);
        }
        // That case writes over content the record holds. Another unloaded Blob (a temporary record's, a copy of an unloaded
        // one) is not pinned: it is marked until a write reaches the stored content's end (`UNLOADED_WRITE`). A mark from an
        // earlier stream stays, as the stored bytes still decide what the Blob holds.
        let unloaded_content = !buf.loaded && !buf.bytes.is_empty();
        if unloaded_content || buf.render_pending.as_deref().is_some_and(is_unloaded_write_mark) {
            buf.render_pending = Some(UNLOADED_WRITE.into());
        } else {
            buf.render_pending = None;
        }
        buf.write_pos = 0;
        buf.loaded = true;
        buf.refetch = false;
        buf.persisted = false;
    }
    set_stream(&a[0], LibraryValue::OutStream(b, enc))?;
    Ok(Value::Null)
}

fn blob_create_in(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let enc = Encoding::from_arg(a)?;
    let b = blob_of(r)?;
    // A query's metadata is only handed to QueryMetadataReader.FromStream; a read of the stream refuses (`with_data`).
    if b.borrow().render_pending.as_deref() != Some(QUERY_METADATA) {
        b.borrow().observe()?;
    }
    set_stream(&a[0], LibraryValue::InStream(InState::live(b, enc)))?;
    Ok(Value::Null)
}

/// `CreateInStream(var InStream [, Encoding])` over no bytes (a `FileUpload` no client filled).
pub(crate) fn create_empty_in_stream(a: &mut [Value]) -> Result<(), Signal> {
    let enc = Encoding::from_arg(a)?;
    let b = BlobCell::new(BlobBuf {
        bytes: Vec::new(),
        write_pos: 0,
        loaded: true,
        fetched: true,
        refetch: false,
        persisted: false,
        temp_row: false,
        written: false,
        render_pending: None,
        inserted: false,
        db_row: false,
        first_write_truncates: false,
    });
    set_stream(
        &a[0],
        LibraryValue::InStream(InState::live(std::sync::Arc::new(b), enc)),
    )
}

fn blob_has_value(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    // streams/TestInOutStream.al InOutStream_WriteText_BlobHasValue: true after Get without CalcFields.
    let b = blob_of(r)?;
    let buf = b.borrow();
    if buf.render_pending.as_deref() == Some(QUERY_METADATA) {
        return Ok(Value::Boolean(true));
    }
    buf.observe()?;
    Ok(Value::Boolean(!buf.bytes.is_empty()))
}

/// `Blob.Import(Name)` (al-language-onprem/lt-o-system-onprem/OBlobAndSession.al:27-58): the blob holds the bytes of the
/// file, read through the host file system. A missing file raises `FileNotFound` and the blob keeps what it held (:74-103,
/// `import missing|code`, `blob length after failed import` 1); the text names the server's path and only its start and end
/// are pinned, so it is raised as an unpinned text. The name it returns is pinned only by its end (`return ends with file
/// name`): a call that uses it is refused, as is a blank name (pinned for Export only).
fn blob_import(c: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let name = match a.first().and_then(crate::native::text_of) {
        Some(t) => t.to_string(),
        None => return unsupported(format!("Blob.Import of {}", a.first().describe())),
    };
    if name.trim().is_empty() {
        return unsupported("Blob.Import with a blank file name (the error is pinned for Blob.Export only)");
    }
    if c.ret_used() {
        return unsupported("the file name Blob.Import returns (only its end is pinned)");
    }
    let Some(bytes) = c.fs_read(&name)? else {
        return Err(Signal::Error(Box::new(crate::native::unpinned_text_error(
            "FileNotFound",
            "the text of Blob.Import's FileNotFound error, which names the server's path (only `File ` and ` was not found.` are pinned)",
        ))));
    };
    let b = blob_of(r)?;
    let mut buf = b.borrow_mut();
    buf.bytes = bytes;
    buf.write_pos = 0;
    buf.loaded = true;
    buf.refetch = false;
    buf.persisted = false;
    buf.written = true;
    buf.inserted = false;
    buf.render_pending = None;
    Ok(Value::Text(name.into()))
}

fn blob_length(c: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    let b = blob_of(r)?;
    let buf = b.borrow();
    buf.observe()?;
    if !buf.loaded {
        // Before CalcFields the length is the stored size. An empty blob is 0 (streams/TestProbeStreamContracts.al:
        // BlobLength_EmptyUnloaded); a temporary row's is its content's (misc/lt-probes-needed-1/PnABlobLength.al:
        // Length_InMemoryAndFindVariants). The stored size is the content plus 6 on every major for 4 bytes (streams/StTStreams.al:640,
        // LengthNeedsCalcFields), 5 bytes (streams/TestStreamsRuntimeContracts.al:264, Blob_Length_WithoutCalcFields_ReportsTheStoredSize),
        // 8 bytes (record/lt-blob-autocalc/BlobAutoCalc.al:144, Persistent_InsertedRecord_Blob) and 12 bytes (:276, :320;
        // record/lt-temporary-blob/TemporaryBlobAutoCalc.al:144), and for 1 and 2 bytes from BC 29 (misc/lt-probes-needed-1/
        // PnABlobLength.al:49-54, Length_ByStoredSize; the test is BC 29 only). 100 bytes and more are a compressed size (:55-60:
        // 10, 31, 118 for 100, 10000, 100000 `x`). The other sizes are not pinned and are refused. Probe wanted (docs/probes-needed.md).
        let n = buf.bytes.len();
        if n == 0 || buf.temp_row {
            return Ok(Value::Integer(n as i32));
        }
        // 2 and 3 bytes on every major: record/lt-blob-calcfields-length/BlobCalcFieldsLength.al:250 ('ab' 8), :222 ('new' 9).
        // The stored size is compressed (40 repeated digits are 19 at :470), so for longer content only the texts the corpus
        // stored are given: 'stored' 12 (:350) and 'a long stored value' 25 (:222).
        let pinned = matches!(n, 2 | 3 | 4 | 5 | 8 | 12)
            || (n == 1 && c.gate(al_version::Gate::ProbedValuesDifferFrom29))
            || matches!(&buf.bytes[..], b"stored" | b"a long stored value");
        if !pinned {
            return unsupported(format!(
                "Blob.Length before CalcFields of a stored blob of {n} bytes (the stored size is not pinned)"
            ));
        }
        return Ok(Value::Integer(n as i32 + 6));
    }
    Ok(Value::Integer(buf.bytes.len() as i32))
}

fn in_state<'a>(r: Option<&'a mut Value>) -> Result<std::cell::RefMut<'a, LibraryValue>, Signal> {
    match r {
        Some(Value::Library(_, h)) => Ok(h.borrow_mut()),
        other => unsupported(format!("InStream method on {}", other.describe())),
    }
}

/// Refuses a stream that looks at the output of a report that was never produced (ADR-040), without reading it.
pub fn refuse_pending_in(v: &mut Value) -> Result<(), Signal> {
    with_in(Some(v), |_| Ok(()))
}

/// Reads the rest of an `InStream` value and moves its position to the end.
/// `HttpContent.WriteFrom(InStream)`: the bytes the stream has left, and the stream stays where it was, not at its end
/// (network/lt-db-http-leads/DbHttpContentLeads.al:310). What a read, `Length`, `Position` or `ResetPosition` of it gives
/// afterwards is not pinned: they are refused.
pub fn hand_to_content(v: &mut Value) -> Result<Vec<u8>, Signal> {
    with_in(Some(v), |s| {
        let pos = s.pos;
        let bytes = s.with_data(|d| d.get(pos..).map(<[u8]>::to_vec).unwrap_or_default());
        s.handed_to_content = true;
        Ok(bytes)
    })
}

pub fn drain_in(v: &mut Value) -> Result<Vec<u8>, Signal> {
    with_in(Some(v), |s| Ok(s.drain()))
}

fn with_in<T>(r: Option<&mut Value>, f: impl FnOnce(&mut InState) -> Result<T, Signal>) -> Result<T, Signal> {
    with_in_of(r, false, false, f)
}

/// `InStream.OwnerOutOfScope` (invented): the routine that created the stream on its own local (a Temp Blob, the Blob field
/// of a temporary record) hands it back and ends. BC reads it as empty: Length 0, EOS Yes, ReadText gives '' and EOS stays
/// Yes (streams/lt-dc-stream-leads/DcInStreamScope.al:107-126, :128-166, al-runner issue 5688). Any other use of it is not
/// pinned and refused.
pub(crate) fn owner_out_of_scope(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    with_in(r, |s| {
        s.source = Source::Snapshot(Vec::new());
        s.pos = 0;
        s.hit_end = false;
        s.owner_gone = true;
        Ok(Value::Null)
    })
}

/// [`with_in`]; `handed_ok` lets a stream HttpContent.WriteFrom took through (only `EOS` reads it), `gone_ok` one whose
/// owner went out of scope (only `Length`, `EOS` and `ReadText` read it).
fn with_in_of<T>(
    r: Option<&mut Value>,
    handed_ok: bool,
    gone_ok: bool,
    f: impl FnOnce(&mut InState) -> Result<T, Signal>,
) -> Result<T, Signal> {
    let mut g = in_state(r)?;
    match &mut *g {
        LibraryValue::InStream(s) => {
            if let Some(report) = s.render_pending() {
                return Err(render_pending_error(&report));
            }
            if s.handed_to_content && !handed_ok {
                return unsupported("use of an InStream after HttpContent.WriteFrom took it (only EOS is pinned)");
            }
            if s.owner_gone && !gone_ok {
                return unsupported(
                    "use of an InStream whose owning local went out of scope (only Length, EOS and ReadText are pinned)",
                );
            }
            f(s)
        }
        _ => unsupported("use of an InStream that was never created"),
    }
}

/// `BigText.Read(InStream)`: the rest of the stream as text, in the stream's encoding, and the stream
/// is drained (corpus types/PEOptionEnumBigText.al:336-512, probed on BC 27.5). A UTF-8 stream drops a
/// byte order mark (EF BB BF), a UTF-16 stream is little endian and drops FF FE, a Windows stream, and
/// a stream created without an encoding (MS-DOS, code page 437) keep a mark as characters.
/// A stream read part way, invalid UTF-8 and an odd UTF-16 length are not pinned.
pub(crate) fn read_all_text(stream: &mut Value) -> Result<String, Signal> {
    with_in(Some(stream), |s| {
        let (enc, pos, len) = (s.enc, s.pos, s.len());
        if pos != 0 && pos < len {
            return unsupported("BigText.Read of a stream that was read part way (not pinned)");
        }
        let bytes = s.drain();
        let body: &[u8] = match enc {
            Encoding::Utf8 if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) => &bytes[3..],
            Encoding::Utf16 if bytes.starts_with(&[0xFF, 0xFE]) => &bytes[2..],
            // The text of a stream created without an encoding ends at the first NUL byte: the UTF-16 bytes `E4 00 AC 20 16 04`
            // read as one character (streams/lt-bigtext-write/BigTextWriteEdges.al:Write_NonAscii_ExplicitEncodings, utf16.readlen).
            Encoding::MsDos => bytes.split(|b| *b == 0).next().unwrap_or(&[]),
            _ => &bytes[..],
        };
        match enc {
            // Bytes that are no UTF-8 raise the stream data error, as ReadText does (corpus pages/lt-pages-xmlport-upload-members/
            // P112XpXml.al:P112_Export_TriggerOrder, af379: the UTF-16 bytes of an export read through a UTF-8 stream).
            Encoding::Utf8 if std::str::from_utf8(body).is_err() => {
                Err(Signal::Error(Box::new(crate::native::AlError {
                    message: "Invalid data encountered in stream, unable to read text.".into(),
                    code: "InvalidStreamData".into(),
                    call_stack: String::new(),
                    ..Default::default()
                })))
            }
            Encoding::Utf16 if body.len() % 2 != 0 => {
                unsupported("BigText.Read of a UTF-16 stream with an odd number of bytes (not pinned)")
            }
            _ if !enc.decode_pinned(body) => unsupported("BigText.Read of MS-DOS bytes whose character is not pinned"),
            _ => Ok(enc.decode(body)),
        }
    })
}

fn in_eos(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    // A stream HttpContent.WriteFrom took is not at its end (network/lt-db-http-leads/DbHttpContentLeads.al:310): its position
    // did not move.
    with_in_of(r, true, true, |s| Ok(Value::Boolean(s.pos >= s.len())))
}

fn in_reset(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    with_in(r, |s| {
        s.pos = 0;
        Ok(Value::Boolean(true))
    })
}

fn in_length(_: &mut dyn Ctx, r: Option<&mut Value>, _: &mut [Value]) -> NResult {
    with_in_of(r, false, true, |s| Ok(Value::BigInteger(s.len() as i64)))
}

/// `Position` is 1-based: 1 at the start, the length plus one after the last byte (streams/TestProbeStreamContracts.al:
/// ReadText_LengthPositionAndTerminators). `Position := P` outside 1 to the length raises.
fn group_thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn in_position(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    if let Some(v) = a.first() {
        let p = match v {
            Value::Integer(n) => i64::from(*n),
            Value::BigInteger(n) => *n,
            other => return unsupported(format!("InStream.Position := {}", other.describe())),
        };
        return with_in(r, |s| {
            let len = s.len() as i64;
            if p < 1 || p > len {
                return crate::native::raise(
                    format!(
                        "The value of Position parameter 1 is outside of the permitted range. The current value is: {}. The permitted range is: from 1 to {len}.",
                        group_thousands(p)
                    ),
                    "OutsidePermittedRange",
                );
            }
            s.pos = (p - 1) as usize;
            Ok(Value::Null)
        });
    }
    with_in(r, |s| Ok(Value::BigInteger(s.pos as i64 + 1)))
}

/// Reads up to the next line feed or the end of the stream. The line feed is consumed and every
/// carriage return in the line is dropped, so CR LF is one terminator, a lone CR is not one, and a
/// final CR adds no line (streams/TestStreamsRuntimeContracts.al:InStream_ReadText_SplitsAtLineFeed_AndDropsALoneCarriageReturn).
fn in_read_text(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    // `ReadText(Text, Length)` reads at most Length characters, and stops after a line feed
    // (streams/TestProbeStreamContracts.al:ReadText_LengthPositionAndTerminators).
    let max_chars = match a.get(1) {
        None => usize::MAX,
        Some(Value::Integer(n)) if *n > 0 => *n as usize,
        // A length of 0 or less raises and leaves the text and the position alone (streams/StTStreams.al:
        // ReadTextLengthZeroAndNegative, `An invalid length 0 was specified in stream function...` for 0 and -1).
        Some(Value::Integer(n)) => {
            return crate::native::raise(
                format!(
                    "An invalid length {n} was specified in stream function. Length must be in range 1 to 2147483647."
                ),
                "InvalidStreamLength",
            );
        }
        Some(_) => {
            return unsupported("InStream.ReadText with a length that is not an Integer (not pinned)");
        }
    };
    if a.len() > 2 {
        return unsupported("InStream.ReadText with more arguments (not pinned)");
    }
    let mut enc_used = Encoding::Windows;
    // `ReadText(Text)` of a stream whose owner went out of scope gives '' (DcInStreamScope.al:107-166); with a length it is
    // not pinned.
    let gone_ok = a.len() == 1;
    let text = with_in_of(r, false, gone_ok, |s| {
        // A UTF-8 mark is taken as one only by a stream created with `TextEncoding::UTF8` (handlers/
        // TestReportSaveAsDatasetContracts.al:DatasetLines); a stream with the default encoding reads it as the three
        // characters 239, 187, 191 (al-language-onprem/handlers/TestRequestPageSaveAsXmlContracts.al:FileText). A UTF-16 mark
        // is no mark to ReadText: the default encoding reads `FF FE` as two characters (misc/lt-probes-needed-3/
        // PnC19XmlPort.al:Export_BothFormatEvaluates, ` þ`).
        if s.pos == 0 {
            let utf8 = s.enc == Encoding::Utf8;
            let s_enc = s.enc;
            let bom = s.with_data(|d| {
                // A stream created as UTF-8 does not take a UTF-16 mark: it reads the bytes as UTF-8 and fails on them (xmlport/
                // lt-xmlport-query-datatransfer/XpTExportBasic.al:Export_ThreeRows).
                if utf8 && d.starts_with(&[0xEF, 0xBB, 0xBF]) {
                    Some((Encoding::Utf8, 3))
                } else if s_enc == Encoding::Utf16 && d.starts_with(&[0xFF, 0xFE]) {
                    // A stream created as UTF-16 skips the mark (xmlport/TestXmlPortExportImportContracts.al:ExportedText).
                    Some((Encoding::Utf16, 2))
                } else {
                    None
                }
            });
            if let Some((e, n)) = bom {
                s.enc = e;
                s.pos = n;
            }
        }
        let (enc, pos) = (s.enc, s.pos);
        if pos >= s.len() {
            s.hit_end = true;
        }
        let unit = enc.unit();
        let text = s.with_data(|data| {
            let rest = &data[pos.min(data.len())..];
            let mut end = 0;
            // A NUL ends the text like a line feed does and is consumed with it: the UTF-16 bytes of an export read one
            // character per call (xmlport/lt-xmlport-query-datatransfer/XpTExportBasic.al:Export_StaticWithRecordNoRows,
            // misc/lt-probes-needed-3/PnC19XmlPort.al:Export_BothFormatEvaluates, streams/StTEncodings.al:CrossEncodingMsDosAndUtf16).
            let ends = |u: &[u8]| is_unit(u, b'\n') || u.iter().all(|b| *b == 0);
            while end + unit <= rest.len() && end / unit < max_chars && !ends(&rest[end..end + unit]) {
                end += unit;
            }
            // Stopping on the length leaves the line feed unread.
            let at_lf = end + unit <= rest.len() && ends(&rest[end..end + unit]);
            let line: Vec<u8> = rest[..end]
                .chunks(unit)
                .filter(|u| !is_unit(u, b'\r'))
                .flatten()
                .copied()
                .collect();
            let consumed = if at_lf || end + unit > rest.len() {
                (end + unit).min(rest.len())
            } else {
                end
            };
            // Bytes that are no UTF-8 cannot be read from a stream created as UTF-8 (xmlport/lt-xmlport-query-datatransfer/
            // XpTExportBasic.al:Export_ThreeRows: the UTF-16 bytes of an export).
            let invalid_utf8 = enc == Encoding::Utf8 && std::str::from_utf8(&line).is_err();
            (
                (!invalid_utf8 && enc.decode_pinned(&line)).then(|| enc.decode(&line)),
                consumed,
                invalid_utf8,
            )
        });
        let (text, consumed, invalid_utf8) = (text.0, text.1, text.2);
        if invalid_utf8 {
            return Err(Signal::Error(Box::new(crate::native::AlError {
                message: "Invalid data encountered in stream, unable to read text.".into(),
                code: "InvalidStreamData".into(),
                call_stack: String::new(),
                ..Default::default()
            })));
        }
        let Some(text) = text else {
            return unsupported("InStream.ReadText of MS-DOS bytes whose character is not pinned");
        };
        s.pos = pos + consumed;
        enc_used = enc;
        Ok(text)
    })?;
    // The count is the bytes of the text in the stream's encoding, without the terminators (a UTF-16 `ab` is 4).
    // UTF-8 counts its own bytes: `aéb` is 4 (streams/lt-stream-non-ascii/StreamRawBytes.al:P175_Raw_EAcute_ReadText `utf8 utf8`).
    let n = if enc_used == Encoding::Utf8 {
        text.len() as i32
    } else {
        (Value::text_len(&text) * enc_used.unit()) as i32
    };
    a[0] = Value::Text(text.into());
    Ok(Value::Integer(n))
}

/// Whether a code unit of the stream (one byte, or two for UTF-16) is the ASCII character `c`.
fn is_unit(u: &[u8], c: u8) -> bool {
    match u {
        [b] => *b == c,
        [b, 0] => *b == c,
        _ => false,
    }
}

/// `Read` into an Integer (4 bytes, little endian: streams/TestInOutStream.al
/// InOutStream_Write_Integer_ReadInteger) or a Byte (one byte: the HexOf helper of
/// streams/TestStreamsRuntimeContracts.al reads a stream byte by byte). Other types are not pinned.
fn in_read(c: &mut dyn Ctx, mut r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    // `Read(Text, Length)` on a single-byte stream takes Length bytes as one character each
    // (xml/TestXmlDocumentWriteToOutStreamContracts.al:LeadingBytes reads the bytes 239 187 191 of a
    // byte order mark into a Text[3]); bytes 128 to 159, other encodings and a used result are not pinned.
    if let (2, Value::Text(_) | Value::Code(_), Some(Value::Integer(n))) = (a.len(), &a[0], a.get(1)) {
        let n = usize::try_from(*n).unwrap_or(0);
        if c.ret_used() {
            return unsupported("the result of InStream.Read(Text, Length) (not pinned)");
        }
        let text = with_in(r, |s| {
            if !matches!(s.enc, Encoding::Windows | Encoding::MsDos) {
                return unsupported(
                    "InStream.Read(Text, Length) on a stream that is neither Windows nor MS-DOS encoded (not pinned)",
                );
            }
            let rest = s.rest();
            let taken = &rest[..n.min(rest.len())];
            if s.enc == Encoding::Windows && taken.iter().any(|b| (0x80..0xA0).contains(b)) {
                return unsupported("InStream.Read(Text, Length) of bytes 128 to 159 (not pinned)");
            }
            if !s.enc.decode_pinned(taken) {
                return unsupported("InStream.Read(Text, Length) of MS-DOS bytes whose character is not pinned");
            }
            s.pos += taken.len();
            Ok(s.enc.decode(taken))
        })?;
        a[0] = Value::Text(text.into());
        return Ok(Value::Null);
    }
    if a.len() > 1 {
        return unsupported("InStream.Read with a length (not pinned)");
    }
    // A Text or Code: the inverse of `Write(Text)` (:OutStream_Write_BinaryFormsOfEachType pins the
    // characters and a zero byte). It takes the characters up to the zero byte, or to the end of the
    // stream when there is none (streams/ProbeJsonAndInStreamBytes.al:InStreamRead_Text_OnWriteTextContent_NoTerminator,
    // no error). The result is the number of bytes taken, the zero byte included
    // (:InStreamRead_Text_AfterWriteTextAndAfterWriteText_NonAscii returns 6 for 'hello' and for 'wörld',
    // :InStreamRead_Text_ReturnValueIsLength returns 7 for 'abcdef'). A Code variable is filled by
    // the caller's assignment, which truncates to its length (:InStreamRead_Text_ReturnValueIsLength).
    if matches!(a[0], Value::Text(_) | Value::Code(_)) {
        let used = c.ret_used();
        let (taken, count) = with_in(r, |s| {
            let unit = s.enc.unit();
            let rest = s.rest();
            // At the end of the stream the Text is emptied, no error. The result is 0 on a stream of the default encoding
            // (misc/lt-probes-needed-3/PnC20JsonStreams.al:312-318, `pastEnd`) and 1 on a UTF-8 one (misc/lt-probes-needed-5/
            // PnE04Streams.al:89 and :113, `atEnd`).
            if rest.is_empty() {
                return Ok((String::new(), usize::from(s.enc == Encoding::Utf8)));
            }
            let end = rest.chunks_exact(unit).position(|u| u.iter().all(|b| *b == 0));
            // On a UTF-8 stream a text with no zero byte after it counts one more than its bytes as well: `hello` is 6
            // (PnE04Streams.al:111, ReadText_Utf8_NoTerminator).
            let utf8_unterminated = end.is_none() && s.enc == Encoding::Utf8;
            if used && ((end.is_none() && !utf8_unterminated) || unit != 1) {
                return unsupported(
                    "the result of InStream.Read into a Text without a zero terminator or on a UTF-16 stream (not pinned)",
                );
            }
            let (body_len, consumed) = match end {
                Some(e) => (e * unit, (e + 1) * unit),
                None => (rest.len(), rest.len()),
            };
            let counted = consumed + usize::from(utf8_unterminated);
            let body = &rest[..body_len];
            if !s.enc.decode_pinned(body)
                || (!s.enc.decode(body).is_ascii() && !matches!(s.enc, Encoding::Windows | Encoding::MsDos))
            {
                return unsupported(
                    "InStream.Read into a Text of non-ASCII characters on a stream that is not Windows-encoded (not pinned)",
                );
            }
            s.pos += consumed;
            Ok((s.enc.decode(body), counted))
        })?;
        a[0] = match a[0] {
            Value::Code(_) => Value::Code(taken.into()),
            _ => Value::Text(taken.into()),
        };
        return Ok(Value::Integer(count as i32));
    }
    let width = match a[0] {
        Value::Integer(_) | Value::Boolean(_) | Value::Date(..) => 4,
        Value::Byte(_) | Value::Char(_) => 1,
        Value::BigInteger(_) => 8,
        Value::Guid(_) => 16,
        // streams/TestStreamsBinaryDateDecimalContracts.al:117-123.
        Value::Decimal(_) => 12,
        _ => {
            return unsupported(format!(
                "InStream.Read into {} (only Integer, Byte and Decimal are pinned)",
                a[0].describe()
            ));
        }
    };
    // At the end of a stream that holds data nothing is read: the result is 0 and the variable takes its default value
    // (streams/lt-stream-read-write-media/P102ReadTyped.al: every `pastend_*`). A stream without any data keeps the error below.
    let at_end = with_in(r.as_deref_mut(), |s| {
        // An empty stream is at its end like any other (types/lt-variant-is-text-streams/LTP89StreamNumbers.al:
        // Integer_RoundTrip_PastEnd `read_empty`, with the second read raising: types/PRFileUpload.al:ReadFromUnfilledStream).
        let at = s.pos >= s.len();
        // The first read at the end answers 0; one after another read already hit the end raises.
        let first = at && !s.hit_end;
        if at {
            s.hit_end = true;
        }
        Ok(first)
    })?;
    if at_end {
        a[0] = Value::default_for(&match &a[0] {
            Value::Integer(_) => crate::value::RtType::Integer,
            Value::Boolean(_) => crate::value::RtType::Boolean,
            Value::Date(..) => crate::value::RtType::Date,
            Value::Byte(_) => crate::value::RtType::Byte,
            Value::Char(_) => crate::value::RtType::Char,
            Value::BigInteger(_) => crate::value::RtType::BigInteger,
            Value::Guid(_) => crate::value::RtType::Guid,
            _ => crate::value::RtType::Decimal,
        });
        return Ok(Value::Integer(0));
    }
    let bytes = with_in(r, |s| {
        let pos = s.pos;
        // Fewer bytes than the type needs, also none at all: the stream data error (streams/TestProbeStreamContracts.al:
        // Read_IntoTypes, a BigInteger or a Decimal read from 4 bytes; types/PRFileUpload.al:ReadFromUnfilledStream, a Byte).
        let got = s
            .with_data(|d| d.get(pos..pos + width).map(<[u8]>::to_vec))
            .ok_or_else(|| {
                Signal::Error(Box::new(crate::native::AlError {
                    message: "Invalid data encountered in stream, unable to read text.".into(),
                    code: "InvalidStreamData".into(),
                    call_stack: String::new(),
                    ..Default::default()
                }))
            })?;
        s.pos += width;
        Ok(got)
    })?;
    a[0] = match a[0] {
        Value::Byte(_) => Value::Byte(bytes[0]),
        Value::Char(_) => Value::Char(u16::from(bytes[0])),
        Value::Boolean(_) => Value::Boolean(bytes.iter().any(|b| *b != 0)),
        Value::BigInteger(_) => Value::BigInteger(i64::from_le_bytes(bytes[..8].try_into().expect("8 bytes"))),
        Value::Guid(_) => {
            let mut g: [u8; 16] = bytes[..16].try_into().expect("16 bytes");
            g[0..4].reverse();
            g[4..6].reverse();
            g[6..8].reverse();
            Value::Guid(g)
        }
        Value::Date(..) => {
            // The inverse of `date_bytes`: twice the day number plus 731, plus 1 for a closing date.
            let n = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            if n == 0 {
                Value::Date(0, false)
            } else {
                let closing = (n - 731) % 2 != 0;
                let day = (n - 731 - i32::from(closing)) / 2;
                // Bytes that are no date raise (misc/lt-probes-needed-1/PnAStreamReadMisc.al:Read_Types_A `op5`).
                if !(1..=3_652_059).contains(&day) {
                    return Err(Signal::Error(Box::new(crate::native::AlError {
                        message: "The date is not valid.".into(),
                        code: "DateInvalid".into(),
                        call_stack: String::new(),
                        ..Default::default()
                    })));
                }
                Value::Date(day, closing)
            }
        }
        Value::Decimal(_) => Value::Decimal(decimal_from_bytes(&bytes)?),
        _ => Value::Integer(i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])),
    };
    Ok(Value::Integer(width as i32))
}

fn out_buf(r: Option<&mut Value>) -> Result<(BlobRef, Encoding), Signal> {
    match r {
        Some(Value::Library(_, h)) => match &*h.borrow() {
            LibraryValue::OutStream(b, e) => Ok((b.clone(), *e)),
            _ => unsupported("use of an OutStream that was never created"),
        },
        other => unsupported(format!("OutStream method on {}", other.describe())),
    }
}

/// Writes serialised JSON (or other text a builtin produces) to an `OutStream` argument at its
/// write position. The bytes are UTF-8 whatever the encoding of the stream: UTF-8, UTF-16, Windows and
/// MS-DOS streams all receive the same bytes, and non-ASCII characters take their UTF-8 form
/// (streams/ProbeJsonAndInStreamBytes.al:WriteTo_OutStream_Utf16_AsciiAndNonAscii, :WriteTo_OutStream_Windows_AsciiAndNonAscii,
/// :WriteTo_OutStream_MsDos_AsciiAndNonAscii).
pub(crate) fn write_serialised_text(stream: &mut Value, text: &str) -> Result<(), Signal> {
    let (b, _) = out_buf(Some(stream))?;
    // The stream does not move: a second `WriteTo` on the same stream writes over the first one's bytes, and the
    // blob is not emptied (misc/lt-probes-needed-3/PnC20JsonStreams.al:WriteTo_DefaultStream_And_Appending,
    // `[1,2]1}`). Pinned only for a stream nothing was written to; after other writes it is refused.
    if b.borrow().write_pos != 0 {
        return unsupported(
            "JSON WriteTo(OutStream) on a stream that was written to (the place it writes at is not pinned)",
        );
    }
    write_bytes(&b, text.as_bytes());
    b.borrow_mut().write_pos = 0;
    Ok(())
}

/// `BigText.Write(OutStream)`: the text in the stream's encoding, appended at the stream's position like `WriteText`, so a stream
/// written to before takes the text after it and a later `WriteText` or `Write` goes after it
/// (streams/lt-bigtext-write/BigTextWriteEdges.al:Write_AfterWriteText_SameStream, WriteText_AfterBigTextWrite_SameStream;
/// corpus streams/StTBigText.al:WriteToStreamAndRead, :WriteReadLargeText). Pinned bytes: ASCII; for a default or MS-DOS stream
/// `84 B1 3F` for a-umlaut, euro sign and the Cyrillic zhe, for Windows `E4 80 3F`, for UTF-8 and UTF-16 the plain encoding of
/// the same three (BigTextWriteEdges.al:Write_NonAscii_*). A character beyond those and the characters the other stream writes
/// pin (`Encoding::encode_pinned`) is refused.
pub(crate) fn write_bigtext(stream: &mut Value, text: &str) -> Result<(), Signal> {
    let (b, enc) = out_buf(Some(stream))?;
    let pinned = |c: char| {
        c.is_ascii()
            || c == '\u{416}'
            || match enc {
                Encoding::Utf8 | Encoding::Utf16 => (c as u32) <= 0xFFFF,
                _ => enc.encode_pinned(c.encode_utf8(&mut [0; 4])),
            }
    };
    if !text.chars().all(pinned) {
        return unsupported("BigText.Write of a character whose bytes in this encoding are not pinned");
    }
    write_bytes(&b, &enc.encode(text));
    Ok(())
}

/// Like [`write_raw_bytes`], for a write that does not move the stream: it starts at the beginning, and the stream is back there
/// after it. Refused when something else was written to the stream first.
pub fn write_raw_bytes_rewinding(stream: &mut Value, bytes: &[u8]) -> Result<(), Signal> {
    let (b, _) = out_buf(Some(stream))?;
    if b.borrow().write_pos != 0 {
        return unsupported(
            "WriteTo(OutStream) on a stream that was written to (the place it writes at is not pinned)",
        );
    }
    write_bytes(&b, bytes);
    b.borrow_mut().write_pos = 0;
    Ok(())
}

/// Writes bytes to an `OutStream` argument whatever its encoding (the caller pins the bytes).
pub fn write_raw_bytes(stream: &mut Value, bytes: &[u8]) -> Result<(), Signal> {
    let (b, _) = out_buf(Some(stream))?;
    write_bytes(&b, bytes);
    Ok(())
}

/// Takes what remains of an `InStream` argument as text, for a parser that reads bytes (JSON
/// `ReadFrom`). The bytes are read as UTF-8 whatever the encoding of the stream, and a UTF-8 byte
/// order mark is skipped (streams/ProbeJsonAndInStreamBytes.al:ReadFrom_InStream_WithBomAndWithout, on a
/// default, Windows-encoded stream). That is the inverse of [`write_serialised_text`]; bytes that are not
/// UTF-8 are refused. The stream does not move.
pub(crate) fn read_serialised_text(stream: &mut Value) -> Result<String, Signal> {
    with_in(Some(stream), |st| {
        let bytes = st.rest();
        let text = String::from_utf8(bytes)
            .map_err(|_| Signal::Unsupported("reading bytes that are not UTF-8 from an InStream as text".into()))?;
        // UTF-16 bytes read as UTF-8 hold NUL characters: what BC makes of them is not pinned.
        if text.contains('\0') {
            return Err(Signal::Unsupported(
                "reading bytes with NUL characters (UTF-16 text?) from an InStream as UTF-8 (not pinned)".into(),
            ));
        }
        // The stream does not move: `EOS` stays false after the read (misc/lt-probes-needed-3/PnC20JsonStreams.al:
        // ReadFrom_RestOfStream, `eosAfter` = No), as for `XmlDocument.ReadFrom`.
        Ok(text.strip_prefix('\u{FEFF}').map(str::to_owned).unwrap_or(text))
    })
}

/// `WriteText(Text)` writes the text in the stream's encoding and no terminator; `WriteText()` writes
/// CR LF (:OutStream_WriteText_WithoutArgument_WritesCarriageReturnLineFeed).
fn out_write_text(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    if a.len() > 1 {
        return unsupported("OutStream.WriteText with a length (not pinned)");
    }
    let text = match a.first() {
        None => "\r\n".to_string(),
        Some(v) => match crate::native::text_of(v) {
            Some(t) => t,
            None => return unsupported("OutStream.WriteText of a value that is not text"),
        }
        .to_string(),
    };
    let (b, enc) = out_buf(r)?;
    let bytes = enc.encode(&text);
    write_bytes(&b, &bytes);
    Ok(Value::Integer(bytes.len() as i32))
}

/// A Date as 4 bytes little endian: 0 for `0D`, else twice the day number (`0001-01-01` is day 1)
/// plus 731, plus 1 for a closing date. Fits every measured value: 1753-01-01, 1900-01-01, 2000-01-01,
/// 2024-03-05, 9999-12-31 and the closing date of 2024-12-31 (streams/TestStreamsBinaryDateDecimalContracts.al:80-90).
fn date_bytes(day: i32, closing: bool) -> Result<Vec<u8>, Signal> {
    let n = match (day, closing) {
        (0, false) => 0,
        (0, true) => return unsupported("OutStream.Write of the closing date of no date (not pinned)"),
        (d, c) => 2 * d + 731 + i32::from(c),
    };
    Ok(n.to_le_bytes().to_vec())
}

/// A Decimal as 12 bytes (streams/TestStreamsBinaryDateDecimalContracts.al:7-12, :94-106): byte 0 holds
/// the sign (0x80) and the decimal exponent of the first digit plus 0x40 (1 is 0x40, 100 is 0x42, 0.01
/// is 0x3E); bytes 1 to 9 hold 18 digits as packed BCD, the first digit in the high nibble of byte 9, the
/// digits after the last significant one are zero; bytes 10 and 11 are zero. Zero is twelve zero bytes.
fn decimal_bytes(d: &rust_decimal::Decimal) -> Result<Vec<u8>, Signal> {
    let mut out = vec![0u8; 12];
    if d.is_zero() {
        return Ok(out);
    }
    let digits = d.mantissa().unsigned_abs().to_string();
    let exponent = digits.len() as i32 - 1 - d.scale() as i32;
    let digits = digits.trim_end_matches('0');
    // Digits after the 18th are dropped (misc/lt-probes-needed-1/PnAStreamDecimalBytes.al:Write_Decimal_Tiny, `thirds-28-places`:
    // 28 threes write 18). Whether a dropped digit of 5 or more rounds up is not pinned: refused.
    let digits = if digits.len() > 18 {
        if digits.as_bytes()[18] >= b'5' {
            return unsupported(
                "OutStream.Write of a Decimal with more than 18 digits whose 19th digit is 5 or more (rounding not pinned)",
            );
        }
        digits[..18].trim_end_matches('0')
    } else {
        digits
    };
    out[0] = (0x40 + exponent) as u8 | if d.is_sign_negative() { 0x80 } else { 0 };
    for (i, c) in digits.bytes().enumerate() {
        let v = c - b'0';
        let at = 9 - i / 2;
        out[at] |= if i % 2 == 0 { v << 4 } else { v };
    }
    Ok(out)
}

/// The inverse of [`decimal_bytes`].
fn decimal_from_bytes(b: &[u8]) -> Result<rust_decimal::Decimal, Signal> {
    if b.iter().all(|&x| x == 0) {
        return Ok(rust_decimal::Decimal::ZERO);
    }
    let exponent = i32::from(b[0] & 0x7F) - 0x40;
    let mut mantissa: i128 = 0;
    for i in 0..18 {
        let byte = b[9 - i / 2];
        let v = if i % 2 == 0 { byte >> 4 } else { byte & 0xF };
        mantissa = mantissa * 10 + i128::from(v);
    }
    let scale = 17 - exponent;
    let bad = || Signal::Unsupported("InStream.Read of a Decimal out of range (not pinned)".into());
    let mut d = if scale >= 0 {
        rust_decimal::Decimal::try_from_i128_with_scale(mantissa, scale as u32).map_err(|_| bad())?
    } else {
        let m = mantissa.checked_mul(10i128.pow((-scale) as u32)).ok_or_else(bad)?;
        rust_decimal::Decimal::try_from_i128_with_scale(m, 0).map_err(|_| bad())?
    };
    d.normalize_assign();
    if b[0] & 0x80 != 0 {
        d.set_sign_negative(true);
    }
    Ok(d)
}

/// `Write(Value)`: Integer and Boolean take 4 bytes, BigInteger 8, a Time 4 (its value, which is the
/// millisecond of the day plus one), a Text its characters and a zero byte
/// (:OutStream_Write_BinaryFormsOfEachType, all little endian), a Date 4 and a Decimal 12 (see
/// [`date_bytes`], [`decimal_bytes`]).
fn out_write(_: &mut dyn Ctx, r: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let (b, enc) = out_buf(r)?;
    // `Write(Text, Length)` writes the first Length characters (all of a shorter text) and the zero byte
    // (streams/TestProbeStreamContracts.al:WriteText_NonAscii_Char_Byte_Length, `write_text_len3`, `write_text_len5_over`).
    if a.len() == 2 {
        let (Some(t), Value::Integer(n)) = (crate::native::text_of(&a[0]), &a[1]) else {
            return unsupported(
                "OutStream.Write with a length of something else than a Text and an Integer (not pinned)",
            );
        };
        // A negative length writes the whole text (streams/lt-stream-read-write-media/P102WriteTextLike.al:P102_Text `lenneg_ret`).
        // A Code ignores the length and writes the whole text (:P102_Code `len2_ret` is 5).
        let take = if *n < 0 || matches!(a[0], Value::Code(_)) {
            usize::MAX
        } else {
            usize::try_from(*n).unwrap_or(0)
        };
        let cut: String = t.chars().take(take).collect();
        if !cut.is_ascii() {
            return unsupported("OutStream.Write(Text, Length) of non-ASCII characters (not pinned)");
        }
        let mut v = enc.encode(&cut);
        v.extend(std::iter::repeat_n(0, enc.unit()));
        write_bytes(&b, &v);
        return Ok(Value::Integer(v.len() as i32));
    }
    if a.len() != 1 {
        return unsupported("OutStream.Write with more arguments (not pinned)");
    }
    let bytes: Vec<u8> = match &a[0] {
        // A Char is one byte in the stream's encoding (streams/TestProbeStreamContracts.al:WriteText_NonAscii_Char_Byte_Length:
        // 65 is 41, 228 is 84 on a default stream).
        Value::Char(c)
            if *c < 256 && char::from_u32(u32::from(*c)).is_some_and(|ch| enc.encode_pinned(&ch.to_string())) =>
        {
            enc.encode(&char::from_u32(u32::from(*c)).expect("checked").to_string())
        }
        // A Guid is written as .NET writes it: the first three groups little endian
        // (streams/TestProbeStreamContracts.al:WriteBooleanGuidBigIntegerOthers).
        Value::Guid(g) => {
            let mut v = g.to_vec();
            v[0..4].reverse();
            v[4..6].reverse();
            v[6..8].reverse();
            v
        }
        // One byte: the probe's blob writer builds the streams BigText.Read decodes from `Write(Byte)` alone
        // (corpus types/PEOptionEnumBigText.al:313-333, whose results fix the bytes on BC 27.5).
        Value::Byte(b) => vec![*b],
        Value::Integer(n) => n.to_le_bytes().to_vec(),
        Value::Boolean(v) => i32::from(*v).to_le_bytes().to_vec(),
        Value::BigInteger(n) => n.to_le_bytes().to_vec(),
        Value::Time(t) => i32::try_from(*t)
            .map_err(|_| Signal::Unsupported("OutStream.Write of a Time outside 32 bits".into()))?
            .to_le_bytes()
            .to_vec(),
        Value::Date(d, closing) => date_bytes(*d, *closing)?,
        Value::Decimal(d) => decimal_bytes(d)?,
        // Non-ASCII characters are pinned only for a Windows stream, as one cp1252 byte each
        // (streams/ProbeJsonAndInStreamBytes.al:InStreamRead_Text_AfterWriteTextAndAfterWriteText_NonAscii:
        // 'wörld' is 77 F6 72 6C 64 00).
        // The text ends at its first NUL character (types/lt-variant-is-text-streams/LTP89StreamText.al:Text_BytesWritten `nul_char`).
        Value::Text(t) | Value::Code(t) if enc.encode_pinned(t.split('\0').next().unwrap_or_default()) => {
            let mut v = enc.encode(t.split('\0').next().unwrap_or_default());
            v.extend(std::iter::repeat_n(0, enc.unit()));
            v
        }
        other => {
            return unsupported(format!(
                "OutStream.Write of {} (its binary form is not pinned)",
                other.describe()
            ));
        }
    };
    write_bytes(&b, &bytes);
    Ok(Value::Integer(bytes.len() as i32))
}

/// `CopyStream(OutStream, InStream [, BytesToRead])`: copies what remains of the source.
fn copy_stream(ctx: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    // One side is a .NET stream: the context copies the bytes through its host (ADR-024).
    if matches!(&a[0], Value::DotNet(_)) || matches!(&a[1], Value::DotNet(_)) {
        let count = match a.get(2) {
            None => None,
            Some(Value::Integer(n)) if *n >= 0 => Some(i64::from(*n)),
            Some(other) => return unsupported(format!("CopyStream with a byte count of {}", other.describe())),
        };
        return match ctx.copy_stream_dotnet(&a[0], &a[1], count) {
            Some(r) => r.map(|()| Value::Boolean(true)),
            None => unsupported("CopyStream with a DotNet stream needs a .NET host"),
        };
    }
    if a.len() > 2 {
        return unsupported("CopyStream with a byte count (not pinned)");
    }
    let (out, _) = out_buf(Some(&mut a[0]))?;
    let bytes = with_in(Some(&mut a[1]), |s| Ok(s.drain()))?;
    write_bytes(&out, &bytes);
    Ok(Value::Boolean(true))
}

// ------------------------------------------------------- arrays and strings

fn array_of(v: &Value) -> Result<Rc<RefCell<Vec<Value>>>, Signal> {
    match v {
        Value::Array(a) => Ok(a.clone()),
        other => unsupported(format!("{} used as an array", other.describe())),
    }
}

/// `CompressArray(StringArray)`: empty strings are removed, the rest shifts left, and the tail is
/// blank; returns the number of non-empty strings (streams/TestArrayStreamContracts.al).
fn compress_array(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let arr = array_of(&a[0])?;
    let mut arr = arr.borrow_mut();
    let blank = |v: &Value| matches!(v, Value::Text(s) | Value::Code(s) if s.is_empty());
    let mut kept: Vec<Value> = arr.iter().filter(|v| !blank(v)).cloned().collect();
    let n = kept.len();
    for v in arr.iter().skip(n) {
        if !matches!(v, Value::Text(_) | Value::Code(_)) {
            return unsupported("CompressArray of an array that is not text");
        }
    }
    while kept.len() < arr.len() {
        kept.push(match &arr[kept.len()] {
            Value::Code(_) => Value::Code(String::new().into()),
            _ => Value::Text(String::new().into()),
        });
    }
    *arr = kept;
    Ok(Value::Integer(n as i32))
}

/// `CopyArray(NewArray, Array, Position [, Length])` (1-based Position).
fn copy_array(_: &mut dyn Ctx, _: Option<&mut Value>, a: &mut [Value]) -> NResult {
    let (dst, src) = (array_of(&a[0])?, array_of(&a[1])?);
    let Value::Integer(pos) = a[2] else {
        return unsupported("CopyArray position that is not an Integer");
    };
    let src = src.borrow();
    let mut dst = dst.borrow_mut();
    // The length parameter is the Length given, else what is left of the source from Position on. When it is not the length of
    // the destination, the copy raises (misc/lt-system-functions/Arrays.al:CopyArray_Forms `af30`, `af37`, `af44`, `into smaller`,
    // `from 6`, `from 0`, `count 6`, `count -1`, `from 5 count 2`; CopyArray_Texts). Position 0 and 6 of 5 are pinned, others outside are not.
    let length = match a.get(3) {
        Some(Value::Integer(n)) => i64::from(*n),
        None if (0..=src.len() as i64 + 1).contains(&i64::from(pos)) => src.len() as i64 + 1 - i64::from(pos),
        None => return unsupported("CopyArray from a position outside the source (not pinned)"),
        Some(other) => return unsupported(format!("CopyArray length {}", other.describe())),
    };
    if length != dst.len() as i64 {
        return raise(
            format!(
                "CopyArray: The length parameter {length} does not match the length of the destination array {}.",
                dst.len()
            ),
            "ArrayLengthMismatch",
        );
    }
    let len = dst.len();
    let Ok(pos) = usize::try_from(pos - 1) else {
        return unsupported("CopyArray position below 1 with a length that fits (not pinned)");
    };
    if pos + len > src.len() {
        return unsupported("CopyArray range outside the source (not pinned)");
    }
    for i in 0..len {
        dst[i] = src[pos + i].clone();
    }
    Ok(Value::Null)
}

// --------------------------------------------------------------- dispatch

pub fn native_for(sig: &Signature) -> Option<NativeFn> {
    let params: Vec<&str> = sig.params.iter().map(|p| p.ty.as_str()).collect();
    let f: NativeFn = match (sig.receiver.as_str(), sig.name.as_str(), params.as_slice()) {
        ("Blob", "CreateOutStream", ["OutStream", ..]) => blob_create_out,
        ("Blob", "CreateInStream", ["InStream", ..]) => blob_create_in,
        ("Blob", "HasValue", []) => blob_has_value,
        ("Blob", "Length", []) => blob_length,
        ("Blob", "Import", ["Text"]) => blob_import,
        ("InStream", "EOS", []) => in_eos,
        ("InStream", "ResetPosition", []) => in_reset,
        ("InStream", "Length", []) => in_length,
        ("InStream", "Position", _) => in_position,
        ("InStream", "ReadText", _) => in_read_text,
        ("InStream", "Read", _) => in_read,
        ("OutStream", "WriteText", _) => out_write_text,
        ("OutStream", "Write", _) => out_write,
        ("System", "CopyStream", _) => copy_stream,
        ("System", "CompressArray", _) => compress_array,
        ("System", "CopyArray", _) => copy_array,
        _ => return None,
    };
    Some(f)
}

#[cfg(test)]
mod blob_cell_tests {
    use super::*;

    #[test]
    fn readers_share_and_a_writer_excludes_them_as_in_a_refcell() {
        let b = new_blob();
        let (r1, r2) = (b.borrow(), b.borrow());
        assert_eq!(r1.bytes, r2.bytes);
        drop((r1, r2));
        b.borrow_mut().bytes.push(1);
        assert_eq!(b.borrow().bytes, vec![1]);
    }

    #[test]
    #[should_panic(expected = "already borrowed")]
    fn a_writer_while_a_reader_is_held_panics() {
        let b = new_blob();
        let _r = b.borrow();
        let _w = b.borrow_mut();
    }

    #[test]
    fn a_blob_is_read_by_two_threads() {
        let b = new_blob();
        b.borrow_mut().bytes.extend_from_slice(b"abc");
        let copies: Vec<Vec<u8>> = std::thread::scope(|scope| {
            let hs: Vec<_> = (0..2)
                .map(|_| scope.spawn(|| copy_blob(&b, false).borrow().bytes.clone()))
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        assert_eq!(copies, vec![b"abc".to_vec(), b"abc".to_vec()]);
        assert!(Value::Blob(b).is_plain());
    }

    // streams/TestStreamsRuntimeContracts.al:WriteText_ExplicitEncodings_Bytes, streams/StTEncodings.al:RoundTripSameEncoding and
    // CrossEncodingUtf8ToOthers: the default encoding is code page 437.
    #[test]
    fn the_default_encoding_is_code_page_437() {
        assert_eq!(Encoding::default(), Encoding::MsDos);
        assert_eq!(Encoding::MsDos.encode("hé€Ж"), vec![0x68, 0x82, 0xB1, 0x3F]);
        assert_eq!(Encoding::Windows.encode("hé"), vec![0x68, 0xE9]);
        // The euro sign comes back from its byte; the bytes whose character the Windows tier changes read as pinned.
        assert_eq!(Encoding::MsDos.decode(&[0xB1]), "€");
        assert_eq!(
            Encoding::MsDos.decode(&[0x61, 0xC3, 0xA4, 0xE2, 0x82]),
            "a\u{201C}ñË\u{e9}"
        );
        assert!(Encoding::MsDos.decode_pinned(&[0x82, 0xA4, 0xC3]));
        // A box-drawing byte nothing pins is not decoded.
        assert!(!Encoding::MsDos.decode_pinned(&[0xC4]));
        // The UTF-8 bytes of U+202F and of `年月日` in a SaveAs Xml dataset read through the default encoding
        // (report/lt-y-report-props/YReportPropsTests.al:80, :96).
        let ja = "2025\u{5E74}1\u{6708}31\u{65E5} 1\u{202F}2".as_bytes();
        assert!(Encoding::MsDos.decode_pinned(ja));
        assert_eq!(
            Encoding::MsDos.decode(ja),
            "2025\u{CE}\u{2030}\u{201E}1\u{B5}\u{A3}\u{EA}31\u{B5}\u{F9}\u{D1} 1\u{CB}\u{C7}\u{BB}2"
        );
    }
}
