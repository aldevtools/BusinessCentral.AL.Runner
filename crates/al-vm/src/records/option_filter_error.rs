//! The errors of an Option filter value that is no member: a word (al-runner issue 8197, shape 1) and the negative number `-1`.

use super::*;

impl Session {
    /// `SetFilter` on field `pos`, whose parse gave `e` (`filter::OPTION_NOT_MEMBER` and the word); `shape`: the call is the
    /// measured Record.SetFilter shape. Only the shape the reporter measured raises BC's error (assumption
    /// `option-filter-non-member-error`); every other text, caller, field and language keeps main's refusal.
    pub(crate) fn option_non_member_error(
        &self,
        shape: Option<&super::measured_setfilter::MeasuredSetFilter>,
        t: &TableDef,
        pos: usize,
        e: &str,
    ) -> Signal {
        let word = &e[al_builtins::filter::OPTION_NOT_MEMBER.len()..];
        let refused = || Signal::unsupported(format!("SetFilter: `{word}` is not an option member"));
        let RtType::Option(members) = &t.fields[pos].ty else {
            return refused();
        };
        // The measured shape (`MeasuredSetFilter`), the filter text as written is the word alone (letters only), an Option field
        // whose members are all non-blank and that has no OptionCaption or OptionCaptionML, GlobalLanguage 1033. The measured words
        // are no prefix of a member; whether BC reads a prefix as the member is not measured.
        let Some(text) = shape.map(|s| s.text).filter(|text| *text == word) else {
            return refused();
        };
        let measured = word.bytes().all(|b| b.is_ascii_alphabetic())
            && members.iter().all(|m| !m.trim().is_empty())
            && !members
                .iter()
                .any(|m| m.to_lowercase().starts_with(&word.to_lowercase()))
            && self.field_property(t, pos, "OptionCaption").is_none()
            && self.field_property(t, pos, "OptionCaptionML").is_none()
            && self.global_language() == 1033;
        if !measured {
            return refused();
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::OptionFilterNonMemberError);
        let reason = format!(
            "'{word}' is not an option. The existing options are: {}",
            members.join(",")
        );
        self.filter_error(t, pos, text, &reason)
    }

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
            && self.global_language() == 1033;
        if !measured {
            return refused();
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::OptionFilterNegativeOneError);
        let reason = format!("'-1' is not an option. The existing options are: {}", members.join(","));
        self.filter_error(t, pos, value, &reason)
    }
}
