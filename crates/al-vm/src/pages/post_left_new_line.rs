//! A journal page whose Post action emptied its view and left Rec, with `CurrPage.Update(false)`, on a primary key no row has
//! (the Base Application's codeunit 241 "Item Jnl.-Post" leaves `"Line No." := 1` there): what the list stands on afterwards,
//! with and without the new-row line (al-runner issues 6027, 6292, 7197, 7821).

use super::pinned_body::{Callees, callees_are, page_is, table_is};
use super::pinned_shape::{NextEnd, PinnedEnd, TestCall};
use super::*;
use al_builtins::page_op::PageMethod;

/// The page of the reporter's repro (al-runner issue 7821, `Journal Post New Row Wks`), line for line (`pinned_body.rs`).
const NEW_LINE_PAGE: &[&str] = &[
    "PageType = Worksheet",
    "SourceTable = \"Journal Post Line\"",
    "ApplicationArea = All",
    "UsageCategory = None",
    "AutoSplitKey = true",
    "DelayedInsert = true",
    "repeater Rows in - {}",
    "field Line No. in Rows = Line No. {applicationarea=All}",
    "field Quantity in Rows = Quantity {applicationarea=All}",
    "action Post in - {applicationarea=All; caption='Post'}",
    "  trigger onaction: var Posted: Record \"Journal Post Line\"; begin Posted.Copy(Rec); Rec.DeleteAll(); if (not \
     Posted.Find('=><')) then begin Posted.Reset(); Posted.FilterGroup(2); Posted.SetRange(field#0, 'B1'); \
     Posted.FilterGroup(0); Posted.\"Line No.\" := 1; end; Rec.Copy(Posted); CurrPage.Update(false); end",
    "area Processing: Post",
    "trigger onopenpage: begin Rec.FilterGroup(2); Rec.SetRange(field#0, 'B1'); Rec.FilterGroup(0); end",
    "trigger onaftergetrecord: var JournalPostLog: Codeunit \"Journal Post Log\"; begin \
     JournalPostLog.Add(('R:' + Format(Rec.\"Line No.\"))); end",
    "trigger onnewrecord: var JournalPostLog: Codeunit \"Journal Post Log\"; begin \
     JournalPostLog.Add(('N:' + Format(Rec.\"Line No.\"))); end",
    "trigger onaftergetcurrrecord: var JournalPostLog: Codeunit \"Journal Post Log\"; begin \
     JournalPostLog.Add(('C:' + Format(Rec.\"Line No.\"))); end",
];

/// The repro's table `Journal Post Line`.
const LINE_TABLE: &[&str] = &[
    "field 1 \"Batch Name\": Code[10]",
    "field 2 \"Line No.\": Integer",
    "field 3 \"Quantity\": Decimal",
    "key \"Batch Name\", \"Line No.\" {Clustered=true}",
];

/// The one procedure the page's triggers call: `Journal Post Log`.Add.
const LOG_ADD: &[&str] = &["\"Journal Post Log\".Add: begin Entries := (Entries + (Entry + ';')); end"];

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
    /// Page.Run; assumption `post-left-other-key-on-new-line` until the corpus probe lands). Pinned only for the repro itself
    /// (`new_line_repro`); the repro then only closes the page, so every other later call is refused.
    ///
    /// Rows left in the view, seven or more rows shown before, or any other shape with the new-row line are not pinned
    /// (`false`: the caller refuses).
    pub(super) fn post_left_other_key_settled(&mut self, st: &St, rows_before: usize) -> Result<bool, Signal> {
        let on_blank = (1..=6).contains(&rows_before) && {
            let s = st.borrow();
            s.gone_blank_no_new_line && s.open && s.cursor == Cursor::None && !s.draft
        };
        if on_blank {
            return Ok(true);
        }
        if !self.new_line_repro(st, rows_before)? {
            return Ok(false);
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::PostLeftOtherKeyOnNewLine);
        st.borrow_mut().pinned_end = Some(PinnedEnd {
            reason: "a call of the test after the Post of a journal that left another primary key in Rec on its new-row line, \
                     other than Close (not pinned)",
            reads: false,
            one_action: false,
            first_ends: false,
            part_visible: false,
            part_moves: false,
            next: NextEnd::Refused,
        });
        Ok(true)
    }

    /// The entry condition of the new-row line case. Every fact is the reporter's (al-runner issue 7821), and all are required:
    /// - the page, its table and the procedure its triggers call read as `NEW_LINE_PAGE`, `LINE_TABLE` and `LOG_ADD`, with no
    ///   page or table extension and no subscriber;
    /// - the page: top-level, opened for edit (by OpenEdit, or by Page.Run on the trapped TestPage), not modal, not a lookup;
    /// - the test's calls since the open: only this Invoke of Post;
    /// - one row shown before the action, the refresh just ran the blank line three times (`settle_after_action`), the page
    ///   stands on it, and the table holds no row at all.
    fn new_line_repro(&mut self, st: &St, rows_before: usize) -> Result<bool, Signal> {
        let on_new_line = rows_before == 1 && {
            let s = st.borrow();
            s.gone_new_line_settled
                && s.open
                && s.cursor == Cursor::NewLine
                && s.placeholder_line
                && s.mode == PageMode::Edit
                && !s.modal
                && !s.lookup
                && s.host.is_none()
        };
        let calls_ok = matches!(
            Self::test_calls(st).as_deref(),
            Some([TestCall::Other(PageMethod::Invoke)])
        );
        if !(on_new_line && calls_ok) {
            return Ok(false);
        }
        let rec = self.page_rec(st);
        let table = rec.borrow().table as usize;
        let engine = self.engine();
        let mut callees = Callees::new();
        let code_ok = page_is(&engine, st.borrow().object as usize, NEW_LINE_PAGE, &mut callees)
            && table_is(&engine, table, LINE_TABLE, &mut callees)
            && callees_are(&engine, &callees, LOG_ADD);
        drop(engine);
        if !code_ok {
            return Ok(false);
        }
        let all_rows = Rc::new(RefCell::new(rec.borrow().clone()));
        {
            let mut a = all_rows.borrow_mut();
            a.filters.clear();
            a.marked_only = false;
        }
        Ok(self.view_rows(&all_rows, 1)?.is_empty())
    }
}
