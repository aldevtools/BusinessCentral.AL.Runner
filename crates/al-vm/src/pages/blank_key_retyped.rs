//! A key typed on a new Card without parts whose row a field outside the key stored with a blank key.
//!
//! BC drops the key: the card shows the blank key again, nothing is renamed, and the edit still waiting is saved at Close
//! (pages/lt-part-link-unsaved-host/PagePartLinkUnsavedHost.al:482 PYB_NoPart_DescrFirstThenKey `db_after_code` `1 [|]`,
//! `code_value` `[]`, `db_after_close` `1 [|d1]`). Pinned for that card only: a Card with no part and no SourceTableView; other
//! pages keep the key they were given (and the save of the renamed row is refused when it finds no row).

use super::*;

impl Session {
    /// `pos`: the field the test just set. Answers whether the card dropped the key.
    pub(super) fn drop_key_typed_on_stored_blank_row(&mut self, st: &St, pos: usize) -> Result<bool, Signal> {
        let shape = {
            let s = st.borrow();
            s.open
                && s.host.is_none()
                && s.parts.is_empty()
                && !s.draft
                && s.cursor == Cursor::Row
                && s.typed_before_key
        };
        if !shape || !self.has_source(st) || !self.is_card(st) {
            return Ok(false);
        }
        let facts = &self.engine.object_facts[st.borrow().object as usize];
        if facts.has_source_table_view || !self.table(&self.page_rec(st))?.keys[0].contains(&pos) {
            return Ok(false);
        }
        self.revert_key_typed_on_stored_row(st, pos)?;
        Ok(true)
    }

    /// A key typed on a host row that a field outside the key stored with a blank key, with parts linked on the key: the same
    /// (NewLineStampsFilters.al:285 PartB_OpenNewHeader_TypeThenNo `typed` `|A|A`, `close` ok, `db.first` `|A|A|x`;
    /// :372 PYB_OpenNew_TypeThenNo_Steps `after_no.card` `[|B]` and `after_close.headers` `1 [|B]`: the card shows the blank key,
    /// the parts keep the blank link value, the edit waiting is saved at Close).
    pub(super) fn revert_key_typed_on_stored_row(&mut self, st: &St, pos: usize) -> Result<(), Signal> {
        let old = self.page_xrec_fields(st);
        if let Some(v) = old.get(pos) {
            self.page_rec(st).borrow_mut().fields[pos] = v.clone();
        }
        Ok(())
    }
}
