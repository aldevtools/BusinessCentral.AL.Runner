//! The view a record of a stored table gets when it is handed by value to a Record parameter of a procedure: the parameter is
//! not declared temporary, is of a table that is not TableType = Temporary, and is not an event subscriber's. Entry:
//! `Session::by_value_view` (by the parameter binding of `call_inner` in `lib.rs`). What a copy whose view is not pinned
//! answers afterwards: its own record methods (`Session::by_value_view_runs`, by `record_op_inner`), and every hand-off of it to
//! code that reads its view (`Session::by_value_view_handed`: a record method with it as the source, `RecordRef.GetTable`, a
//! page, report, XmlPort or FilterPageBuilder it is handed to); Copy and CopyFilters hand its marks to their target.

use crate::Session;
use crate::records::{FLOWFIELD_KEY, Rec};
use al_builtins::assumptions::{self, Assumption};
use al_builtins::native::{Signal, unsupported};
use al_builtins::record_op::RecordOp;
use al_builtins::value::{RtType, Value};
use std::rc::Rc;

/// The `RecordState::unpinned_buffer` reason of a by-value copy whose view [`Session::by_value_view`] does not decide.
pub(crate) const BY_VALUE_VIEW: &str = "a record handed by value whose caller had filters outside filter group 0, stood in another filter group, read in descending order or had marks: real BC hands such a copy the primary key and no filter of group 0, the rest of its view is not pinned";

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
    /// Record of the same table, not temporary and of a table that is not TableType = Temporary, not a subscriber's, `source`
    /// carries no link filter, and its key is the primary key or an ad hoc one (`RecordState::adhoc`: SetCurrentKey on fields
    /// no key of the table starts with). Every other copy keeps main's view (`record_by_value`): a declared secondary key too.
    /// - Caller in filter group 0, ascending, no marks, MarkedOnly off, no filter: the copy has the primary key, also after an
    ///   ad hoc key on a normal field, on a FlowField whose value depends on the row (whose `FLOWFIELD_KEY` mark it does not
    ///   take) or on a FlowField and then the primary key field (record/keys-sorting/PwFlowFieldKeyRule.al:249-252
    ///   `byValue=Id order=1 2 3`).
    /// - The same with filters, every one in group 0, and no mark on `source`: the primary key and no filters (assumption
    ///   `by-value-record-no-filters`, al-runner issue 8185), marked on the copy (`RecordState::view_assumption`) so that a
    ///   read of its view marks the assumption (`by_value_view_read`, `by_value_view_handed`).
    /// - Else, with no mark on `source`, the copy keeps main's view and is marked [`BY_VALUE_VIEW`]: a read of its view
    ///   refuses (`by_value_view_runs`, `by_value_view_handed`). A copy of a marked `source` keeps main's view and the mark.
    pub(crate) fn by_value_view(&self, source: &Rec, copy: &Rec, param: &RtType, subscriber: bool) {
        let s = source.borrow();
        let ad_hoc_key = s.adhoc.as_ref().is_some_and(|(k, _)| *k == s.key);
        let entry = matches!(param, RtType::Record(t, false) if *t == s.table
                && !self.engine.object_facts[*t as usize].temporary())
            && !s.temporary
            && s.company.is_none()
            && !s.crm
            && !subscriber
            && s.run_page_link.is_none()
            && (s.key == 0 || ad_hoc_key);
        if !entry {
            return;
        }
        let plain_view =
            s.filter_group == 0 && s.ascending && s.field_descending == 0 && s.marks.is_empty() && !s.marked_only;
        let no_filter = s.filters.is_empty();
        let group_zero = s.filters.iter().all(|f| f.group == 0);
        let mut c = copy.borrow_mut();
        match s.unpinned_buffer {
            None if plain_view && no_filter => {
                c.key = 0;
                c.adhoc = None;
            }
            Some(FLOWFIELD_KEY) if plain_view && no_filter => {
                c.key = 0;
                c.adhoc = None;
                c.unpinned_buffer = None;
            }
            None if plain_view && group_zero => {
                c.key = 0;
                c.adhoc = None;
                c.filters.clear();
                c.view_assumption = true;
            }
            None => c.unpinned_buffer = Some(BY_VALUE_VIEW),
            Some(_) => {}
        }
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
