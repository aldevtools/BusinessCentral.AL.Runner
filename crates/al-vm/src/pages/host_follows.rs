//! TestPart.Next on the ListPart without a SubPageLink of a host with function controls and a second, linked ListPart, after a
//! TestField.Lookup (al-runner issue 6639). The host does not move, and the controls of every page keep showing what they showed,
//! but the server side reads the row the part moved to: the host runs OnAfterGetRecord twice and OnAfterGetCurrRecord for it
//! (its function controls after each), the linked part runs OnAfterGetRecord alone (pages/lt-y15/Y15LookupThreeFieldLink.al:1163
//! Y63_TwoPart_PartsNextAfterCancel `other next log` `o-agr R2;o-agr R2;o-fn R2;o-agcr R2;o-fn R2;h-agr R2;h-agr R2;h-fn R2;
//! h-agcr R2;h-fn R2;p-agr R2`, :1083 Y63_TwoPart_LinkedPartNextFirst, :1124 _OtherPartNextFirst, :1192
//! Y63_PartThenOther_LinkedPartNextFirst). Closing the page then reads the row the host stands on: the linked part, then the host
//! (:1124 `close log` `p-agr R1;h-agr R1`).

use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// `sync_dependents` for the part `st` that moved, then the host's reads for the row of an unlinked ListPart (`host_follows_part`).
    pub(super) fn sync_dependents_then_host_follows(&mut self, st: &St, name: PageMethod) -> Result<(), Signal> {
        self.sync_dependents(st)?;
        if name == PageMethod::Next {
            self.host_follows_part(st)?;
        }
        Ok(())
    }

    /// `refresh_parts_asked_by_host` after a move of the host; the host no longer stands apart from its unlinked ListPart.
    pub(super) fn refresh_parts_unfollow(
        &mut self,
        st: &St,
        waiting: &[(usize, St)],
        a: bool,
        b: bool,
    ) -> Result<(), Signal> {
        let r = self.refresh_parts_asked_by_host(st, waiting, a, b);
        st.borrow_mut().host_followed_part = false;
        r
    }

    /// Next on the unlinked ListPart is pinned after a TestField.Lookup: pending, run by a part's move, or cancelled.
    pub(super) fn host_follows_part_pinned(&self, root: &St) -> bool {
        let r = root.borrow();
        r.lookup_refresh_deferred || r.lookup_refresh_used || r.lookup_cancelled
    }

    /// The linked ListPart of `host` (the one with a SubPageLink) when `host` has exactly two ListParts and one is linked.
    fn linked_part_of(&mut self, host: &St) -> Option<St> {
        let engine = self.engine();
        let page = page_of(&engine, host);
        let parts = host.borrow().parts.clone();
        let linked: Vec<St> = parts
            .iter()
            .filter(|(i, _)| {
                page.controls[*i]
                    .part_link
                    .as_ref()
                    .is_some_and(|l| !l.entries.is_empty())
            })
            .map(|(_, q)| q.clone())
            .collect();
        (linked.len() == 1).then(|| linked[0].clone())
    }

    /// After the unlinked ListPart `part` moved with Next to a row (`host_follows_part_pinned`).
    pub(super) fn host_follows_part(&mut self, part: &St) -> Result<(), Signal> {
        let Some(host) = part.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(());
        };
        if !self.unlinked_part_of_two(part, &host) || !self.host_follows_part_pinned(&host) {
            return Ok(());
        }
        let Some(linked) = self.linked_part_of(&host) else {
            return unsupported("a host with function controls and two ListParts, none or both linked (not pinned)");
        };
        if part.borrow().cursor != Cursor::Row || part.borrow().object == host.borrow().object {
            return Ok(());
        }
        let row = self.page_rec(part).borrow().fields.clone();
        let host_rec = self.page_rec(&host);
        let linked_rec = self.page_rec(&linked);
        let saved = (
            host_rec.borrow().fields.clone(),
            linked_rec.borrow().fields.clone(),
            host.borrow().instance.as_ref().map(|i| i.borrow().globals.clone()),
        );
        host_rec.borrow_mut().fields = row.clone();
        linked_rec.borrow_mut().fields = row;
        let r = self.host_reads_part_row(&host, &linked);
        host_rec.borrow_mut().fields = saved.0;
        linked_rec.borrow_mut().fields = saved.1;
        if let (Some(globals), Some(i)) = (saved.2, host.borrow().instance.as_ref()) {
            i.borrow_mut().globals = globals;
        }
        host.borrow_mut().host_followed_part = true;
        r
    }

    fn host_reads_part_row(&mut self, host: &St, linked: &St) -> Result<(), Signal> {
        self.page_trigger_skipping_fn(host)?;
        self.fn_current_stage(host)?;
        self.page_trigger_skipping_fn(linked)
    }

    /// Close of a host that read the row of its unlinked ListPart: the linked part and the host read the row they stand on.
    pub(super) fn close_after_host_followed_part(&mut self, host: &St) -> Result<(), Signal> {
        if !std::mem::take(&mut host.borrow_mut().host_followed_part) {
            return Ok(());
        }
        let Some(linked) = self.linked_part_of(host) else {
            return Ok(());
        };
        self.page_trigger_skipping_fn(&linked)?;
        self.page_trigger_skipping_fn(host)
    }
}
