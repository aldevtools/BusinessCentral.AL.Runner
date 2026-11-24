//! The digest of committed rows: the one place the login key hashes rows (al-runner issue 8211).
//!
//! Each row's `value_codec::encode_row` bytes (al-builtins, the one owner of "a row as bytes": the install baseline, the
//! login image's rows and the frozen store use the same form) go straight into SHA-256 through one reused buffer: no
//! String and no Debug text per row (on a 50,000-row store rendering and hashing the rows' Debug text was most of
//! the key's cost). The form is exact: each value is self-delimiting and `value_codec::decode` reads it back
//! (value_codec.rs, a Blob with every field of its buffer, `blob_form.rs`), so equal digests mean equal rows. A value
//! that has no byte form (`value_codec::encode` answers false: a SqlCount, a placeholder's size, a handle) gives
//! [`NoKey`], never a lossy digest, and the session runs its login.
//!
//! One gap the form shares with the earlier Debug text: a Text with half of a surrogate pair is written with U+FFFD at the
//! lone position (value_codec.rs, `Txt::as_str`). No stored row can hold one (every write refuses it,
//! al-data `memory/held_cols.rs` `note_held_in`), so it does not reach this
//! digest; the exact form of such a text in the codec is a follow-up.

use super::LinesDigest;
#[cfg(test)]
use super::digest;
use sha2::{Digest, Sha256};

/// Why a session has no login key: a row holds a value with no byte form, named by its kind (al-data
/// `memory::not_shareable`). The session then runs its login, counted in the starts line (`login_start::NotTaken::NoKey`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NoKey(pub(crate) &'static str);

/// The first bytes of every digest: a digest of this form never equals one of another form (the earlier key hashed
/// Debug lines).
const FORM: &[u8] = b"al-runner login key rows: value_codec v1\0";

/// How many bytes the buffer gathers before it goes into the hasher.
const FLUSH: usize = 64 << 10;

/// The rows of one or more tables going into one SHA-256: the form tag first, then per table its company, object and rows.
struct RowsHasher {
    hash: Sha256,
    buf: Vec<u8>,
}

impl RowsHasher {
    fn new() -> RowsHasher {
        let mut hash = Sha256::new();
        hash.update(FORM);
        RowsHasher {
            hash,
            buf: Vec::with_capacity(FLUSH + 4096),
        }
    }

    /// Table `object` of `company` and its rows in key order, then their count.
    fn table<'a>(
        &mut self,
        company: &str,
        object: u32,
        rows: impl Iterator<Item = &'a al_data::Row>,
    ) -> Result<(), NoKey> {
        use al_builtins::value_codec::{encode_row, put_str, put_u32, put_u64};
        put_str(&mut self.buf, company);
        put_u32(&mut self.buf, object);
        let mut n: u64 = 0;
        for row in rows {
            if !encode_row(row, &mut self.buf) {
                return Err(no_key(row));
            }
            #[cfg(test)]
            counts::ENCODED_ROWS.with(|c| c.set(c.get() + 1));
            n += 1;
            if self.buf.len() >= FLUSH {
                self.hash.update(&self.buf);
                self.buf.clear();
            }
        }
        put_u64(&mut self.buf, n);
        Ok(())
    }

    fn finish(mut self) -> LinesDigest {
        self.hash.update(&self.buf);
        LinesDigest(self.hash.finalize().into())
    }
}

/// The kind of the first value of `row` that has no byte form.
fn no_key(row: &al_data::Row) -> NoKey {
    let first = row
        .iter()
        .find(|v| !al_builtins::value_codec::encode(v, &mut Vec::new()));
    NoKey(
        first
            .and_then(al_data::memory::not_shareable)
            .unwrap_or("a value with no byte form"),
    )
}

/// The digest of the committed rows `rows` of table `object` in `company`, in key order (the per-table digest of the key's
/// `database` part, `key_parts.rs`). The form tag is hashed for an empty table too.
pub(crate) fn table_digest<'a>(
    company: &str,
    object: u32,
    rows: impl Iterator<Item = &'a al_data::Row>,
) -> Result<LinesDigest, NoKey> {
    let mut h = RowsHasher::new();
    h.table(company, object, rows)?;
    Ok(h.finish())
}

/// The digest of every row `store` holds now, the uncommitted changes included (the key's `uncommitted` part), table after
/// table in the store's order; a table entry without rows is left out.
pub(crate) fn current_digest(store: &al_data::memory::MemStore) -> Result<LinesDigest, NoKey> {
    let mut h = RowsHasher::new();
    // A table entry without rows wrote no Debug line, so it is not hashed here either (as `store_table_digests`).
    for t in store.export_current().iter().filter(|t| !t.rows.is_empty()) {
        h.table(&t.company, t.object, t.rows.iter().map(|(_, row)| row))?;
    }
    Ok(h.finish())
}

/// How many rows the digests of this thread wrote as bytes, for tests: every row a digest reads.
#[cfg(test)]
pub(crate) mod counts {
    use std::cell::Cell;

    thread_local! {
        pub(super) static ENCODED_ROWS: Cell<u64> = const { Cell::new(0) };
    }

    /// The rows written as bytes on this thread since the last [`reset`].
    pub(crate) fn rows() -> u64 {
        ENCODED_ROWS.with(Cell::get)
    }

    /// The count back to 0.
    pub(crate) fn reset() {
        ENCODED_ROWS.with(|c| c.set(0));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use al_builtins::value::Value;

    // A guard that holds before and after stage 1: one content gives one digest, and each row is written once, in one of
    // the two forms.
    #[test]
    fn one_content_gives_one_digest_and_writes_each_row_once() {
        let rows = |n: i32| -> Vec<al_data::Row> {
            (0..n)
                .map(|i| vec![Value::Integer(i), Value::Text("a".into())])
                .collect()
        };
        counts::reset();
        let a = table_digest("C", 7, rows(3).iter());
        let b = table_digest("C", 7, rows(3).iter());
        assert_eq!(a, b);
        assert_ne!(a, table_digest("C", 7, rows(2).iter()));
        assert_eq!(counts::rows(), 8);
    }

    fn rows(n: i32) -> Vec<al_data::Row> {
        (0..n)
            .map(|i| vec![Value::Integer(i), Value::Text("a".into())])
            .collect()
    }

    /// The earlier digest of the same rows: their Debug lines.
    fn debug_lines_digest(company: &str, object: u32, rows: &[al_data::Row]) -> LinesDigest {
        let lines: Vec<String> = rows.iter().map(|r| format!("{company}|{object}|{r:?}")).collect();
        digest(&lines)
    }

    fn blob_row(tweak: impl Fn(&mut al_builtins::stream::BlobBuf)) -> al_data::Row {
        let b = al_builtins::stream::new_blob();
        al_builtins::stream::write_bytes(&b, b"xy");
        tweak(&mut b.borrow_mut());
        vec![Value::Integer(1), Value::Blob(b)]
    }

    // A value with no byte form gives no key, never a lossy digest.
    #[test]
    fn a_value_with_no_byte_form_gives_no_key() {
        use al_builtins::value::{SqlCount, SqlForm, SqlType};
        let count = Value::SqlCount(SqlCount {
            n: 3,
            form: SqlForm::Delta,
            ty: SqlType::BigInteger,
        });
        let size = Value::PlaceholderSize("mark".into(), SqlType::Integer);
        for (what, v) in [("SqlCount", count), ("PlaceholderSize", size)] {
            let table = vec![vec![Value::Integer(1)], vec![Value::Integer(2), v.clone()]];
            let got = table_digest("C", 7, table.iter());
            assert!(
                matches!(got, Err(NoKey(kind)) if !kind.is_empty()),
                "{what}: table {got:?}"
            );
            let store = al_data::memory::MemStore::from_exported(vec![exported(table)]);
            let got = current_digest(&store);
            assert!(
                matches!(got, Err(NoKey(kind)) if !kind.is_empty()),
                "{what}: current {got:?}"
            );
        }
    }

    fn exported(rows: Vec<al_data::Row>) -> al_data::memory::ExportedTable {
        al_data::memory::ExportedTable {
            company: "C".into(),
            object: 7,
            pk_storage: Default::default(),
            rows: (rows.into_iter().enumerate())
                .map(|(i, r)| (vec![Value::Integer(i as i32)], r))
                .collect(),
        }
    }

    // A guard (the Debug text rendered every BlobBuf field too): a field of a Blob that differs is another digest.
    #[test]
    fn a_blob_field_difference_gives_another_digest() {
        let digest_of = |r: al_data::Row| table_digest("C", 7, [r].iter()).expect("a digest");
        let base = digest_of(blob_row(|_| {}));
        assert_eq!(base, digest_of(blob_row(|_| {})), "GUARD equal blobs");
        assert_ne!(
            base,
            digest_of(blob_row(|b| b.utf16_binary = true)),
            "GUARD utf16_binary"
        );
        assert_ne!(
            base,
            digest_of(blob_row(|b| b.render_pending = Some("m".into()))),
            "GUARD render_pending"
        );
        assert_ne!(base, digest_of(blob_row(|b| b.bytes = b"xz".to_vec())), "GUARD bytes");
    }

    // A guard: one content built twice, from two stores (other stamps, other pointers), gives one digest.
    #[test]
    fn one_content_built_twice_from_two_stores_gives_one_digest() {
        let a = al_data::memory::MemStore::from_exported(vec![exported(rows(4))]);
        let b = al_data::memory::MemStore::from_exported(vec![exported(rows(4))]);
        assert_eq!(current_digest(&a), current_digest(&b), "GUARD");
        assert!(current_digest(&a).is_ok(), "GUARD");
    }

    // Every row of a table digest is written as bytes.
    #[test]
    fn every_row_of_a_table_digest_is_encoded() {
        counts::reset();
        let r = rows(5);
        assert!(table_digest("C", 7, r.iter()).is_ok());
        assert_eq!(counts::rows(), 5, "table_digest");
    }

    // The key's uncommitted part: the same for `current_digest`.
    #[test]
    fn every_row_of_the_current_digest_is_encoded() {
        counts::reset();
        let store = al_data::memory::MemStore::from_exported(vec![exported(rows(6))]);
        assert!(current_digest(&store).is_ok());
        assert_eq!(counts::rows(), 6, "current_digest");
    }

    // Issue 8211: the key's uncommitted part sees what the Debug lines saw. A table entry without rows wrote no line, so
    // two stores that differ only in such an entry gave one key, and still do; the committed part skips it too
    // (`key_parts::store_table_digests`).
    #[test]
    fn a_table_entry_without_rows_leaves_the_uncommitted_digest_as_it_is() {
        let rowless = al_data::memory::ExportedTable {
            object: 9,
            rows: Vec::new(),
            ..exported(Vec::new())
        };
        let a = al_data::memory::MemStore::from_exported(vec![exported(rows(2))]);
        let b = al_data::memory::MemStore::from_exported(vec![exported(rows(2)), rowless]);
        assert_eq!(b.export_current().len(), 2, "GUARD the store exports the rowless entry");
        assert_eq!(current_digest(&a), current_digest(&b));
    }

    // The digest of the byte form is never the Debug lines' digest of the same rows (a version tag), so a start
    // recorded by another binary can never meet a new key.
    #[test]
    fn the_byte_form_digest_never_equals_the_debug_lines_digest() {
        for n in [1, 3] {
            let r = rows(n);
            assert_ne!(
                table_digest("C", 7, r.iter()).unwrap(),
                debug_lines_digest("C", 7, &r),
                "{n} rows"
            );
        }
    }
}
