//! `TestFilter.GetFilter` of the link field of a part whose host row is a new row that is not stored yet and whose key was typed.
//!
//! The corpus shows the typed key in the filter there (pages/triggers/NewLineStampsFilters.al:412
//! PYB_OpenNew_NoThenType_Steps `after_no.filter_b` `[HN]`), while the part's rows link to a blank value until the host is
//! saved (pages/parts/PartLineBeforeHostKey.al:398, :400). The runtime filters the part on the blank value,
//! so it would answer an empty filter: refused.

use super::*;

impl Session {
    pub(super) fn refuse_link_filter_of_unsaved_host(&mut self, st: &St, pos: usize) -> Result<(), Signal> {
        let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(());
        };
        let unsaved = {
            let h = host.borrow();
            h.draft && !h.draft_stored && h.cursor != Cursor::None
        };
        if !unsaved || !self.has_source(&host) {
            return Ok(());
        }
        let Some(control) = host
            .borrow()
            .parts
            .iter()
            .find(|(_, c)| Rc::ptr_eq(c, st))
            .map(|(i, _)| *i)
        else {
            return Ok(());
        };
        let engine = self.engine();
        let Some(link) = page_of(&engine, &host).controls[control].part_link.as_ref() else {
            return Ok(());
        };
        let host_rec = self.page_rec(&host);
        let table = self.table(&host_rec)?;
        for e in &link.entries {
            if let ir::LinkEntry::Field(part_pos, host_pos) = e
                && *part_pos == pos
            {
                let blank = Value::default_for(&table.fields[*host_pos].ty);
                let typed = !Self::same_values(
                    std::slice::from_ref(&host_rec.borrow().fields[*host_pos]),
                    std::slice::from_ref(&blank),
                );
                if typed {
                    return unsupported(
                        "TestFilter.GetFilter of the link field of a part whose host row is new and unsaved with its key typed (the filter shows the key; not built)",
                    );
                }
            }
        }
        Ok(())
    }
}
