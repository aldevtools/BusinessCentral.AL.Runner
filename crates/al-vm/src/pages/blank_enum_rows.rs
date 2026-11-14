//! A text of one space, the Integer 0 or the blank member itself typed into an Enum control on the rows of a list, where the
//! enum's first member has the caption ' ' (al-runner issue 7380). It selects that member as on a card with a row
//! (`pinned_card` in `controls.rs`), and the control then shows ' ' (pages/enum-setvalue-blank/EnumBlankCaptionListRows.al,
//! BC 27, 28 and 29):
//! - a ListPart on a Card the test opened with OpenEdit (:408-:577 EnumBlank_Part_*), and a List the test opened with OpenEdit
//!   (:582-:741 EnumBlank_List_*);
//! - on every position the file measured: an existing row holding another member (A) or the blank one (B), the new-row line after
//!   Last and Next (C), the same after another control of the line was set (D), no rows (E), and New() (F).
//!
//! On an existing row and on a new-row line that already holds a typed value (D) the input then goes the path every other value
//! takes on these pages: the row is stored with the blank member. On a new-row line that holds nothing typed yet (C, E, F) the
//! blank member is the value the line already holds, and the line stays unchanged: nothing is stored at the Close (`stored=2
//! rows` at :474, :488, :648, :662; `stored=0 rows` at :532, :546, :706, :720; :561, :575, :735, :749 after New()). Every other
//! shape keeps the refusals of `controls.rs`.

use super::pinned_shape::PinnedPart;
use super::*;

/// The pages of the pin (EnumBlankCaptionListRows.al:94-:112 ListPart 62431 with AutoSplitKey and DelayedInsert, no trigger,
/// controls over fields of Rec; :114-:132 Card 62432 with a field and the part, `SubPageLink = "Head No." = field("No.")`).
const PART_PAGES: PinnedPart = PinnedPart {
    host_type: ir::PageType::Card,
    host_triggers: &[],
    part_triggers: &[],
    part_properties: &["autosplitkey", "delayedinsert"],
    part_variable_controls: false,
    part_variable_validate: false,
    part_field_validate_update: false,
    part_expression_controls: false,
    unlinked: false,
    sibling_parts: false,
    user_filter: false,
};

impl Session {
    /// Whether control `idx` of `st`, over a field of Rec of Enum `enum_id`, is in the pinned cell, every fact required:
    /// - the enum's first member has ordinal 0 and the caption ' ' (one space), and no other member has a blank caption
    ///   (EnumBlankCaptionListRows.al:20-25 `value(0; Empty) { Caption = ' ', Locked = true; }`, `value(1; Replace)`);
    /// - the control is a field of Rec without NotBlank on the control or the field (:59-:60);
    /// - the page is the ListPart of `PART_PAGES` on a Card the test opened with OpenEdit (:207 `Host.OpenEdit()`), or a List with
    ///   no trigger, no property but the plain ones and no extension or subscriber, opened by the test with OpenEdit (:341
    ///   `List.OpenEdit()`; page 62430 at :74-:92).
    pub(super) fn blank_enum_list_rows_pinned(&self, st: &St, idx: usize, enum_id: u32) -> Result<bool, Signal> {
        let engine = self.engine();
        let e = &engine.program.objects[enum_id as usize];
        let enum_ok = e.enum_values.first().is_some_and(|(v, _)| *v == 0)
            && e.enum_captions.iter().any(|(v, c)| *v == 0 && c == " ")
            && e.enum_captions.iter().filter(|(_, c)| c.trim().is_empty()).count() == 1;
        let control = &page_of(&engine, st).controls[idx];
        let control_ok = matches!(control.binding, ir::Binding::Field(_))
            && control.prop("notblank").is_none()
            && self.control_field(st, idx).is_some_and(|f| !field_not_blank(&f));
        drop(engine);
        if !enum_ok || !control_ok || !self.has_source(st) {
            return Ok(false);
        }
        let page_type = self.engine().object_facts[st.borrow().object as usize]
            .page_type
            .clone();
        match page_type {
            Some(ir::PageType::List) => Ok(st.borrow().test_open_edit() && self.page_declares_within(st, &[], &[])),
            Some(ir::PageType::Other(t)) if t == "listpart" => Ok(Self::part_host_opened(st)
                == Some((true, PageMode::Edit, false, false))
                && self.part_pages_are(st, &PART_PAGES)?),
            _ => Ok(false),
        }
    }

    /// Whether the input in the pinned cell (`pinned`: `blank_enum_list_rows_pinned`) leaves the new-row line unchanged: the line holds
    /// nothing typed (cursor on the new-row line, not dirty), `selected` is the value the control holds, and no validation code
    /// can run (the field has no OnValidate, no table extension validates it, and no subscriber listens to the table; the pages
    /// of the pin have no trigger, `blank_enum_list_rows_pinned`). The pin's table has no trigger (:44-:56).
    pub(super) fn blank_enum_new_line_unchanged(
        &self,
        st: &St,
        idx: usize,
        pinned: bool,
        selected: Option<&Value>,
        held: &Value,
    ) -> bool {
        if !pinned {
            return false;
        }
        let clean_line = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine && !s.dirty
        };
        let table_object = self.page_rec(st).borrow().table as usize;
        clean_line
            && matches!((selected, held), (Some(Value::Enum(a, x)), Value::Enum(b, y)) if a == b && x == y)
            && self.control_field(st, idx).is_some_and(|f| {
                f.on_validate.is_none() && f.ext_before_validate.is_empty() && f.ext_after_validate.is_empty()
            })
            && !self.engine().events.any_subscriber_of(table_object)
    }
}

/// The field declares `NotBlank = true`.
pub(super) fn field_not_blank(f: &ir::Field) -> bool {
    f.properties
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("NotBlank") && v.trim().eq_ignore_ascii_case("true"))
}
