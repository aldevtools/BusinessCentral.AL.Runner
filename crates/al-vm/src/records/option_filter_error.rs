//! The error of an Option filter value that is no member.

use super::*;

impl Session {
    /// `SetFilter` on field `pos`, whose parse gave `e` (`filter::OPTION_NEGATIVE` and the value); `shape`: the call is the
    /// measured Record.SetFilter shape. Only the measured cell raises BC's error (assumption `option-filter-negative-one-error`,
    /// corpus PR 1428 FilterNonMemberOptionEnum.al:143 at 04e1fa0a): the measured shape, the filter text as written `-1`, an
    /// Option field whose members are all non-blank and that has no OptionCaption or OptionCaptionML, GlobalLanguage 1033. Every
    /// other number, text, caller, field and language refuses (the Danish text of 1030, :160, is measured and not answered).
    pub(crate) fn option_negative_error(
        &self,
        shape: Option<&super::measured_setfilter::MeasuredSetFilter>,
        t: &TableDef,
        pos: usize,
        e: &str,
    ) -> Signal {
        let value = &e[al_builtins::filter::OPTION_NEGATIVE.len()..];
        let refused = || {
            Signal::unsupported(format!(
                "SetFilter: `{value}` is not an option member (a negative number)"
            ))
        };
        let RtType::Option(members) = &t.fields[pos].ty else {
            return refused();
        };
        let measured = shape.is_some_and(|s| s.text == "-1")
            && value == "-1"
            && members.iter().all(|m| !m.trim().is_empty())
            && self.field_property(t, pos, "OptionCaption").is_none()
            && self.field_property(t, pos, "OptionCaptionML").is_none()
            && self.global_language == 1033;
        if !measured {
            return refused();
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::OptionFilterNegativeOneError);
        let reason = format!("'-1' is not an option. The existing options are: {}", members.join(","));
        self.filter_error(t, pos, value, &reason)
    }
}
