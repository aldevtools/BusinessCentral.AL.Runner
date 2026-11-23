//! A data item's `DataItemTableView`: the filters it gives the item's record in filter group [`ITEM_VIEW_GROUP`], and the
//! one question about them another module asks. A record handed by value from its data item's `OnAfterGetRecord` while
//! it still holds exactly those filters (`Session::own_item_view_handed`, by `records/by_value_view.rs`) is the shape
//! corpus PR 1450 measured: BC hands the copy none of them (record/filters/ByValueRecordView.al codeunit 78731
//! `ByValue_ReportProcedure_DataItemView` :1675 and `_DataItemViewToCodeunit` :1690), also read descending (corpus PR 1544,
//! record/filters/ByValueDataItemView.al:220-261, al-runner issue 8838). A filter
//! that code sets in that group by number, the view of an XmlPort (the same group number) and a copy of the record are not
//! that shape, so the question needs this record of what the run set, not the group number.

use super::*;
use al_builtins::filter::FieldFilter;
use al_builtins::value::RecordState;
use std::rc::Weak;

/// The `DataItemTableView` filters one run gave the record of one of the report's own data items
/// (`ReportState::item_views`).
pub(crate) struct ItemView {
    rec: Weak<RefCell<RecordState>>,
    /// The instance of the report the run executes: the record holds the view only while that run is the innermost one.
    report: Weak<RefCell<Instance>>,
    /// The report's object index and the data item's `OnAfterGetRecord`, the one caller the cells measure.
    object: usize,
    on_after_get: Option<usize>,
    filters: Vec<FieldFilter>,
}

/// Whether `a` and `b` are the same filters, in order: field, group and expression.
fn same_filters<'a>(a: impl Iterator<Item = &'a FieldFilter>, b: &[FieldFilter]) -> bool {
    let a: Vec<&FieldFilter> = a.collect();
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| x.field == y.field && x.group == y.group && x.expr.identical(&y.expr))
}

impl Session {
    /// Puts the `DataItemTableView` of data item `r` of `run` in force on its record, and notes the filters it gave a data item
    /// of the report's own ([`ItemView`]).
    pub(super) fn apply_item_view(&mut self, run: &Run, r: ItemRef, zero_date_pinned: bool) -> Result<(), Signal> {
        let Some(view) = run.item(r).view.as_deref().map(crate::views::property_view) else {
            return Ok(());
        };
        let rec = self.report_item_record(run, r)?;
        let t = self.table(&rec)?;
        let blank_dates = crate::views::blank_date_conditions(&t, &view);
        if !blank_dates.is_empty() {
            // `where(Date field = const(0D))` compiles, but the run fails before OnPreReport and before any data item
            // trigger: Evaluate of the date '' (streams/CjDataset.al:190-211,
            // ZeroDateView_ConstZeroDate_SaveAsXml; record/P136Report.al:331-351,
            // BN3_Report_DataItemTableViewConstZeroDate). Pinned for a report with no request page and no
            // OnInitReport; where the error falls against those, and `filter(0D)`, are not pinned.
            if blank_dates.iter().any(|(_, kind)| kind != "CONST") {
                return unsupported("DataItemTableView filter(0D) on a Date field (not pinned)");
            }
            if !zero_date_pinned || self.report_has_init_or_request_page(run) {
                return unsupported(
                    "DataItemTableView const(0D) on a Date field in a report with a request page or OnInitReport (where the error falls is not pinned)",
                );
            }
            self.report.zero_date_view_failed = true;
            return Err(Signal::Error(Box::new(al_builtins::native::AlError {
                message: "The format of the Date '''' does not match your device's date settings.\\\\Make sure that the day, month, and year are in the correct order and use the correct separator (such as - or /). Alternatively, you can always use the format 'yyyy-mm-dd'.".into(),
                code: "Evaluate".into(),
                ..Default::default()
            })));
        }
        // A view that only sorts leaves the filters a SetTableView gave the item in force
        // (reports/TestReportRuntimeContracts.al:393-410).
        let kept = Some(rec.borrow().filters.clone());
        let clauses = crate::views::view_clauses(&view);
        // A view that only filters keeps the key and direction a SetTableView gave the item (corpus
        // reports/dataset/ReportSetTableViewOverWhere.al:P163_SetTableView_OverWhere_KeyOfTheView: key Txt, 4|3|2).
        let kept_order = {
            let r = rec.borrow();
            (r.key, r.adhoc.clone(), r.ascending, r.field_descending)
        };
        // The filters of a DataItemTableView are the report's, not the record's: GetFilters does not show them, in
        // OnPostReport either (reports/TestProbeReportContracts.al:TriggerOrder_NestedAndSecondItem).
        self.set_view_in(&rec, &t, &view, Some(ITEM_VIEW_GROUP), false, false, false)?;
        if !clauses.sorting {
            let mut r = rec.borrow_mut();
            (r.key, r.adhoc, r.ascending, r.field_descending) = kept_order;
        }
        if let Some(mut filters) = kept {
            // A view that filters adds its own filters to the ones a SetTableView gave the item.
            if clauses.filters {
                filters.extend(
                    rec.borrow()
                        .filters
                        .iter()
                        .filter(|f| f.group == ITEM_VIEW_GROUP)
                        .cloned(),
                );
            }
            rec.borrow_mut().filters = filters;
        }
        if let ItemRef::Base(i) = r {
            let filters = rec
                .borrow()
                .filters
                .iter()
                .filter(|f| f.group == ITEM_VIEW_GROUP)
                .cloned()
                .collect();
            let views = &mut self.report.item_views;
            views.retain(|v| v.rec.strong_count() > 0 && !std::ptr::eq(v.rec.as_ptr(), Rc::as_ptr(&rec)));
            views.push(ItemView {
                rec: Rc::downgrade(&rec),
                report: Rc::downgrade(&run.base),
                object: run.object,
                on_after_get: run.report().items[i].on_after_get,
                filters,
            });
        }
        Ok(())
    }

    /// Whether `rec`, handed by value now, is the record of a data item of the innermost running report, handed from that
    /// item's own `OnAfterGetRecord`, and holds in filter group [`ITEM_VIEW_GROUP`] exactly the filters the run's
    /// `DataItemTableView` gave it. The cells hand it from there (record/filters/ByValueRecordView.al:1675 and :1690 of corpus
    /// PR 1450, record/filters/ByValueDataItemView.al:220, :234 and :248 of corpus PR 1544).
    pub(crate) fn own_item_view_handed(&self, rec: &Rec) -> bool {
        let Some(v) = self
            .report
            .item_views
            .iter()
            .find(|v| std::ptr::eq(v.rec.as_ptr(), Rc::as_ptr(rec)))
        else {
            return false;
        };
        let innermost = self
            .report
            .report_running
            .last()
            .is_some_and(|run| std::ptr::eq(run.inst.as_ptr(), v.report.as_ptr()) && v.report.strong_count() > 0);
        let from_trigger = self.frames.last() == Some(&v.object)
            && v.on_after_get.is_some()
            && self.call_stack.last().map(|f| f.routine) == v.on_after_get;
        innermost
            && from_trigger
            && same_filters(
                rec.borrow().filters.iter().filter(|f| f.group == ITEM_VIEW_GROUP),
                &v.filters,
            )
    }
}
