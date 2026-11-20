//! The Record.SetFilter shape the GlobalLanguage filter cells measured, and the filter texts read only in that shape: the Danish
//! Boolean words (`danish_boolean_filter`) and the closing-date letter of a Date filter.

use super::*;

/// The caller and shape of every GlobalLanguage filter cell (record/filters/FilterGlobalLanguageDateBool.al:40-46 TrySetFilter,
/// record/filters/FilterNonMemberOptionEnum.al:47-57): `Record.SetFilter` called in AL code with the field and the filter text
/// only (no placeholder argument), on a record that is not temporary, in filter group 0. A rule pinned in this shape takes it as
/// an argument, so no other caller or shape reaches it.
pub(crate) struct MeasuredSetFilter<'a> {
    /// The filter text as written.
    pub(crate) text: &'a str,
}

impl<'a> MeasuredSetFilter<'a> {
    /// `from_al`: the call is Record.SetFilter in AL code; `arg_count`: the call's arguments, the field included.
    pub(crate) fn of(from_al: bool, r: &RecordState, text: &'a str, arg_count: usize) -> Option<Self> {
        (from_al && arg_count == 2 && !r.temporary && r.filter_group == 0).then_some(MeasuredSetFilter { text })
    }
}

impl Session {
    /// The filter of field `pos` that a GlobalLanguage entry reads in the measured shape, before the parser: `Ok(None)` keeps
    /// the parser's reading for every other field, language, caller and text.
    pub(crate) fn measured_language_filter(
        &self,
        shape: Option<&MeasuredSetFilter>,
        t: &TableDef,
        pos: usize,
    ) -> Result<Option<FilterExpr>, Signal> {
        let Some(shape) = shape else {
            return Ok(None);
        };
        if let Some(expr) = self.danish_boolean_filter(shape, t, pos)? {
            return Ok(Some(expr));
        }
        self.closing_letter_date_filter(shape, t, pos)
    }

    /// A Date filter whose whole text is a closing-date text of `format::closing_letter_text` (the letter and mm/dd/yy) under
    /// 1030, 1031, 1036, 1044 or 1053 (record/filters/FilterGlobalLanguageDateBool.al:239, :255, :271, :287, :303 for
    /// `C12/31/23`, :325-326, :336-337, :347-348, :358-359, :369-370 for `U12/31/23` and `S12/31/23`, upper-case letters
    /// measured): the letter of the language filters to the closing date, another language's letter raises the language's date
    /// error. A lower-case letter and a date part that does not read keep the parser's reading.
    fn closing_letter_date_filter(
        &self,
        shape: &MeasuredSetFilter,
        t: &TableDef,
        pos: usize,
    ) -> Result<Option<FilterExpr>, Signal> {
        if !matches!(t.fields[pos].ty, RtType::Date) {
            return Ok(None);
        }
        let Some((own, letter, rest)) = al_builtins::format::closing_letter_text(shape.text, self.global_language)
        else {
            return Ok(None);
        };
        let Some(Value::Date(d, false)) = al_builtins::evaluate::read_text(&RtType::Date, rest) else {
            return Ok(None);
        };
        if !letter.is_ascii_uppercase() || d <= 0 {
            return Ok(None);
        }
        if own {
            return Ok(Some(FilterExpr::Eq(Value::Date(d, true))));
        }
        Err(self.date_filter_error_in_language(t, pos, shape.text))
    }
}
