//! What Rec of a ListPart that shows no row holds once the part has read its (empty) view: the values its SubPageLink pins
//! (al-runner issue 6924; Base Application page 1178 "Doc. Attachment List Factbox" exits its OpenInDetail action on
//! `Rec."Table ID" = 0`).
//!
//! Measured on BC 27.5 and 28.5 by corpus pull request 1335, pages/parts/EmptyPartRecLink.al (an action of the part reads Rec):
//! a `const(..)`, a `field(..)` and a `filter(..)` of one value put their value on Rec, a `filter(..)` of a range does not
//! (:373 const + field `act=75160/A/0/`, :433 field only `act=0/A/0/`, :448 const only `act=75160//0/`, :463 filter of one
//! value, :478 filter of a range `act=0/A/0/`); in a FactBox and in the content area (:403, :418), on a Card and on a Document
//! host (:558, :573, :588), opened for view or for edit (:388), after GoToRecord of a host opened for view to a row without
//! lines (:509), on the new row of OpenNew (:528 `act=75160//0/`, the host's No. is blank), and again at a second action
//! (:634). The fields outside the link stay blank. The part's OnAfterGetCurrRecord sees the same values
//! (pages/parts/SelectionFilterLinkedPartNoRow.al:153).
//!
//! Only that shape is changed. Every link field is a key field and the link is the part's only filter; the part has no
//! SourceTableView, no PopulateAllFields and no OnNewRecord (trigger or subscriber); the host is a top-level Card or Document
//! page; the part lands as the host opens or after a GoToRecord of a host opened for view. The values are those
//! `stamp_from_filters` puts on a new line, which in that shape are exactly the link's single values on key fields. A part of
//! a top-level List host outside the cells gets the fields its filters name unknown, and a read of one is refused (al-runner
//! issue 6924: the blank record there was a wrong result on a List host whose part control has UpdatePropagation = Both).
//! Every other part keeps the blank record it had before (`suite/empty_part_rec_link.rs`
//! `empty_list_part_rec_outside_the_measured_shape`).
//!
//! The List host (al-runner issue 6924, comment 6822249976: the reporter's host is a List page) runs through the assumption
//! `empty-part-rec-list-host` until corpus pull request 1339 (pages/parts/EmptyPartRecLinkList.al) is pinned; its allowlist
//! is `empty_part_rec_list.rs`. There the landing leaves the fields the part's filters name unknown, and Rec holds the values
//! only while a read the cell measured runs: a test Invoke of an action of the part right after the landing
//! (`Session::reading_empty_list_rec`).

use super::parts::LINK_GROUP;
use super::pinned_shape::{PartStep, TestCall};
use super::*;
use al_builtins::assumptions::{self, Assumption};
use al_builtins::page_op::PageMethod;

impl Session {
    /// Rec of part `st`, which shows no row, is the blank record with the single values of its SubPageLink on it (see the
    /// module). On a top-level List host outside the measured cells, what BC puts on the fields the part's filters name is not
    /// known: each of them holds a `Value::Unknown` whose read is refused, with the first fact that left the cells (al-runner
    /// issue 6924: the blank record was a wrong result there). The other fields are blank, as in every measured cell
    /// (EmptyPartRecLink.al:373 `act=75160/A/0/`, the Line No. outside the link). A part of another host outside the measured
    /// shape keeps the blank record: on a ListPlus host, R6889RealShape.al:1405 (the part's action reads the link field and
    /// falls back to its filter) and TestStandardEmptyPages.Codeunit.al:33 (page 380's AutoFormatExpression reads it) pass
    /// through it on BC.
    pub(super) fn empty_part_rec(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let table_no = rec.borrow().table as usize;
        let fresh = self.initial_fields(table_no);
        rec.borrow_mut().fields = fresh;
        st.borrow_mut().empty_list_landing = None;
        let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(());
        };
        match self.empty_part_rec_shape(st, &host)? {
            Shape::Measured => self.stamp_from_filters(st, false)?,
            Shape::ListCell(reads) => {
                // The values are on Rec only inside a measured read (`reading_empty_list_rec`).
                unknown_filtered_fields(&rec, OUTSIDE_READ);
                st.borrow_mut().empty_list_landing = Self::part_steps(st).map(|steps| EmptyListLanding {
                    steps: steps.len(),
                    reads,
                    store: self.landing_store_state(),
                    reading: false,
                });
            }
            Shape::Miss(miss) if self.top_list(&host) => {
                let why = format!(
                    "a read of Rec of a part that shows no row, on a field its filters name, on a List host outside the cells \
                     BC was measured in ({miss}): whether BC puts the filter's value on Rec is not pinned"
                );
                unknown_filtered_fields(&rec, &why);
            }
            Shape::Miss(_) => {}
        }
        Ok(())
    }

    /// Runs `f`, the test's call `call` on a control of page `st`. When `st` is a part that landed in a List-host cell
    /// (`empty_part_rec`) and the call is a read the cells measured (`measured_read`), Rec holds the link's single values
    /// while `f` runs, through the assumption `empty-part-rec-list-host`; before and after, the fields the part's filters name
    /// are unknown and a read of one is refused. So host code, a control's source expression, a trigger outside such a call
    /// and any later call cannot read the values.
    pub(super) fn reading_empty_list_rec<T>(
        &mut self,
        st: &St,
        call: Option<TestCall>,
        f: impl FnOnce(&mut Self) -> Result<T, Signal>,
    ) -> Result<T, Signal> {
        let Some(landing) = st.borrow().empty_list_landing else {
            return f(self);
        };
        if landing.reading || !self.measured_read(st, &landing, call) {
            return f(self);
        }
        assumptions::assume(Assumption::EmptyPartRecListHost);
        if let Some(l) = st.borrow_mut().empty_list_landing.as_mut() {
            l.reading = true;
        }
        let r = self.link_values_on_rec(st).and_then(|()| f(self));
        // A landing inside `f` withheld the values itself and is not `reading`; a part that read a row holds no landing.
        let still_reading = st.borrow().empty_list_landing.is_some_and(|l| l.reading);
        if still_reading && st.borrow().cursor == Cursor::Row {
            st.borrow_mut().empty_list_landing = None;
        } else if still_reading {
            if let Some(l) = st.borrow_mut().empty_list_landing.as_mut() {
                l.reading = false;
            }
            unknown_filtered_fields(&self.page_rec(st), OUTSIDE_READ);
        }
        r
    }

    /// Whether the test's call `call` is a read of Rec the List-host cells measured after `landing` of part `st`
    /// (EmptyPartRecLinkList.al at commit 831ae544). Every fact is required as the cells hold it:
    /// - the call is an Invoke of an action of the part, and every step on the host and its parts since the landing is such an
    ///   Invoke, up to the reads the cell measured (:1800-2031 OpenEdit or OpenView, at most one GoToRecord, then
    ///   `P.Box.ReadRec.Invoke()`; :1370 the same read-only action twice; a read of the part's controls or of its filter is no
    ///   step, `Session::part_steps`);
    /// - the store is as the landing found it (`landing_store_state`), and no call on the host or its parts since the host
    ///   opened has raised (no cell writes, rolls back or fails between the landing and its read; the probe's action only
    ///   appends to a codeunit's text, :630).
    fn measured_read(&self, st: &St, landing: &EmptyListLanding, call: Option<TestCall>) -> bool {
        let read = TestCall::Other(PageMethod::Invoke);
        let (at, last) = (landing.steps, landing.steps + landing.reads);
        let measured_steps = call == Some(read)
            && Self::part_steps(st).is_some_and(|s| {
                s.len() > at && s.len() <= last && s[at..].iter().all(|step| *step == PartStep::Own(read))
            });
        let none_raised = || Self::tree_root(st).0.borrow().tree_calls.iter().all(|c| !c.failed);
        measured_steps && landing.store == self.landing_store_state() && none_raised()
    }

    /// The fields the filters of part `st` name hold what a part that shows no row holds on Rec in the measured shape: the
    /// table's blank value, then the single values of the link (`stamp_from_filters`; see the module).
    fn link_values_on_rec(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let table_no = rec.borrow().table as usize;
        let fresh = self.initial_fields(table_no);
        {
            let mut r = rec.borrow_mut();
            let filtered: Vec<usize> = r.filters.iter().map(|f| f.field).collect();
            for f in filtered {
                r.fields[f] = fresh[f].clone();
            }
        }
        self.stamp_from_filters(st, false)
    }

    /// Part `st` reads its view again, or its page code calls a method of the page: the landing of a List-host cell it held is
    /// over. Inside a measured read (`reading_empty_list_rec`) the values leave Rec with it.
    pub(super) fn end_empty_list_landing(&mut self, st: &St) {
        if st.borrow_mut().empty_list_landing.take().is_some_and(|l| l.reading) {
            unknown_filtered_fields(&self.page_rec(st), OUTSIDE_READ);
        }
    }

    /// What tells that the store is as a landing found it: the store's own change counter (every write to a stored table
    /// moves it, by whatever call), the write mark (it also moves with a write to a temporary table) and the count of
    /// rollbacks (a rollback takes rows back and moves neither).
    fn landing_store_state(&self) -> (u64, (i64, u64), u64) {
        (self.store.change_counter(), self.write_mark(), self.rollbacks)
    }

    /// Runs `f` (a host's parts reading again) with `host_move_running` set to `m`, and restores it after, on an error too.
    pub(super) fn during_host_move<T>(
        &mut self,
        m: HostMove,
        f: impl FnOnce(&mut Self) -> Result<T, Signal>,
    ) -> Result<T, Signal> {
        let before = self.host_move_running.replace(m);
        let r = f(self);
        self.host_move_running = before;
        r
    }

    /// The shape of part `st` of `host` (see the module).
    fn empty_part_rec_shape(&mut self, st: &St, host: &St) -> Result<Shape, Signal> {
        let (host_row, host_view) = {
            let h = host.borrow();
            let row = (h.cursor == Cursor::Row && !h.draft) || (h.draft && !h.draft_stored && h.draft_from_open);
            (row, h.mode == PageMode::View)
        };
        if !host_row {
            return Ok(Shape::Miss("the host stands on no stored row"));
        }
        let goto_record = self.host_move_running == Some(HostMove::moved(PageMethod::GoToRecord));
        let card_shape = (self.part_open_running || (goto_record && host_view)) && self.top_card_or_document(host);
        let list_cell = self.list_host_cell(st, host)?;
        if !card_shape && let Err(miss) = list_cell {
            return Ok(Shape::Miss(miss));
        }
        if !(self.has_source(st) && self.has_repeater(st) && self.part_linked(st)) {
            return Ok(Shape::Miss(
                "the part has no source table, no repeater or no SubPageLink",
            ));
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let own_view = engine.program.objects[object].properties.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("SourceTableView") || (k.eq_ignore_ascii_case("PopulateAllFields") && is_true(v))
        });
        if own_view || self.declares(st, "onnewrecord") || engine.events.is_subscribed(object, "onnewrecordevent") {
            return Ok(Shape::Miss(
                "the part has a SourceTableView, PopulateAllFields or an OnNewRecord",
            ));
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let r = rec.borrow();
        let measured = r
            .filters
            .iter()
            .all(|f| f.group == LINK_GROUP && table.keys[0].contains(&f.field));
        if !measured {
            return Ok(Shape::Miss(
                "a filter of the part that is not its SubPageLink on a key field",
            ));
        }
        let Ok(reads) = list_cell else {
            return Ok(Shape::Measured);
        };
        // A test whose part landed in a cell is flagged, whether or not it reads the values.
        assumptions::assume(Assumption::EmptyPartRecListHost);
        Ok(Shape::ListCell(reads))
    }
}

/// The landing of part at a List-host cell (`Session::empty_part_rec`), with what `Session::measured_read` compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmptyListLanding {
    /// The number of steps on the host and its parts (`Session::part_steps`) up to the landing.
    steps: usize,
    /// The number of reads by an action of the part the landing's cell measured.
    reads: usize,
    /// The store's state at the landing (`Session::landing_store_state`).
    store: (u64, (i64, u64), u64),
    /// A measured read runs: Rec holds the link values (`Session::reading_empty_list_rec`).
    reading: bool,
}

/// What `empty_part_rec` puts on Rec of a part that shows no row.
enum Shape {
    /// The pinned Card or Document shape: the link's single values.
    Measured,
    /// A List-host cell (`empty_part_rec_list.rs`): the link's single values, through the assumption, only inside the reads the
    /// cell measured, with this many reads by an action of the part (`Session::reading_empty_list_rec`).
    ListCell(usize),
    /// Outside both, with the first fact that left them.
    Miss(&'static str),
}

/// The refusal of a read of a List-host cell's link fields outside the reads the cells measured (`Session::measured_read`).
const OUTSIDE_READ: &str = "a read of Rec of a part that shows no row, on a field its filters name, on a List host outside the \
     reads the List-host cells measured (an action of the part the test invokes right after the landing, with no other call, \
     write to the store, rollback or error since): whether BC keeps the filter's value on Rec there is not pinned";

/// The fields the filters of `rec` name hold a `Value::Unknown` whose read is refused with `why`.
fn unknown_filtered_fields(rec: &Rec, why: &str) {
    let why: std::sync::Arc<str> = why.into();
    let mut r = rec.borrow_mut();
    let filtered: Vec<usize> = r.filters.iter().map(|f| f.field).collect();
    for f in filtered {
        r.fields[f] = Value::Unknown(why.clone());
    }
}
