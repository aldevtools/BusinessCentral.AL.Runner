//! A journal page whose Post action emptied its view and left Rec, with `CurrPage.Update(false)`, on a primary key no row has
//! (the Base Application's codeunit 241 "Item Jnl.-Post" leaves `"Line No." := 1` there): what the list stands on afterwards,
//! with and without the new-row line (al-runner issues 6027, 6292, 7197, 7821).

use super::pinned_shape::PinnedCode;
use super::*;

/// The page of the reporter's repro (al-runner issue 7821, `Journal Post New Row Wks`): a Worksheet with AutoSplitKey and
/// DelayedInsert (the new-row line), OnOpenPage (the batch filter in filter group 2), OnAfterGetRecord, OnNewRecord and
/// OnAfterGetCurrRecord, fields of Rec without triggers and one action; a source table without code.
const NEW_LINE_PAGE: PinnedCode = PinnedCode {
    triggers: &["onopenpage", "onaftergetrecord", "onnewrecord", "onaftergetcurrrecord"],
    control_triggers: &["onaction"],
    properties: &["autosplitkey", "delayedinsert"],
    actions: true,
};

impl Session {
    /// Whether the list settled after an action that emptied its view and left another primary key, no stored row's, in Rec
    /// with `CurrPage.Update(false)` (`rec_moved_other` in `action_invoke_body`); `rows_before` is the count of rows shown
    /// before the action.
    ///
    /// Without the new-row line the list stands on the blank Rec as when Rec keeps the deleted row's key
    /// (`settle_on_blank_rec`; pages/RtFilteredGoneRow.al:640 TD_LeavesKey_Line1WithUpdate_OneLine `R:10000;N:B1:0;C:B1:0;`,
    /// :700 _OtherBatchRowRemains; al-runner issue 6027). With two to six rows shown before the action BC 28 runs OnNewRecord
    /// and OnAfterGetCurrRecord once more, BC 27 and BC 29 twice (:656 TD_LeavesKey_Line1WithUpdate_ThreeLines, :679
    /// _TwoLines, :701 _FourLines, :724 _FiveLines, :747 _SixLines, `#if BC28PLUS and not BC29PLUS`; the reporter's page
    /// testpage/Y1WorksheetPostLog.al:161 Y1_Post_TwoLines, :177 Y1_Post_ThreeLines; al-runner issues 6292, 7197; the caller
    /// runs those).
    ///
    /// With the new-row line, filtered to one batch in filter group 2, the list runs its blank line three times as when the
    /// action only deleted the row (`settle_after_action`, misc/lt-probes-needed-7/PnGRefresh.al:702): `R:10000;N:0;C:0;N:0;C:0;
    /// N:0;C:0;` after one row (al-runner issue 7821, the reporter's repro passed on BC 29.0, opened with OpenEdit and with
    /// Page.Run; assumption `post-left-other-key-on-new-line` until the corpus probe lands). Pinned only for the repro's page
    /// (`NEW_LINE_PAGE`) and one row shown before.
    ///
    /// Rows left in the view, seven or more rows shown before, more than one row with the new-row line, or another page are not
    /// pinned (`false`: the caller refuses).
    pub(super) fn post_left_other_key_settled(&mut self, st: &St, rows_before: usize) -> Result<bool, Signal> {
        let new_line_settled = std::mem::take(&mut st.borrow_mut().gone_new_line_settled);
        let on_blank = (1..=6).contains(&rows_before) && {
            let s = st.borrow();
            s.gone_blank_no_new_line && s.open && s.cursor == Cursor::None && !s.draft
        };
        if on_blank {
            return Ok(true);
        }
        let on_new_line = new_line_settled && rows_before == 1 && {
            let s = st.borrow();
            s.open
                && s.cursor == Cursor::NewLine
                && s.placeholder_line
                && s.mode == PageMode::Edit
                && !s.modal
                && !s.lookup
                && s.host.is_none()
        };
        if !(on_new_line
            && self.engine.object_facts[st.borrow().object as usize].page_type == Some(al_ir::PageType::Worksheet)
            && self.page_code_is(st, &NEW_LINE_PAGE)?)
        {
            return Ok(false);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PostLeftOtherKeyOnNewLine);
        Ok(true)
    }
}
