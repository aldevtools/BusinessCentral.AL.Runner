//! `TestFilter.GetFilter` of an Option field the test filtered with `<>` and the first member's name (`'<>Quote'`): Business
//! Central reads the member's number, `<>0` (corpus PR 1392, testpage/filters/WorksheetEnumEqFilter.al:450 and :456 on the
//! temporary Worksheet, :606 and :611 on the temporary List). Only that shape takes this path, under the assumption
//! `testpage-filter-option-ne-number`; every other Option filter keeps the text `filter.rs` gives it.

use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::filter::{FieldFilter, FilterExpr};

impl Session {
    /// Whether `f`, the field's first filter, reads as `<>0`. Every fact is required as WorksheetEnumEqFilter.al has it: the test's
    /// own filter (filter group 0) `<>` the Option's first member (ordinal 0), on a top-level List or Worksheet over a temporary
    /// source (:83, :131) the test opened with OpenView (:319, :260).
    pub(super) fn option_ne_reads_number(&self, st: &St, rec: &Rec, f: Option<&FieldFilter>) -> bool {
        let Some(f) = f else { return false };
        let page_type = self.engine.object_facts[st.borrow().object as usize].page_type.clone();
        let shape = f.group == 0
            && matches!(f.expr, FilterExpr::Ne(Value::Option(0, _)))
            && rec.borrow().temporary
            && st.borrow().test_open_view()
            && matches!(page_type, Some(al_ir::PageType::List | al_ir::PageType::Worksheet));
        if shape {
            assume(Assumption::TestpageFilterOptionNeNumber);
        }
        shape
    }
}
