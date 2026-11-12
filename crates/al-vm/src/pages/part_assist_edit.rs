//! AssistEdit on the primary-key control of an empty linked CardPart.

use super::*;
use al_builtins::filter::FilterExpr;

impl Session {
    pub(super) fn seed_empty_card_part_assist_edit(
        &mut self,
        st: &St,
        control: usize,
        method: PageMethod,
    ) -> Result<Option<(usize, Value, Value)>, Signal> {
        if method != PageMethod::AssistEdit {
            return Ok(None);
        }
        let engine = self.engine();
        let Some((host, part)) = st.borrow().host.clone() else {
            return Ok(None);
        };
        let Some(host) = host.upgrade() else {
            return Ok(None);
        };
        if st.borrow().cursor != Cursor::None
            || st.borrow().draft
            || !self.has_source(st)
            || engine.object_facts[st.borrow().object as usize].page_type_lower() != "cardpart"
            || engine.object_facts[host.borrow().object as usize].page_type != Some(ir::PageType::Document)
        {
            return Ok(None);
        }
        let ir::Binding::Field(field) = page_of(&engine, st).controls[control].binding else {
            return Ok(None);
        };
        let Some(link) = &page_of(&engine, &host).controls[part].part_link else {
            return Ok(None);
        };
        if link.provider.is_some()
            || link.view.is_some()
            || !matches!(link.entries.as_slice(), [ir::LinkEntry::Field(target, _)] if *target == field)
        {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        if self.table(&rec)?.keys[0].as_slice() != [field] {
            return Ok(None);
        }
        let (key, blank) = {
            let r = rec.borrow();
            let Value::Code(blank) = &r.fields[field] else {
                return Ok(None);
            };
            if !blank.is_empty() {
                return Ok(None);
            }
            let Some(key) = r.filters.iter().find_map(|f| match &f.expr {
                FilterExpr::Range(Some(Value::Code(a)), Some(Value::Code(b)))
                    if f.field == field && f.group == super::parts::LINK_GROUP && a == b && !a.is_empty() =>
                {
                    Some(Value::Code(a.clone()))
                }
                _ => None,
            }) else {
                return Ok(None);
            };
            (key, Value::Code(blank.clone()))
        };
        // The real-BC LinkedPartAssist probe reads the filter's key in Rec before the row exists.
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::LinkedCardPartAssistEdit);
        rec.borrow_mut().fields[field] = key.clone();
        Ok(Some((field, key, blank)))
    }

    pub(super) fn finish_empty_card_part_assist_edit(
        &mut self,
        st: &St,
        linked_key: Option<(usize, Value, Value)>,
        completed: bool,
    ) -> Result<(), Signal> {
        if !completed {
            st.borrow_mut().refresh = false;
        }
        let Some((field, key, blank)) = linked_key else {
            return Ok(());
        };
        let rec = self.page_rec(st);
        if st.borrow().cursor != Cursor::None
            || !Self::same_key(&rec.borrow().fields[field..=field], std::slice::from_ref(&key))
        {
            return Ok(());
        }
        if completed
            && self.view_row_count(st)? == 1
            && let Some(stored) = self.stored_row(&rec)?
        {
            st.borrow_mut().cursor = Cursor::Row;
            self.set_xrec_fields(st, stored);
        } else {
            rec.borrow_mut().fields[field] = blank;
        }
        Ok(())
    }
}
