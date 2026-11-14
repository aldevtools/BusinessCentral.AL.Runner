//! The Blob buffer a temporary row shares with the record variable that inserted it while the Blob was empty
//! (`records/op_inner.rs`, Insert: `BlobBuf::temp_inserted` stays false, the row holds the variable's own buffer). A write
//! after the Insert under the same key reaches the row (streams/TestBlobUncommittedWriteIsolation.al:171). The one owner of
//! what a write through the variable does to such a row once the variable's key has moved on, checked before the builtin
//! runs ([`Session::temp_blob_share_guard`]):
//! - `Blob.CreateOutStream`, `Blob.Import` and `Clear` of the field, against the record's key at that call;
//! - every later call that takes an OutStream opened on such a field, against what the stream kept from its open
//!   ([`al_builtins::stream::OutOpen`]).

use al_builtins::assumptions::{Assumption, assume};
use al_builtins::builtin_id::{BuiltinId, ids};
use al_builtins::library_kind::LibraryKind;
use al_builtins::native::Signal;
use al_builtins::stream::{BlobCell, BlobRef, OutOpen};
use al_builtins::value::{LibraryValue, RecordState, RtType, TempId, Value};
use al_data::TableDef;
use al_ir as ir;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use crate::exec::BuiltinSite;
use crate::{Frame, Session};

/// The stream `Blob.CreateOutStream` is about to open on a Blob field of a temporary record, and what it keeps from the open
/// once the builtin has run ([`Session::temp_blob_attach_open`]).
pub(crate) struct PendingOpen {
    stream: Rc<RefCell<LibraryValue>>,
    open: OutOpen,
}

/// Who holds a Blob buffer, against the record's current primary key, in the record's own table.
struct Holders {
    /// The row under the record's key holds it.
    at_key: bool,
    /// Rows under other keys that hold it, counted until every holder is accounted for.
    elsewhere: usize,
    /// Every holder is accounted for: the values the caller counted, the rows of the record's own table and one for each other
    /// record variable that shares the table. A holder outside them (a by-value copy, a Variant, a stream) leaves it false.
    all_found: bool,
    /// How many holders those are.
    found: usize,
}

impl Session {
    /// Before a builtin runs. Two checks:
    /// - a call that takes an OutStream opened on a Blob field of a temporary record ([`Session::check_temp_out_stream`]);
    /// - `Blob.CreateOutStream`, `Blob.Import` or `Clear` on a Blob field of a temporary record. A buffer that stored rows share
    ///   under another key than the variable's: `CreateOutStream` on it in the measured shape ([`measured_detach`]) gives the
    ///   variable a buffer of its own first, so the write does not reach those rows (assumption
    ///   `temp-blob-empty-insert-key-change`); every other write into it refuses. A record handed by value whose buffer a row of the
    ///   caller's table, or a holder found nowhere, shares refuses too ([`Session::by_value_holders_found`]): the write would
    ///   reach the caller's rows, which BC was not measured on.
    ///
    /// A `Clear` of an empty buffer changes nothing those rows show, and keeps main's path. `Ok(Some)`: the open of a stream
    /// on a temporary record's Blob field, for [`Session::temp_blob_attach_open`] once the builtin has run.
    pub(crate) fn temp_blob_share_guard(
        &mut self,
        site: &BuiltinSite,
        nl: u16,
        recv: &mut Option<Value>,
        values: &[Value],
        f: &mut Frame,
    ) -> Result<Option<PendingOpen>, Signal> {
        let id = site.id;
        self.check_temp_out_streams(id, recv.as_ref(), values)?;
        // The operand, and the register it came from when that register is a variable's slot, which keeps a copy (`take`).
        let (place, b, operand) = match id {
            ids::BLOB_CREATEOUTSTREAM_1 | ids::BLOB_IMPORT_1 => match (&site.receiver_arg, recv.as_ref()) {
                (Some(ir::Arg::Ref(p)), Some(Value::Blob(b))) => {
                    (p, b, 1 + usize::from(site.receiver.is_some_and(|r| r < nl)))
                }
                _ => return Ok(None),
            },
            ids::SYSTEM_CLEAR_1 => match (site.args.first(), values.first()) {
                (Some(ir::Arg::Ref(p)), Some(Value::Blob(b))) => {
                    (p, b, 1 + usize::from(site.arg_regs.first().is_some_and(|&r| r < nl)))
                }
                _ => return Ok(None),
            },
            _ => return Ok(None),
        };
        let opens = id == ids::BLOB_CREATEOUTSTREAM_1;
        // A buffer only the variable's field and this operand hold has no row to share it with.
        if !opens && Arc::strong_count(b) <= 1 + operand {
            return Ok(None);
        }
        let b = b.clone();
        // A buffer an Insert stored while it held content is copied before a write and replaced by Clear
        // (`BlobBuf::temp_inserted`); a Clear that gives a new buffer leaves the rows alone, and a Clear of an empty one changes
        // nothing they show.
        if !opens
            && (b.borrow().temp_inserted
                || (id == ids::SYSTEM_CLEAR_1
                    && (!al_builtins::native::blob_clears_in_place(&*self, &b) || b.borrow().bytes.is_empty())))
        {
            return Ok(None);
        }
        let ir::Place::Field(base, pos) = place else {
            return Ok(None);
        };
        let Value::Record(rc) = self.read(base, f)? else {
            return Ok(None);
        };
        if !rc.borrow().temporary || rc.borrow().temp_id.is_none() {
            return Ok(None);
        }
        let t = self.table(&rc)?;
        let mut pending = match (opens, values.first()) {
            (true, Some(Value::Library(_, h))) => Some(PendingOpen {
                stream: h.clone(),
                open: OutOpen {
                    record: Rc::downgrade(&rc),
                    field: *pos,
                    key: t.primary_key(&rc.borrow().fields),
                    detached: false,
                },
            }),
            _ => None,
        };
        // CreateOutStream copies a buffer an Insert stored while it held content; the stream keeps what it saw at the open.
        if b.borrow().temp_inserted {
            return Ok(pending);
        }
        // The record's own copies, the operand and `b`.
        let counted = operand + 1 + rc.borrow().blob_copies_of(*pos, &b);
        if Arc::strong_count(&b) <= counted {
            return Ok(pending);
        }
        let h = self.blob_holders(&rc, &t, *pos, &b, counted)?;
        if h.elsewhere == 0 {
            if !self.by_value_holders_found(&rc, *pos, &b, &h)? {
                return Err(refusal(&t, *pos));
            }
            return Ok(pending);
        }
        if let Some(p) = pending
            .as_mut()
            .filter(|_| !h.at_key && measured_detach(&b, &t, *pos, base, h.all_found, self.xrec_holds_blob(&b)))
        {
            assume(Assumption::TempBlobEmptyInsertKeyChange);
            let mut copy = b.borrow().clone();
            copy.inserted = false;
            *recv = Some(Value::Blob(Arc::new(BlobCell::new(copy))));
            p.open.detached = true;
            return Ok(pending);
        }
        Err(refusal(&t, *pos))
    }

    /// After `Blob.CreateOutStream` ran: the stream keeps what [`Session::temp_blob_share_guard`] saw at its open.
    pub(crate) fn temp_blob_attach_open(&mut self, p: PendingOpen) {
        if let LibraryValue::OutStream(_, _, kept) = &mut *p.stream.borrow_mut() {
            *kept = Some(Rc::new(p.open));
        }
    }

    /// Every OutStream a builtin takes that was opened on a Blob field of a temporary record ([`Session::check_temp_out_stream`]).
    /// A call that opens a stream into the variable or clears it writes nothing through the old one.
    fn check_temp_out_streams(&mut self, id: BuiltinId, recv: Option<&Value>, values: &[Value]) -> Result<(), Signal> {
        if matches!(
            id,
            ids::BLOB_CREATEOUTSTREAM_1 | ids::FILE_CREATEOUTSTREAM_1 | ids::SYSTEM_CLEAR_1
        ) {
            return Ok(());
        }
        let measured_write = measured_detached_write(id, values);
        for (i, v) in recv.into_iter().chain(values).enumerate() {
            let Value::Library(LibraryKind::OutStream, h) = v else {
                continue;
            };
            let (b, open) = match &*h.borrow() {
                LibraryValue::OutStream(b, _, Some(open)) => (b.clone(), open.clone()),
                _ => continue,
            };
            self.check_temp_out_stream(measured_write && recv.is_some() && i == 0, &b, &open)?;
        }
        Ok(())
    }

    /// A call that takes an OutStream opened on Blob field `open.field` of a temporary record, writing to buffer `b`. Main's
    /// path runs while no value other than the record's field and the stream holds the buffer, or while the record is still on
    /// the key it had at the open and no row under another key holds the buffer (the pinned same-key write,
    /// streams/TestBlobUncommittedWriteIsolation.al:171). A stream the assumed detach opened takes only the measured writes, on
    /// it as the receiver (`measured_write`, [`measured_detached_write`]). Everything else refuses: the record moved to another
    /// key, or a row under another key holds the buffer, since the open.
    fn check_temp_out_stream(&mut self, measured_write: bool, b: &BlobRef, open: &OutOpen) -> Result<(), Signal> {
        let Some(rc) = open.record.upgrade() else {
            // `b` and the stream.
            if Arc::strong_count(b) <= 2 {
                return Ok(());
            }
            return Err(Signal::unsupported(
                "a write through an OutStream on a Blob of a temporary record that is gone while a row or another value still holds the buffer, in a form the corpus does not pin",
            ));
        };
        if open.detached && measured_write {
            return Ok(());
        }
        let t = self.table(&rc)?;
        if open.detached {
            return Err(refusal(&t, open.field));
        }
        // `b`, the stream and the field.
        let counted = 2 + rc.borrow().blob_copies_of(open.field, b);
        if Arc::strong_count(b) <= counted {
            return Ok(());
        }
        let h = self.blob_holders(&rc, &t, open.field, b, counted)?;
        let same_key = {
            let now = t.primary_key(&rc.borrow().fields);
            now.len() == open.key.len()
                && now
                    .iter()
                    .zip(&open.key)
                    .enumerate()
                    .all(|(i, (a, k))| t.cmp_key_field(t.keys[0][i], a, k).is_eq())
        };
        let rows = h.at_key || h.elsewhere > 0;
        if (!rows || (same_key && h.elsewhere == 0)) && self.by_value_holders_found(&rc, open.field, b, &h)? {
            return Ok(());
        }
        Err(refusal(&t, open.field))
    }

    /// Who holds buffer `b` of Blob field `pos` of temporary record `rc`, besides the `counted` holders the caller knows: the
    /// row under the record's key (one lookup), then the rows under other keys from the last one back, until the first one and
    /// until every holder is accounted for. Each other record variable that shares the table (`Copy(R, true)`) may hold the
    /// buffer once (TempBlobEmptyInsertShare.al:426, :447: R2 shares the table before the Insert).
    fn blob_holders(
        &mut self,
        rc: &Rc<RefCell<RecordState>>,
        t: &TableDef,
        pos: usize,
        b: &BlobRef,
        counted: usize,
    ) -> Result<Holders, Signal> {
        let cx = self.cx_for(rc, t)?;
        let key = t.primary_key(&rc.borrow().fields);
        let sharers = rc.borrow().temp_id.as_ref().map_or(1, TempId::sharers);
        let total = Arc::strong_count(b);
        let is_b = |v: &Value| matches!(v, Value::Blob(o) if Arc::ptr_eq(o, b));
        let store = self.store_for(rc);
        let (at_key, elsewhere) = if sharers == 1 {
            // Every other holder is a row: one walk that stops once they are all found tells the row under the key from
            // the others (an unvisited row under the key cannot hold the buffer then).
            store.field_holders_from_last(&cx, t, &key, pos, total.saturating_sub(counted), is_b)
        } else {
            // A sharer may or may not hold the buffer, so the count does not prove an unvisited row under the key empty.
            let at_key = store.row_field_holds(&cx, t, &key, pos, is_b);
            let found = counted + usize::from(at_key);
            let enough = total
                .saturating_sub(found + sharers - 1)
                .max(usize::from(total > found));
            (
                at_key,
                store.count_field_holders_from_last(&cx, t, &key, pos, enough, is_b),
            )
        };
        let found = counted + usize::from(at_key) + elsewhere + sharers - 1;
        Ok(Holders {
            at_key,
            elsewhere,
            all_found: found >= total,
            found,
        })
    }

    /// For a record handed by value (`TempId::is_by_value_copy`) whose own table left holders `h` of buffer `b` unaccounted
    /// for: whether the record it was copied from accounts for the rest (its fields and their snapshots, and the rows and other
    /// variables of its table), and, while holders stay unaccounted for and that record was handed by value too, the record
    /// it was copied from in turn (an event argument handed on to a subscriber), with no row of those tables holding the
    /// buffer. Then no row any write reaches holds the buffer, and main's path runs. A source that is gone, or a holder still
    /// unaccounted for, answers false. Any other record: true.
    fn by_value_holders_found(
        &mut self,
        rc: &Rc<RefCell<RecordState>>,
        pos: usize,
        b: &BlobRef,
        h: &Holders,
    ) -> Result<bool, Signal> {
        if h.all_found || !rc.borrow().temp_id.as_ref().is_some_and(TempId::is_by_value_copy) {
            return Ok(true);
        }
        let mut found = h.found;
        let mut copy = rc.clone();
        loop {
            let Some(src) = copy.borrow().temp_id.as_ref().and_then(TempId::by_value_source) else {
                return Ok(false);
            };
            found += src.borrow().blob_copies_of(pos, b);
            if Arc::strong_count(b) <= found {
                return Ok(true);
            }
            if !src.borrow().temporary || src.borrow().temp_id.is_none() {
                return Ok(false);
            }
            let t = self.table(&src)?;
            let hs = self.blob_holders(&src, &t, pos, b, found)?;
            if hs.at_key || hs.elsewhere > 0 {
                return Ok(false);
            }
            if hs.all_found {
                return Ok(true);
            }
            found = hs.found;
            copy = src;
        }
    }
}

/// The writes the cells of corpus PR 1391 make through a stream the assumed detach opened (TempBlobEmptyInsertShare.al,
/// assumption `temp-blob-empty-insert-key-change`): `WriteText` (`Lib.Write`, :66-72), and `Write` of one Text argument
/// (Excel Buffer's ParseCellValue, the cell at :527). A Text variable resolves to the first overload (`Value: Any`), so the
/// argument's value says it is a Text; a text literal or Label arrives as the same value and is not told apart.
fn measured_detached_write(id: BuiltinId, values: &[Value]) -> bool {
    id == ids::OUTSTREAM_WRITETEXT_1 || (id == ids::OUTSTREAM_WRITE_1 && matches!(values, [Value::Text(_)]))
}

fn refusal(t: &TableDef, pos: usize) -> Signal {
    Signal::unsupported(format!(
        "a write into the Blob {} of a temporary record of {} whose buffer a row inserted under another primary key or another value still holds, in a form the corpus does not pin",
        t.fields[pos].name, t.name
    ))
}

/// The facts of the cells of corpus PR 1391 (tests/al-language/streams/TempBlobEmptyInsertShare.al, assumption
/// `temp-blob-empty-insert-key-change`), each required by equality:
/// - the buffer is empty: every cell writes after an empty Insert, with Clear (:142, :167, :206, :283, :366, :426, :447, :465)
///   or without (:188, :226, :484), and nothing was written into it since;
/// - every holder of the buffer is the variable, the operand, a row of the variable's own table under another key (one row in
///   each cell, two in :283) or another variable that shares the table (:426, :447): `all_found`;
/// - the record is a local, a parameter or a global, not an array element (`Lib.Write(var R)` :66-72, `Rec` in Excel Buffer's
///   own procedures for :527);
/// - the table has one Blob field, the one written (tables 75230 and 75231; Excel Buffer in :527);
/// - the primary key is made of Integer fields (tables 75230 and 75231; Excel Buffer's Row No. and Column No. in :527);
/// - no live xRec holds the buffer (`xrec`).
fn measured_detach(b: &BlobRef, t: &TableDef, pos: usize, base: &ir::Place, all_found: bool, xrec: bool) -> bool {
    let is_blob = |ty: &RtType| matches!(ty, RtType::Library(LibraryKind::Blob));
    b.borrow().bytes.is_empty()
        && all_found
        && matches!(base, ir::Place::Local(_) | ir::Place::Global(_))
        && is_blob(&t.fields[pos].ty)
        && t.fields.iter().filter(|f| is_blob(&f.ty)).count() == 1
        && t.keys[0].iter().all(|&i| matches!(t.fields[i].ty, RtType::Integer))
        && !xrec
}
