//! A list handed a record whose OnOpenPage then moves Rec (al-runner issue 6924).

use super::*;

impl Session {
    /// The primary key Rec holds before OnOpenPage runs (a page with a source table).
    pub(super) fn rec_key_before_open(&mut self, st: &St) -> Result<Option<Vec<Value>>, Signal> {
        if !self.has_source(st) {
            return Ok(None);
        }
        Ok(Some(self.row_key(&self.page_rec(st))?))
    }

    /// After OnOpenPage of a page: whether OnOpenPage left Rec on a primary key other than the one it was handed.
    pub(super) fn note_handed_rec_moved(&mut self, st: &St, before: Option<Vec<Value>>) -> Result<(), Signal> {
        let moved = match before {
            Some(before) => !Self::same_key(&before, &self.row_key(&self.page_rec(st))?),
            None => false,
        };
        st.borrow_mut().handed_rec_moved_by_onopenpage = moved;
        Ok(())
    }

    /// A list run with Page.Run(Id, Rec), whose OnOpenPage moved Rec (Get, directly or through a table procedure that hands Rec to a
    /// codeunit), opens on the row OnOpenPage left Rec on, as a list opened without a record does (`rec_moved_by_onopenpage`),
    /// and on the nearest row of the view when that row is outside the view (corpus
    /// pages/onopenpage-moves-rec/OnOpenPageGetChain.al:376 Og_List_PageRun_Mode1 `open log F:;O:C;R:C;R:D;R:C;R:B;R:A;R:C;R:C;C:C;`,
    /// Og_List_PageRunNoBD_Mode1 No. B..D, Og_List_PageRunNoAB_Mode1 `B` for C outside A..B, Og_List_PageRunNameNotBravo_Mode1 `D`
    /// for C outside Name <> bravo, the same with Mode2 and for the extended and the sorted list). Not
    /// run modally, not in lookup mode, without OnNextRecord, over a database table; anything else keeps the open it had.
    pub(super) fn handed_rec_list_opens_on_moved_row(&mut self, st: &St) -> Result<bool, Signal> {
        let rec = self.page_rec(st);
        let shape = {
            let s = st.borrow();
            s.handed_rec_moved_by_onopenpage && !s.modal && !s.lookup && s.host.is_none()
        };
        if !shape || !self.is_list(st) || self.declares(st, "onnextrecord") || rec.borrow().temporary {
            return Ok(false);
        }
        if self.stored_row(&rec)?.is_none() {
            return Ok(false);
        }
        // In a key other than the primary key (the sorted list, `SourceTableView = sorting(Name) order(descending)`) the row of
        // Rec's primary key is read first, as for a list opened without a record (Og_ViewList_PageRun_Mode1
        // `F:;O:C;R:C;R:C;R:D;R:C;R:B;R:A;R:C;R:C;R:C;C:C;`).
        self.read_rec_in_its_key(st)?;
        Ok(true)
    }
}
