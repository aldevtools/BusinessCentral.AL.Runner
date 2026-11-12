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
//! is `empty_part_rec_list.rs`.

use super::parts::LINK_GROUP;
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
        let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(());
        };
        match self.empty_part_rec_shape(st, &host)? {
            None => self.stamp_from_filters(st, false)?,
            Some(miss) if self.top_list(&host) => {
                let mut r = rec.borrow_mut();
                let filtered: Vec<usize> = r.filters.iter().map(|f| f.field).collect();
                let why: std::sync::Arc<str> = format!(
                    "a read of Rec of a part that shows no row, on a field its filters name, on a List host outside the cells \
                     BC was measured in ({miss}): whether BC puts the filter's value on Rec is not pinned"
                )
                .into();
                for f in filtered {
                    r.fields[f] = Value::Unknown(why.clone());
                }
            }
            Some(_) => {}
        }
        Ok(())
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

    /// `None` when `st` is a part of the measured shape, or of a List-host cell (`list_host_miss`; see the module); else the
    /// first fact that leaves the shape.
    fn empty_part_rec_shape(&mut self, st: &St, host: &St) -> Result<Option<&'static str>, Signal> {
        let (host_row, host_view) = {
            let h = host.borrow();
            let row = (h.cursor == Cursor::Row && !h.draft) || (h.draft && !h.draft_stored && h.draft_from_open);
            (row, h.mode == PageMode::View)
        };
        if !host_row {
            return Ok(Some("the host stands on no stored row"));
        }
        let goto_record = self.host_move_running == Some(HostMove::moved(PageMethod::GoToRecord));
        let card_shape = (self.part_open_running || (goto_record && host_view)) && self.top_card_or_document(host);
        let list_miss = self.list_host_miss(st, host)?;
        if !card_shape && list_miss.is_some() {
            return Ok(list_miss);
        }
        if !(self.has_source(st) && self.has_repeater(st) && self.part_linked(st)) {
            return Ok(Some("the part has no source table, no repeater or no SubPageLink"));
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let own_view = engine.program.objects[object].properties.iter().any(|(k, v)| {
            k.eq_ignore_ascii_case("SourceTableView") || (k.eq_ignore_ascii_case("PopulateAllFields") && is_true(v))
        });
        if own_view || self.declares(st, "onnewrecord") || engine.events.is_subscribed(object, "onnewrecordevent") {
            return Ok(Some(
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
            return Ok(Some("a filter of the part that is not its SubPageLink on a key field"));
        }
        if list_miss.is_none() {
            assumptions::assume(Assumption::EmptyPartRecListHost);
        }
        Ok(None)
    }
}
