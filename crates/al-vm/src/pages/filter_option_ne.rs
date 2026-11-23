//! `TestFilter.GetFilter` of an Option field the test filtered with `<>` and the first member's name (`'<>Quote'`): Business
//! Central reads the member's number, `<>0` (tests/al-language/testpage/filters/WorksheetEnumEqFilter.al:442
//! TempWorksheet_Option_Texts, :450 and :456; :598 TempList_Option_OtherValues, :606 and :611). Only that shape takes this
//! path; every other Option filter keeps the text `filter.rs` gives it.
//!
//! `TestFilter.GetFilter` of an Option filtered `<>` its first member reads `<>0` when the test typed
//! that filter (testpage/filters/WorksheetEnumEqFilter.al:442, :598). Entry points:
//! `option_ne_reads_number` (by `filter.rs`).

use super::*;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_data::TableDef;

impl Session {
    /// Whether `f`, the field's first filter, reads as `<>0`. Every fact is required as WorksheetEnumEqFilter.al has it: the test
    /// typed `<>` and the first member's name (:450, :606) and that filter is still the field's one filter of group 0
    /// (`PageState::test_typed_filter`; a filter page code wrote reads its member's name); an Option without OptionCaption or
    /// OptionCaptionML (:45); a top-level List or Worksheet over a temporary source (:83, :131) the test opened with OpenView
    /// (:319, :260).
    pub(super) fn option_ne_reads_number(&self, st: &St, rec: &Rec, table: &TableDef, f: Option<&FieldFilter>) -> bool {
        let Some(f) = f else { return false };
        let RtType::Option(members) = &table.fields[f.field].ty else {
            return false;
        };
        let typed = members.first().map(|m| format!("<>{m}"));
        let page_type = self.engine.object_facts[st.borrow().object as usize].page_type.clone();
        f.group == 0
            && matches!(f.expr, FilterExpr::Ne(Value::Option(0, _)))
            && typed.is_some()
            && st.borrow().test_typed_filter(&rec.borrow(), f.field) == typed.as_deref()
            && self.option_without_captions(table, f.field)
            && rec.borrow().temporary
            && st.borrow().test_open_view()
            && matches!(page_type, Some(al_ir::PageType::List | al_ir::PageType::Worksheet))
    }

    /// Whether the Option field at `pos` shows its member names: it declares neither OptionCaption nor OptionCaptionML (the
    /// Option of WorksheetEnumEqFilter.al:45 and TempListOffPageBooleanFilter.al:35).
    pub(super) fn option_without_captions(&self, table: &TableDef, pos: usize) -> bool {
        self.field_property(table, pos, "OptionCaption").is_none()
            && self.field_property(table, pos, "OptionCaptionML").is_none()
    }
}
