//! A `TestPage.Filter.SetFilter` text that starts with `=` (`'=1'`, `'=Yes'`, `'=Quote'`): Business Central sets no filter. The
//! filter reads back as it was (empty, or the test's earlier filter on the field) and the rows stay, whether the field has a control
//! on the page or not (corpus testpage/filters/TempListOffPageBooleanFilter.al; al-runner issue 6924). Only the measured shapes take
//! this path (`eq_text_sets_no_filter`). Another text that starts with `=` (also after blanks) on a Boolean, Enum or Option field
//! of a List or Worksheet refuses; a text on another page type or field type keeps the filter `filter.rs` parses.

use super::pinned_shape::{NextEnd, PinnedEnd, Reads, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;
use al_builtins::value::RtType;
use al_data::TableDef;

/// What a page lets through after the OpenEdit cell of WorksheetOpenEditTemp.al: the filter's GetFilter, First, Next and GoToKey
/// (WalkEq :144-154, GoToEq :228-236), the control reads of the walk, the Fill action's Invoke when it did not come before the
/// filter (BeforeEq :207), and Close (:188, :209, :236).
const OPEN_EDIT_END: PinnedEnd = PinnedEnd {
    reason: "a call of the test on a temporary Worksheet it opened with OpenEdit, after the '=0' or '=1' filter, other than \
             GetFilter, First, Next, GoToKey, a control's value read, one action Invoke or Close (not measured; al-runner issue \
             6924)",
    reads: Reads::Any,
    caption: false,
    part_reads: false,
    one_action: false,
    first_ends: false,
    part_visible: false,
    part_moves: false,
    no_writes: false,
    next: NextEnd::Refused,
    close: true,
    ok: false,
    walk: true,
};

/// The Boolean words after the `=` the corpus measured on a List, as written: TempListOffPageBooleanFilter.al:335-340 and :343-348
/// (Hidden, no control), :360-365 and :368-373 (Shown, a control), :508-513 and :516-521 (database list), :534-539 and :542-547.
const LIST_BOOLEAN_WORDS: [&str; 6] = ["1", "0", "Yes", "No", "true", "false"];

/// The Boolean words WorksheetEnumEqFilter.al measured on the temporary Worksheet (:472-473).
const TEMP_WORKSHEET_BOOLEAN_WORDS: [&str; 2] = ["1", "0"];

/// The Boolean word WorksheetEnumEqFilter.al measured on the database Worksheet (:640).
const DATABASE_WORKSHEET_BOOLEAN_WORDS: [&str; 1] = ["1"];

/// The positions of the Enum members WorksheetEnumEqFilter.al measured on the database Worksheet: Quote and Order (:634-635, :638).
const DATABASE_WORKSHEET_ENUM_POSITIONS: [usize; 2] = [0, 1];

/// The position of the Option member WorksheetEnumEqFilter.al measured on the database Worksheet: Order (:639).
const DATABASE_WORKSHEET_OPTION_POSITION: usize = 1;

/// The member count of the Enum and the Option WorksheetEnumEqFilter.al measured (:25-46: Quote, Order, Invoice); the Enum has no
/// enumextension member. An Enum or Option of another size is not measured.
const PROBE_MEMBER_COUNT: usize = 3;

/// The ordinals after the `=` WorksheetEnumEqFilter.al measured on an Enum, on the temporary Worksheet only (:418-419).
const ENUM_ORDINALS: [&str; 2] = ["0", "1"];

/// The ordinal after the `=` WorksheetEnumEqFilter.al measured on an Option, on the temporary Worksheet and List (:458, :612).
const OPTION_ORDINALS: [&str; 1] = ["1"];

/// The text after the `=`, when it has the form of a measured one (`Session::eq_text_kind`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum EqText<'a> {
    Boolean(&'a str),
    /// The name of the Enum member at this position.
    EnumName(usize),
    EnumOrdinal,
    /// The name of the Option member at this index.
    OptionName(usize),
    OptionOrdinal,
    /// A member's name of an Enum or Option that shows captions, or a name that is not one word; `option_first`: the first
    /// member of an Option.
    OtherName {
        option_first: bool,
    },
    /// A member's name or an ordinal of `ENUM_ORDINALS` of an Enum whose ordinals are not its positions (0, 1, 2, ...), or that
    /// does not have `PROBE_MEMBER_COUNT` members, or has an enumextension member; or a member's name or the ordinal 1 of an
    /// Option that does not have `PROBE_MEMBER_COUNT` members. The corpus enum and option are as the probe wrote them
    /// (WorksheetEnumEqFilter.al:25-46).
    UnmeasuredEnum,
    /// The first member's name of an Option that does not have `PROBE_MEMBER_COUNT` members: only the pinned cell of the
    /// temporary List (TempListOffPageBooleanFilter.al:440) covers it.
    UnmeasuredOptionFirst,
}

/// What a shape of the rule does on its page (`Session::eq_text_cell`).
#[derive(Clone, Copy, PartialEq, Eq)]
enum EqCell {
    /// Measured: the text sets no filter. TempListOffPageBooleanFilter.al; tests/al-language/testpage/filters/
    /// WorksheetEnumEqFilter.al:398, :425, :442, :463, :480, :513, :545, :575, :598, :620; on a page the test opened with
    /// OpenEdit, tests/al-language/testpage/filters/WorksheetOpenEditTemp.al:494, :511, :524, :588, :605, :618, :682, :699,
    /// :712, :776, :793.
    Pinned,
    /// Not measured, for this reason: refused.
    Unmeasured(&'static str),
}

impl Session {
    /// Whether `text` on the field at `pos` of `table` is a filter text Business Central drops on page `st`.
    ///
    /// The rule's neighborhood is a text that starts with `=`, after any blanks, on a Boolean, Enum or Option field of a List or
    /// Worksheet page: the probe found the text dropped on every such shape it measured (WorksheetEnumEqFilter.al:14-17), so main's
    /// filter is not an answer there. In it, a measured cell (`eq_text_measured`) sets no filter and every other shape refuses. A text on
    /// another page type or field type keeps the filter `filter.rs` parses.
    pub(super) fn eq_text_sets_no_filter(
        &self,
        st: &St,
        rec: &Rec,
        table: &TableDef,
        pos: usize,
        text: &str,
    ) -> Result<bool, Signal> {
        if !text.trim_start().starts_with('=') {
            return Ok(false);
        }
        let list = self.is_list_page(st);
        let worksheet =
            self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::Worksheet);
        let measured_field = matches!(
            table.fields[pos].ty,
            RtType::Boolean | RtType::Option(_) | RtType::Enum(_)
        );
        if !((list || worksheet) && measured_field) {
            return Ok(false);
        }
        // The probes wrote every text with the `=` first (WorksheetEnumEqFilter.al:412-419, :634-640;
        // TempListOffPageBooleanFilter.al:335-373): a blank before it is in the neighborhood and not measured.
        let cell = match text.strip_prefix('=') {
            Some(rest) => self.eq_text_measured(st, rec, table, pos, rest, list),
            None => EqCell::Unmeasured("with a blank before the '=' (only a text with the '=' first is measured)"),
        };
        match cell {
            EqCell::Pinned => Ok(true),
            EqCell::Unmeasured(why) => Err(Signal::unsupported(format!(
                "TestPage.Filter.SetFilter: the text '{text}' on a {} page over a {} table {why} (Business Central sets no \
                 filter for a text that starts with '=' on every shape the corpus measured; this shape is not measured; \
                 al-runner issue 6924)",
                if list { "List" } else { "Worksheet" },
                if rec.borrow().temporary {
                    "temporary"
                } else {
                    "database"
                },
            ))),
        }
    }

    /// The cell of the text after the `=` (`rest`) on a List (`list`) or Worksheet `st`. Every fact is required as the corpus
    /// measured it:
    /// - the page opened by the test's own `OpenView` (TempListOffPageBooleanFilter.al:273, :293, :435, :461;
    ///   WorksheetEnumEqFilter.al:260, :319, :378), at the top level (not a part, not a page AL code ran); or by its own
    ///   `OpenEdit` in the one cell `eq_text_open_edit_cell` covers;
    /// - the text after the `=`, exactly as written (`eq_text_kind`), and the page and source table it was measured on
    ///   (`eq_text_cell`);
    /// - no filter of group 0 on the field yet; or, on the temporary Worksheet with an Enum member's name, the test's own filter
    ///   typed as a member's name (WorksheetEnumEqFilter.al:554-560: `Order`, then `=Quote` and `=Invoice` keep `Order`), which
    ///   stays.
    fn eq_text_measured(&self, st: &St, rec: &Rec, table: &TableDef, pos: usize, rest: &str, list: bool) -> EqCell {
        let (open_view, open_edit) = {
            let s = st.borrow();
            (s.test_open_view(), s.test_open_edit())
        };
        if !open_view && !open_edit {
            return EqCell::Unmeasured(
                "that the test did not open with its own OpenView or OpenEdit (only those are measured)",
            );
        }
        let Some(kind) = self.eq_text_kind(table, pos, rest) else {
            return EqCell::Unmeasured(
                "whose text after the '=' is not a measured Boolean word, member name or ordinal",
            );
        };
        let temporary = rec.borrow().temporary;
        let unfiltered = rec.borrow().filters.iter().all(|f| !(f.field == pos && f.group == 0));
        if open_edit {
            return self.eq_text_open_edit_cell(st, rec, list, temporary, unfiltered, kind);
        }
        if unfiltered {
            return Self::eq_text_cell(list, temporary, kind);
        }
        let earlier_name = {
            let s = st.borrow();
            let r = rec.borrow();
            s.test_typed_filter(&r, pos)
                .is_some_and(|t| matches!(self.eq_text_kind(table, pos, t), Some(EqText::EnumName(_))))
        };
        if !list && temporary && matches!(kind, EqText::EnumName(_)) && earlier_name {
            return EqCell::Pinned;
        }
        EqCell::Unmeasured(
            "after a filter on the field (only the test's own Enum member name, then an Enum member name, on a temporary \
             Worksheet is measured)",
        )
    }

    /// The one cell WorksheetOpenEditTemp.al measured on a page the test opened with its own `OpenEdit`, every
    /// fact required as the probe has it:
    /// - a Worksheet (:22, :63, :92; WEQ Temp Worksheet of WorksheetEnumEqFilter.al) over a temporary table (:24, :65, :94), the
    ///   Boolean field IsHist with or without a control (:106; :35-37), no earlier filter on the field, and the text `=0` or `=1`
    ///   (:499-504, :516-519, :529-536 and the same steps at :593, :606, :619, :687, :700, :713, :781, :794);
    /// - the page (`eq_text_open_edit_page`) and the test's calls on it before the SetFilter (`eq_text_open_edit_calls`).
    ///
    /// The text sets no filter and every row shows, as under OpenView; then only the calls of the measured walks follow
    /// (`OPEN_EDIT_END`). Every other text, page, source or call sequence opened with OpenEdit is not measured.
    fn eq_text_open_edit_cell(
        &self,
        st: &St,
        rec: &Rec,
        list: bool,
        temporary: bool,
        unfiltered: bool,
        kind: EqText,
    ) -> EqCell {
        let word = matches!(kind, EqText::Boolean(w) if TEMP_WORKSHEET_BOOLEAN_WORDS.contains(&w));
        if list || !temporary || !unfiltered || !word {
            return EqCell::Unmeasured(
                "that the test opened with OpenEdit (only the Boolean words 0 and 1 on a temporary Worksheet, with no earlier \
                 filter on the field, are measured there)",
            );
        }
        let Some(action) = self.eq_text_open_edit_page(st, rec) else {
            return EqCell::Unmeasured(
                "that the test opened with OpenEdit, on a page whose triggers, properties, controls or source table no cell of \
                 WorksheetOpenEditTemp.al has",
            );
        };
        let Some(action_before) = Self::eq_text_open_edit_calls(st, action) else {
            return EqCell::Unmeasured(
                "that the test opened with OpenEdit, after test calls on the page no cell of WorksheetOpenEditTemp.al makes \
                 before the filter (only none, or one action Invoke)",
            );
        };
        st.borrow_mut().pinned_end = Some(PinnedEnd {
            one_action: action && !action_before,
            ..OPEN_EDIT_END
        });
        EqCell::Pinned
    }

    /// Whether page `st` with Rec `rec` is a page of WorksheetOpenEditTemp.al, and `Some(true)` when it has the Fill action:
    /// - no page trigger and one action whose only trigger is OnAction (WEQ Temp Worksheet, WorksheetEnumEqFilter.al:79-125;
    ///   WOE Temp WS Act and WOE Temp WS Shown, :20-59, :90-129), or OnOpenPage and no action (WOE Temp WS Open, :61-88);
    /// - no property but the plain ones, SourceTableTemporary = true and Editable = false or not set (:24;
    ///   WorksheetEnumEqFilter.al:87); no extension, no subscriber (`page_declares_within`);
    /// - controls that are repeaters, groups, fields of Rec without a trigger, and that action (`controls_are_rec_fields`);
    /// - no TestPage rule ran before (no `pinned_end`);
    /// - the source table a normal table (WEQ Row has no TableType) without triggers, extensions or subscribers.
    fn eq_text_open_edit_page(&self, st: &St, rec: &Rec) -> Option<bool> {
        if st.borrow().pinned_end.is_some()
            || !self.page_declares_within(st, &["onopenpage"], &["sourcetabletemporary", "editable"])
            || !self.controls_are_rec_fields(st, true)
        {
            return None;
        }
        let engine = self.engine();
        let object = st.borrow().object as usize;
        let page = page_of(&engine, st);
        let actions = page
            .controls
            .iter()
            .filter(|c| c.kind == ir::ControlKind::Action)
            .count();
        let open_page = matches!(page.triggers.as_slice(), [(n, _)] if n == "onopenpage");
        let shape = match (page.triggers.is_empty(), open_page, actions) {
            (true, false, 1) => true,
            (false, true, 0) => false,
            _ => return None,
        };
        let prop = |name: &str| {
            engine.program.objects[object]
                .properties
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(name))
                .map(|(_, v)| v.trim().to_ascii_lowercase())
        };
        let props = prop("SourceTableTemporary").as_deref() == Some("true")
            && prop("Editable").as_deref().is_none_or(|v| v == "false");
        let table = rec.borrow().table as usize;
        let normal = matches!(
            engine.object_facts[table].table_type,
            None | Some(al_ir::TableType::Normal)
        );
        drop(engine);
        (props && normal && self.table_runs_no_trigger(table)).then_some(shape)
    }

    /// The test's calls on page `st` before this SetFilter, as WorksheetOpenEditTemp.al makes them, read with their outcomes
    /// (`test_calls_all_succeeded`): none (BeforeEq :198-203, and the page that fills in OnOpenPage), or one `Invoke` of the
    /// Fill action (`action`; AfterEq :179-184, GoToEq :219-225). `Some(true)`: the Invoke came before.
    fn eq_text_open_edit_calls(st: &St, action: bool) -> Option<bool> {
        // `P.Filter` is a call of the page, then `SetFilter` one of the filter.
        let filter_set = |filter: &TestCall, set: &TestCall| {
            *filter == TestCall::Page(PageMethod::Filter) && *set == TestCall::Other(PageMethod::SetFilter)
        };
        let calls = Self::test_calls_all_succeeded(st)?;
        match calls.as_slice() {
            [filter, set] if filter_set(filter, set) => Some(false),
            [TestCall::Other(PageMethod::Invoke), filter, set] if action && filter_set(filter, set) => Some(true),
            _ => None,
        }
    }

    /// The measured cells of a text `kind` on a List (`list`) or Worksheet over a temporary or database table, with no earlier
    /// filter on the field.
    fn eq_text_cell(list: bool, temporary: bool, kind: EqText) -> EqCell {
        const DATABASE_LIST: &str = "with a member name or ordinal (only the Boolean words are measured there)";
        const DATABASE_WORKSHEET: &str = "with a text other than the Enum members at the positions 0 and 1 (ordinals equal to \
                                          positions), the Option member at position 1, or the Boolean word 1";
        const CAPTIONS: &str =
            "with a member name of an Enum or Option that shows captions, or a name of more than one word";
        const SIZE: &str = "with a member of an Enum or Option that does not have the measured 3 members at the positions \
                            0-2 (Enum ordinals equal to positions, no enumextension member)";
        match (list, temporary, kind) {
            // TempListOffPageBooleanFilter.al:335-373 (temporary), :508-547 (database).
            (true, _, EqText::Boolean(w)) if LIST_BOOLEAN_WORDS.contains(&w) => EqCell::Pinned,
            // TempListOffPageBooleanFilter.al:440 `'=Quote'`, the first of `Quote,"Order"` (:35).
            (true, true, EqText::OptionName(0) | EqText::UnmeasuredOptionFirst) => EqCell::Pinned,
            // WorksheetEnumEqFilter.al:587-592 (Enum names), :609-610 (Option names), :612 (`=1`).
            (true, true, EqText::EnumName(_) | EqText::OptionName(_) | EqText::OptionOrdinal) => EqCell::Pinned,
            (true, true, EqText::EnumOrdinal) => {
                EqCell::Unmeasured("with an Enum ordinal (measured on the Worksheet only)")
            }
            (true, false, _) => EqCell::Unmeasured(DATABASE_LIST),
            // WorksheetEnumEqFilter.al:412-419 (Enum), :434-435 (Enum without a control), :453-458 (Option), :472-473 (Boolean).
            (false, true, EqText::Boolean(w)) if TEMP_WORKSHEET_BOOLEAN_WORDS.contains(&w) => EqCell::Pinned,
            (
                false,
                true,
                EqText::EnumName(_) | EqText::EnumOrdinal | EqText::OptionName(_) | EqText::OptionOrdinal,
            ) => EqCell::Pinned,
            (_, true, EqText::Boolean(_)) => EqCell::Unmeasured("with a Boolean word not measured on this page"),
            // WorksheetEnumEqFilter.al:634-635, :638 (Enum `=Quote`, `=Order`), :639 (Option `=Order`), :640 (`=1`).
            (false, false, EqText::Boolean(w)) if DATABASE_WORKSHEET_BOOLEAN_WORDS.contains(&w) => EqCell::Pinned,
            (false, false, EqText::EnumName(i)) if DATABASE_WORKSHEET_ENUM_POSITIONS.contains(&i) => EqCell::Pinned,
            (false, false, EqText::OptionName(i)) if i == DATABASE_WORKSHEET_OPTION_POSITION => EqCell::Pinned,
            (_, _, EqText::OtherName { .. }) => EqCell::Unmeasured(CAPTIONS),
            (_, true, EqText::UnmeasuredEnum | EqText::UnmeasuredOptionFirst) => EqCell::Unmeasured(SIZE),
            (false, false, _) => EqCell::Unmeasured(DATABASE_WORKSHEET),
        }
    }

    /// The text after the `=` (`rest`) on the field at `pos`, when it has the form of a measured one: a Boolean word; the name of
    /// a member of an Enum or Option that shows its names (no Caption, CaptionML, OptionCaption or OptionCaptionML; the corpus
    /// enum and option have none: WorksheetEnumEqFilter.al:25-46, TempListOffPageBooleanFilter.al:35), written as one word of
    /// letters and digits (Quote, Order, Invoice); or an ordinal of `ENUM_ORDINALS` or `OPTION_ORDINALS`. Another member's name
    /// is `OtherName`; a name or ordinal of an Enum whose ordinals are not its positions is `UnmeasuredEnum`.
    fn eq_text_kind<'a>(&self, table: &TableDef, pos: usize, rest: &'a str) -> Option<EqText<'a>> {
        let one_word = !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric());
        match &table.fields[pos].ty {
            RtType::Boolean => LIST_BOOLEAN_WORDS.contains(&rest).then_some(EqText::Boolean(rest)),
            RtType::Option(members) if self.option_without_captions(table, pos) => {
                let sized = members.len() == PROBE_MEMBER_COUNT;
                match members.iter().position(|m| m.as_str() == rest) {
                    Some(i) if one_word && sized => Some(EqText::OptionName(i)),
                    Some(0) if one_word => Some(EqText::UnmeasuredOptionFirst),
                    Some(_) if one_word => Some(EqText::UnmeasuredEnum),
                    Some(i) => Some(EqText::OtherName { option_first: i == 0 }),
                    None => OPTION_ORDINALS.contains(&rest).then_some(if sized {
                        EqText::OptionOrdinal
                    } else {
                        EqText::UnmeasuredEnum
                    }),
                }
            }
            RtType::Option(members) => members
                .iter()
                .position(|m| m.as_str() == rest)
                .map(|i| EqText::OtherName { option_first: i == 0 }),
            RtType::Enum(id) => {
                let object = &self.engine.program.objects[*id as usize];
                let names_shown = object.enum_captions.is_empty() && object.enum_captions_ml.is_empty();
                let extended = self
                    .engine
                    .program
                    .objects
                    .iter()
                    .any(|o| o.kind == al_ir::ObjKind::EnumExtension && o.extends_id == object.id);
                let positional = !extended
                    && object.enum_values.len() == PROBE_MEMBER_COUNT
                    && object
                        .enum_values
                        .iter()
                        .enumerate()
                        .all(|(i, (o, _))| usize::try_from(*o) == Ok(i));
                if let Some(i) = object.enum_values.iter().position(|(_, n)| n.as_str() == rest) {
                    Some(match (one_word && names_shown, positional) {
                        (true, true) => EqText::EnumName(i),
                        (true, false) => EqText::UnmeasuredEnum,
                        (false, _) => EqText::OtherName { option_first: false },
                    })
                } else if !names_shown {
                    None
                } else {
                    (ENUM_ORDINALS.contains(&rest) && object.enum_values.iter().any(|(o, _)| o.to_string() == rest))
                        .then_some(if positional {
                            EqText::EnumOrdinal
                        } else {
                            EqText::UnmeasuredEnum
                        })
                }
            }
            _ => None,
        }
    }
}
