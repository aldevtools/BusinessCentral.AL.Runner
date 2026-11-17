//! The view a record of a stored table gets when it is handed by value to a Record parameter of a procedure: the parameter is
//! not declared temporary, is of a table that is not TableType = Temporary, and is not an event subscriber's. Entry:
//! `Session::by_value_view` (by the parameter binding of `call_inner` in `lib.rs`), `Session::by_value_view_op` (by
//! `unpinned_buffer_op`).

use crate::Session;
use crate::records::Rec;
use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::{Signal, unsupported};
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RtType, Value};

/// The `RecordState::unpinned_buffer` reason of a by-value copy whose view [`Session::by_value_view`] does not decide.
pub(crate) const BY_VALUE_VIEW: &str = "a record handed by value whose caller had filters outside filter group 0, stood in another filter group, read in descending order or had marks: real BC hands such a copy the primary key and no filter of group 0, the rest of its view is not pinned";

impl Session {
    /// The copy of `rc` a by-value parameter of type `param` gets (`record_by_value_into`): it keeps what Record.IsDirty
    /// compares with (record/Y8RecordIsDirtyShapes.al:321-324 `by_value_dirty` Yes, `by_value_clean` No) and has the view
    /// [`Session::by_value_view`] gives it.
    pub(crate) fn record_param_copy(&mut self, rc: &Rec, param: &RtType, subscriber: bool) -> Value {
        let v = self.record_by_value_into(rc, param);
        if let Value::Record(copy) = &v {
            copy.borrow_mut().is_dirty_base = rc.borrow().is_dirty_base.copy();
            self.by_value_view(rc, copy, param, subscriber);
        }
        v
    }

    /// Gives `copy`, the by-value copy of `source` for a parameter of type `param` (`subscriber`: an event subscriber's), the
    /// view BC hands it. The entry is one allowlist: `source` is not temporary and of no other company, the parameter is a
    /// Record of the same table, not temporary and of a table that is not TableType = Temporary, not a subscriber's, and
    /// `source` carries no link filter and no other unpinned mark. Every other copy keeps main's view (`record_by_value`).
    /// - Caller in filter group 0, ascending, no marks, MarkedOnly off: the copy has the primary key (record/keys-sorting/
    ///   PwFlowFieldKeyRule.al:252 `normalField ... byValue=Id order=1 2 3`, a key on a normal field and no filter) and, when every
    ///   filter is in group 0, no filters (assumption `by-value-record-no-filters`, al-runner issue 8185), marked on the copy
    ///   (`RecordState::view_assumption`) so that a read of its view marks the assumption (`by_value_view_read`).
    /// - Else the copy keeps main's view and is marked [`BY_VALUE_VIEW`]: a read of its view refuses (`by_value_view_op`).
    pub(crate) fn by_value_view(&self, source: &Rec, copy: &Rec, param: &RtType, subscriber: bool) {
        let s = source.borrow();
        let entry = matches!(param, RtType::Record(t, false) if *t == s.table
                && !self.engine.object_facts[*t as usize].temporary())
            && !s.temporary
            && s.company.is_none()
            && !s.crm
            && !subscriber
            && s.run_page_link.is_none()
            && s.unpinned_buffer.is_none();
        if !entry {
            return;
        }
        let plain_view =
            s.filter_group == 0 && s.ascending && s.field_descending == 0 && s.marks.is_empty() && !s.marked_only;
        let group_zero = s.filters.iter().all(|f| f.group == 0);
        let mut c = copy.borrow_mut();
        if plain_view && s.filters.is_empty() {
            c.key = 0;
            c.adhoc = None;
        } else if plain_view && group_zero {
            c.key = 0;
            c.adhoc = None;
            c.filters.clear();
            c.view_assumption = true;
        } else {
            c.unpinned_buffer = Some(BY_VALUE_VIEW);
        }
    }

    /// A record method `method` on `rec` (`own`) or with `rec` as its source argument, where `rec` holds
    /// `RecordState::view_assumption`: a method that reads the view marks `by-value-record-no-filters`; `Reset`, `SetView`
    /// and `Clear` on `rec` give it a view of its own, which ends the mark. The methods that read neither the view nor the
    /// marks of `rec` itself mark nothing. A copy into a RecordRef reads the view (`refs.rs` `gettable_record_copy`).
    pub(crate) fn by_value_view_read(method: RecordOp, rec: &Rec, own: bool) {
        if own && matches!(method, RecordOp::Reset | RecordOp::SetView | RecordOp::Clear) {
            rec.borrow_mut().view_assumption = false;
            return;
        }
        let unread = matches!(
            method,
            RecordOp::Get
                | RecordOp::GetBySystemId
                | RecordOp::Insert
                | RecordOp::Modify
                | RecordOp::Delete
                | RecordOp::Rename
                | RecordOp::Init
                | RecordOp::TestField
                | RecordOp::Validate
                | RecordOp::FieldNo
                | RecordOp::FieldName
                | RecordOp::FieldCaption
                | RecordOp::FieldError
                | RecordOp::IsTemporary
                | RecordOp::TableName
                | RecordOp::TableCaption
                | RecordOp::RecordId
                | RecordOp::TransferFields
                | RecordOp::SetLoadFields
                | RecordOp::AddLoadFields
                | RecordOp::LockTable
                | RecordOp::IsDirty
        );
        if !(own && unread) {
            assumptions::assume(Assumption::ByValueRecordNoFilters);
        }
    }

    /// A record method on a copy marked [`BY_VALUE_VIEW`]: the methods that read neither the view nor the marks run (a field's
    /// own value, Get, Insert, Modify, Delete); `Reset` gives the copy the view of a reset record, so the mark goes when the copy
    /// holds no marks; every other method refuses.
    pub(crate) fn by_value_view_op(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        let runs = matches!(
            method,
            RecordOp::Get
                | RecordOp::Insert
                | RecordOp::Modify
                | RecordOp::Delete
                | RecordOp::Init
                | RecordOp::TestField
                | RecordOp::Validate
                | RecordOp::CalcFields
                | RecordOp::FieldNo
                | RecordOp::IsTemporary
                | RecordOp::TableName
                | RecordOp::TableCaption
                | RecordOp::Reset
        );
        if !runs {
            return unsupported(format!("Record.{}(): {BY_VALUE_VIEW}", method.name()));
        }
        let flag = rec.borrow_mut().unpinned_buffer.take();
        let r = self.record_op_inner(method, rec, args);
        let reset = method == RecordOp::Reset && r.is_ok() && rec.borrow().marks.is_empty();
        if !reset {
            rec.borrow_mut().unpinned_buffer = flag;
        }
        r
    }
}
