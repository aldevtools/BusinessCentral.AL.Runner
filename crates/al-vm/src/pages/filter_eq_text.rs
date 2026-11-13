//! A `TestPage.Filter.SetFilter` text that starts with `=` (`'=1'`, `'=Yes'`, `'=Quote'`): Business Central sets no filter. The
//! filter reads back empty and every row shows, whether the field has a control on the page or not, on a temporary and on a
//! database source table (corpus testpage/filters/TempListOffPageBooleanFilter.al; al-runner issue 6924). Only the measured
//! shape takes this path (`eq_text_sets_no_filter`); every other text and page keeps the filter `filter.rs` parses.

use super::*;
use al_builtins::value::RtType;

/// The Boolean words after the `=` the corpus measured, as written: TempListOffPageBooleanFilter.al:335-340 and :343-348 (Hidden,
/// no control), :360-365 and :368-373 (Shown, a control), :508-513 and :516-521 (database list), :534-539 and :542-547.
const BOOLEAN_WORDS: [&str; 6] = ["1", "0", "Yes", "No", "true", "false"];

impl Session {
    /// Whether `text` on the field at `pos` (of type `ty`) is a filter text Business Central drops on page `st`. Every fact is
    /// required as the corpus measured it (TempListOffPageBooleanFilter.al):
    /// - the page: PageType = List (:47, :142), opened by the test's own `OpenView` (:273, :293, :435, :461), at the top level
    ///   (not a part, not a page AL code ran);
    /// - the field: a Boolean with one of `BOOLEAN_WORDS` after the `=`, or an Option with its first member's name after the
    ///   `=` (:440 `'=Quote'`, the first of `Quote,"Order"` at :35; read back `''` at :447 and every row at :449, :466-475).
    ///   Another member after the `=` is not measured and keeps the filter;
    /// - the page holds no test filter (filter group 0) on the field yet (each measured call is the first one on the field
    ///   that set a filter; :469 follows a dropped `'=1'`).
    pub(super) fn eq_text_sets_no_filter(&self, st: &St, rec: &Rec, pos: usize, ty: &RtType, text: &str) -> bool {
        let Some(rest) = text.strip_prefix('=') else {
            return false;
        };
        let value_measured = match ty {
            RtType::Boolean => BOOLEAN_WORDS.contains(&rest),
            RtType::Option(members) => members.first().is_some_and(|m| m.as_str() == rest),
            _ => false,
        };
        value_measured
            && st.borrow().test_open_view()
            && self.is_list_page(st)
            && rec.borrow().filters.iter().all(|f| !(f.field == pos && f.group == 0))
    }
}
