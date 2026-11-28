//! What an Insert or a Modify stores from a record's Blobs, and the Blob bookkeeping after an Insert: the one owner of
//! the rule that a Blob the record never loaded is not written (issue 9286).
//!
//! A Blob whose stored content the record never loaded (a read without CalcFields, or a copy of such a record) shows
//! HasValue true and empty text in BC (streams/BlobCopyStates.al:1216 by value, :1332 Copy, :34 a temporary row;
//! record/PnABlobAssignRename.al:93 an assignment). The corpus probe of pull request 1631 (merged, not pinned yet:
//! record/lt-blob-field-assign/BlobInsertUnloaded.al, 31 tests, measured on a local Business Central and on every
//! supported major by the corpus CI) measured that Insert and Modify never write such a Blob. Until it is pinned the
//! rule runs through the assumption `blob-never-loaded-not-stored`, marked only where it changes what is stored or
//! read.

use super::*;

impl Session {
    /// A Blob that holds stored content the record never loaded, and that no stream wrote since.
    fn never_loaded(v: &Value) -> bool {
        matches!(v, Value::Blob(b) if { let b = b.borrow(); !b.loaded && !b.written && !b.bytes.is_empty() })
    }

    /// Before an Insert stores `stored`, the row of `rec`: a Blob the record never loaded ([`Session::never_loaded`])
    /// is not stored. A database row gets no Blob content (BlobInsertUnloaded.al codeunit 73781 C01 to C10: :=, the
    /// same variable under a new key, Delete and Insert, Copy, a by-value parameter, a Variant, a RecordRef, FindFirst,
    /// FindSet, SetLoadFields; codeunit 73782 I2 a temporary row copied into a database record, G1 another company's
    /// source and the reverse, R3 another company's source copied with := without switching back, which lands in the
    /// session's company; codeunit 73783 of BlobInsertUnloadedOrigin.al, corpus pull request 1654, O1 to O6 the reads
    /// FindLast, Find('-'), Find('='), Find('+'), Next, a RecordRef FindFirst with RecordRef.Insert, O6b, and a record
    /// RecordRef.SetTable filled, O7). The stored Blob
    /// carries `BlobBuf::never_loaded_insert`, so a later read of the row is flagged. A temporary row keeps the buffer,
    /// which answers HasValue Yes until CalcFields loads it empty (T12, T13; I1 a database row copied into a temporary
    /// record). Then the Blobs are marked as an Insert's ([`Session::mark_blobs_inserted`]). Answers the positions of
    /// those Blobs, which [`Session::blobs_after_insert`] marks once the Insert stored the row.
    pub(crate) fn blobs_to_insert(rec: &Rec, stored: &mut [Value]) -> Vec<usize> {
        if !rec.borrow().fields.iter().any(Self::never_loaded) {
            Self::mark_blobs_inserted(rec, stored, true);
            return Vec::new();
        }
        let positions: Vec<usize> = (rec.borrow().fields.iter().enumerate())
            .filter(|(_, v)| Self::never_loaded(v))
            .map(|(i, _)| i)
            .collect();
        let temporary = rec.borrow().temporary;
        for &i in &positions {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::BlobNeverLoadedNotStored);
            if !temporary {
                let empty = al_builtins::stream::new_blob();
                empty.borrow_mut().never_loaded_insert = true;
                stored[i] = Value::Blob(empty);
            }
        }
        Self::mark_blobs_inserted(rec, stored, true);
        for &i in &positions {
            if let Value::Blob(b) = &rec.borrow().fields[i] {
                b.borrow_mut().inserted = false;
            }
        }
        positions
    }

    /// The record's own Blob an Insert did not store (`never_loaded_insert`) keeps answering HasValue Yes and reading
    /// empty at its CalcFields (I4), also after a Modify or a second Insert (codeunit 73783 S1, S6), while the row's
    /// Blob is still empty: another variable's write reloads that Blob (S2, S2b), and after Delete (no row, S4) or Get
    /// (a new buffer, S5) CalcFields reads the row. A Commit since the Insert does not end it (S3), as measured under
    /// the Codeunit isolation of test runner 130450; under another isolation, or with no row version to tell,
    /// a Commit since the Insert is not measured and CalcFields reads the row as on main.
    pub(crate) fn unloaded_view_kept(&self, rec: &Rec, row_blob: Option<&Value>) -> bool {
        let codeunit_runner = self.in_test_codeunit && !self.isolation_from_hub && !self.hub_disabled;
        matches!(row_blob, Some(Value::Blob(b)) if b.borrow().bytes.is_empty())
            && (codeunit_runner || rec.borrow().row_origin.is_some_and(|o| o.commits == self.commits))
    }

    /// Before a Modify of a database record stores `stored` over the row `old`: a Blob the record never loaded
    /// ([`Session::never_loaded`]) leaves the row's Blob as it is (BlobInsertUnloaded.al C11 a copy under the key of
    /// another row; codeunit 73782 M1 the key it was read under, M2 a stale buffer whose row another variable changed,
    /// M3 Insert then Modify of the same variable, R1 a row another variable cleared; codeunit 73783 M4 Modify(true),
    /// M5 RecordRef.Modify, M6 a by-value copy modified in the callee). The row as it was read holds the same bytes
    /// (streams/BlobCopyStates.al:1219 `bv get row`). A temporary record, and one that called ChangeCompany (M7), are
    /// not served and store as before. Then the Blobs are marked as not an Insert's.
    pub(crate) fn blobs_to_modify(rec: &Rec, stored: &mut [Value], old: Option<&[Value]>) {
        // Not served: a record that called ChangeCompany (M7, BC raises DB:RecordChanged, the runner modifies the row).
        let measured = !rec.borrow().temporary && !rec.borrow().change_company_called;
        if let (Some(old), true) = (old, measured) {
            for (i, v) in rec.borrow().fields.iter().enumerate() {
                let (Value::Blob(b), Value::Blob(o)) = (v, &old[i]) else {
                    continue;
                };
                if Self::never_loaded(v) && b.borrow().bytes != o.borrow().bytes {
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::BlobNeverLoadedNotStored);
                    let kept = al_builtins::stream::copy_blob(o, true);
                    // A row the assumption stored with no Blob keeps the mark that flags its reads.
                    kept.borrow_mut().never_loaded_insert = o.borrow().never_loaded_insert;
                    stored[i] = Value::Blob(kept);
                }
            }
        }
        Self::mark_blobs_inserted(rec, stored, false);
    }

    /// After an Insert stored the row of `rec`: its Blobs count as saved ([`Session::clear_blob_written`]), the table
    /// notes an empty one ([`Session::note_empty_blob_insert`]), and the content a temporary record stored is
    /// persisted, as one a Modify stored (see Rename). The Blobs at `never_loaded` ([`Session::blobs_to_insert`]) get
    /// `never_loaded_insert` only here: a failed Insert leaves no mark (codeunit 73783 F1 to F4).
    pub(crate) fn blobs_after_insert(rec: &Rec, t: &TableDef, empty_blob: bool, never_loaded: &[usize]) {
        for &i in never_loaded {
            if let Value::Blob(b) = &rec.borrow().fields[i] {
                b.borrow_mut().never_loaded_insert = true;
            }
        }
        Self::note_empty_blob_insert(rec, t, empty_blob);
        Self::clear_blob_written(rec);
        if rec.borrow().temp_id.is_some() {
            for v in &rec.borrow().fields {
                if let Value::Blob(b) = v {
                    b.borrow_mut().persisted = true;
                }
            }
        }
    }

    /// The Blob a temporary record holds after its Rename, for the Blob `old` it held before the Rename: `empty`, the
    /// field's default, as before; but a Blob a Get handed out unloaded (`BlobBuf::temp_row`) and that no stream wrote
    /// since stays an unloaded Blob with its bytes, marked `never_loaded_insert`: after a new key and Insert, or after
    /// a Modify, the row answers HasValue Yes before CalcFields and No and empty after it, and the record reads empty
    /// (codeunit 73783 T1, T2 of BlobInsertUnloadedOrigin.al, corpus pull request 1654).
    pub(crate) fn blob_after_temp_rename(old: &Value, empty: Value) -> Value {
        match old {
            Value::Blob(b) if Self::never_loaded(old) && b.borrow().temp_row => {
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::BlobNeverLoadedNotStored);
                let kept = al_builtins::stream::copy_blob_temp_row(b);
                kept.borrow_mut().never_loaded_insert = true;
                Value::Blob(kept)
            }
            _ => empty,
        }
    }

    /// Marks the Blobs that hold content, in the stored row and in the record, as stored by an `Insert` (`inserted`) or
    /// not (a `Modify`).
    fn mark_blobs_inserted(rec: &Rec, stored: &[Value], inserted: bool) {
        for v in stored.iter().chain(rec.borrow().fields.iter()) {
            if let Value::Blob(b) = v {
                let mut b = b.borrow_mut();
                b.inserted = inserted && !b.bytes.is_empty();
            }
        }
    }
}
