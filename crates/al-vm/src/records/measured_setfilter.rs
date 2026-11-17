//! The Record.SetFilter shape the GlobalLanguage filter cells of corpus PR 1428 measured.

use super::*;

/// The caller and shape every filter cell of corpus PR 1428 measured (FilterGlobalLanguageDateBool.al:40-46 TrySetFilter,
/// FilterNonMemberOptionEnum.al:47-57 TrySetFilter, at 04e1fa0a): `Record.SetFilter` called in AL code with the field and the
/// filter text only (no placeholder argument), on a record that is not temporary, in filter group 0. An entry that names this
/// shape takes it as an argument, so no other caller or shape reaches it.
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
