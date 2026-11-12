//! A ListPart that serves its rows from OnFindRecord reads its rows again from its first row after its own
//! `CurrPage.Update(true)` in a control's OnValidate (al-runner issue 7820): the client asks OnFindRecord `=>`, not the `=><`
//! of an open read.

use super::pinned_shape::{PartStep, PinnedPart, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;

/// The pages of the reporter's repro (al-runner issue 7820, `Repro Order Card` and `Repro Order Subform`): a Document host with
/// no trigger, fields of Rec and this one part, linked `"Order No." = field("No.")` on the leading field of the part's primary
/// key; a ListPart with AutoSplitKey that declares OnFindRecord, OnAfterGetRecord and OnAfterGetCurrRecord (no OnNextRecord),
/// whose controls are fields of Rec, one with an OnValidate that only runs `CurrPage.Update(true)`.
const REFRESH_PAGES: PinnedPart = PinnedPart {
    host_type: ir::PageType::Document,
    host_triggers: &[],
    part_triggers: &["onfindrecord", "onaftergetrecord", "onaftergetcurrrecord"],
    part_properties: &["autosplitkey"],
    part_variable_controls: false,
    part_variable_validate: false,
    part_field_validate_update: true,
    part_expression_controls: false,
    unlinked: false,
    sibling_parts: false,
    user_filter: false,
};

impl Session {
    /// A part on its first row whose code ran `CurrPage.Update` in a control's OnValidate, with the host's stored row unchanged
    /// and without UpdatePropagation Both (`part_update_host_unchanged`), reads its rows again from its first row (`sync_part`).
    /// A part that serves its rows from OnFindRecord asks it `=>` there, reads on to the end of the view, puts the first row back,
    /// asks `=` and reads it with OnAfterGetCurrRecord: `F=>;R1;F=;R1;C1;` with one row, `F=>;R1;R2;R3;R4;R5;R1;F=;R1;C1;` with
    /// five (al-runner issue 7820, the reporter's repro passed on BC 29.1; assumption `part-update-first-row-find-forward` until
    /// the corpus probe lands). Pinned only for the repro's pages (`REFRESH_PAGES`), the host opened by the test for edit, the
    /// test's calls `GoToRecord` on the host, `First` and the SetValue on the part, and one to five rows in the part's view.
    /// Another part with OnFindRecord keeps the open read's `=><` as before (it reads the same rows; only its Which differs
    /// in the measured shape, and nothing measures the others; Base Application's sales subforms take this path under
    /// Microsoft's tests, so a refusal would stop them).
    pub(super) fn part_reread_from_first_row(&mut self, host: &St, c: usize, st: &St) -> Result<(), Signal> {
        if self.declares(st, "onfindrecord") && self.part_refresh_find_forward_pinned(host, c, st)? {
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PartUpdateFirstRowFindForward);
            st.borrow_mut().part_refind_forward = true;
        }
        let r = self.sync_part(host, c, st);
        st.borrow_mut().part_refind_forward = false;
        r
    }

    fn part_refresh_find_forward_pinned(&mut self, host: &St, c: usize, st: &St) -> Result<bool, Signal> {
        use PartStep::{Host, Own};
        let both_unset = page_of(&self.engine(), host).controls[c]
            .prop("updatepropagation")
            .is_none();
        let calls_ok = matches!(
            Self::part_steps(st).as_deref(),
            Some([
                Host(TestCall::Page(PageMethod::GoToRecord)),
                Own(TestCall::Page(PageMethod::First)),
                Own(TestCall::Write(_))
            ])
        );
        if !(both_unset
            && calls_ok
            && Self::part_host_opened(st) == Some((true, PageMode::Edit, false, false))
            && self.part_key_is_link_then_integer(st)?
            && self.part_pages_are(st, &REFRESH_PAGES)?)
        {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let rows = self.view_rows(&rec, 6)?.len();
        Ok((1..=5).contains(&rows))
    }
}
