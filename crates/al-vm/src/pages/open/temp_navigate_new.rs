//! TestPage.OpenNew() of a NavigatePage over a temporary source whose OnOpenPage inserted its one record (al-runner issue
//! 7811): the page shows that record as OpenEdit would, and a SetValue edits it instead of inserting a second one.
//! No corpus test pins it yet (assumption `open-new-temp-navigate-page-edits-inserted-row`); Microsoft's tests need it.

use super::*;

impl Session {
    /// The entry condition of the rule: every fact is what both shapes that need it have, and all of them are required.
    /// Every other page keeps the OpenNew it had (a new row, OnNewRecord).
    /// - the reporter's repro (al-runner issue 7811, page "Probe Setup Guide") and Microsoft's Email - SMTP Connector,
    ///   page "SMTP Account Wizard" (`SMTP Account Auth Tests`, TestAccountNameValidation, TestOAuth2FlowWithoutCustomOAuth
    ///   and the other wizard tests, which pass on BC): PageType NavigatePage, SourceTableTemporary, the page's only own
    ///   trigger OnOpenPage (`Rec.Init(); Rec.Insert();`), no page extension, no subscriber of the page;
    /// - the source table has no OnInsert, OnModify or OnRename trigger and no table extension;
    /// - how it opened: the test's own OpenNew of a top-level page, not modal, not in lookup mode, no record handed;
    /// - after OnOpenPage: Rec has no filter and stands, as stored, on the one row of the temporary table.
    pub(super) fn open_new_on_onopenpage_row(&mut self, st: &St) -> Result<bool, Signal> {
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let opened = {
            let s = st.borrow();
            s.test_opened_new && s.host.is_none() && !s.modal && !s.lookup && s.set_record.is_none()
        };
        let page_ok = engine.object_facts[object].page_type == Some(al_ir::PageType::NavigatePage)
            && page.triggers.len() == 1
            && page.triggers[0].0.eq_ignore_ascii_case("onopenpage")
            && page.extensions.is_empty()
            && page.ext_triggers.is_empty()
            && !engine.events.any_subscriber_of(object);
        if !(opened && page_ok) {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        if !rec.borrow().temporary || !rec.borrow().filters.is_empty() || rec.borrow().marked_only {
            return Ok(false);
        }
        let t = self.table(&rec)?;
        let table_ok = ["OnInsert", "OnModify", "OnRename"]
            .iter()
            .all(|n| self.table_trigger(&t, n).is_none())
            && engine.program.objects[t.object as usize]
                .table
                .as_ref()
                .is_some_and(|x| x.extensions.is_empty());
        if !table_ok {
            return Ok(false);
        }
        let Some(stored) = self.stored_row(&rec)? else {
            return Ok(false);
        };
        Ok(Self::same_values(&stored, &rec.borrow().fields) && self.view_row_count(st)? == 1)
    }
}
