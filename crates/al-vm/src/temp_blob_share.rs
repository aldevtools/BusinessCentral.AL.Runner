//! The Blob buffer a temporary row shares with the record variable that inserted it while the Blob was empty
//! (`records/op_inner.rs`, Insert: `BlobBuf::temp_inserted` stays false, the row holds the variable's own buffer). A write
//! after the Insert under the same key reaches the row (streams/TestBlobUncommittedWriteIsolation.al:171). The one owner of
//! what a write through the variable does to such a row once the variable's key has moved on: `Blob.CreateOutStream`,
//! `Blob.Import` and `Clear` of the field are checked before the builtin runs ([`Session::temp_blob_share_guard`]).

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::{BuiltinId, ids};
use al_builtins::native::Signal;
use al_builtins::stream::{BlobCell, BlobRef};
use al_builtins::value::{RtType, Value};
use al_ir as ir;
use std::sync::Arc;

use crate::{Frame, Session};

/// Where the rows that share a variable's Blob buffer are, against the variable's current primary key.
#[derive(Debug, PartialEq, Eq)]
enum Holders {
    /// No stored row holds the buffer, or every row that holds it has the variable's key: main's path (the pinned same-key
    /// write, streams/TestBlobUncommittedWriteIsolation.al:171; TempBlobEmptyInsertShare.al:241, :257, :502 in corpus PR 1391).
    NoneElsewhere,
    /// Rows hold it, and none of them has the variable's key.
    OnlyElsewhere,
    /// Rows hold it under the variable's key and under another key.
    Mixed,
}

impl Session {
    /// Before `Blob.CreateOutStream`, `Blob.Import` or `Clear` runs on a Blob field of a temporary record variable. A buffer
    /// that stored rows share under another key than the variable's:
    /// - `CreateOutStream` on it while it is empty gives the variable a buffer of its own first, so the write does not reach
    ///   those rows (assumption `temp-blob-empty-insert-key-change`; [`measured_detach`] lists the facts);
    /// - every other write into it refuses: the write would reach rows under another key, which BC was not measured on.
    ///
    /// A `Clear` of an empty buffer changes nothing those rows show, and keeps main's path.
    pub(crate) fn temp_blob_share_guard(
        &mut self,
        id: BuiltinId,
        receiver: Option<&ir::Arg>,
        args: &[ir::Arg],
        recv: &mut Option<Value>,
        values: &[Value],
        f: &mut Frame,
    ) -> Result<(), Signal> {
        let (place, b) = match id {
            ids::BLOB_CREATEOUTSTREAM_1 | ids::BLOB_IMPORT_1 => match (receiver, recv.as_ref()) {
                (Some(ir::Arg::Ref(p)), Some(Value::Blob(b))) => (p, b),
                _ => return Ok(()),
            },
            ids::SYSTEM_CLEAR_1 => match (args.first(), values.first()) {
                (Some(ir::Arg::Ref(p)), Some(Value::Blob(b))) => (p, b),
                _ => return Ok(()),
            },
            _ => return Ok(()),
        };
        // A buffer only the variable's field and this operand hold has no row to share it with.
        if Arc::strong_count(b) <= 2 {
            return Ok(());
        }
        let b = b.clone();
        let ir::Place::Field(base, pos) = place else {
            return Ok(());
        };
        let Value::Record(rc) = self.read(base, f)? else {
            return Ok(());
        };
        if !rc.borrow().temporary || rc.borrow().temp_id.is_none() {
            return Ok(());
        }
        let clears_in_place = id == ids::SYSTEM_CLEAR_1 && al_builtins::native::blob_clears_in_place(&*self, &b);
        // A buffer an Insert stored while it held content is copied by CreateOutStream and replaced by Clear
        // (`BlobBuf::temp_inserted`); a Clear that gives a new buffer leaves the rows alone.
        if b.borrow().temp_inserted || (id == ids::SYSTEM_CLEAR_1 && !clears_in_place) {
            return Ok(());
        }
        if clears_in_place && b.borrow().bytes.is_empty() {
            return Ok(());
        }
        let t = self.table(&rc)?;
        let cx = self.cx_for(&rc, &t)?;
        let keys = self
            .store_for(&rc)
            .keys_where(&cx, &t, *pos, |v| matches!(v, Value::Blob(o) if Arc::ptr_eq(o, &b)));
        let current = t.primary_key(&rc.borrow().fields);
        let same = |k: &[Value]| {
            k.iter()
                .zip(&current)
                .enumerate()
                .all(|(i, (a, c))| t.cmp_key_field(t.keys[0][i], a, c).is_eq())
        };
        let at_key = keys.iter().filter(|k| same(k)).count();
        let holders = match (at_key, keys.len() - at_key) {
            (_, 0) => Holders::NoneElsewhere,
            (0, _) => Holders::OnlyElsewhere,
            _ => Holders::Mixed,
        };
        if holders == Holders::NoneElsewhere {
            return Ok(());
        }
        let integer_key = t.keys[0].iter().all(|&i| matches!(t.fields[i].ty, RtType::Integer));
        if id == ids::BLOB_CREATEOUTSTREAM_1
            && holders == Holders::OnlyElsewhere
            && measured_detach(&b, integer_key, self.xrec_holds_blob(&b))
        {
            assume(Assumption::TempBlobEmptyInsertKeyChange);
            let mut copy = b.borrow().clone();
            copy.inserted = false;
            *recv = Some(Value::Blob(Arc::new(BlobCell::new(copy))));
            return Ok(());
        }
        Err(Signal::unsupported(format!(
            "a write into the Blob {} of a temporary record of {} whose buffer a row inserted under another primary key still holds, in a form the corpus does not pin",
            t.fields[*pos].name, t.name
        )))
    }
}

/// The facts of the cells of corpus PR 1391 (tests/al-language/streams/TempBlobEmptyInsertShare.al, assumption
/// `temp-blob-empty-insert-key-change`), each required by equality:
/// - the buffer is empty: every cell writes after an empty Insert, with Clear (:142, :167, :206, :283, :366, :385, :426, :447,
///   :465) or without (:188, :226, :406, :484), and nothing was written into it since;
/// - the stored rows that hold it are each under another key than the variable's (one row in each cell, two in :283);
/// - the primary key is made of Integer fields (tables 75230 and 75231; Excel Buffer's Row No. and Column No. in :527);
/// - no live xRec holds the buffer (`xrec`).
///
/// Not told apart: a Variant or another value that holds the same buffer (it keeps the rows' buffer).
fn measured_detach(b: &BlobRef, integer_key: bool, xrec: bool) -> bool {
    b.borrow().bytes.is_empty() && integer_key && !xrec
}
