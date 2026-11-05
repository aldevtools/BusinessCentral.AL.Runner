//! A page control bound to a field of another record variable of the page (`ir::Control::var_field`) on a list that shows no row
//! (al-runner issue 7005).

use super::*;

impl Session {
    /// Whether what a control bound to an Option field of another record variable shows is known for the target: the caption from
    /// BC 29 (`Gate::OtherRecordOptionCaptionFrom29`), the member name where it is confirmed on Windows
    /// (`al_version::other_record_option_member_names_confirmed`).
    pub(super) fn other_record_option_shown(&self, _st: &St) -> bool {
        let major = self.engine.target.as_ref().map(|t| t.bc.major);
        self.has(al_version::Gate::OtherRecordOptionCaptionFrom29)
            || major.is_some_and(al_version::other_record_option_member_names_confirmed)
    }

    /// Whether control `idx` reads blank: a control bound to the page's record on a page that shows no row (`shows_no_row`), and on
    /// a page of type List that shows no row a control bound to a field of another record variable, in a group or in the repeater, whatever the
    /// field's type (pages/controls/OtherRecordOptionMembers.al:174 OtherRecordOption_NoRows_Values `[]` for an Option field with and
    /// without OptionCaption and an Enum field, on the CI legs; the reporter's Windows BC 28.5 run agrees). Pinned on a List page
    /// where the Option control's text is known (`other_record_option_shown`); elsewhere (a Worksheet, BC 27) the control goes on
    /// as before.
    pub(super) fn reads_blank_without_row(&self, st: &St, idx: usize) -> bool {
        if self.shows_no_row(st, idx) {
            return true;
        }
        let engine = self.engine();
        let var_bound = page_of(&engine, st).controls[idx].var_field.is_some();
        var_bound && st.borrow().cursor == Cursor::None && self.is_list_page(st) && self.other_record_option_shown(st)
    }
}
