//! SetValue after New() on a trapped Card whose part shows rows of the blank link key.

use super::*;

impl Session {
    pub(super) fn trapped_card_blank_part_set_value(&self, st: &St, index: usize) -> Result<bool, Signal> {
        let engine = self.engine();
        let page = st.borrow();
        let object = page.object as usize;
        if !page.caught_by_trap
            || !page.draft
            || page.draft_from_open
            || page.draft_stored
            || engine.object_facts[object].page_type != Some(ir::PageType::Card)
            || effective_prop(&engine, object, "DelayedInsert")?
                .is_some_and(|value| value.trim().eq_ignore_ascii_case("true"))
        {
            return Ok(false);
        }
        let [(control, part)] = page.parts.as_slice() else {
            return Ok(false);
        };
        let control = *control;
        let part = part.clone();
        drop(page);
        let layout = page_of(&engine, st);
        if !layout.triggers.is_empty()
            || !self.unextended_unsubscribed(st)
            || !self.unextended_unsubscribed(&part)
            || engine.object_facts[part.borrow().object as usize].page_type_lower() != "listpart"
            || !layout.controls[control].part_link.as_ref().is_some_and(|link| {
                link.provider.is_none()
                    && link.view.is_none()
                    && matches!(link.entries.as_slice(), [ir::LinkEntry::Field(_, _)])
            })
        {
            return Ok(false);
        }
        let ir::Binding::Field(field) = page_of(&engine, st).controls[index].binding else {
            return Ok(false);
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        if rec.borrow().temporary || table.keys[0].contains(&field) {
            return Ok(false);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TrappedCardDraftBlankPartSetValue);
        Ok(true)
    }
}
