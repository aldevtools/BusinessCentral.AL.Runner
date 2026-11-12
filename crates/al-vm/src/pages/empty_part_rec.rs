//! What Rec of a ListPart that shows no row holds once the part has read its (empty) view: the values its SubPageLink pins
//! (al-runner issue 6924; Base Application page 1178 "Doc. Attachment List Factbox" exits its OpenInDetail action on
//! `Rec."Table ID" = 0`).
//!
//! Measured on BC 27.5 and 28.5 by corpus pull request 1335, pages/parts/EmptyPartRecLink.al (an action of the part reads Rec):
//! a `const(..)`, a `field(..)` and a `filter(..)` of one value put their value on Rec, a `filter(..)` of a range does not
//! (:373 const + field `act=75160/A/0/`, :433 field only `act=0/A/0/`, :448 const only `act=75160//0/`, :463 filter of one
//! value, :478 filter of a range `act=0/A/0/`); in a FactBox and in the content area (:403, :418), on a Card and on a Document
//! host (:558, :573, :588), opened for view or for edit (:388), after GoToRecord to a row without lines (:509), on the new row
//! of OpenNew (:528 `act=75160//0/`, the host's No. is blank), and again at a second action (:634). The fields outside the
//! link stay blank. The part's OnAfterGetCurrRecord sees the same values (pages/parts/SelectionFilterLinkedPartNoRow.al:153).
//!
//! Only that shape is changed: a ListPart (a page with a repeater) of a top-level Card or Document host, its value from
//! the link's filter group alone, on a key field (the fields `stamp_from_filters` stamps). Every other part keeps the blank
//! record it had before.

use super::parts::LINK_GROUP;
use super::*;

impl Session {
    /// Rec of part `st`, which shows no row, is the blank record with the single values of its SubPageLink on it (see the
    /// module). A part outside the measured shape gets the blank record, as before.
    pub(super) fn empty_part_rec(&mut self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let table_no = rec.borrow().table as usize;
        let fresh = self.initial_fields(table_no);
        rec.borrow_mut().fields = fresh;
        if !self.empty_part_rec_measured(st) {
            return Ok(());
        }
        let table = self.table(&rec)?;
        let stamped: Vec<(usize, Value)> = {
            let r = rec.borrow();
            table.keys[0]
                .iter()
                .filter_map(|&pos| {
                    let mut on_field = r.filters.iter().filter(|f| f.field == pos);
                    match (on_field.next(), on_field.next()) {
                        (Some(f), None) if f.group == LINK_GROUP => {
                            crate::platform::mat::single_value_of(&f.expr).map(|v| (pos, v))
                        }
                        _ => None,
                    }
                })
                .collect()
        };
        let mut r = rec.borrow_mut();
        for (pos, v) in stamped {
            r.fields[pos] = v;
        }
        Ok(())
    }

    /// Whether `st` is a part of the measured shape: a page with a repeater and a source table, shown by a top-level Card or
    /// Document page that stands on a stored row or on the new row OpenNew started (EmptyPartRecLink.al).
    fn empty_part_rec_measured(&self, st: &St) -> bool {
        let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return false;
        };
        let host_row = {
            let h = host.borrow();
            (h.cursor == Cursor::Row && !h.draft) || (h.draft && !h.draft_stored && h.draft_from_open)
        };
        host_row
            && self.top_card_or_document(&host)
            && self.has_source(st)
            && self.has_repeater(st)
            && self.part_linked(st)
    }
}
