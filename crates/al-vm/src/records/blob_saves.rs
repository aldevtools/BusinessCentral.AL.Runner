//! What an Insert or a Modify stores from a record's Blobs, and the Blob bookkeeping after an Insert: the one owner of the rule
//! that a Blob the record never loaded is not written (issue 9286).
//!
//! A Blob whose stored content the record never loaded (a read without CalcFields, or a copy of such a record) shows HasValue
//! true and empty text in BC (streams/BlobCopyStates.al:1216 by value, :1332 Copy, :34 a temporary row;
//! record/PnABlobAssignRename.al:93 an assignment). The open corpus probe (pull request 1631,
//! record/lt-blob-field-assign/BlobInsertUnloaded.al, measured on BC 29.0, its CI green on BC 27.5, 28.5 and 29.0) measured
//! that Insert and Modify never write such a Blob. Until it is pinned the rule runs through the assumption
//! `blob-never-loaded-not-stored`, marked only where it changes what is stored or read.

use super::*;

impl Session {
    /// A Blob that holds stored content the record never loaded, and that no stream wrote since.
    fn never_loaded(v: &Value) -> bool {
        matches!(v, Value::Blob(b) if { let b = b.borrow(); !b.loaded && !b.written && !b.bytes.is_empty() })
    }

    /// Before an Insert stores `stored`, the row of `rec`: a Blob the record never loaded ([`Session::never_loaded`]) is not
    /// stored. A database row gets no Blob content (BlobInsertUnloaded.al codeunit 73781 C01 to C10: :=, the same variable
    /// under a new key, Delete and Insert, Copy, a by-value parameter, a Variant, a RecordRef, FindFirst, FindSet,
    /// SetLoadFields; codeunit 73782 I2 a temporary row copied into a database record, G1 another company's source and the
    /// reverse). A temporary row keeps the buffer, which answers HasValue Yes until CalcFields loads it empty (T12, T13; I1 a
    /// database row copied into a temporary record). The record keeps answering HasValue Yes and reading empty, also after its
    /// own CalcFields (I4): both are `BlobBuf::never_loaded_insert`, which `Session::load_blobs` reads. Then the Blobs are
    /// marked as an Insert's ([`Session::mark_blobs_inserted`]).
    pub(crate) fn blobs_to_insert(rec: &Rec, stored: &mut [Value]) {
        let positions: Vec<usize> = (rec.borrow().fields.iter().enumerate())
            .filter(|(_, v)| Self::never_loaded(v))
            .map(|(i, _)| i)
            .collect();
        let temporary = rec.borrow().temporary;
        for &i in &positions {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::BlobNeverLoadedNotStored);
            if !temporary {
                stored[i] = Value::Blob(al_builtins::stream::new_blob());
            }
        }
        Self::mark_blobs_inserted(rec, stored, true);
        for &i in &positions {
            if let Value::Blob(b) = &rec.borrow().fields[i] {
                let mut b = b.borrow_mut();
                b.inserted = false;
                b.never_loaded_insert = true;
            }
        }
    }

    /// Before a Modify of a database record stores `stored` over the row `old`: a Blob the record never loaded
    /// ([`Session::never_loaded`]) leaves the row's Blob as it is (BlobInsertUnloaded.al C11 a copy under the key of another
    /// row; codeunit 73782 M1 the key it was read under, M2 a stale buffer whose row another variable changed, M3 Insert then
    /// Modify of the same variable). The row as it was read holds the same bytes (streams/BlobCopyStates.al:1219 `bv get
    /// row`). A temporary record is not measured and stores as before. Then the Blobs are marked as not an Insert's.
    pub(crate) fn blobs_to_modify(rec: &Rec, stored: &mut [Value], old: Option<&[Value]>) {
        if let (Some(old), false) = (old, rec.borrow().temporary) {
            for (i, v) in rec.borrow().fields.iter().enumerate() {
                let (Value::Blob(b), Value::Blob(o)) = (v, &old[i]) else {
                    continue;
                };
                if Self::never_loaded(v) && b.borrow().bytes != o.borrow().bytes {
                    al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::BlobNeverLoadedNotStored);
                    stored[i] = Value::Blob(al_builtins::stream::copy_blob(o, true));
                }
            }
        }
        Self::mark_blobs_inserted(rec, stored, false);
    }

    /// After an Insert stored the row of `rec`: its Blobs count as saved ([`Session::clear_blob_written`]), the table notes an
    /// empty one ([`Session::note_empty_blob_insert`]), and the content a temporary record stored is persisted, as one a Modify
    /// stored (see Rename).
    pub(crate) fn blobs_after_insert(rec: &Rec, t: &TableDef, empty_blob: bool) {
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

    /// Marks the Blobs that hold content, in the stored row and in the record, as stored by an `Insert` (`inserted`) or not (a `Modify`).
    fn mark_blobs_inserted(rec: &Rec, stored: &[Value], inserted: bool) {
        for v in stored.iter().chain(rec.borrow().fields.iter()) {
            if let Value::Blob(b) = v {
                let mut b = b.borrow_mut();
                b.inserted = inserted && !b.bytes.is_empty();
            }
        }
    }
}
