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
    fields { field(1; Id; Integer) { } field(2; Data; Blob) { } }
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
