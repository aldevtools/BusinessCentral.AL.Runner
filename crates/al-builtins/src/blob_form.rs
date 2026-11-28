//! Every field of a Blob buffer ([`BlobBuf`]), in one place: its bytes form (the install baseline on disk and between
//! threads, ADR-069) and the comparison that names the fields two buffers differ in. Each destructures the whole buffer,
//! with no `..`, so a field added to `BlobBuf` does not compile until it is written, read and compared here: the round trip
//! of a buffer stays exact. The bytes form used to keep only the bytes, the write position and eight flags: a row the
//! install code wrote with a mark that `copy_blob` hands on (`utf16_binary`, `render_pending`, `inserted`) lost it on a worker
//! thread and on a run that read the baseline from the cache (al-vm `baseline/blob_tests.rs`).
//!
//! Every field of a Blob buffer (`BlobBuf`) in one place: its bytes form (`value_codec` tag 17: the
//! install baseline on disk and between threads, ADR-069) and `BlobBuf::differing_fields`, the
//! comparison that names the fields two buffers differ in (and `BlobBuf`'s `PartialEq`). Tests: here
//! and al-vm `baseline/blob_tests.rs`. Each destructures the whole buffer, so a new field does not
//! compile until it is carried and compared. Entry points: `put`, `read`,
//! `BlobBuf::differing_fields`.

use crate::stream::{AssignedWrite, BlobBuf, Encoding};
use crate::stream_line_breaks::LineBreakHistory;
use crate::value_codec::{Reader, put_bytes, put_str, put_u8, put_u32, put_u64};

/// The bits of the Boolean fields, in this order.
const LOADED: u32 = 1;
const FETCHED: u32 = 1 << 1;
const REFETCH: u32 = 1 << 2;
const PERSISTED: u32 = 1 << 3;
const TEMP_ROW: u32 = 1 << 4;
const UNCOMPRESSED: u32 = 1 << 5;
const WRITTEN: u32 = 1 << 6;
const INSERTED: u32 = 1 << 7;
const TEMP_INSERTED: u32 = 1 << 8;
const DB_ROW: u32 = 1 << 9;
const FIRST_WRITE_TRUNCATES: u32 = 1 << 10;
const FILE: u32 = 1 << 11;
const ASSIGNED_UNSAVED: u32 = 1 << 12;
const UNREAD_CONTENT: u32 = 1 << 13;
const UTF16_BINARY: u32 = 1 << 14;

/// Appends every field of `b`.
pub(crate) fn put(b: &BlobBuf, out: &mut Vec<u8>) {
    let BlobBuf {
        bytes,
        write_pos,
        loaded,
        fetched,
        refetch,
        persisted,
        temp_row,
        uncompressed,
        written,
        written_base,
        render_pending,
        inserted,
        temp_inserted,
        db_row,
        first_write_truncates,
        file,
        dotnet_view,
        assigned_unsaved,
        assigned_write,
        unread_content,
        utf16_binary,
        unicode_read,
        content_edits,
        line_breaks,
    } = b;
    put_bytes(out, bytes);
    put_u64(out, *write_pos as u64);
    let flags = [
        (*loaded, LOADED),
        (*fetched, FETCHED),
        (*refetch, REFETCH),
        (*persisted, PERSISTED),
        (*temp_row, TEMP_ROW),
        (*uncompressed, UNCOMPRESSED),
        (*written, WRITTEN),
        (*inserted, INSERTED),
        (*temp_inserted, TEMP_INSERTED),
        (*db_row, DB_ROW),
        (*first_write_truncates, FIRST_WRITE_TRUNCATES),
        (*file, FILE),
        (*assigned_unsaved, ASSIGNED_UNSAVED),
        (*unread_content, UNREAD_CONTENT),
        (*utf16_binary, UTF16_BINARY),
    ];
    put_u32(out, flags.iter().filter(|(on, _)| *on).map(|(_, bit)| bit).sum());
    match written_base {
        None => put_u8(out, 0),
        Some(base) => {
            put_u8(out, 1);
            put_bytes(out, base);
        }
    }
    match render_pending {
        None => put_u8(out, 0),
        Some(mark) => {
            put_u8(out, 1);
            put_str(out, mark);
        }
    }
    put_u32(out, *dotnet_view);
    put_u8(out, assigned_write_byte(*assigned_write));
    put_u8(out, unicode_read.map_or(0, |e| 1 + encoding_byte(e)));
    put_u32(out, *content_edits);
    line_breaks.put(out);
}

/// The buffer [`put`] wrote; `None` for bytes it did not write.
pub(crate) fn read(r: &mut Reader) -> Option<BlobBuf> {
    let bytes = r.bytes()?.to_vec();
    let write_pos = usize::try_from(r.u64()?).ok()?;
    let flags = r.u32()?;
    let on = |bit: u32| flags & bit != 0;
    let written_base = match r.u8()? {
        0 => None,
        1 => Some(r.bytes()?.to_vec()),
        _ => return None,
    };
    let render_pending = match r.u8()? {
        0 => None,
        1 => Some(r.str()?.into()),
        _ => return None,
    };
    let dotnet_view = r.u32()?;
    let assigned_write = assigned_write_of(r.u8()?)?;
    let unicode_read = match r.u8()? {
        0 => None,
        n => Some(encoding_of(n - 1)?),
    };
    Some(BlobBuf {
        bytes,
        write_pos,
        loaded: on(LOADED),
        fetched: on(FETCHED),
        refetch: on(REFETCH),
        persisted: on(PERSISTED),
        temp_row: on(TEMP_ROW),
        uncompressed: on(UNCOMPRESSED),
        written: on(WRITTEN),
        written_base,
        render_pending,
        inserted: on(INSERTED),
        temp_inserted: on(TEMP_INSERTED),
        db_row: on(DB_ROW),
        first_write_truncates: on(FIRST_WRITE_TRUNCATES),
        file: on(FILE),
        dotnet_view,
        assigned_unsaved: on(ASSIGNED_UNSAVED),
        assigned_write,
        unread_content: on(UNREAD_CONTENT),
        utf16_binary: on(UTF16_BINARY),
        unicode_read,
        content_edits: r.u32()?,
        line_breaks: LineBreakHistory::read(r)?,
    })
}

fn assigned_write_byte(w: AssignedWrite) -> u8 {
    match w {
        AssignedWrite::No => 0,
        AssignedWrite::Other => 1,
        AssignedWrite::NeverInsertedEmpties => 2,
        AssignedWrite::Unmeasured => 3,
    }
}

fn assigned_write_of(b: u8) -> Option<AssignedWrite> {
    [
        AssignedWrite::No,
        AssignedWrite::Other,
        AssignedWrite::NeverInsertedEmpties,
        AssignedWrite::Unmeasured,
    ]
    .into_iter()
    .find(|&w| assigned_write_byte(w) == b)
}

fn encoding_byte(e: Encoding) -> u8 {
    match e {
        Encoding::Utf8 => 0,
        Encoding::Windows => 1,
        Encoding::MsDos => 2,
        Encoding::Utf16 => 3,
    }
}

fn encoding_of(b: u8) -> Option<Encoding> {
    [Encoding::Utf8, Encoding::Windows, Encoding::MsDos, Encoding::Utf16]
        .into_iter()
        .find(|&e| encoding_byte(e) == b)
}

impl BlobBuf {
    /// The names of the fields in which `self` and `other` differ, in the order `BlobBuf` declares them; empty when the two
    /// buffers are equal. Names only: what a field holds is never part of the answer.
    pub fn differing_fields(&self, other: &BlobBuf) -> Vec<&'static str> {
        let BlobBuf {
            bytes,
            write_pos,
            loaded,
            fetched,
            refetch,
            persisted,
            temp_row,
            uncompressed,
            written,
            written_base,
            render_pending,
            inserted,
            temp_inserted,
            db_row,
            first_write_truncates,
            file,
            dotnet_view,
            assigned_unsaved,
            assigned_write,
            unread_content,
            utf16_binary,
            unicode_read,
            content_edits,
            line_breaks,
        } = self;
        let same = [
            ("bytes", *bytes == other.bytes),
            ("write_pos", *write_pos == other.write_pos),
            ("loaded", *loaded == other.loaded),
            ("fetched", *fetched == other.fetched),
            ("refetch", *refetch == other.refetch),
            ("persisted", *persisted == other.persisted),
            ("temp_row", *temp_row == other.temp_row),
            ("uncompressed", *uncompressed == other.uncompressed),
            ("written", *written == other.written),
            ("written_base", *written_base == other.written_base),
            ("render_pending", *render_pending == other.render_pending),
            ("inserted", *inserted == other.inserted),
            ("temp_inserted", *temp_inserted == other.temp_inserted),
            ("db_row", *db_row == other.db_row),
            (
                "first_write_truncates",
                *first_write_truncates == other.first_write_truncates,
            ),
            ("file", *file == other.file),
            ("dotnet_view", *dotnet_view == other.dotnet_view),
            ("assigned_unsaved", *assigned_unsaved == other.assigned_unsaved),
            ("assigned_write", *assigned_write == other.assigned_write),
            ("unread_content", *unread_content == other.unread_content),
            ("utf16_binary", *utf16_binary == other.utf16_binary),
            ("unicode_read", *unicode_read == other.unicode_read),
            ("content_edits", *content_edits == other.content_edits),
            ("line_breaks", *line_breaks == other.line_breaks),
        ];
        same.into_iter().filter(|(_, s)| !s).map(|(name, _)| name).collect()
    }
}

/// Two buffers are equal when no field differs ([`BlobBuf::differing_fields`]).
impl PartialEq for BlobBuf {
    fn eq(&self, other: &BlobBuf) -> bool {
        self.differing_fields(other).is_empty()
    }
}

impl Eq for BlobBuf {}

#[cfg(test)]
mod tests {
    use super::*;

    /// A buffer whose every field is apart from its default (and from `new_blob`'s).
    fn every_field_set() -> BlobBuf {
        let mut line_breaks = LineBreakHistory::cleared(5);
        line_breaks.note_out_open(Encoding::MsDos, true);
        line_breaks.note_text_write(Encoding::MsDos);
        line_breaks.in_creates = 2;
        line_breaks.in_uses = 4;
        BlobBuf {
            bytes: vec![1, 2, 3],
            write_pos: 2,
            loaded: false,
            fetched: false,
            refetch: true,
            persisted: true,
            temp_row: true,
            uncompressed: true,
            written: true,
            written_base: Some(Vec::new()),
            render_pending: Some("a report".into()),
            inserted: true,
            temp_inserted: true,
            db_row: true,
            first_write_truncates: true,
            file: true,
            dotnet_view: 9,
            assigned_unsaved: true,
            assigned_write: AssignedWrite::NeverInsertedEmpties,
            unread_content: true,
            utf16_binary: true,
            unicode_read: Some(Encoding::Windows),
            content_edits: 7,
            line_breaks,
        }
    }

    #[test]
    fn every_field_round_trips_through_the_bytes_form() {
        for b in [
            every_field_set(),
            BlobBuf::default(),
            crate::stream::new_blob().borrow().clone(),
        ] {
            let mut out = Vec::new();
            put(&b, &mut out);
            let mut r = Reader::new(&out);
            let back = read(&mut r).expect("reads back");
            assert!(r.is_empty(), "all bytes used");
            assert_eq!(b.differing_fields(&back), Vec::<&str>::new());
            assert_eq!(format!("{b:?}"), format!("{back:?}"));
        }
    }

    #[test]
    fn the_comparison_names_each_field_that_differs_and_no_value() {
        let a = every_field_set();
        assert_eq!(a.differing_fields(&a.clone()), Vec::<&str>::new());
        let mut b = a.clone();
        b.written_base = None;
        b.content_edits = 0;
        assert_eq!(a.differing_fields(&b), vec!["written_base", "content_edits"]);
        assert_ne!(a, b);
        let all = a.differing_fields(&crate::stream::new_blob().borrow());
        assert_eq!(
            all.len(),
            24,
            "every field of the set buffer is apart from a new one's: {all:?}"
        );
    }
}
