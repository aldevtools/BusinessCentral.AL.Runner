//! `Record.IsDirty` (BC 29, runtime 18.0): whether the buffer of a record variable differs from what the variable last
//! read or wrote. Pinned by lt-ac/ACRecordIsDirty.al (codeunit 77504). The snapshot lives in `RecordState::is_dirty_base`
//! and is kept by [`Session::record_op_tracked`], around every record method.
//!
//! What the corpus pins, and so what is implemented:
//! - a declared variable is compared with a fresh buffer: not dirty, also after Init and DeleteAll; dirty after a field
//!   assignment (ACRecordIsDirty.al:37-43);
//! - Get, FindFirst, FindLast, Insert, Modify and Rename take a snapshot (ACRecordIsDirty.al:44-51, 59-60, 103-110);
//! - values are compared, not writes: assigning the value a field holds, or setting a field back, is not dirty
//!   (ACRecordIsDirty.al:52-56); Validate is a write like any other (ACRecordIsDirty.al:61-62);
//! - Init and Reset keep the snapshot (ACRecordIsDirty.al:111-115);
//! - `:=` copies it (ACRecordIsDirty.al:63-64);
//! - TransferFields and Delete give a result where both readings (the snapshot stays, or the buffer becomes the snapshot)
//!   agree (ACRecordIsDirty.al:65-71); where they differ IsDirty is refused. Whether system fields count is not pinned
//!   either: an answer that depends on them is refused.
//!
//! - lt-ac/Y8RecordIsDirtyShapes.al (codeunit 91415) adds: a temporary record behaves like a database record; Find, FindSet,
//!   Next and GetBySystemId clean like Get; a read or write that answers false keeps the snapshot; Insert(true) and Modify(true)
//!   clean, Delete(true) keeps the snapshot; inside the write's own trigger Rec (and xRec in OnModify) answer the caller's state;
//!   Copy and a by-value parameter copy the state, RecordRef.SetTable takes the ref's; Clear makes a fresh variable;
//!   CalcFields, a FlowField and SystemId do not count; an unloaded field assigned and a Blob a stream wrote count; ModifyAll
//!   and DeleteAll leave the variable alone; a declared variable of a table with InitValue fields compares with them.
//!
//! Everything else is refused (the rows of docs/probes-needed.md "Record.IsDirty"): a write that failed, Rename or a write that
//! ran triggers or subscribers outside the pinned shapes, a "not dirty" answer after a false read or Delete(true), a copy of a
//! record variable other than the pinned ones (xRec outside a trigger, an event parameter), a buffer whose answer differs from a
//! comparison with the row stored under its key (the buffer may have been filled by something that is not a record method,
//! such as a page; nothing pins that).

use crate::Session;
use al_builtins::native::Signal;
use al_builtins::ops;
use al_builtins::record_op::RecordOp;
use al_builtins::value::{DirtyBase, RecordState, RtType, Value};
use al_data::Store;
use std::cell::RefCell;
use std::cmp::Ordering;
use std::rc::Rc;

type Rec = Rc<RefCell<RecordState>>;

/// What a record method does to the snapshot.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Effect {
    /// Neither reads nor writes the buffer.
    Keep,
    /// Changes the stored rows but not the variable (Y8RecordIsDirtyShapes.al:410-421 ModifyAll and DeleteAll after a read,
    /// clean and dirty; ACRecordIsDirty.al:37-38 on a fresh variable).
    KeepStore,
    /// Writes fields, keeps the snapshot (Validate, ACRecordIsDirty.al:61-62; SetPosition keeps a dirty record dirty and
    /// changes only the key, Y8RecordIsDirtyShapes.al:256-260). AL code that runs inside sees no snapshot.
    Write,
    /// A successful read or write without triggers: the buffer becomes the snapshot. A read that answers false keeps the
    /// snapshot (Y8RecordIsDirtyShapes.al:278-297).
    Snapshot,
    /// The buffer after the call is a second candidate (TransferFields, Delete).
    Alt,
    /// `Copy`: the record takes the source's snapshot (Y8RecordIsDirtyShapes.al:236-242 `copy_of_dirty` Yes,
    /// `copy_of_clean` No).
    Copy,
    /// `Clear`: a fresh variable again (Y8RecordIsDirtyShapes.al:243-245 `after_clear` No).
    Clear,
    /// `CalcFields` does not count (Y8RecordIsDirtyShapes.al:246-248 a FlowField, :394-396 a Blob, both right after a Get).
    CalcFields,
    /// Not pinned: IsDirty is refused until the next snapshot.
    Lose,
}

fn effect(method: RecordOp) -> Effect {
    use RecordOp as O;
    match method {
        // Find, FindSet, Next and GetBySystemId clean the record like Get (Y8RecordIsDirtyShapes.al:160-177, :251-255).
        O::Get
        | O::FindFirst
        | O::FindLast
        | O::Find
        | O::FindSet
        | O::Next
        | O::GetBySystemId
        | O::Insert
        | O::Modify
        | O::Rename => Effect::Snapshot,
        O::TransferFields | O::Delete => Effect::Alt,
        O::Validate | O::SetPosition => Effect::Write,
        O::DeleteAll | O::ModifyAll => Effect::KeepStore,
        O::Copy => Effect::Copy,
        O::Clear => Effect::Clear,
        O::CalcFields => Effect::CalcFields,
        // Init and Reset are pinned (ACRecordIsDirty.al:39-40, 111-115); the rest neither read nor write the buffer.
        O::Init
        | O::Reset
        | O::IsDirty
        | O::AreFieldsLoaded
        | O::Ascending
        | O::CalcSums
        | O::ClearMarks
        | O::Consistent
        | O::CopyFilter
        | O::CopyFilters
        | O::Count
        | O::CountApprox
        | O::CurrentCompany
        | O::CurrentKey
        | O::FieldActive
        | O::FieldCaption
        | O::FieldError
        | O::FieldName
        | O::FieldNo
        | O::FilterGroup
        | O::GetAscending
        | O::GetFilter
        | O::GetFilters
        | O::GetPosition
        | O::GetRangeMax
        | O::GetRangeMin
        | O::GetView
        | O::HasFilter
        | O::HasLinks
        | O::IsEmpty
        | O::IsTemporary
        | O::LockTable
        | O::Mark
        | O::MarkedOnly
        | O::ReadConsistency
        | O::ReadIsolation
        | O::ReadPermission
        | O::RecordId
        | O::RecordLevelLocking
        | O::Relation
        | O::SecurityFiltering
        | O::SetAscending
        | O::SetCurrentKey
        | O::SetFilter
        | O::SetRange
        | O::SetRecFilter
        | O::SetView
        | O::TableCaption
        | O::TableName
        | O::TestField
        | O::WritePermission
        | O::AddLoadFields
        | O::SetLoadFields
        | O::SetBaseLoadFields
        | O::SetAutoCalcFields => Effect::Keep,
        _ => Effect::Lose,
    }
}

/// Whether the trigger flag of a write asks for triggers (`Insert(true)`, `Modify(true)`, `Delete(true)`).
fn run_trigger(method: RecordOp, args: &[Value]) -> bool {
    matches!(method, RecordOp::Insert | RecordOp::Modify | RecordOp::Delete)
        && matches!(args.first(), Some(Value::Boolean(true)))
}

/// A write with triggers (`Insert(true)`, `Modify(true)`, `Delete(true)`) that is running: inside its trigger, `Rec` and, in
/// OnModify, `xRec` report the caller's state before the call (Y8RecordIsDirtyShapes.al:209 `trigger_log`
/// `ins:Yes;mod:Yes/x:Yes;mod:No/x:No;del:No;`, :332-335 `modify_trigger_log`). Pinned only while the trigger has not
/// changed the record yet: a buffer that differs from `pre` is refused.
pub(crate) struct DirtyFrame {
    rec: Rec,
    method: RecordOp,
    /// The buffer when the call started.
    pre: Vec<Value>,
    /// The table trigger of the write runs (not a subscriber or another trigger before it).
    in_trigger: bool,
    /// The trigger was entered once: a later trigger of the same record (OnValidate called inside) does not count.
    entered: bool,
    /// xRec of OnModify.
    xrec: Option<Rec>,
}

/// Whether two buffers hold the same values in the fields AL code wrote (a Blob counts as the same buffer only). The system
/// fields the write itself stamps before the trigger runs (SystemId, the audit fields) are left out.
fn same_fields(t: &al_data::TableDef, a: &[Value], b: &[Value]) -> bool {
    a.len() == b.len()
        && t.fields.iter().zip(a.iter().zip(b)).all(|(f, (x, y))| {
            f.id >= 2_000_000_000
                || match (x, y) {
                    (Value::Blob(p), Value::Blob(q)) => std::sync::Arc::ptr_eq(p, q),
                    _ => matches!(ops::compare(x, y), Ok(Ordering::Equal)),
                }
        })
}

impl Session {
    pub(crate) fn record_op_tracked(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        let effect = effect(method);
        match effect {
            Effect::Keep => return self.record_op_inner(method, rec, args),
            Effect::KeepStore => {
                let r = self.record_op_inner(method, rec, args);
                let mut s = rec.borrow_mut();
                let RecordState {
                    is_dirty_base, fields, ..
                } = &mut *s;
                is_dirty_base.materialize(fields);
                if let DirtyBase::Row { stale_store, .. } = is_dirty_base {
                    *stale_store = true;
                }
                return r;
            }
            Effect::Lose => {
                let r = self.record_op_inner(method, rec, args);
                rec.borrow_mut().is_dirty_base = DirtyBase::Unknown(lose_reason(method));
                return r;
            }
            Effect::Copy => {
                let r = self.record_op_inner(method, rec, args);
                let base = match (&r, args.first()) {
                    (Ok(_), Some(Value::Record(src))) if !Rc::ptr_eq(src, rec) => src.borrow().is_dirty_base.copy(),
                    _ => DirtyBase::Unknown("after a Copy that failed or copied from itself (not pinned)"),
                };
                rec.borrow_mut().is_dirty_base = base;
                return r;
            }
            Effect::Clear => {
                let r = self.record_op_inner(method, rec, args);
                rec.borrow_mut().is_dirty_base = match r {
                    Ok(_) => DirtyBase::Fresh,
                    Err(_) => DirtyBase::Unknown("after a Clear that failed (not pinned)"),
                };
                return r;
            }
            Effect::CalcFields => {
                let synced = matches!(rec.borrow().is_dirty_base, DirtyBase::Synced);
                let r = self.record_op_inner(method, rec, args);
                // Pinned right after a read only; CalcFields of a changed or fresh record is not.
                rec.borrow_mut().is_dirty_base = match (&r, synced) {
                    (Ok(_), true) => DirtyBase::Synced,
                    _ => DirtyBase::Unknown(
                        "after CalcFields of a record that changed since its last read or write (not pinned)",
                    ),
                };
                return r;
            }
            Effect::Write | Effect::Snapshot | Effect::Alt => {}
        }
        let triggers = run_trigger(method, args);
        if effect != Effect::Snapshot {
            // Validate, TransferFields and Delete write the buffer or compare with it afterwards: a snapshot that is the
            // buffer itself becomes a copy first.
            let mut s = rec.borrow_mut();
            let RecordState {
                is_dirty_base, fields, ..
            } = &mut *s;
            is_dirty_base.materialize(fields);
        }
        if triggers {
            let pre = rec.borrow().fields.clone();
            self.dirty_frames.push(DirtyFrame {
                rec: rec.clone(),
                method,
                pre,
                in_trigger: false,
                entered: false,
                xrec: None,
            });
        }
        // AL code that runs inside (a trigger, a subscriber) may call IsDirty or another record method on the same record:
        // the first is answered only inside the write's own trigger (`DirtyFrame`), the second makes `ran_code` true.
        let calls = self.calls_made;
        self.dirty_ops_open += 1;
        let r = self.record_op_inner(method, rec, args);
        self.dirty_ops_open -= 1;
        if triggers {
            self.dirty_frames.pop();
        }
        let ran_code = self.calls_made != calls;
        let mut s = rec.borrow_mut();
        let saved = std::mem::replace(&mut s.is_dirty_base, DirtyBase::Synced);
        let answered_false =
            matches!(r, Ok(Value::Boolean(false))) || (method == RecordOp::Next && matches!(r, Ok(Value::Integer(0))));
        s.is_dirty_base = match (&r, effect) {
            (Err(_), _) => DirtyBase::Unknown("after a record method that failed (not pinned)"),
            // A read or write that answers false keeps the snapshot; pinned only on a buffer that differs from it
            // (Y8RecordIsDirtyShapes.al:278-297, :175-177 Next at the end).
            (Ok(_), Effect::Snapshot | Effect::Alt) if answered_false => match saved {
                DirtyBase::Row { base, alt: None, .. } => DirtyBase::Row {
                    base,
                    alt: None,
                    dirty_only: true,
                    stale_store: false,
                },
                _ => DirtyBase::Unknown(
                    "after a read or write that returned false on a record that had not changed (not pinned)",
                ),
            },
            // Insert(true) and Modify(true) take the buffer after the trigger as the snapshot (Y8RecordIsDirtyShapes.al:
            // 196-205 `insert_true` and `modify_true` No, with the trigger's change in the buffer).
            (Ok(_), Effect::Snapshot) if triggers && matches!(method, RecordOp::Insert | RecordOp::Modify) => {
                DirtyBase::Synced
            }
            // Delete(true) keeps the snapshot: the trigger's change makes the record dirty (Y8RecordIsDirtyShapes.al:206-208
            // `delete_true` Yes). Without a change the answer is not pinned.
            (Ok(_), Effect::Alt) if triggers && method == RecordOp::Delete => match saved {
                DirtyBase::Row { base, .. } => DirtyBase::Row {
                    base,
                    alt: None,
                    dirty_only: true,
                    stale_store: false,
                },
                _ => DirtyBase::Unknown("after Delete(true) of a record that was not read (not pinned)"),
            },
            _ if triggers || ran_code => {
                DirtyBase::Unknown("after a record method that ran triggers or subscribers (not pinned)")
            }
            (Ok(_), Effect::Write) => saved,
            // No copy: the buffer is the snapshot until a field is written (RecordState::before_field_write).
            (Ok(_), Effect::Snapshot) => DirtyBase::Synced,
            (Ok(_), _) => match saved {
                DirtyBase::Row { base, dirty_only, .. } => DirtyBase::Row {
                    base,
                    alt: Some(s.fields.clone()),
                    dirty_only,
                    stale_store: false,
                },
                _ => DirtyBase::Unknown("TransferFields or Delete on a record that was not read (not pinned)"),
            },
        };
        r
    }

    /// A table trigger is about to run on `rec`: inside the first one of a write with triggers, IsDirty of `Rec` (and of
    /// `xRec` in OnModify) is answered (`DirtyFrame`). Returns whether a frame was entered, for [`Self::dirty_trigger_exit`].
    pub(crate) fn dirty_trigger_enter(&mut self, rec: &Rec, xrec: Option<&Rec>) -> bool {
        match self.dirty_frames.last_mut() {
            Some(f) if !f.entered && Rc::ptr_eq(&f.rec, rec) => {
                f.entered = true;
                f.in_trigger = true;
                if f.method == RecordOp::Modify {
                    f.xrec = xrec.cloned();
                }
                true
            }
            _ => false,
        }
    }

    pub(crate) fn dirty_trigger_exit(&mut self, entered: bool) {
        if entered && let Some(f) = self.dirty_frames.last_mut() {
            f.in_trigger = false;
            f.xrec = None;
        }
    }

    /// `Record.IsDirty()`.
    pub(crate) fn record_is_dirty(&mut self, rec: &Rec) -> Result<Value, Signal> {
        if self.dirty_ops_open > 0 {
            let Some(f) = self.dirty_frames.last().filter(|f| f.in_trigger) else {
                return refuse("inside a trigger or subscriber of a record method (not pinned)");
            };
            let is_rec = Rc::ptr_eq(&f.rec, rec);
            if !is_rec && !f.xrec.as_ref().is_some_and(|x| Rc::ptr_eq(x, rec)) {
                return refuse("inside a trigger or subscriber of a record method (not pinned)");
            }
            let target = f.rec.clone();
            let t = self.table(&target)?;
            let f = self.dirty_frames.last().expect("frame");
            if !same_fields(&t, &f.rec.borrow().fields, &f.pre) {
                return refuse("inside the trigger of a write, after the trigger changed the record (not pinned)");
            }
            return self.dirty_answer(&target);
        }
        self.dirty_answer(rec)
    }

    fn dirty_answer(&mut self, rec: &Rec) -> Result<Value, Signal> {
        let t = self.table(rec)?;
        let (fields, base, alt, dirty_only, stale_store, partial) = {
            let s = rec.borrow();
            // A temporary record behaves like a database record (Y8RecordIsDirtyShapes.al:119-151).
            if s.company.is_some() {
                return refuse("after ChangeCompany (not pinned)");
            }
            // A partial record: a read is clean, an unloaded field assigned makes it dirty (Y8RecordIsDirtyShapes.al:
            // 382-388). An unloaded field assigned its stored value is not pinned (`assigned_unloaded`).
            let partial = s.load.partial;
            let assigned_unloaded = partial && s.load.buffer.as_ref().is_some_and(|b| !b.assigned.is_empty());
            let (base, alt, dirty_only, stale_store) = match &s.is_dirty_base {
                DirtyBase::Unknown(why) => return refuse(why),
                DirtyBase::Fresh => (None, None, false, false),
                DirtyBase::Synced => (Some(s.fields.clone()), None, false, false),
                DirtyBase::Row {
                    base,
                    alt,
                    dirty_only,
                    stale_store,
                } => (Some(base.clone()), alt.clone(), *dirty_only, *stale_store),
            };
            (
                s.fields.clone(),
                base,
                alt,
                dirty_only || assigned_unloaded,
                stale_store,
                partial,
            )
        };
        let engine = self.engine.clone();
        let schema = engine.program.objects[t.object as usize].table.as_ref().expect("table");
        let base = match base {
            Some(b) => b,
            None => {
                // A declared variable of a table with InitValue fields is compared with a buffer that holds them
                // (Y8RecordIsDirtyShapes.al:353-358, a Decimal and a Boolean InitValue). A date or time InitValue is not pinned.
                let timed = schema.fields.iter().zip(&t.fields).any(|(f, d)| {
                    f.init_text.is_some() && matches!(d.ty, RtType::Date | RtType::Time | RtType::DateTime)
                });
                if timed {
                    return refuse("on a declared variable of a table with a date or time InitValue (not pinned)");
                }
                self.initial_fields(t.object as usize)
            }
        };
        // (whether a field other than a system field differs, whether any field differs). Nothing pins whether the
        // system fields other than SystemId (SystemCreatedAt, ...) count: where the two differ the answer is refused.
        // `skip`: the fields the comparison leaves out (the unloaded fields of a partial record against the stored row).
        let differs = |other: &[Value], skip: &dyn Fn(usize) -> bool, stored: bool| -> Result<(bool, bool), Signal> {
            let (mut user, mut any) = (false, false);
            for (i, f) in t.fields.iter().enumerate() {
                if skip(i) {
                    continue;
                }
                // SystemId does not count (Y8RecordIsDirtyShapes.al:359-361 `systemid_assigned` No).
                if f.id == SYSTEM_ID {
                    continue;
                }
                if !f.stored {
                    // A FlowField does not count (Y8RecordIsDirtyShapes.al:249-250 `flowfield_assigned` No); a FlowFilter
                    // is not pinned.
                    if !schema.fields[i].flow_filter {
                        continue;
                    }
                    if !matches!(ops::compare(&fields[i], &other[i]), Ok(Ordering::Equal)) {
                        return refuse(&format!("on a FlowFilter {} that differs (not pinned)", f.name));
                    }
                    continue;
                }
                let equal = match (&fields[i], &other[i]) {
                    // A Blob a stream wrote counts (Y8RecordIsDirtyShapes.al:390-393 `blob_written` Yes); one no stream wrote
                    // does not (:394-396 `blob_calcfields` No). Two loaded buffers with other content are not pinned. Against the
                    // stored row (`stored`) only the stream write counts.
                    (Value::Blob(p), Value::Blob(_)) if stored => !p.borrow().written,
                    (Value::Blob(p), Value::Blob(q)) => {
                        if p.borrow().written {
                            false
                        } else if std::sync::Arc::ptr_eq(p, q) {
                            true
                        } else {
                            let (pb, qb) = (p.borrow(), q.borrow());
                            if qb.written || (pb.loaded && qb.loaded && pb.bytes != qb.bytes) {
                                return refuse(&format!("on a Blob {} that differs (not pinned)", f.name));
                            }
                            true
                        }
                    }
                    (a, b) => match ops::compare(a, b) {
                        Ok(o) => o == Ordering::Equal,
                        Err(_) => {
                            return refuse(&format!(
                                "on field {} of a type that is not compared (not pinned)",
                                f.name
                            ));
                        }
                    },
                };
                if !equal {
                    any = true;
                    user |= f.id < 2_000_000_000;
                }
            }
            Ok((user, any))
        };
        let none = |_: usize| false;
        let (dirty, any) = differs(&base, &none, false)?;
        match alt {
            None if any != dirty => return refuse("where only system fields differ from the snapshot (not pinned)"),
            None => {}
            // After TransferFields or Delete the snapshot may be kept or may become the buffer. ACRecordIsDirty.al:65-68
            // (`after_transferfields` No, after Init, where TransferFields leaves the system fields empty) rules out
            // "the snapshot is kept and system fields count"; the other three readings must agree.
            Some(alt) => {
                let (alt_user, alt_any) = differs(&alt, &none, false)?;
                if alt_user != dirty || alt_any != dirty {
                    return refuse(
                        "after TransferFields or Delete, where the corpus does not tell which snapshot BC keeps",
                    );
                }
            }
        }
        if dirty_only && !dirty {
            return refuse(
                "after a read or write that answered false, Delete(true), or an unloaded field assigned, where the record did not change (not pinned)",
            );
        }
        // The buffer may have been filled by something other than a record method (a page): the answer must be the same
        // against the row stored under the key (fields other than system fields). After ModifyAll or DeleteAll the stored
        // row differs on purpose (`stale_store`).
        if !stale_store {
            let cx = self.cx_for(rec, &t)?;
            let pk = t.primary_key(&fields);
            let stored = self
                .store_for(rec)
                .get(&cx, &t, &pk)
                .map_err(|e| Signal::Unsupported(format!("{e}")))?;
            if let Some(row) = stored {
                // An unloaded field the record did not assign is not compared.
                let have = if partial {
                    rec.borrow()
                        .load
                        .buffer
                        .as_ref()
                        .map(|b| (b.have.clone(), b.assigned.clone()))
                } else {
                    None
                };
                let skip = |i: usize| have.as_ref().is_some_and(|(h, a)| !h.contains(i) && !a.contains(&i));
                if differs(&row, &skip, true)?.0 != dirty {
                    return refuse("where the snapshot and the stored row give different answers (not pinned)");
                }
            }
        }
        Ok(Value::Boolean(dirty))
    }
}

/// The field number of SystemId.
const SYSTEM_ID: u32 = 2_000_000_000;

fn refuse<T>(why: &str) -> Result<T, Signal> {
    Err(Signal::Unsupported(format!("Record.IsDirty {why}")))
}

fn lose_reason(method: RecordOp) -> &'static str {
    use RecordOp as O;
    match method {
        O::Find | O::FindSet | O::Next => "after Find, FindSet or Next (not pinned)",
        O::Copy => "after Copy (not pinned)",
        O::Clear => "after Clear (not pinned)",
        O::CalcFields => "after CalcFields (not pinned)",
        O::GetBySystemId => "after GetBySystemId (not pinned)",
        _ => "after a record method whose effect on the snapshot is not pinned",
    }
}
