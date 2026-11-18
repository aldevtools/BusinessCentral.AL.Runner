//! The view a record gets when it is handed by value to a Record parameter of a procedure: the parameter is not declared
//! temporary, is of a table that is not TableType = Temporary, and is not an event subscriber's. Entry:
//! `Session::by_value_view` for a record of a stored table, `Session::temporary_into_stored_param` for a temporary one (by the
//! parameter binding of `call_inner` in `lib.rs`). What a copy whose view is not pinned
//! answers afterwards: its own record methods (`Session::by_value_view_runs`, by `record_op_inner`), and every hand-off of it to
//! code that reads its view (`Session::by_value_view_handed`: a record method with it as the source, `RecordRef.GetTable`, a
//! page, report, XmlPort or FilterPageBuilder it is handed to); Copy and CopyFilters hand its marks to their target.

use crate::Session;
use crate::records::{FLOWFIELD_KEY, Rec};
use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::{Signal, unsupported};
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RecordState, RtType, Value};
use al_ir::ObjKind;
use std::cell::RefCell;
use std::rc::Rc;

/// The `RecordState::unpinned_buffer` reason of a by-value copy whose view [`Session::by_value_view`] does not decide.
pub(crate) const BY_VALUE_VIEW: &str = "a record handed by value whose view the probe did not measure (a filter in filter group 1 or in a group other than -1, 0, 2 and 4, the caller standing in a group other than 0 and 2, a field sorted descending by SetAscending, or a parameter of an object other than a codeunit or the record's own table): what the copy's view holds is not pinned";

/// Whether record method `method` reads neither the view (filters, key, order) nor the marks of the record it is called on: it
/// reads or writes the record's own fields or its row by primary key, reads the table's metadata, or names the fields to load.
/// The one list [`Session::by_value_view_read`] and [`Session::by_value_view_runs`] take.
fn reads_no_view(method: RecordOp) -> bool {
    matches!(
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
    )
}

/// Whether the probe measured the view of `s`, handed by value to a callee [`Session::measured_callee`] names
/// (assumption `by-value-record-no-filters`, corpus PR 1430, record/filters/ByValueRecordView.al): every filter in filter
/// group -1, 0, 2 or 4 (`ByValue_GroupMinus1Filter`, `ByValue_GroupMinus1_TwoFields`, `ByValue_Group0Filter_*`,
/// `ByValue_Group2Filter`, `ByValue_Group4Filter`, `ByValue_FlowFieldFilter_*`, `ByValue_CalcFields_*`), the caller standing
/// in group 0 or 2 (`ByValue_CallerInGroup2_*`), no field sorted descending by SetAscending. The key (primary, declared
/// secondary in `ByValue_SecondaryKey_*`, ad hoc in `ByValue_AdHocKey_Group0Filter`), Ascending (`ByValue_Descending_*`)
/// and the marks with MarkedOnly on or off (`ByValue_Marks_*`) are each measured. A filter in group 1 is seen by every
/// record of the table in the session (`ByValue_Group1Filter_OtherInstance`), which the runtime does not model.
fn measured_view(s: &RecordState) -> bool {
    s.filters.iter().all(|f| matches!(f.group, -1 | 0 | 2 | 4))
        && matches!(s.filter_group, 0 | 2)
        && s.field_descending == 0
}

/// The view of a fresh record: no filters in any filter group, filter group 0, the primary key ascending, no marks,
/// MarkedOnly off. What a by-value copy gets in [`Session::by_value_view`] (each callee:
/// `f={} n=3 key=Id asc=Yes grp=0 first=1`, `mo=No n=3 marked n=0`), and a copy for a temporary parameter (al-vm
/// `Session::temporary_param_copy`).
pub(crate) fn primary_view(st: &mut RecordState) {
    st.key = 0;
    st.adhoc = None;
    st.ascending = true;
    st.field_descending = 0;
    st.marks.clear();
    st.marked_only = false;
    st.filters.clear();
    st.filter_group = 0;
}

impl Session {
    /// The copy of `rc` a by-value parameter of type `param` of a procedure of object `callee` gets (`record_by_value_into`;
    /// `subscriber`: the procedure is an event subscriber): it keeps what Record.IsDirty compares with
    /// (record/Y8RecordIsDirtyShapes.al:321-324 `by_value_dirty` Yes, `by_value_clean` No) and has the view
    /// [`Session::by_value_view`] gives it. A subscriber's copy keeps main's view (not measured).
    pub(crate) fn record_param_copy(&mut self, rc: &Rec, param: &RtType, callee: usize, subscriber: bool) -> Value {
        let v = self.record_by_value_into(rc, param);
        if let Value::Record(copy) = &v {
            copy.borrow_mut().is_dirty_base = rc.borrow().is_dirty_base.copy();
            if !subscriber {
                self.by_value_view(rc, copy, param, callee);
            }
        }
        v
    }

    /// Whether the probe measured a by-value Record parameter of type `param` of a procedure of object `callee`
    /// (assumption `by-value-record-no-filters`, corpus PR 1430, record/filters/ByValueRecordView.al): a codeunit's
    /// procedure (`BVV Callee`, and `DescribeLocal`, a local procedure of the test codeunit), or a table's procedure whose
    /// parameter is a record of that table (`CountSameStatus`, called as `Row.CountSameStatus(Row)` in
    /// `ByValue_TableProcedure_OwnRecByValue`).
    fn measured_callee(&self, callee: usize, param: &RtType) -> bool {
        match self.engine.program.objects[callee].kind {
            ObjKind::Codeunit => true,
            ObjKind::Table => matches!(param, RtType::Record(t, false) if *t as usize == callee),
            _ => false,
        }
    }

    /// Gives `copy`, the by-value copy of `source` for a parameter of type `param` of a procedure of object `callee`, the
    /// view BC hands it. The entry is one allowlist: `source` is not temporary and of no other company, the parameter is a
    /// Record of the same table, not temporary and of a table that is not TableType = Temporary, and `source` carries no
    /// link filter. Every other copy keeps main's view (`record_by_value`).
    /// - Caller in filter group 0, ascending, no marks, MarkedOnly off, no filter, on the primary key or an ad hoc key
    ///   (`RecordState::adhoc`), any callee: the copy has the primary key, also after an ad hoc key on a normal field, on a
    ///   FlowField whose value depends on the row (whose `FLOWFIELD_KEY` mark it does not take) or on a FlowField and then
    ///   the primary key field (record/keys-sorting/PwFlowFieldKeyRule.al:249-252 `byValue=Id order=1 2 3`).
    /// - A callee [`Session::measured_callee`] names, no mark on `source`, a view [`measured_view`] names: the view of
    ///   [`primary_view`], and a SetLoadFields of a caller that read no row is dropped (`ByValue_SetLoadFields`
    ///   `amount loaded=Yes`). Marked on the copy (`RecordState::view_assumption`) so that a read of its view marks the
    ///   assumption (`by_value_view_read`, `by_value_view_handed`).
    /// - Another callee, no filter and no mark on `source`, the caller in filter group 0, ascending, with no marks: main's
    ///   view (a declared secondary key stays).
    /// - Else, with no mark on `source`, the copy keeps main's view and is marked [`BY_VALUE_VIEW`]: a read of its view
    ///   refuses (`by_value_view_runs`, `by_value_view_handed`). A copy of a marked `source` keeps main's view and the mark.
    pub(crate) fn by_value_view(&self, source: &Rec, copy: &Rec, param: &RtType, callee: usize) {
        let s = source.borrow();
        let entry = matches!(param, RtType::Record(t, false) if *t == s.table
                && !self.engine.object_facts[*t as usize].temporary())
            && !s.temporary
            && s.company.is_none()
            && !s.crm
            && s.run_page_link.is_none();
        if !entry {
            return;
        }
        let measured = self.measured_callee(callee, param);
        let plain_view =
            s.filter_group == 0 && s.ascending && s.field_descending == 0 && s.marks.is_empty() && !s.marked_only;
        let no_filter = s.filters.is_empty();
        let pinned_key = s.key == 0 || s.adhoc.as_ref().is_some_and(|(k, _)| *k == s.key);
        let load_dropped = measured && s.load.request.is_some() && s.load.buffer.is_none();
        let mut c = copy.borrow_mut();
        match s.unpinned_buffer {
            None | Some(FLOWFIELD_KEY) if plain_view && no_filter && pinned_key && !load_dropped => {
                c.key = 0;
                c.adhoc = None;
                c.unpinned_buffer = None;
            }
            None if measured && measured_view(&s) => {
                primary_view(&mut c);
                if load_dropped {
                    c.load.request = None;
                }
                c.view_assumption = true;
            }
            None if !measured && plain_view && no_filter => {}
            None => c.unpinned_buffer = Some(BY_VALUE_VIEW),
            Some(_) => {}
        }
    }

    /// The copy a by-value parameter that is not declared temporary, of a table that is not TableType = Temporary, of a
    /// procedure of object `callee` gets of the temporary record `rc`: a record of the stored table
    /// (record/temporary/TempRecordByValue.al:TempRecord_ByValue_ToPersistentParameter, :TempRecord_ByValue_ModifyAndDeleteAll),
    /// with the record's current values. The probe measured one view (assumption `by-value-record-no-filters`, corpus PR
    /// 1430, record/filters/ByValueRecordView.al `ByValue_TemporaryToPlainParameter`: `temp=No f={}`): a codeunit's procedure,
    /// the caller with filters in filter group 0 only, standing in group 0, on the primary key, ascending, with no marks and
    /// no SetLoadFields. That copy has no filters, marked as in [`Session::by_value_view`]. Every other copy keeps the
    /// caller's view (main; not pinned).
    pub(crate) fn temporary_into_stored_param(&self, rc: &Rec, callee: usize) -> Value {
        let mut st = rc.borrow().copy_as_new_variable();
        st.temporary = false;
        st.temp_id = None;
        let measured = self.engine.program.objects[callee].kind == ObjKind::Codeunit
            && !st.filters.is_empty()
            && st.filters.iter().all(|f| f.group == 0)
            && st.filter_group == 0
            && st.key == 0
            && st.adhoc.is_none()
            && st.ascending
            && st.field_descending == 0
            && st.marks.is_empty()
            && !st.marked_only
            && st.load.request.is_none()
            && st.company.is_none()
            && st.run_page_link.is_none()
            && st.unpinned_buffer.is_none();
        if measured {
            st.filters.clear();
            st.view_assumption = true;
        }
        Value::Record(Rc::new(RefCell::new(st)))
    }

    /// A record method `method` on `rec`, which holds `RecordState::view_assumption`: a method that reads the view marks
    /// `by-value-record-no-filters` (every method but [`reads_no_view`]); `Reset`, `SetView` and `Clear` give it a view of
    /// its own, which ends the mark.
    fn by_value_view_read(method: RecordOp, rec: &Rec) {
        if matches!(method, RecordOp::Reset | RecordOp::SetView | RecordOp::Clear) {
            rec.borrow_mut().view_assumption = false;
            return;
        }
        if !reads_no_view(method) {
            assumptions::assume(Assumption::ByValueRecordNoFilters);
        }
    }

    /// The view marks `record_op_inner` checks before `method` runs on `rec` with `args`. Copy and CopyFilters give `rec` the
    /// view of their source, and with it the source's `view_assumption` (marked now) or [`BY_VALUE_VIEW`] mark, so a later read
    /// of `rec`'s view marks or refuses; `rec` holds no other mark here (`record_op_inner` sent it to `unpinned_buffer_op`).
    /// Another method with a record as its first argument is handed it (`by_value_view_handed`), except TransferFields, which
    /// reads its fields only. Else a read of `rec`'s own view marks the assumption (`by_value_view_read`).
    pub(crate) fn by_value_view_marks(method: RecordOp, rec: &Rec, args: &[Value]) -> Result<(), Signal> {
        match (method, args.first()) {
            (RecordOp::TransferFields, _) => {}
            (RecordOp::Copy | RecordOp::CopyFilters, Some(Value::Record(src))) if !Rc::ptr_eq(src, rec) => {
                let (assumed, unpinned) = {
                    let s = src.borrow();
                    (s.view_assumption, s.unpinned_buffer == Some(BY_VALUE_VIEW))
                };
                if assumed {
                    assumptions::assume(Assumption::ByValueRecordNoFilters);
                }
                let mut r = rec.borrow_mut();
                r.view_assumption = assumed;
                if unpinned {
                    r.unpinned_buffer = Some(BY_VALUE_VIEW);
                }
                return Ok(());
            }
            (_, Some(Value::Record(src))) if !Rc::ptr_eq(src, rec) => {
                Self::by_value_view_handed(src, || format!("Record.{}() from a record", method.name()))?;
            }
            _ => {}
        }
        if rec.borrow().view_assumption {
            Self::by_value_view_read(method, rec);
        }
        Ok(())
    }

    /// `rec` handed to code that reads its view (`to` names it): a copy that holds `RecordState::view_assumption` marks the
    /// assumption, one marked [`BY_VALUE_VIEW`] refuses. Callers: `by_value_view_marks` (the source of a record method other
    /// than Copy, CopyFilters and TransferFields), `refs.rs` `gettable_record_copy`, the record a page opens on
    /// (`pages/open/record_refusals.rs` `page_record_refusal`: Page.Run, RunModal, SetRecord or SetTableView), the view of a
    /// report (`reports.rs` `report_set_view`) and of an XmlPort (`xmlports/mod.rs` `xp_set_view`), and
    /// FilterPageBuilder.AddRecord (`filterpage.rs`).
    pub(crate) fn by_value_view_handed(rec: &Rec, to: impl FnOnce() -> String) -> Result<(), Signal> {
        let r = rec.borrow();
        if r.view_assumption {
            assumptions::assume(Assumption::ByValueRecordNoFilters);
        }
        if r.unpinned_buffer == Some(BY_VALUE_VIEW) {
            return unsupported(format!("{}: {BY_VALUE_VIEW}", to()));
        }
        Ok(())
    }

    /// Whether `method` runs on `rec` with its mark kept, by `record_op_inner`, when `rec` is marked [`BY_VALUE_VIEW`]
    /// (`false`: another mark, for `unpinned_buffer_op`). It runs when it reads neither the view nor the marks
    /// ([`reads_no_view`]), when it is `CalcFields` and no filter of the copy is on a FlowFilter field (in no filter group: BC
    /// hands the copy none of group 0), and when it is `Reset`, which gives the copy the view of a reset record: the mark
    /// goes when the copy holds no marks. Every other method refuses. The triggers and events a method runs see the mark.
    pub(crate) fn by_value_view_runs(&self, method: RecordOp, rec: &Rec) -> Result<bool, Signal> {
        let marked = rec.borrow().unpinned_buffer == Some(BY_VALUE_VIEW);
        if !marked {
            return Ok(false);
        }
        let runs = reads_no_view(method)
            || method == RecordOp::Reset
            || (method == RecordOp::CalcFields && !self.holds_flow_filter_filter(rec));
        if !runs {
            return unsupported(format!("Record.{}(): {BY_VALUE_VIEW}", method.name()));
        }
        if method == RecordOp::Reset && rec.borrow().marks.is_empty() {
            rec.borrow_mut().unpinned_buffer = None;
        }
        Ok(true)
    }

    /// Whether a filter of `rec`, in any filter group, is on a FlowFilter field.
    fn holds_flow_filter_filter(&self, rec: &Rec) -> bool {
        let r = rec.borrow();
        let Some(schema) = self.engine.program.objects[r.table as usize].table.as_ref() else {
            return true;
        };
        r.filters
            .iter()
            .any(|f| schema.fields.get(f.field).is_none_or(|field| field.flow_filter))
    }
}
