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
        drop(page);
        let ir::Binding::Field(field) = page_of(&engine, st).controls[index].binding else {
            return Ok(false);
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        if table.keys[0].contains(&field) {
            return Ok(false);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TrappedCardDraftBlankPartSetValue);
        Ok(true)
    }
}
