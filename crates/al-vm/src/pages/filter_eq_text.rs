//! A `TestPage.Filter.SetFilter` text that starts with `=` (`'=1'`, `'=Yes'`, `'=Quote'`): Business Central sets no filter. The
//! filter reads back as it was (empty, or the test's earlier filter on the field) and the rows stay, whether the field has a control
//! on the page or not, on a temporary and on a database source table (corpus testpage/filters/TempListOffPageBooleanFilter.al;
//! al-runner issue 6924). Only the measured shapes take this path (`eq_text_sets_no_filter`); every other text and page keeps the
//! filter `filter.rs` parses.

use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::value::RtType;

/// The Boolean words after the `=` the corpus measured, as written: TempListOffPageBooleanFilter.al:335-340 and :343-348 (Hidden,
/// no control), :360-365 and :368-373 (Shown, a control), :508-513 and :516-521 (database list), :534-539 and :542-547.
/// WorksheetEnumEqFilter.al (corpus PR 1392) measured `=1` and `=0` on the Worksheets (:468-473, :632) and `=1` on the List (:608).
const BOOLEAN_WORDS: [&str; 6] = ["1", "0", "Yes", "No", "true", "false"];

/// The ordinals after the `=` WorksheetEnumEqFilter.al measured on an Enum (:409-410 `=0`, `=1`); another one keeps the filter.
const ENUM_ORDINALS: [&str; 2] = ["0", "1"];

/// The ordinal after the `=` WorksheetEnumEqFilter.al measured on an Option (:452 `=1`, :607); another one keeps the filter.
const OPTION_ORDINALS: [&str; 1] = ["1"];

impl Session {
    /// Whether `text` on the field at `pos` (of type `ty`) is a filter text Business Central drops on page `st`. Every fact is
    /// required as the corpus measured it:
    /// - the page: PageType = List (TempListOffPageBooleanFilter.al:47, :142; WorksheetEnumEqFilter.al:129) or Worksheet
    ///   (WorksheetEnumEqFilter.al:81, :177), opened by the test's own `OpenView` (TempListOffPageBooleanFilter.al:273, :293,
    ///   :435, :461; WorksheetEnumEqFilter.al:260, :319, :378), at the top level (not a part, not a page AL code ran);
    /// - the field and the text after the `=`, exactly as written: a Boolean with one of `BOOLEAN_WORDS`; an Enum with a member's
    ///   name (WorksheetEnumEqFilter.al:403-405, :430-431, :580-585, :626-630) or one of `ENUM_ORDINALS`; an Option with a
    ///   member's name (TempListOffPageBooleanFilter.al:440; WorksheetEnumEqFilter.al:447-449, :604-605, :631) or one of
    ///   `OPTION_ORDINALS`;
    /// - the page holds no test filter (filter group 0) on the field yet, or the page is the temporary Worksheet with an Enum
    ///   (WorksheetEnumEqFilter.al:554-566: `Order`, then `=Quote` keeps `Order`; the only shape measured with one).
    ///
    /// The shapes TempListOffPageBooleanFilter.al pins (a List, a Boolean word or the Option's first member, no earlier filter)
    /// run clean; the others run under the assumption `testpage-filter-eq-text-dropped` (corpus PR 1392).
    pub(super) fn eq_text_sets_no_filter(&self, st: &St, rec: &Rec, pos: usize, ty: &RtType, text: &str) -> bool {
        let Some(rest) = text.strip_prefix('=') else {
            return false;
        };
        let worksheet = self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::Worksheet);
        if !(st.borrow().test_open_view() && (self.is_list_page(st) || worksheet)) {
            return false;
        }
        let first_member = matches!(ty, RtType::Option(m) if m.first().is_some_and(|m| m.as_str() == rest));
        let value_measured = match ty {
            RtType::Boolean => BOOLEAN_WORDS.contains(&rest),
            RtType::Option(members) => {
                members.iter().any(|m| !m.is_empty() && m.as_str() == rest) || OPTION_ORDINALS.contains(&rest)
            }
            RtType::Enum(id) => {
                let members = &self.engine.program.objects[*id as usize].enum_values;
                members.iter().any(|(_, n)| n.as_str() == rest)
                    || (ENUM_ORDINALS.contains(&rest) && members.iter().any(|(o, _)| o.to_string() == rest))
            }
            _ => false,
        };
        if !value_measured {
            return false;
        }
        let unfiltered = rec.borrow().filters.iter().all(|f| !(f.field == pos && f.group == 0));
        let temp_worksheet_enum = worksheet && rec.borrow().temporary && matches!(ty, RtType::Enum(_));
        if !(unfiltered || temp_worksheet_enum) {
            return false;
        }
        let pinned = self.is_list_page(st) && unfiltered && (matches!(ty, RtType::Boolean) || first_member);
        if !pinned {
            assume(Assumption::TestpageFilterEqTextDropped);
        }
        true
    }
}
