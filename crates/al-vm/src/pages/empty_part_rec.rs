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
//! `stamp_from_filters` puts on a new line, which in that shape are exactly the link's single values on key fields. Every
//! other part keeps the blank record it had before (`suite/empty_part_rec_link.rs`
//! `empty_list_part_rec_stays_blank_outside_the_measured_shape`).
//!
//! One List-host cell runs through the assumption `empty-part-rec-list-host` (the outside report of issue 6924; probe
//! pages/parts/EmptyPartRecLinkList.al): the same part shape in the FactBoxes area of a top-level List page opened for edit
//! and moved by GoToRecord, linked by one const (or one-value filter) and one field. Every other List-host part keeps the
//! blank record (`suite/empty_part_rec_list.rs`).

use super::parts::LINK_GROUP;
use super::*;
use al_builtins::assumptions::{self, Assumption};

impl Session {
    /// Rec of part `st`, which shows no row, is the blank record with the single values of its SubPageLink on it (see the
    /// module). A part outside the measured shape gets the blank record, as before.
    pub(super) fn empty_part_rec(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let table_no = rec.borrow().table as usize;
        let fresh = self.initial_fields(table_no);
        rec.borrow_mut().fields = fresh;
        if self.empty_part_rec_measured(st)? {
            self.stamp_from_filters(st, false)?;
        }
        Ok(())
    }

    /// Whether `st` is a part of the measured shape, or of the assumed List-host cell (`list_host_cell`; see the module).
    fn empty_part_rec_measured(&self, st: &St) -> Result<bool, Signal> {
        let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(false);
        };
        let (host_row, host_view) = {
            let h = host.borrow();
            let row = (h.cursor == Cursor::Row && !h.draft) || (h.draft && !h.draft_stored && h.draft_from_open);
            (row, h.mode == PageMode::View)
        };
        let card_landing = self.part_open_running || (self.host_goto_record_running && host_view);
        let card_shape = card_landing && self.top_card_or_document(&host);
        let list_shape = self.list_host_cell(st, &host);
        if !(host_row
            && (card_shape || list_shape)
            && self.has_source(st)
            && self.has_repeater(st)
            && self.part_linked(st))
        {
            return Ok(false);
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let own_view = engine.program.objects[object].properties.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("SourceTableView") || (k.eq_ignore_ascii_case("PopulateAllFields") && is_true(v))
        });
        if own_view || self.declares(st, "onnewrecord") || engine.events.is_subscribed(object, "onnewrecordevent") {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let r = rec.borrow();
        let measured = r
            .filters
            .iter()
            .all(|f| f.group == LINK_GROUP && table.keys[0].contains(&f.field));
        if !list_shape {
            return Ok(measured);
        }
        // The List-host cell: each link filter admits one value (`const(..)` and `filter(..)` of one value read the same).
        let single = r.filters.iter().all(|f| Self::single_filter_value(&f.expr).is_some());
        if measured && single {
            assumptions::assume(Assumption::EmptyPartRecListHost);
        }
        Ok(measured && single)
    }

    /// Whether part `st` of `host` is the List-host cell the outside report shows (al-runner issue 6924, comment
    /// 6822249976; probe: corpus pull request 1339, pages/parts/EmptyPartRecLinkList.al): a top-level List host opened
    /// for edit and moved by GoToRecord, the part in its FactBoxes area, linked by one `const(..)` and one `field(..)`
    /// and no SubPageView or Provider. Every other List-host part keeps the blank record.
    fn list_host_cell(&self, st: &St, host: &St) -> bool {
        let engine = self.engine();
        let (top_list, host_edit) = {
            let h = host.borrow();
            let object = h.object as usize;
            (
                h.host.is_none() && engine.object_facts[object].page_type == Some(al_ir::PageType::List),
                h.mode == PageMode::Edit,
            )
        };
        let Some(control) = st.borrow().host.as_ref().map(|(_, c)| *c) else {
            return false;
        };
        let control = &page_of(&engine, host).controls[control];
        let const_and_field = control.part_link.as_ref().is_some_and(|l| {
            let consts = l
                .entries
                .iter()
                .filter(|e| matches!(e, ir::LinkEntry::Filter(..)))
                .count();
            let fields = l
                .entries
                .iter()
                .filter(|e| matches!(e, ir::LinkEntry::Field(..)))
                .count();
            l.entries.len() == 2 && consts == 1 && fields == 1 && l.view.is_none() && l.provider.is_none()
        });
        top_list && host_edit && self.host_goto_record_running && control.factbox && const_and_field
    }
}
