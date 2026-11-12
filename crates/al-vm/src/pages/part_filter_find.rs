//! A ListPart that serves its rows from OnFindRecord and OnNextRecord, after a TestPart filter (al-runner issue 7476): the
//! client reads the rows again from the first row of the view, and the function controls' first repeated read of each window of a part. The function controls of such a
//! read are `fn_controls.rs`.

use super::pinned_shape::{PartStep, PinnedPart, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;

/// The pages of the pin (PartFilterExpressionControl.al, host 62656 and parts 62653, 62655), see `part_pages_are`: a ListPlus
/// host whose only trigger is OnAfterGetRecord (:278), with fields of Rec and several parts (:261..:270); a ListPart with
/// `Editable = false` (:103) that declares OnFindRecord, OnNextRecord, OnAfterGetRecord, OnAfterGetCurrRecord and, one of
/// them, OnOpenPage (:126); controls over fields of Rec, expressions and page variables, none with a trigger (:111..:114); no
/// SubPageLink (:261) or `"Item No." = field("No.")` (:273), no view; no filter but the link's and filter group 0 (the test's
/// filter; the Periods part sets its own there in OnFindRecord, :135); no page extension, table extension or subscriber.
const FIND_PAGES: PinnedPart = PinnedPart {
    host_type: ir::PageType::ListPlus,
    host_triggers: &["onaftergetrecord"],
    part_triggers: &[
        "onopenpage",
        "onfindrecord",
        "onnextrecord",
        "onaftergetrecord",
        "onaftergetcurrrecord",
    ],
    part_properties: &["editable"],
    part_variable_controls: true,
    part_variable_validate: false,
    part_field_validate_update: false,
    part_expression_controls: true,
    unlinked: true,
    sibling_parts: true,
    user_filter: true,
};

/// The rule that asks `Session::find_part_calls`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FindPartAt {
    /// The read after `Filter.SetFilter` (`filter_changed`).
    FilterLanding,
    /// `First()` that reads nothing (`end_move_no_read.rs`).
    First,
    /// `Last()` that reads nothing.
    Last,
    /// The host's `GoToKey` moved: a part without a link is not read again (`parts.rs`).
    HostGoToKey,
    /// The host's `Next` moved: the open read of the part starts at the view's start (`open_read_steps_back`).
    HostNext,
}

impl Session {
    /// A ListPart with OnFindRecord, OnNextRecord, OnAfterGetRecord and OnAfterGetCurrRecord reads its rows again after a
    /// TestPart filter with OnFindRecord(`-`), then OnNextRecord(1) to the end of the view or of the window, then the tail of an
    /// open read without the step back of OnNextRecord(-1) (testpage/parts/PartFilterExpressionControl.al:356
    /// Periods_FilterOneDay_First_Read `filter log` `[P.find(-)=1 P.agr=2 calc=5 P.next=1 P.find(=)=1 P.agcr=1 ]`, :414
    /// Periods_FilterThreeDays `row 1 log` `P.next=3`, :493 Periods_FilterThenClearFilter_Read `cleared log` a full window
    /// `P.next=19`, :676 EntriesFind_FilterOneDay_First_Read, :732 EntriesFind_FilterTwoDays_FirstNextNext). A top-level list
    /// and a part without one of the four triggers are not pinned and keep `=><` (`page_first_row`).
    /// The same part after its host moved reads `=><` and steps back with OnNextRecord(-1), which answers 0 on the first row
    /// of the view (:758 EntriesFind_FilterOneDay_HostMovesToNextItem `I3 no move log` `F.find(=><)=1 F.agr=2 F.next=2`): that window
    /// starts at the view's start too.
    ///
    /// Every fact the corpus pins is required (`FIND_PAGES`, the `TP PF Host` page, PartFilterExpressionControl.al:251): the
    /// test opened the ListPlus host with OpenView (`OpenHost`, :334); the part declares the four triggers and reads in its
    /// primary key, ascending. The rules that ask this also require the test's calls of their pin (`find_part_calls`). Any
    /// other host, open mode, part or call keeps `=><` and the step back.
    pub(super) fn find_list_part(&self, st: &St) -> Result<bool, Signal> {
        let plain_order = {
            let rec = self.page_rec(st);
            let r = rec.borrow();
            r.key == 0 && r.ascending
        };
        Ok(Self::part_host_opened(st) == Some((true, PageMode::View, false, false))
            && plain_order
            && [
                "onfindrecord",
                "onnextrecord",
                "onaftergetrecord",
                "onaftergetcurrrecord",
            ]
            .iter()
            .all(|t| self.declares(st, t))
            && self.part_pages_are(st, &FIND_PAGES)?)
    }

    /// Whether the test's calls on the host and the part since the host opened, the running one last, are the ones `at` is
    /// pinned for (reads of the part's fields and of its filter stand between them). After `OpenHost`'s `GoToKey` (:338):
    /// - `FilterLanding`: `Filter.SetFilter` as the first call on the part (:361, :681), or after one `SetFilter` and `First`
    ///   (:501, :810 the filter cleared again);
    /// - `First`: after the landing (:363, :683, :718), after the landing, `First` and the landing of a second filter (:812),
    ///   or after the landing, `First` and a host `Next` that moved (:461, :769);
    /// - `Last`: after the landing, `First`, one or more `Next` that moved and one that answered No (:428);
    /// - `HostGoToKey`: the host's `GoToKey` itself, nothing on the part yet (:338);
    /// - `HostNext`: the host's `Next` after the landing and `First` (:766).
    ///
    /// A call on another part of the host, any other call, or a write to the store between two calls is not pinned.
    pub(super) fn find_part_calls(st: &St, at: FindPartAt) -> bool {
        use PageMethod as M;
        use PartStep::{Host, Own, OwnNo};
        use TestCall::{Other, Page};
        const GO: PartStep = Host(Page(M::GoToKey));
        const FILTER: PartStep = Own(Other(M::SetFilter));
        const FIRST: PartStep = Own(Page(M::First));
        let Some(steps) = Self::part_steps(st) else {
            return false;
        };
        match at {
            FindPartAt::FilterLanding => matches!(&steps[..], [GO, FILTER] | [GO, FILTER, FIRST, FILTER]),
            FindPartAt::First => matches!(
                &steps[..],
                [GO, FILTER, FIRST]
                    | [GO, FILTER, FIRST, FILTER, FIRST]
                    | [GO, FILTER, FIRST, Host(Page(M::Next)), FIRST]
            ),
            FindPartAt::Last => match &steps[..] {
                [GO, FILTER, FIRST, moved @ .., OwnNo(Page(M::Next)), Own(Page(M::Last))] => {
                    !moved.is_empty() && moved.iter().all(|c| *c == Own(Page(M::Next)))
                }
                _ => false,
            },
            FindPartAt::HostGoToKey => matches!(&steps[..], [GO]),
            FindPartAt::HostNext => matches!(&steps[..], [GO, FILTER, FIRST, Host(Page(M::Next))]),
        }
    }

    /// A ListPart on a ListPlus host opened for view, neither page with a page extension or an event subscriber (the host and the
    /// parts of PartFilterExpressionControl.al:251).
    fn part_on_list_plus_view_host(&self, st: &St) -> bool {
        let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return false;
        };
        let engine = self.engine();
        let page_type = |p: &St| engine.object_facts[p.borrow().object as usize].page_type.clone();
        page_type(&host) == Some(ir::PageType::ListPlus)
            && host.borrow().mode == PageMode::View
            && page_type(st) == Some(ir::PageType::Other("listpart".into()))
            && self.unextended_unsubscribed(&host)
            && self.unextended_unsubscribed(st)
    }

    /// `find_list_part` with a TestPart filter on the part (filter group 0, `filter.rs`): the open read after a host move then
    /// starts at the view's start (:758 EntriesFind_FilterOneDay_HostMovesToNextItem, the filter `03/16/25` still set). Without a
    /// filter the open read is not pinned to do so and keeps the step back.
    fn find_list_part_filtered(&self, st: &St) -> Result<bool, Signal> {
        Ok(self.page_rec(st).borrow().filters.iter().any(|f| f.group == 0)
            && Self::find_part_calls(st, FindPartAt::HostNext)
            && self.find_list_part(st)?)
    }

    /// A ListPart that a TestPart filter left on no row (pinned for the ListPlus host opened for view of `find_list_part`, where the part shows no blank line; for edit it is not pinned) runs its function
    /// controls once after OnAfterGetCurrRecord there, with OnFindRecord or without (PartFilterExpressionControl.al:629 Entries_FilterAdmitsNoRow_First `filter log`
    /// `[E.agcr=1 calc=1 ]`, :785 EntriesFind_FilterAdmitsNoRow_First `[F.find(-)=1 F.agcr=1 calc=1 ]`). Nothing reads the
    /// control there, so what it shows is not noted.
    pub(super) fn part_filter_runs_fn_on_new_line(&self, st: &St) -> bool {
        st.borrow().cursor != Cursor::Row && self.part_on_list_plus_view_host(st)
    }

    /// A part starts to read a window. Each window a part reads runs no function control at its first repeated
    /// OnAfterGetRecord, not only the open's (`fn_controls.rs`; PartFilterExpressionControl.al:732
    /// EntriesFind_FilterTwoDays_FirstNextNext `F.agr=4 calc=4` after a TestPart filter, :576 Entries_FilterTwoDays `E.agr=4
    /// calc=4`, :450 Periods_FilterOneDay_HostMovesToNextItem `I3 no move log` `E.agr=4 calc=8 ... F.agr=4` after the host moved).
    pub(super) fn part_window_read_starts(&self, st: &St) {
        let mut s = st.borrow_mut();
        if s.host.is_some() {
            s.fn_open_skipped = false;
        }
        s.find_window_at_view_start = s.landed_from_view_start;
    }

    /// The step back of the open read of a list that serves its rows from OnNextRecord: OnNextRecord(-1) after the first row
    /// is put back; whether it moved. A window found with OnFindRecord(`-`) after a TestPart filter starts at the first row of
    /// the view and asks nothing (`find_list_part`; :356 `P.next=1` for a view of one row).
    pub(super) fn open_read_steps_back(&mut self, st: &St, moved_open: bool) -> Result<bool, Signal> {
        let from_start = st.borrow().landed_from_view_start;
        let stepped_back = !from_start && self.declares(st, "onnextrecord") && self.nav_next(st, -1)?;
        if !stepped_back && !moved_open && self.find_list_part_filtered(st)? {
            st.borrow_mut().find_window_at_view_start = true;
        }
        Ok(stepped_back)
    }
}
