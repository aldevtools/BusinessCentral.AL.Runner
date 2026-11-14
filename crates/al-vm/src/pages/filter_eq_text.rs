//! A `TestPage.Filter.SetFilter` text that starts with `=` (`'=1'`, `'=Yes'`, `'=Quote'`): Business Central sets no filter. The
//! filter reads back as it was (empty, or the test's earlier filter on the field) and the rows stay, whether the field has a control
//! on the page or not (corpus testpage/filters/TempListOffPageBooleanFilter.al; al-runner issue 6924). Only the measured shapes take
//! this path (`eq_text_sets_no_filter`). Another text that starts with `=` (also after blanks) on a Boolean, Enum or Option field
//! of a List or Worksheet refuses; a text on another page type or field type keeps the filter `filter.rs` parses.

use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::value::RtType;
use al_data::TableDef;

/// The Boolean words after the `=` the corpus measured on a List, as written: TempListOffPageBooleanFilter.al:335-340 and :343-348
/// (Hidden, no control), :360-365 and :368-373 (Shown, a control), :508-513 and :516-521 (database list), :534-539 and :542-547.
const LIST_BOOLEAN_WORDS: [&str; 6] = ["1", "0", "Yes", "No", "true", "false"];

/// The Boolean words WorksheetEnumEqFilter.al (corpus PR 1392) measured on the temporary Worksheet (:472-473).
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
    /// Measured by TempListOffPageBooleanFilter.al: the text sets no filter, clean.
    Pinned,
    /// Measured by WorksheetEnumEqFilter.al (corpus PR 1392): the text sets no filter, under the assumption.
    Assumed,
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
            EqCell::Assumed => {
                assume(Assumption::TestpageFilterEqTextDropped);
                Ok(true)
            }
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
    ///   WorksheetEnumEqFilter.al:260, :319, :378), at the top level (not a part, not a page AL code ran);
    /// - the text after the `=`, exactly as written (`eq_text_kind`), and the page and source table it was measured on
    ///   (`eq_text_cell`);
    /// - no filter of group 0 on the field yet; or, on the temporary Worksheet with an Enum member's name, the test's own filter
    ///   typed as a member's name (WorksheetEnumEqFilter.al:554-560: `Order`, then `=Quote` and `=Invoice` keep `Order`), which
    ///   stays.
    fn eq_text_measured(&self, st: &St, rec: &Rec, table: &TableDef, pos: usize, rest: &str, list: bool) -> EqCell {
        if !st.borrow().test_open_view() {
            return EqCell::Unmeasured("that the test did not open with its own OpenView (only OpenView is measured)");
        }
        let Some(kind) = self.eq_text_kind(table, pos, rest) else {
            return EqCell::Unmeasured(
                "whose text after the '=' is not a measured Boolean word, member name or ordinal",
            );
        };
        let temporary = rec.borrow().temporary;
        let unfiltered = rec.borrow().filters.iter().all(|f| !(f.field == pos && f.group == 0));
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
            return EqCell::Assumed;
        }
        EqCell::Unmeasured(
            "after a filter on the field (only the test's own Enum member name, then an Enum member name, on a temporary \
             Worksheet is measured)",
        )
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
            (true, true, EqText::EnumName(_) | EqText::OptionName(_) | EqText::OptionOrdinal) => EqCell::Assumed,
            (true, true, EqText::EnumOrdinal) => {
                EqCell::Unmeasured("with an Enum ordinal (measured on the Worksheet only)")
            }
            (true, false, _) => EqCell::Unmeasured(DATABASE_LIST),
            // WorksheetEnumEqFilter.al:412-419 (Enum), :434-435 (Enum without a control), :453-458 (Option), :472-473 (Boolean).
            (false, true, EqText::Boolean(w)) if TEMP_WORKSHEET_BOOLEAN_WORDS.contains(&w) => EqCell::Assumed,
            (
                false,
                true,
                EqText::EnumName(_) | EqText::EnumOrdinal | EqText::OptionName(_) | EqText::OptionOrdinal,
            ) => EqCell::Assumed,
            (_, true, EqText::Boolean(_)) => EqCell::Unmeasured("with a Boolean word not measured on this page"),
            // WorksheetEnumEqFilter.al:634-635, :638 (Enum `=Quote`, `=Order`), :639 (Option `=Order`), :640 (`=1`).
            (false, false, EqText::Boolean(w)) if DATABASE_WORKSHEET_BOOLEAN_WORDS.contains(&w) => EqCell::Assumed,
            (false, false, EqText::EnumName(i)) if DATABASE_WORKSHEET_ENUM_POSITIONS.contains(&i) => EqCell::Assumed,
            (false, false, EqText::OptionName(i)) if i == DATABASE_WORKSHEET_OPTION_POSITION => EqCell::Assumed,
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
