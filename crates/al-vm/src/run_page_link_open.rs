//! Rec in the OnOpenPage of a page an action opens through `RunPageLink` ([`crate::records::RUN_PAGE_LINK_OPEN`] and
//! [`crate::records::RUN_PAGE_LINK_OPEN_FILLED`]): which record methods run there.

use crate::Session;
use crate::records::{RUN_PAGE_LINK_OPEN, RUN_PAGE_LINK_OPEN_FILLED, Rec, arg_pos};
use al_builtins::filter::FieldFilter;
use al_builtins::native::Signal;
use al_builtins::record_op::RecordOp;
use al_builtins::value::Value;

fn unsupported<T>(what: impl Into<String>) -> Result<T, Signal> {
    Err(Signal::Unsupported(what.into()))
}

impl Session {
    /// A record method on Rec in the OnOpenPage of a page an action opens through `RunPageLink`. Real BC fills the linked fields
    /// there and `GetFilters()` of a copy of Rec reads '' (corpus issue 712, writer QZ), while the runtime holds the link as a
    /// filter: every read of the buffer, the filters, the view or the rows refuses. What runs: `GetFilter` of one field and
    /// `FilterGroup` (pinned per filter group, handlers/TestActionRunPageLinkFilterGroup.al:119-131), `CurrentKey` and `Ascending`
    /// (pinned with a RunPageView sorting, handlers/TestPageActionRunPageView.al:121-134, asserted at :341), the filter and key
    /// setters, the table's names, and a `Get`, which ignores the filters and fills the buffer again.
    pub(crate) fn run_page_link_open_op(
        &mut self,
        method: RecordOp,
        rec: &Rec,
        args: &mut [Value],
    ) -> Result<Value, Signal> {
        let runs = matches!(
            method,
            RecordOp::Get
                | RecordOp::GetFilter
                | RecordOp::FilterGroup
                | RecordOp::CurrentKey
                | RecordOp::Ascending
                | RecordOp::Reset
                | RecordOp::SetCurrentKey
                | RecordOp::SetAscending
                | RecordOp::SetRange
                | RecordOp::SetFilter
                | RecordOp::FieldNo
                | RecordOp::SetLoadFields
                | RecordOp::AddLoadFields
                | RecordOp::SetAutoCalcFields
                | RecordOp::IsTemporary
                | RecordOp::TableName
                | RecordOp::TableCaption
                | RecordOp::LockTable
                | RecordOp::ReadIsolation
                | RecordOp::SecurityFiltering
        );
        // Rec itself holds the link as filters in filter group 0: GetFilters reads them, GetRangeMin and GetRangeMax of a linked
        // field the linked value (pages/lt-outside-worksheet-2/
        // RtLinkRange.al:275 RT_Lr_StatusAndCode_Trap_H1 `A:Order No.: H1, Status: Released`, `Min:H1;Max:H1`, :335 a Code-only
        // link, :354 const, :373 a filter(..) range `Min:H1;Max:H2`). Filters in another group, or several on the field, are not
        // pinned. The target's SourceTableView puts its filters in group 2 beside the link, and GetFilters in group 0 reads the
        // link alone (pages/W1RunObject.al:420 ViewTargetLink `id=0 filters=Grp: 2` on a Card with `where(Grp = const(1))`), and so
        // does a RunPageView beside the link in group 3 (pages/lt-y41-page-probes/Y41RunPageLinkOnOpen.al:553 `filters=Code: P1`).
        let link_read = match method {
            RecordOp::GetFilters => {
                let r = rec.borrow();
                r.filter_group == 0 && r.filters.iter().all(|f| matches!(f.group, 0 | 2 | 3))
            }
            RecordOp::GetRangeMin | RecordOp::GetRangeMax => {
                let pos = arg_pos(args, 0)?;
                let r = rec.borrow();
                let on_field: Vec<&FieldFilter> = r.filters.iter().filter(|f| f.field == pos).collect();
                matches!(on_field.as_slice(), [only] if only.group == 0)
            }
            _ => false,
        };
        // With the linked fields filled (`RUN_PAGE_LINK_OPEN_FILLED`), Rec answers like any record of that buffer and view:
        // Count() counts the linked rows of the view, Find() looks for the buffer's primary key (No for a List whose key is
        // blank, Yes on a Card linked on its own key, reading the row), FindFirst() and FindLast() move Rec to the first or
        // last linked row of the view, and leave the buffer as it was when there is none
        // (pages/lt-rz-outside-page-moves/SjRunPageLinkOpenFind.al:14-21, :314-334, :353-365, :397-425, :427-450).
        let filled = rec.borrow().unpinned_buffer == Some(RUN_PAGE_LINK_OPEN_FILLED);
        let find_read = filled
            && match method {
                RecordOp::Count | RecordOp::FindFirst | RecordOp::FindLast => args.is_empty(),
                RecordOp::Find => match args {
                    [] => true,
                    [Value::Text(w)] => w.as_str() == "=",
                    _ => false,
                },
                // CalcFields uses the FlowFilter the link set (pages/lt-y41-page-probes/Y41RunPageLinkOnOpen.al:458
                // RPL_List_FlowFilterLink `cf=P1 hostcount=1`), and a temporary Rec inserts a row, also one outside the link
                // (:476 RPL_TempList_InsertInOpen `insert blank=Yes`, then `count=1` inside the link), with Init between the inserts.
                RecordOp::CalcFields => true,
                RecordOp::Insert | RecordOp::Init => rec.borrow().temporary && args.is_empty(),
                _ => false,
            };
        if !runs && !link_read && !find_read {
            let why = rec.borrow().unpinned_buffer.unwrap_or(RUN_PAGE_LINK_OPEN);
            return unsupported(format!("Rec.{}() {why}", method.name()));
        }
        let flag = rec.borrow_mut().unpinned_buffer.take();
        let r = self.record_op_inner(method, rec, args);
        let refilled = matches!(
            method,
            RecordOp::Get | RecordOp::Find | RecordOp::FindFirst | RecordOp::FindLast
        );
        if !(refilled && matches!(r, Ok(Value::Boolean(true)))) {
            rec.borrow_mut().unpinned_buffer = flag;
        } else if find_read && matches!(method, RecordOp::FindFirst | RecordOp::FindLast) {
            self.pages.run_page_link_open_moved = true;
        }
        r
    }
}
