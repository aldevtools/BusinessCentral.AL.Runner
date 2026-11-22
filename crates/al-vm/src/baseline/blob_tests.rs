//! A committed Blob row read back from the install baseline's bytes (`Baseline::from_bytes(to_bytes())`: what a run that
//! reads the baseline from the cache, and every worker thread of a shared baseline, ADR-069, starts from) holds the buffer
//! the captured baseline holds, every field of it (al-builtins `blob_form`).

use super::Baseline;
use crate::{Engine, Session};
use al_builtins::stream::{AssignedWrite, BlobRef, Encoding};
use al_builtins::value::Value;

const SRC: &str = r#"
table 50100 "Blob Row"
{
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } field(3; Pic; Media) { } }
    keys { key(PK; Id) { } }
}
codeunit 50100 Writer
{
    // Two Byte values through a UTF-16 OutStream: BlobBuf::utf16_binary.
    procedure WriteUtf16Bytes()
    var R: Record "Blob Row"; O: OutStream; B: Byte;
    begin
        R.Id := 1; R.Data.CreateOutStream(O, TextEncoding::UTF16);
        B := 65; O.Write(B); B := 0; O.Write(B);
        R.Insert();
    end;
    procedure ReadUtf16(): Text
    var R: Record "Blob Row"; I: InStream; T: Text;
    begin
        R.Get(1); R.CalcFields(Data); R.Data.CreateInStream(I, TextEncoding::UTF16); I.Read(T); exit(T);
    end;
    // The same two bytes as a Text: no utf16_binary mark.
    procedure WriteUtf16Text()
    var R: Record "Blob Row"; O: OutStream;
    begin
        R.Id := 1; R.Data.CreateOutStream(O, TextEncoding::UTF16); O.WriteText('A');
        R.Insert();
    end;
    // A Text through the default encoding; the Insert leaves BlobBuf::inserted.
    procedure WriteText()
    var R: Record "Blob Row"; O: OutStream;
    begin
        R.Id := 1; R.Data.CreateOutStream(O); O.WriteText('content');
        R.Insert();
    end;
    procedure ReadText(): Text
    var R: Record "Blob Row"; I: InStream; T: Text;
    begin
        R.Get(1); R.CalcFields(Data); R.Data.CreateInStream(I); I.ReadText(T); exit(T);
    end;
    procedure ImportOwnBlob(): Boolean
    var R: Record "Blob Row"; I: InStream;
    begin
        R.Get(1); R.CalcFields(Data); R.Data.CreateInStream(I); R.Pic.ImportStream(I, 'pic'); exit(R.Pic.HasValue());
    end;
}
"#;

/// A session on `SRC` and the index of codeunit Writer.
fn session() -> (Session, usize) {
    let l = al_testkit::compile(SRC);
    let engine = Engine::new(l.program.clone(), al_testkit::blocked(&l));
    let writer = engine
        .program
        .objects
        .iter()
        .position(|o| o.name == "Writer")
        .expect("Writer");
    (Session::new(engine), writer)
}

/// What `routine` of Writer answers, or the refusal it ends with.
fn outcome(s: &mut Session, writer: usize, routine: &str) -> String {
    let ri = (s.engine.program.objects[writer].routines.iter())
        .position(|r| r.name == routine)
        .unwrap_or_else(|| panic!("no routine {routine}"));
    let inst = s.codeunit_instance(writer);
    match s.call(writer, ri, inst, vec![]) {
        Ok((v, _)) => format!("{v:?}"),
        Err(e) => format!("{e:?}"),
    }
}

/// A new session of `s`'s engine, restored from `b` read back from its bytes.
fn restored_from_bytes(s: &Session, b: &Baseline) -> Session {
    let back = Baseline::from_bytes(&b.to_bytes().expect("plain rows")).expect("reads back");
    let mut other = Session::new(s.engine.clone());
    other.restore_baseline(&back);
    other
}

/// The buffer of the Blob of the committed row of `b` (the store's own, not a copy).
fn row_blob(b: &Baseline) -> BlobRef {
    let tables = b.store.export_committed().expect("no backing");
    (tables.iter().flat_map(|t| &t.rows))
        .find_map(|(_, row)| {
            row.iter()
                .find_map(|v| if let Value::Blob(b) = v { Some(b.clone()) } else { None })
        })
        .expect("a Blob row")
}

// What AL sees: the captured baseline refuses a UTF-16 Read(Text) of bytes a Write of a Byte put there (stream_read_len.rs
// `utf16`; streams/PnE04Streams.al:162-183); a session restored from its bytes refuses alike.
#[test]
fn a_blob_row_read_back_from_the_bytes_refuses_as_the_captured_one() {
    let (mut s, writer) = session();
    outcome(&mut s, writer, "WriteUtf16Bytes");
    let b = s.capture_baseline();
    let mut captured = Session::new(s.engine.clone());
    captured.restore_baseline(&b);
    let ours = outcome(&mut captured, writer, "ReadUtf16");
    assert!(ours.contains("UTF-16"), "the captured baseline refuses: {ours}");
    assert_eq!(outcome(&mut restored_from_bytes(&s, &b), writer, "ReadUtf16"), ours);
}

// Every field of the buffer comes back, each set apart from what a new buffer holds (issue 8588: a test-data row's
// `written_base` and `content_edits` did not survive the bytes).
#[test]
fn a_blob_row_read_back_from_the_bytes_keeps_every_buffer_field() {
    let (mut s, writer) = session();
    outcome(&mut s, writer, "WriteUtf16Bytes");
    let b = s.capture_baseline();
    {
        let blob = row_blob(&b);
        let mut buf = blob.borrow_mut();
        buf.bytes = vec![b'x'; 168];
        buf.write_pos = 168;
        buf.loaded = false;
        buf.fetched = false;
        buf.refetch = true;
        buf.persisted = true;
        buf.temp_row = true;
        buf.uncompressed = true;
        buf.written = true;
        buf.written_base = Some(Vec::new());
        buf.render_pending = Some("a report".into());
        buf.inserted = true;
        buf.temp_inserted = true;
        buf.db_row = true;
        buf.first_write_truncates = true;
        buf.file = true;
        buf.dotnet_view = 3;
        buf.assigned_unsaved = true;
        buf.assigned_write = AssignedWrite::Unmeasured;
        buf.unread_content = true;
        buf.utf16_binary = true;
        buf.unicode_read = Some(Encoding::Utf16);
        buf.content_edits = 1;
        buf.line_breaks = al_builtins::stream_line_breaks::LineBreakHistory::cleared(5);
        buf.line_breaks.in_creates = 2;
        buf.line_breaks.in_uses = 4;
    }
    let ours = format!("{:?}", *row_blob(&b).borrow());
    assert!(
        ours.contains("written_base: Some([])"),
        "the store's own buffer: {ours}"
    );
    let back = Baseline::from_bytes(&b.to_bytes().expect("plain rows")).expect("reads back");
    assert_eq!(format!("{:?}", *row_blob(&back).borrow()), ours);
}

/// What `read` answers on a session restored from the captured `b`, and on one restored from `b`'s bytes.
fn captured_and_from_bytes(s: &Session, writer: usize, b: &Baseline, read: &str) -> (String, String) {
    let mut captured = Session::new(s.engine.clone());
    captured.restore_baseline(b);
    let ours = outcome(&mut captured, writer, read);
    (ours, outcome(&mut restored_from_bytes(s, b), writer, read))
}

// BlobBuf::render_pending (ADR-040): the output of a report saved to the Blob and not rendered. Any look at it is refused,
// and a stored row hands the mark to the record that reads it (`copy_blob`). Issue 8588: the bytes form dropped it, so a
// session restored from them read the content.
#[test]
fn a_report_output_row_read_back_from_the_bytes_keeps_its_mark() {
    let (mut s, writer) = session();
    outcome(&mut s, writer, "WriteText");
    let b = s.capture_baseline();
    row_blob(&b).borrow_mut().render_pending = Some("Neighbor Report".into());
    let (ours, theirs) = captured_and_from_bytes(&s, writer, &b, "ReadText");
    assert!(
        ours.contains("Neighbor Report"),
        "the captured baseline refuses: {ours}"
    );
    assert_eq!(theirs, ours);
    let back = Baseline::from_bytes(&b.to_bytes().expect("plain rows")).expect("reads back");
    assert_eq!(
        row_blob(&back).borrow().render_pending.as_deref(),
        Some("Neighbor Report")
    );
}

// BlobBuf::inserted: the content is what an Insert stored. Media.ImportStream from a stream on that Blob of the same record
// is refused (streams/PnE06Media.al:66-84; `refuse_media_import_from_own_blob`). Issue 8588: the bytes form dropped the
// mark, so a session restored from them imported.
#[test]
fn an_inserted_row_read_back_from_the_bytes_keeps_its_mark() {
    let (mut s, writer) = session();
    outcome(&mut s, writer, "WriteText");
    let b = s.capture_baseline();
    assert!(row_blob(&b).borrow().inserted, "the Insert marks the stored buffer");
    let (ours, theirs) = captured_and_from_bytes(&s, writer, &b, "ImportOwnBlob");
    assert!(
        ours.contains("Media.ImportStream"),
        "the captured baseline refuses: {ours}"
    );
    assert_eq!(theirs, ours);
}

// The install-order comparison names the Blob fields that differ (`BlobBuf::differing_fields`), not only that a Blob does:
// the two orders here store the same two bytes, one through Byte writes (utf16_binary), one through WriteText.
#[test]
fn a_differing_install_order_names_the_blob_fields() {
    let (mut s, writer) = session();
    let start = s.capture_baseline();
    let order = |routine: &str| {
        let mut o = Session::new(s.engine.clone());
        o.restore_baseline(&start);
        outcome(&mut o, writer, routine);
        o.capture_baseline()
    };
    let (a, b) = (order("WriteUtf16Bytes"), order("WriteUtf16Text"));
    let d = a
        .install_order_differences(&b, &start, &s.engine, ((0, 0), (0, 0)))
        .expect("tables only");
    let lines: Vec<&String> = d.values().flatten().collect();
    assert!(
        lines
            .iter()
            .any(|l| l.starts_with("key 1, field Data: the Blob differs in ") && l.contains("utf16_binary")),
        "{lines:?}"
    );
}
