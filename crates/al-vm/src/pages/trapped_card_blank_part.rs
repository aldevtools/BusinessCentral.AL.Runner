//! SetValue after New() on a trapped Card whose part shows rows of the blank link key.
//!
//! Measured shape (AL Runner issue 7879, corpus pull request 1343; assumption
//! `trapped-card-draft-blank-part-set-value`): a Card caught by `Trap()`, `New()`, then SetValue of a non-key field.
//! The Card has no page or control triggers, no DelayedInsert, no page extension and no subscriber; its stored source table is keyed by
//! one field, and the table's OnInsert gives the new row a key that is not blank. Its single part is a ListPart without
//! extension or subscriber, linked by one `field(...)` entry on that key field. The SetValue's validation changes no field of
//! Rec but the typed one, and the key stays blank. After the SetValue only a read of a host
//! field and Close follow, and Close stores the Card under a key that is not blank. Every other shape keeps the refusal of
//! `blank_part_host_refusal`.

use super::*;
use al_builtins::page_op::PageMethod;

/// The refusal of a call on a new, unsaved host whose part shows rows of the blank key.
pub(super) fn blank_part_host_refusal<T>(m: PageMethod) -> Result<T, Signal> {
    unsupported(format!(
        "TestField.{m} on a new, unsaved host whose part shows rows of the blank key (not pinned)"
    ))
}

impl Session {
    /// Whether SetValue of control `index` is the measured edit of the module doc. Marks the assumption when it is.
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
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        // The link is one `field(...)` entry whose host field is the table's whole primary key (the issue's
        // `SubPageLink = "Header Code" = field(Code)` on a table keyed by Code).
        let link_on_key = layout.controls[control].part_link.as_ref().is_some_and(|link| {
            link.provider.is_none()
                && link.view.is_none()
                && matches!(link.entries.as_slice(), [ir::LinkEntry::Field(_, host)] if table.keys[0] == [*host])
        });
        if !layout.triggers.is_empty()
            || !layout.controls.iter().all(|c| c.triggers.is_empty())
            || !self.unextended_unsubscribed(st)
            || !self.unextended_unsubscribed(&part)
            || engine.object_facts[part.borrow().object as usize].page_type_lower() != "listpart"
            || !link_on_key
        {
            return Ok(false);
        }
        let ir::Binding::Field(field) = layout.controls[index].binding else {
            return Ok(false);
        };
        if rec.borrow().temporary || table.keys[0].contains(&field) {
            return Ok(false);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::TrappedCardDraftBlankPartSetValue);
        st.borrow_mut().trapped_blank_part_edit = true;
        Ok(true)
    }

    /// After the measured SetValue of control `index`: in the issue's repro its validation changed only the typed field, and
    /// the key stayed blank. Validation code that changed another field or the key (a table field trigger, a subscriber) is
    /// not measured: it keeps the refusal. `before` is Rec before the SetValue.
    pub(super) fn trapped_card_blank_part_validated(
        &self,
        st: &St,
        index: usize,
        before: Option<&[Value]>,
    ) -> Result<(), Signal> {
        if !st.borrow().trapped_blank_part_edit {
            return Ok(());
        }
        let ir::Binding::Field(typed) = page_of(&self.engine(), st).controls[index].binding else {
            return blank_part_host_refusal(PageMethod::SetValue);
        };
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let after = rec.borrow().fields.clone();
        let mut kept = before.is_some_and(|before| {
            before.len() == after.len()
                && before.iter().zip(&after).enumerate().all(|(i, (b, a))| {
                    i == typed || Self::same_values(std::slice::from_ref(b), std::slice::from_ref(a))
                })
        });
        for &k in &table.keys[0] {
            kept &= crate::recmeta::is_blank(&after[k])?;
        }
        if kept {
            Ok(())
        } else {
            blank_part_host_refusal(PageMethod::SetValue)
        }
    }

    /// After the save of Close: in the issue's repro the table's OnInsert gave the new Card a key that is not blank, so the
    /// part no longer shows the blank-key rows. A Card stored under the blank key still shows them, which is not measured:
    /// it keeps the refusal.
    pub(super) fn trapped_card_blank_part_closed(&self, st: &St) -> Result<(), Signal> {
        if !std::mem::take(&mut st.borrow_mut().trapped_blank_part_edit) {
            return Ok(());
        }
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let mut blank_key = true;
        for &k in &table.keys[0] {
            blank_key &= crate::recmeta::is_blank(&rec.borrow().fields[k])?;
        }
        if blank_key {
            return blank_part_host_refusal(PageMethod::SetValue);
        }
        Ok(())
    }
}
