//! A part whose own `CurrPage.Update` was asked while its view no longer started with the rows the client holds (al-runner
//! issue 8328: a host control's OnValidate calls a part procedure that copies filters onto the part's Rec and runs
//! `CurrPage.Update(false)`; on a host opened for edit no host step carries the refresh out, so the part kept answering from
//! the held rows). Which rows the part shows then is not pinned, so every test call that reads or writes a row of the part is
//! refused until the refresh runs.
//!
//! The answer is decided once, when the Update is asked (`note_part_update_left_view`, by `ask_update_refresh`), and kept in
//! `RefreshRequest::update_left_view` (`state/refresh.rs`) with the keys of the held rows; rows the test stores after that keep the answer from the held
//! rows, as before, and so does a step that read the part's rows again (a host action, the test's own `Filter.SetFilter`). It cannot
//! tell who changed the rows before the Update was asked: rows the test stored then refuse too.
//!
//! A pending refresh whose view still starts with the held rows keeps the answer from the held rows, as before: the corpus cases
//! that reach it pass with it (pages/parts/R6889RealShape.al:1346 PG6889RS_Step1_HostCallsPartSetEditable and the Step2* and
//! Step3 cases: the host's OnAfterGetCurrRecord calls `CurrPage.Lines.Page.SetEditable(true)`, whose `CurrPage.Update(false)`
//! still waits when the test calls `First()` on the part; testpage/ScLookupGlobalRecordField.al:644
//! TW_LookupOnValidateUpdatesThePart). A part that serves its rows through OnFindRecord or OnNextRecord is not compared: its
//! rows are not the view of its Rec.
//!
//! A step that read the rows again keeps main's answer only when the rows it read differ from the held ones. The test's own
//! `Filter.SetFilter` can read rows with the keys of the held rows (al-runner issue 8537: the host's OnAfterGetCurrRecord makes a
//! ListPart on a temporary table delete and insert its rows, each step ending with the part's `CurrPage.Update(false)`, and the
//! test filters the part to one day before and after). The calls a probe cell made after that filter answer from the rows it
//! read, as main does, under assumption `part-left-view-test-filter` (pages/parts/PartRefillTestFilter.al, corpus PR 1474,
//! merged, not pinned yet; its two-field key cell PartRefill_TypedKeyTriggerPartActionThenFilterFirst_FindsRefilledRow is in
//! corpus PR 1475, open): `TestFilterReread` admits each in turn (`measured_step_after_test_filter`). Every other call keeps the
//! refusal.

use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

/// The calls the test made on a part since its own `Filter.SetFilter` read the part's rows again while the refresh of
/// `update_left_view` waits (`PageState::left_view_test_filter`). Each step admits only the calls a probe cell made after it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum TestFilterReread {
    /// No such filter, or a call ended the steps.
    #[default]
    Off,
    /// The test's filter read the rows again.
    Filtered,
    /// Then `First()`.
    First,
    /// Then `Next()` after that `First()`.
    Next,
    /// Then `Previous()` after that `Next()`.
    Previous,
    /// Then `Last()`.
    Last,
}

impl Session {
    /// Sets the refresh request's `update_left_view` of part `st` to the keys of the held rows as its own `CurrPage.Update` is asked: the client holds
    /// rows read from the start of the view, the part reads its rows from that view, and the view now starts with other rows (or
    /// with more rows, when the held rows were the whole view).
    pub(super) fn note_part_update_left_view(&mut self, st: &St) -> Result<(), Signal> {
        st.borrow_mut().refresh.set_update_left_view(None);
        st.borrow_mut().left_view_test_filter = TestFilterReread::Off;
        let (held, short) = {
            let s = st.borrow();
            if s.host.is_none() || !s.open || !s.window_from_start || s.window.is_empty() {
                return Ok(());
            }
            let keys: Vec<Vec<Value>> = s.window.iter().map(|w| w.key.clone()).collect();
            (keys, s.window.len() < Self::WINDOW)
        };
        if !self.has_source(st) || self.declares(st, "onfindrecord") || self.declares(st, "onnextrecord") {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let used = self.ret_used;
        let view = self.view_rows(&rec, held.len() + usize::from(short));
        self.ret_used = used;
        let view = view?;
        let same = view.len() == held.len()
            && view
                .iter()
                .zip(&held)
                .all(|(row, key)| Self::same_key(&table.primary_key(row), key));
        if !same {
            st.borrow_mut().refresh.set_update_left_view(Some(held));
        }
        Ok(())
    }

    /// Refuses test call `m` on part `st` (on its control `target`, `None` for a TestPart call; `field_read`: a Value read)
    /// while the refresh its `update_left_view` Update asked for waits and the client still holds, from the start of the view,
    /// the rows it held then. A step that read other rows again or left them as not read from the start (a host action, the
    /// test's own `Filter.SetFilter`) keeps main's answer. Kept too: the calls `reads_no_row` lists, and after the test's own
    /// filter the calls `measured_step_after_test_filter` admits.
    pub(super) fn refuse_row_access_on_part_left_by_its_view(
        &self,
        st: &St,
        m: PageMethod,
        target: Option<&ControlTarget>,
        field_read: bool,
    ) -> Result<(), Signal> {
        let waits = {
            let s = st.borrow();
            s.refresh.asked_by_update()
                && s.open
                && s.window_from_start
                && s.refresh.update_left_view().is_some_and(|held| {
                    held.len() == s.window.len() && held.iter().zip(&s.window).all(|(k, w)| Self::same_key(k, &w.key))
                })
        };
        if !waits || self.reads_no_row(st, m, target)? {
            return Ok(());
        }
        if let Some(next) = self.measured_step_after_test_filter(st, m, target, field_read) {
            assume(Assumption::PartLeftViewTestFilter);
            st.borrow_mut().left_view_test_filter = next;
            return Ok(());
        }
        unsupported(format!(
            "{m}() on a part whose view no longer started with the rows it holds when its own CurrPage.Update was asked (which rows it shows then is not pinned)"
        ))
    }

    /// The test's own `Filter.SetFilter` on part `st` read its rows again (`filter.rs` `filter_changed`, `set_filter`) while the
    /// refresh of `update_left_view` waits: the steps of `TestFilterReread` start when `measured_reread` holds. Any other re-read
    /// (`Ascending`, `SetCurrentKey`, a filter outside the measured shape) ends them.
    pub(super) fn note_part_rows_read_by_test_filter(&self, st: &St, set_filter: bool) -> Result<(), Signal> {
        let waits = {
            let s = st.borrow();
            s.open && s.refresh.asked_by_update() && s.refresh.update_left_view().is_some()
        };
        let start = set_filter && waits && self.measured_reread(st)?;
        st.borrow_mut().left_view_test_filter = if start {
            TestFilterReread::Filtered
        } else {
            TestFilterReread::Off
        };
        Ok(())
    }

    /// The shape every cell of the probe shared (pages/parts/PartRefillTestFilter.al, merged in corpus PR 1474, the two-field key
    /// part in corpus PR 1475):
    /// - the part: PageType ListPart over a temporary source (`SourceTableTemporary`), shown directly on a top-level Worksheet
    ///   (:152 "LT PR Load Host", :248 "LT PR Load Bg Host", :403 "LT PR Load Trig Host") or List (:200 "LT PR Load List Host")
    ///   host;
    /// - its page triggers none (:90 "LT PR Load Part") or exactly OnOpenPage and OnAfterGetRecord (:323 "LT PR Load Trig Part",
    ///   PR 1475's "LT PR Typed Trig Part"), no page extension, no subscriber;
    /// - its Rec in the primary key, ascending, not MarkedOnly, with one filter: the test's, a single value (`Eq`) on the one
    ///   field of a one-field primary key, or on the second field of a two-field key on the part with those two triggers, in
    ///   group 0 (the part's own `Rec.Reset()` took any other).
    fn measured_reread(&self, st: &St) -> Result<bool, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        let Some(host) = host else {
            return Ok(false);
        };
        let host_type = engine.object_facts[host.borrow().object as usize].page_type.clone();
        let top_host = host.borrow().host.is_none()
            && matches!(host_type, Some(al_ir::PageType::Worksheet | al_ir::PageType::List));
        let list_part = engine.object_facts[object].page_type == Some(al_ir::PageType::Other("listpart".into()));
        let page = page_of(&engine, st);
        let mut triggers: Vec<&str> = page.triggers.iter().map(|(n, _)| n.as_str()).collect();
        triggers.sort_unstable();
        let trigger_part = match triggers.as_slice() {
            [] => false,
            ["onaftergetrecord", "onopenpage"] => true,
            _ => return Ok(false),
        };
        let code =
            page.extensions.is_empty() && page.ext_triggers.is_empty() && !engine.events.any_subscriber_of(object);
        if !(top_host && list_part && code) {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        // A one-field key; or, on the part with OnOpenPage and OnAfterGetRecord, a two-field key filtered on its second field
        // ("LT PR Typed Trig Part", as Base Application's Load Buffer: "Period Type", "Period Start").
        let key = match (table.keys[0].as_slice(), trigger_part) {
            ([key], _) | ([_, key], true) => *key,
            _ => return Ok(false),
        };
        let s = st.borrow();
        let r = rec.borrow();
        let filter = match s.test_filters.as_slice() {
            [t] if t.field == key => matches!(t.expr, al_builtins::filter::FilterExpr::Eq(_)),
            _ => false,
        };
        Ok(filter
            && r.temporary
            && r.key == 0
            && r.ascending
            && !r.marked_only
            && matches!(r.filters.as_slice(), [f] if f.field == key && f.group == 0))
    }

    /// The step call `m` on part `st` (on its control `target`) makes after the test's filter, when a probe cell made it there
    /// (PartRefillTestFilter.al of corpus PR 1474): AsDate and AsDecimal of a field control after any step (the `Fact` reads of
    /// every cell) and a Value read (:723 PartRefill_TriggerPartActionThenFilterFirst_FindsRefilledRow, `QtyText`); `First()`
    /// (:512 PartRefill_WorksheetActionThenFilterFirst_FindsRefilledRow) or `Last()` (:590
    /// PartRefill_WorksheetActionThenFilterLast_FindsRefilledRow) after the filter; `Next()` after that `First()` and
    /// `Previous()` after that `Next()` (:561 PartRefill_WorksheetActionThenFilterFirstNextPrevious_StayOnRefilledRow). `None`
    /// for any other call, a Value with an argument
    /// included.
    fn measured_step_after_test_filter(
        &self,
        st: &St,
        m: PageMethod,
        target: Option<&ControlTarget>,
        field_read: bool,
    ) -> Option<TestFilterReread> {
        use TestFilterReread as S;
        let step = st.borrow().left_view_test_filter;
        if step == S::Off {
            return None;
        }
        let field = match target {
            Some(ControlTarget::Control(idx)) => {
                page_of(&self.engine(), st).controls[*idx].kind == ir::ControlKind::Field
            }
            _ => false,
        };
        let read = field && (matches!(m, PageMethod::AsDate | PageMethod::AsDecimal) || field_read);
        match (step, m, target) {
            _ if read => Some(step),
            (S::Filtered, PageMethod::First, None) => Some(S::First),
            (S::Filtered, PageMethod::Last, None) => Some(S::Last),
            (S::First, PageMethod::Next, None) => Some(S::Next),
            (S::Next, PageMethod::Previous, None) => Some(S::Previous),
            _ => None,
        }
    }

    /// A test call on page `st` ended with an error: the steps after the test's filter end, so the calls after it keep the
    /// refusal (no cell raised an error there).
    pub(super) fn end_test_filter_steps_on_error<T>(st: &St, r: &Result<T, Signal>) {
        if r.is_err() {
            st.borrow_mut().left_view_test_filter = TestFilterReread::Off;
        }
    }

    /// The calls that answer without the part's current row: `Filter` and its `GetFilter` and `SetFilter`, `CurrentKey` and
    /// `Ascending` read the view, `GetField` only returns a handle. TestPart `Visible` and `Enabled` read the host's part
    /// control; TestPart `Caption` of a part without a DataCaptionExpression (its own or an extension's) is kept.
    /// A control's `Editable`, `Enabled` and `Visible` read that property of the control and of the controls around it, and
    /// its `Caption` reads its CaptionClass and that of its source field: kept only when each of them is absent or a literal,
    /// since an expression (`Rec.Name <> 'First'`, a variable OnAfterGetCurrRecord sets) runs on the current row.
    fn reads_no_row(&self, st: &St, m: PageMethod, target: Option<&ControlTarget>) -> Result<bool, Signal> {
        if matches!(
            m,
            PageMethod::Filter
                | PageMethod::GetFilter
                | PageMethod::SetFilter
                | PageMethod::CurrentKey
                | PageMethod::Ascending
                | PageMethod::GetField
        ) {
            return Ok(true);
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let idx = match target {
            None => {
                return Ok(match m {
                    PageMethod::Visible | PageMethod::Enabled => true,
                    PageMethod::Caption => super::effective_prop(&engine, object, "DataCaptionExpression")?.is_none(),
                    _ => false,
                });
            }
            Some(ControlTarget::Control(idx)) => *idx,
            Some(_) => return Ok(false),
        };
        let page = page_of(&engine, st);
        let fixed = |i: usize, name: &str| matches!(page.controls[i].prop(name), None | Some(ir::PropVal::Lit(_)));
        let up_the_chain = |name: &str| {
            let mut at = Some(idx);
            while let Some(i) = at {
                if !fixed(i, name) {
                    return false;
                }
                at = page.controls[i].parent;
            }
            true
        };
        Ok(match m {
            PageMethod::Editable => up_the_chain("editable"),
            PageMethod::Enabled => up_the_chain("enabled"),
            PageMethod::Visible => up_the_chain("visible"),
            PageMethod::Caption => {
                fixed(idx, "captionclass")
                    && self.control_field(st, idx).is_none_or(|f| {
                        f.properties
                            .iter()
                            .filter(|(k, _)| k.eq_ignore_ascii_case("captionclass"))
                            .all(|(_, v)| crate::recmeta::string_literal(v.trim()).is_some())
                    })
            }
            _ => false,
        })
    }
}
