//! A text of one space, the Integer 0 or the blank member itself typed into an Enum control on the rows of a list, where the
//! enum's first member has the caption ' ' (al-runner issue 7380). It selects that member as on a card with a row
//! (`pinned_card` in `controls.rs`), and the control then shows ' ' (pages/enum-setvalue-blank/EnumBlankCaptionListRows.al,
//! BC 27, 28 and 29):
//! - a ListPart on a Card the test opened with OpenEdit (:408-:577 EnumBlank_Part_*), and a List the test opened with OpenEdit
//!   (:582-:754 EnumBlank_List_*);
//! - on every position the file measured: an existing row holding another member (A) or the blank one (B), the new-row line after
//!   Last and Next (C), the same after another control of the line was set (D), no rows (E), and New() (F).
//!
//! On an existing row and on a new-row line that already holds a typed value (D) the input then goes the path every other value
//! takes on these pages: the row is stored with the blank member. On a new-row line that holds nothing typed yet (C, E, F) the
//! blank member is the value the line already holds, and the line stays unchanged: nothing is stored at the Close (`stored=2
//! rows` at :474, :488, :648, :662; `stored=0 rows` at :532, :546, :706, :720; :561, :575, :735, :749 after New()). In the cell
//! no control, field, table or extension trigger and no subscriber can run, so skipping the write skips no validation. Every
//! other shape keeps the refusals of `controls.rs`.

use super::pinned_shape::PinnedPart;
use super::*;

/// The ListPart and its Card of the pin (EnumBlankCaptionListRows.al:84-:103 ListPart 62431 with AutoSplitKey and DelayedInsert,
/// no trigger, controls over fields of Rec; :105-:123 Card 62432 with a field and the part, `SubPageLink = "Head No." =
/// field("No.")`).
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
    /// Whether `arg` typed into control `idx` of `st`, over a field of Rec of Enum `enum_id`, is in the pinned cell, every fact
    /// required:
    /// - the input: a Text of one space (:299-:302 `TextInput := ' '`, :305 `Format` of the member, which is ' ' by :401), the
    ///   Integer 0 (:316-:319) or the member itself (:322), none handed in through a Variant (`from_variant`);
    /// - the enum: two members, ordinals 0 and 1; the caption of 0 is ' ' (one space), 1 has a caption that is not blank; no
    ///   CaptionML; no enumextension extends it (:20-:26); the session language is 1033, the corpus run's default, so no
    ///   translation applies (the IR does not keep `Locked`, which only stops a translation);
    /// - the control: a field of Rec inside the repeater, declared by the page, whose only property is `ApplicationArea = All`
    ///   and which has no trigger (:76-:78, :97-:99); the field declares no property (:54-:58);
    /// - the page: a List opened by the test with OpenEdit (:341 `List.OpenEdit()`) in its plain view (primary key, no filter,
    ///   no marks), with no trigger, no property but the plain ones, no extension or subscriber, and only a repeater of fields
    ///   of Rec without a trigger (:64-:82 page 62430); or the
    ///   ListPart of `PART_PAGES` with exactly AutoSplitKey = true and DelayedInsert = true (:84-:89) on a Card the test opened
    ///   with OpenEdit (:239 `Host.OpenEdit()`) whose table runs no code (:36-:46);
    /// - the table: writing the field and storing the row run no code: no table trigger, no trigger on the field, no table
    ///   extension and no subscriber (:48-:62).
    pub(super) fn blank_enum_list_rows_pinned(
        &self,
        st: &St,
        idx: usize,
        enum_id: u32,
        arg: &Value,
        from_variant: bool,
    ) -> Result<bool, Signal> {
        let input_ok = !from_variant
            && match arg {
                Value::Text(t) => t.as_str() == " ",
                Value::Integer(n) => *n == 0,
                Value::Enum(id, v) => *id == enum_id && *v == 0,
                _ => false,
            };
        if !input_ok
            || !self.blank_enum_pinned(enum_id)
            || !self.has_source(st)
            || !self.blank_enum_control_pinned(st, idx)
        {
            return Ok(false);
        }
        let ir::Binding::Field(field) = page_of(&self.engine(), st).controls[idx].binding else {
            return Ok(false);
        };
        let table = self.table(&self.page_rec(st))?.object as usize;
        let page_type = self.engine().object_facts[st.borrow().object as usize]
            .page_type
            .clone();
        match page_type {
            Some(ir::PageType::List) => {
                let list_ok = st.borrow().test_open_edit()
                    && self.rec_view_is(st, 0)
                    && self.page_declares_within(st, &[], &[])
                    && self.controls_are_rec_fields(st, false);
                Ok(list_ok && self.table_runs_no_code_for(table, field))
            }
            Some(ir::PageType::Other(t)) if t == "listpart" => {
                let (host, _) = Self::tree_root(st);
                let host_ok = host.borrow().test_open_edit() && self.has_source(&host);
                if !host_ok
                    || !self.part_pages_are(st, &PART_PAGES)?
                    || !self.page_property_true(st, "AutoSplitKey")
                    || !self.page_property_true(st, "DelayedInsert")
                {
                    return Ok(false);
                }
                let host_table = self.table(&self.page_rec(&host))?.object as usize;
                Ok(self.table_code_is(host_table, &[]) && self.table_runs_no_code_for(table, field))
            }
            _ => Ok(false),
        }
    }

    /// The enum facts of `blank_enum_list_rows_pinned`.
    fn blank_enum_pinned(&self, enum_id: u32) -> bool {
        let engine = self.engine();
        let e = &engine.program.objects[enum_id as usize];
        let caption = |v: i32| e.enum_captions.iter().find(|(w, _)| *w == v).map(|(_, c)| c.as_str());
        let extended = engine
            .program
            .objects
            .heads()
            .iter()
            .any(|o| o.kind == ir::ObjKind::EnumExtension && o.extends_id == e.id);
        matches!(e.enum_values.as_slice(), [(0, _), (1, _)])
            && caption(0) == Some(" ")
            && caption(1).is_some_and(|c| !c.trim().is_empty())
            && e.enum_captions.len() == 2
            && e.enum_captions_ml.is_empty()
            && !extended
            && al_builtins::format::FormatCtx::language(self) == 1033
    }

    /// The control facts of `blank_enum_list_rows_pinned`.
    fn blank_enum_control_pinned(&self, st: &St, idx: usize) -> bool {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let control = &page.controls[idx];
        let in_repeater = control
            .parent
            .is_some_and(|p| page.controls[p].kind == ir::ControlKind::Repeater);
        let only_application_area = matches!(control.props.as_slice(), [(k, ir::PropVal::Lit(v))]
            if k == "applicationarea" && v.trim().eq_ignore_ascii_case("All"));
        let control_ok = control.kind == ir::ControlKind::Field
            && matches!(control.binding, ir::Binding::Field(_))
            && in_repeater
            && only_application_area
            && control.triggers.is_empty()
            && control.ext_triggers.is_empty()
            && control.owner.is_none();
        drop(engine);
        control_ok && self.control_field(st, idx).is_some_and(|f| f.properties.is_empty())
    }

    /// Whether the input in the pinned cell (`pinned`: `blank_enum_list_rows_pinned`) leaves the new-row line unchanged: the
    /// line holds nothing typed (cursor on the new-row line, not dirty) and `selected` is the value the control holds. The cell
    /// runs no validation code (no control, field, table or extension trigger and no subscriber), so nothing else would run.
    pub(super) fn blank_enum_new_line_unchanged(
        &self,
        st: &St,
        pinned: bool,
        selected: Option<&Value>,
        held: &Value,
    ) -> bool {
        let clean_line = {
            let s = st.borrow();
            s.cursor == Cursor::NewLine && !s.dirty
        };
        pinned
            && clean_line
            && matches!((selected, held), (Some(Value::Enum(a, x)), Value::Enum(b, y)) if a == b && x == y)
    }
}

/// The field declares `NotBlank = true`.
pub(super) fn field_not_blank(f: &ir::Field) -> bool {
    f.properties
        .iter()
        .any(|(k, v)| k.eq_ignore_ascii_case("NotBlank") && v.trim().eq_ignore_ascii_case("true"))
}
