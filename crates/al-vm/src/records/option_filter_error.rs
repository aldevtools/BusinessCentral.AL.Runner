//! The error of an Option filter word that is no member (al-runner issue 8197, shape 1).

use super::*;

impl Session {
    /// `SetFilter` on field `pos` with `text` and `arg_count` arguments, whose parse gave `e` (`filter::OPTION_NOT_MEMBER` and
    /// the word). Only the shape the reporter measured raises BC's error (assumption `option-filter-non-member-error`); every other
    /// text, caller, field and language keeps main's refusal.
    pub(crate) fn option_non_member_error(
        &self,
        from_al: bool,
        t: &TableDef,
        pos: usize,
        text: &str,
        arg_count: usize,
        e: &str,
    ) -> Signal {
        let word = &e[al_builtins::filter::OPTION_NOT_MEMBER.len()..];
        let refused = || Signal::unsupported(format!("SetFilter: `{word}` is not an option member"));
        let RtType::Option(members) = &t.fields[pos].ty else {
            return refused();
        };
        // The measured shape: Record.SetFilter in AL code, no placeholder argument, the filter text as written is the word alone
        // (letters only), an Option field whose members are all non-blank and that has no OptionCaption or OptionCaptionML,
        // GlobalLanguage 1033.
        let measured = from_al
            && arg_count == 2
            && text == word
            && word.bytes().all(|b| b.is_ascii_alphabetic())
            && members.iter().all(|m| !m.trim().is_empty())
            && self.field_property(t, pos, "OptionCaption").is_none()
            && self.field_property(t, pos, "OptionCaptionML").is_none()
            && self.global_language == 1033;
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
}
