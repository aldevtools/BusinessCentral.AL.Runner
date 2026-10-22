//! Page parts (ADR-012 follow-up): a `part(...)` control of an open page owns a child page state
//! (the `TestPart` handle is that child's `Value::TestPage`). The child opens right after the
//! host's OnOpenPage, and its rows follow the host's current row through `SubPageLink`.

use super::*;
use al_builtins::filter::{FieldFilter, FilterExpr};
use al_builtins::page_op::PageMethod;
use al_builtins::record_op::RecordOp;

/// The filter group `SubPageLink` filters live in
/// (corpus handlers/TestPagePartLinkFilterGroup.al: `FilterGroup(4)` answers the link).
pub(super) const LINK_GROUP: i32 = 4;

/// Which parts [`Session::sync_parts_with`] re-points, and how they read.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Sync {
    /// The host opens: hidden parts read too.
    Open,
    /// Every visible part, first to last.
    Plain,
    /// The host moved with First, Last, Next or Previous: only the linked parts, hidden ones included.
    Move { backward: bool },
    /// The host's GoToKey or GoToRecord stayed on its row: every visible part as `Plain`, except a list part with OnFindRecord, a
    /// filtered SourceTableView and no row trigger, which is not asked again (corpus branch al-runtime-rs-staging-qc eca98051,
    /// pagefindrecord/QcFindViewTests.al:505 DocPart_OpenEdit_LinesOfH1: OpenEdit, GoToKey('H1'), `part_log = find(=><)=Yes:10000;`
    /// once).
    Stayed,
}

impl Session {
    /// Sets `open = false` on `st` and on the pages it shows as parts.
    pub(super) fn mark_closed(&mut self, st: &St) {
        st.borrow_mut().open = false;
        let parts = std::mem::take(&mut st.borrow_mut().parts);
        for (_, child) in parts {
            self.mark_closed(&child);
        }
    }

    /// Creates the page of every part control, before the host's own triggers run: the host's
    /// OnInit and OnOpenPage reach it through `CurrPage.<Part>.Page` (corpus
    /// handlers/TestPageTempPart_Host.al: OnOpenPage hands rows to the part page). A part whose
    /// `Visible` is the literal false is not in the control tree
    /// (testpart/TestTestPart.al:TestPart_InvisiblePart_IsNotInTheControlTreeAtAll).
    pub(super) fn create_parts(&mut self, st: &St) -> Result<(), Signal> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        for (i, c) in page.controls.iter().enumerate() {
            if c.kind != ir::ControlKind::Part || self.control_eliminated(st, i) || self.area_removed(st, i) {
                continue;
            }
            let Some(part_page) = c.part_page else {
                return unsupported(format!("part {} shows no page", c.name));
            };
            // A part page the host's code reached before the host ran (see `part_page`, pinned by pages/lt-part-never-ran) is the
            // one the page opens with: what that call set on it stays (Warehouse Pick Summary.SetRecords hands the summary
            // rows to its part before the page runs).
            let early = {
                let mut s = st.borrow_mut();
                let at = s
                    .hidden_parts
                    .iter()
                    .position(|(c, p)| *c == i && !p.borrow().open && !p.borrow().ran);
                at.map(|at| s.hidden_parts.remove(at).1)
            };
            if let Some(child) = early {
                st.borrow_mut().parts.push((i, child));
                continue;
            }
            let child = Rc::new(RefCell::new(PageState::closed(part_page as u32)));
            child.borrow_mut().host = Some((Rc::downgrade(st), i));
            self.page_instance(&child)?;
            st.borrow_mut().parts.push((i, child));
        }
        Ok(())
    }

    fn init_one_part(&mut self, child: &St) -> Result<(), Signal> {
        if !std::mem::replace(&mut child.borrow_mut().inited, true) {
            self.page_trigger(child, "oninit", vec![])?;
        }
        Ok(())
    }

    /// Runs the OnInit of every part page that has not run it yet.
    pub(super) fn init_parts(&mut self, st: &St) -> Result<(), Signal> {
        let parts = st.borrow().parts.clone();
        for (_, child) in parts {
            if !std::mem::replace(&mut child.borrow_mut().inited, true) {
                self.page_trigger(&child, "oninit", vec![])?;
            }
        }
        Ok(())
    }

    /// Opens the parts right after the host's OnOpenPage: the ones whose `Visible` came out false
    /// are dropped, the others run their OnInit and OnOpenPage.
    pub(super) fn open_parts(&mut self, st: &St) -> Result<(), Signal> {
        // A part is opened inside its host, so the host's read-only state reaches it (corpus
        // handlers/TestPageNewRowLine_Tests.al:ReadOnlyHostPart_NextPastLastDataRow_ReturnsFalse).
        let mode = if self.page_editable(st) {
            PageMode::Edit
        } else {
            PageMode::View
        };
        let parts = st.borrow().parts.clone();
        let mut kept = Vec::new();
        let mut hidden = Vec::new();
        for (i, child) in parts {
            if self.control_visible(st, i)? {
                kept.push((i, child));
            } else {
                hidden.push((i, child));
            }
        }
        st.borrow_mut().parts = kept.clone();
        st.borrow_mut().hidden_parts = hidden.clone();
        for (_, child) in kept.into_iter().chain(hidden) {
            self.open_page_as(&child, mode, None, None, true)?;
        }
        Ok(())
    }

    /// Sets the mode of the pages `st` shows as parts, and of theirs.
    pub(super) fn set_part_modes(&mut self, st: &St, mode: PageMode) {
        let parts = st.borrow().parts.clone();
        for (_, child) in parts {
            child.borrow_mut().mode = mode;
            self.set_part_modes(&child, mode);
        }
    }

    /// Whether the part control that shows `st` says `Editable = false`. What that does to the
    /// part's rows is pinned only for SetValue, which writes; New() and Next() onto the new-row line
    /// of such a part are refused.
    pub(super) fn part_control_read_only(&self, st: &St) -> bool {
        let Some((host, idx)) = st.borrow().host.clone() else {
            return false;
        };
        let Some(host) = host.upgrade() else { return false };
        let engine = self.engine();
        matches!(
            page_of(&engine, &host).controls[idx].prop("editable"),
            Some(ir::PropVal::Lit(v)) if v.trim().eq_ignore_ascii_case("false")
        )
    }

    /// What the part controls that show `st` (its host's, and theirs up the chain) say about `Editable()` (`editable`
    /// true) or `Enabled()` of a field of `st`. `Some(false)`: the answer is No. `None`: the part controls say nothing, the
    /// field answers by itself. Pinned only for a field read through a part whose control says the literal `Editable =
    /// false`: Editable() No (misc/lt-probes-needed-7/PnGPartRows.al:117 ReadOnlyPartSetValue `ro-descr-editable`; a part a page
    /// extension adds, pageext/lt-u-page-props/UPageProps.al:239-240 PageExt_PartStates `ext_part_row_*_editable`).
    /// An Editable or Enabled expression that comes out true counts as no property (assumption
    /// part-field-under-true-part-expression). Everything else a part control's Editable or Enabled can do to its fields
    /// is refused (hard rule 4): an expression that comes out false (Tests-ERM ERM Sales Quotes BlankSalesQuoteLinesUneditable
    /// reads a field behind `Editable = IsSalesLinesEditable` with the flag false), `Enabled = false`, Enabled() behind
    /// `Editable = false`, and `Editable = false` on a part control further up the chain.
    pub(super) fn part_field_gate(&mut self, st: &St, editable: bool) -> Result<Option<bool>, Signal> {
        let what = if editable { "Editable()" } else { "Enabled()" };
        let engine = self.engine();
        let mut answer = None;
        let mut at = st.borrow().host.clone();
        let mut direct = true;
        while let Some((host, idx)) = at {
            let Some(host) = host.upgrade() else { break };
            for name in ["enabled", "editable"] {
                let shown = if name == "enabled" { "Enabled" } else { "Editable" };
                match page_of(&engine, &host).controls[idx].prop(name) {
                    None => {}
                    Some(ir::PropVal::Lit(v)) => {
                        let v = is_true(v);
                        if !v && name == "editable" && editable && direct {
                            answer = Some(false);
                            continue;
                        }
                        if !v {
                            return unsupported(format!(
                                "{what} of a field on a part whose control says {shown} = false (not pinned)"
                            ));
                        }
                    }
                    // An expression that comes out true is taken as no property (assumption
                    // part-field-under-true-part-expression); one that comes out false is not pinned.
                    Some(_) => {
                        if !self.prop_bool_now(&host, idx, name, true)? {
                            return unsupported(format!(
                                "{what} of a field on a part whose control has an {shown} expression that is false (not pinned)"
                            ));
                        }
                        al_builtins::assumptions::assume(
                            al_builtins::assumptions::Assumption::PartFieldUnderTruePartExpression,
                        );
                    }
                }
            }
            direct = false;
            at = host.borrow().host.clone();
        }
        Ok(answer)
    }

    pub(super) fn refuse_read_only_part(&self, st: &St, what: &str) -> Result<(), Signal> {
        if self.part_control_read_only(st) {
            return unsupported(format!(
                "{what} through a part whose control says Editable = false (not pinned)"
            ));
        }
        Ok(())
    }

    /// The `SubPageView` of the part control that shows `st`.
    pub(super) fn sub_page_view(&self, st: &St) -> Option<String> {
        let (host, idx) = st.borrow().host.clone()?;
        let host = host.upgrade()?;
        let engine = self.engine();
        page_of(&engine, &host).controls[idx].part_link.as_ref()?.view.clone()
    }

    /// The page shown by part control `control` of `st`.
    pub(super) fn part_page(&mut self, st: &St, control: usize, runtime: bool) -> Result<St, Signal> {
        // A Page variable's procedure that reaches a part before the page ever ran (Check Credit Limit.PopulateDataOnNotification,
        // which Cust-Check Cr. Limit calls on a Page variable to fill a notification) gets the part page created and not opened, as
        // a hidden part is (pages/lt-part-never-ran/NeverRanReadRec.al, NeverRanUpdate.al, NeverRanGetRecord.al, NeverRanSetRecord.al); after the page ran and closed it stays refused.
        let never_ran = runtime && !st.borrow().open && !st.borrow().ran;
        if never_ran {
        } else {
            match self.require_open(st) {
                // The page's own code reaching a part after the page ran and closed is not a TestPage the test closed (Posted Purchase
                // Document Lines.CopyLineToDoc reads `CurrPage.PostedRcpts.PAGE` after its RunModal returned): refused, as the other
                // calls of such a page are (see test_page_op).
                Err(Signal::Error(e)) if runtime && e.code == "TestPageNotOpened" => {
                    return unsupported(
                        "a part of a page that is not open, reached by the code of that page (what it does then is not pinned)",
                    );
                }
                r => r?,
            }
        }
        let found = st
            .borrow()
            .parts
            .iter()
            .find(|(i, _)| *i == control)
            .map(|(_, c)| c.clone());
        // The page's own code reaches a part whose Visible came out false (Base Application's journals call
        // `CurrPage.WorkflowStatusBatch.PAGE.SetFilterOnWorkflowRecord` behind `Visible = <variable>`; Tests-General
        // Journal, 7 tests; corpus pages/lt-hidden-part-currpage/HiddenPartCurrPageEditable.al).
        // A part whose Visible expression came out false is in the test page too, answering Visible() No and First() Yes
        // (misc/lt-probes-needed-7/PnGPartTriggers.al:HiddenPartExpression); only a literal `Visible = false` leaves it out
        // (testpart/TestPartNotOnThePage, which never creates the part).
        let found = found.or_else(|| {
            // Test code finds it only when the Visible expression hid it; a part a literal `Visible = false` left out, which page
            // code created on first reach, stays "not found" (misc/lt-probes-needed-9/P99InvisiblePart.al:P99_StaticInvisible).
            st.borrow()
                .hidden_parts
                .iter()
                .find(|(i, _)| *i == control)
                .map(|(_, c)| c.clone())
                // A part the session's application areas removed is not on the test page either (types/lt-conversions-hidden-parts-doctype/
                // P142HiddenPartArea.al:P142_ServiceAreaPartRemoved: `The part with ID = n was not found on the page.`).
                .filter(|_| runtime || !(self.control_eliminated(st, control) || self.area_removed(st, control)))
        });
        // A part the control tree dropped (Visible = false, or an ApplicationArea the session does not enable) is
        // still there for the page's own code: Base Application's Sales Credit Memo calls
        // `CurrPage.IncomingDocAttachFactBox.Page.LoadDataFromRecord(Rec)` in OnAfterGetCurrRecord. It is created on the first such reach, not opened, and kept out of the tree (corpus pages/lt-hidden-part-currpage/HiddenPartCurrPageEditable.al: Editable_LiteralInvisible_OpenEdit, Editable_ExprInvisible_OpenEdit).
        let found = match found {
            None if runtime => {
                let engine = self.engine();
                let part_page = page_of(&engine, st).controls[control].part_page;
                match part_page {
                    Some(pp) => {
                        let child = Rc::new(RefCell::new(PageState::closed(pp as u32)));
                        child.borrow_mut().host = Some((Rc::downgrade(st), control));
                        self.page_instance(&child)?;
                        st.borrow_mut().hidden_parts.push((control, child.clone()));
                        self.init_one_part(&child)?;
                        // It is not opened: its OnOpenPage and OnAfterGetRecord do not run, only the call that reached it
                        // (pages/lt-page-filtergroup-assumptions/P99InvisiblePart.al:P99_StaticInvisible, the log is
                        // `host-OnAfterGetCurrRecord | part-Ping | host-after-Ping`).
                        Some(child)
                    }
                    None => None,
                }
            }
            other => other,
        };
        match found {
            Some(c) => Ok(c),
            None => {
                // Text and code: testpart/TestPartTriggerOrderAndRefresh.al:316-340. The id is
                // `control_id` of the declaring object and the part's name (testpart/TestTestPart.al:233).
                let engine = self.engine();
                let ctl = &page_of(&engine, st).controls[control];
                let declaring = self.object_id_of(ctl.owner.unwrap_or(st.borrow().object as usize) as u32);
                Err(error_code(
                    format!(
                        "The part with ID = {} was not found on the page.",
                        control_id(declaring, &ctl.name)
                    ),
                    "TestPartNotFound",
                ))
            }
        }
    }

    /// Reads the link of every part off the host's current row and loads the parts' first rows.
    /// Providers go first: a part linked through `Provider` reads the provider's current row.
    pub(super) fn sync_parts(&mut self, st: &St) -> Result<(), Signal> {
        self.sync_parts_with(st, Sync::Plain)
    }

    /// [`Self::sync_parts`] as the host opens: the parts hidden by a Visible expression read their rows too, in the order of the
    /// controls (pages/lt-page-filtergroup-assumptions/P99InvisiblePart.al:P99_FlagFalseAtOpen).
    pub(super) fn sync_parts_at_open(&mut self, st: &St) -> Result<(), Signal> {
        self.sync_parts_with(st, Sync::Open)
    }

    /// [`Self::sync_parts`] after the host moved to another row with First, Last, Next or Previous. Only the parts with a
    /// SubPageLink read their rows again; a part without one keeps what it shows and runs no trigger
    /// (pages/lt-part-reads/PartReadsOpen.al:269-283 and :336-379, `-Plain` and `-View` empty;
    /// pages/lt-part-subpageview-filter/PartViewFilter.al:381 `next-GtOnly`; types/lt-conversions-hidden-parts-doctype/
    /// P142HiddenPartArea.al:386, :420). A linked part hidden by a Visible expression reads too (PartReadsOpen.al `-Hidden`).
    /// After Previous and First the linked parts read their rows last to first, once each (PartReadsOpen.al:358-379: 1, 2
    /// and 3 rows; PartViewFilter.al:387-389); after Next and Last first to last, then the first row once more.
    pub(super) fn sync_parts_after_move(&mut self, st: &St, backward: bool) -> Result<(), Signal> {
        self.sync_parts_with(st, Sync::Move { backward })
    }

    /// [`Self::sync_parts`] after a GoToKey or GoToRecord of the host that stayed on its row (`Sync::Stayed`).
    /// A host's Next() from a row whose part showed no row: the part's triggers on the row it lands on see as xRec the row the
    /// part held before it landed on its blank line, not the blank line (pages/lt-qj-part-propagation/QjPartPropagation.al:2625
    /// QN_EmptyPart_HostNext_ThenNextAgain `log-move = S.AGCR(50000 r=5 x=3)` from H3 without lines after H2's line 30000 r=3;
    /// :4521 QT_EmptyPart_HostNext_Steps `log-next-3`). Pinned for Next only; the other moves keep the blank xRec as before.
    pub(super) fn restore_held_xrec_of_empty_parts(&mut self, st: &St) {
        let parts: Vec<St> = st.borrow().parts.iter().map(|(_, c)| c.clone()).collect();
        for child in parts {
            let held = {
                let c = child.borrow();
                (c.open && c.cursor == Cursor::NewLine && c.draft && !c.dirty && c.window.is_empty())
                    .then(|| c.empty_part_xrec.clone())
                    .flatten()
            };
            if let Some(x) = held {
                self.set_xrec_fields(&child, x);
            }
        }
    }

    pub(super) fn sync_parts_stayed(&mut self, st: &St) -> Result<(), Signal> {
        self.sync_parts_with(st, Sync::Stayed)
    }

    fn sync_parts_with(&mut self, st: &St, how: Sync) -> Result<(), Signal> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let opening = how == Sync::Open;
        let mut parts = st.borrow().parts.clone();
        if opening || matches!(how, Sync::Move { .. }) {
            let hidden = st.borrow().hidden_parts.clone();
            parts.extend(
                hidden
                    .into_iter()
                    .filter(|(i, c)| !self.control_eliminated(st, *i) && (opening || c.borrow().open)),
            );
            parts.sort_by_key(|(i, _)| *i);
        }
        if let Sync::Move { .. } = how {
            let mut linked = Vec::new();
            for (i, child) in parts {
                let entries = page.controls[i]
                    .part_link
                    .as_ref()
                    .map(|l| &l.entries[..])
                    .unwrap_or(&[]);
                if entries.is_empty() {
                    continue;
                }
                // A part linked by `const` and `filter` terms only reads nothing off the host's row; whether it reads its rows
                // again after a move is not pinned.
                let reads_a_field = entries.iter().any(|e| matches!(e, ir::LinkEntry::Field(..)));
                if !reads_a_field && child.borrow().open {
                    return unsupported(
                        "a host move with a part linked by const or filter terms only (whether it reads its rows again is not pinned)",
                    );
                }
                linked.push((i, child));
            }
            parts = linked;
        }
        let has_provider = |i: usize| {
            page.controls[i]
                .part_link
                .as_ref()
                .is_some_and(|l| l.provider.is_some())
        };
        for pass in [false, true] {
            for (i, child) in &parts {
                if has_provider(*i) == pass {
                    // A GoToKey of a Document host onto the row it stands on reads the host again, not its parts (pages/
                    // lt-qj-part-propagation/QjPartPropagation.al:485 QJ_Both_PartModifiesHost_ThenHostSave `log-open`, :714
                    // QJ_SubformSaveRecord_StoredRow `log-open = S.AGCR(10000 r=1 x=0)`). So does a Card or ListPlus host
                    // (:3963 QT_HostCard_OpenEdit_GoToKey_PartUpdate and :4135 QT_HostListPlus_ `log-gotokey = H.AGR(H1 t=1
                    // x=H1) H.AGCR(H1 t=1 x=H1)`, with Both :4049 and :4221; :2128 QN_Card_PartUpdate_HostRowUnchanged `log-open`).
                    let document_host = matches!(
                        self.engine.object_facts[st.borrow().object as usize].page_type,
                        Some(al_ir::PageType::Document | al_ir::PageType::Card | al_ir::PageType::ListPlus)
                    );
                    if how == Sync::Stayed && child.borrow().open && document_host {
                        continue;
                    }
                    if how == Sync::Stayed && child.borrow().open && self.filtered_find_list_without_row_triggers(child)
                    {
                        // The part asks its page nothing while the host stays (pagefindrecord/QcFindViewTests.al:540 DocPart_OpenEdit_LinesOfH2, `part_walk_log` empty).
                        continue;
                    }
                    if opening && self.view_names_link_field(st, *i, child)? {
                        child.borrow_mut().read_backwards = Some(al_builtins::value::ReadBackwards::AtOpen);
                    }
                    if let Sync::Move { backward: true } = how {
                        // The pinned parts walk the table; a part with its own OnFindRecord or OnNextRecord is not pinned.
                        if self.declares(child, "onfindrecord") || self.declares(child, "onnextrecord") {
                            return unsupported(
                                "a host moved back with a linked part that declares OnFindRecord or OnNextRecord (reads not pinned)",
                            );
                        }
                        child.borrow_mut().read_backwards = Some(al_builtins::value::ReadBackwards::AfterMove);
                    }
                    self.sync_part(st, *i, child)?;
                    // A part that found no row to fill does not carry the order to a later fill.
                    child.borrow_mut().read_backwards = None;
                }
            }
        }
        Ok(())
    }

    /// A host of another type than Document: the parts with a SubPageLink read their rows again from the first, as after a
    /// refresh of the host (testpart/TestPartTriggerOrderAndRefresh.al: HostCurrPageUpdate_ReReadsThePart), without the host
    /// reading its own row. Not pinned: assumption host-save-record-refreshes-linked-parts.
    fn refresh_linked_parts_after_save(&mut self, st: &St) -> Result<(), Signal> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let parts: Vec<(usize, St)> = st
            .borrow()
            .parts
            .iter()
            .filter(|(i, c)| {
                c.borrow().open
                    && page.controls[*i]
                        .part_link
                        .as_ref()
                        .is_some_and(|l| l.entries.iter().any(|e| matches!(e, ir::LinkEntry::Field(..))))
            })
            .cloned()
            .collect();
        if parts.is_empty() {
            return Ok(());
        }
        al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::HostSaveRecordRefreshesLinkedParts);
        let has_provider = |i: usize| {
            page.controls[i]
                .part_link
                .as_ref()
                .is_some_and(|l| l.provider.is_some())
        };
        for pass in [false, true] {
            for (i, child) in &parts {
                if has_provider(*i) == pass {
                    self.sync_part(st, *i, child)?;
                }
            }
        }
        Ok(())
    }

    /// A host control's OnValidate called `CurrPage.SaveRecord` (and no `CurrPage.Update`), whether the row changed or not:
    /// every part of the host reads its rows again where it stands, the linked and the unlinked ones, and the host reads
    /// nothing (testpart/QiHostSaveParts.al:362 QI_HostSaveRecord_ChangedValue, :386 _SameValue, :412 _PartMovedToSecondRow,
    /// :580 QI_PartNewRow_ThenHostSaveRecord). Pinned for a Document host whose parts are ListParts of 2 to 4 rows standing
    /// on a stored row; any other shape is refused.
    pub(super) fn reread_parts_after_host_save_record(&mut self, st: &St) -> Result<(), Signal> {
        let parts: Vec<(usize, St)> = st
            .borrow()
            .parts
            .iter()
            .filter(|(_, c)| c.borrow().open)
            .cloned()
            .collect();
        if parts.is_empty() {
            return Ok(());
        }
        let facts = &self.engine.object_facts;
        let object = st.borrow().object as usize;
        if facts[object].page_type != Some(al_ir::PageType::Document) {
            // Kept, not pinned: a host of another type reads its linked parts again from their first rows, as before the
            // Document host was pinned (baseline case 60412 testpart/... NewHostRowSavedByItsOwnTrigger_IsInsertedOnce).
            return self.refresh_linked_parts_after_save(st);
        }
        if !st.borrow().hidden_parts.is_empty() {
            // A host with a part hidden by a Visible expression is not pinned: its linked parts read again from their first
            // rows as before, under assumption host-save-record-refreshes-linked-parts (Microsoft's Tests-ERM ERM Purchase
            // Subform 134394 and Document Totals Pages: Purchase documents have such parts; 30 tests pass through it).
            return self.refresh_linked_parts_after_save(st);
        }
        for (_, child) in &parts {
            let o = child.borrow().object as usize;
            let list_part =
                matches!(&self.engine.object_facts[o].page_type, Some(al_ir::PageType::Other(t)) if t == "listpart");
            let (cursor, draft, dirty) = {
                let c = child.borrow();
                (c.cursor, c.draft, c.dirty)
            };
            if !list_part
                || !self.has_source(child)
                || cursor != Cursor::Row
                || draft
                || dirty
                || self.declares(child, "onfindrecord")
                || self.declares(child, "onnextrecord")
            {
                return unsupported(
                    "CurrPage.SaveRecord in a host control's OnValidate with a part that is not a ListPart standing on a stored row (the part reads are not pinned)",
                );
            }
        }
        for (_, child) in &parts {
            self.reread_part_in_place(child)?;
        }
        Ok(())
    }

    /// A part reads its rows again where it stands: the current row, the rows after it to the end of the view, the rows before
    /// it back to the start, the current row twice more, then OnAfterGetCurrRecord (testpart/QiHostSaveParts.al:362 on row
    /// 10000 of 2 `L.AGR(10000);L.AGR(20000);L.AGR(10000);L.AGR(10000);L.AGCR(10000)`, :412 on row 20000 of 2 `20000;10000;
    /// 20000;20000`, :580 on row 30000 of 3 `30000;20000;10000;30000;30000`; the unlinked part of 3 and 4 rows on its first).
    /// A part that shows a row a sibling saved first reads the row it stands on (:580 `U.AGR(D1/10000 x=D1/30000)` before the
    /// read from there). Views of 2 to 4 rows are pinned; others are refused.
    fn reread_part_in_place(&mut self, child: &St) -> Result<(), Signal> {
        let rec = self.page_rec(child);
        let stale = child.borrow_mut().shows_sibling_save.take();
        if let Some(at) = &stale {
            rec.borrow_mut().fields = at.clone();
            child.borrow_mut().unpinned_state = None;
        }
        let table = self.table(&rec)?;
        let key = table.primary_key(&rec.borrow().fields);
        let Some(current) = self.stored_row(&rec)? else {
            return unsupported("a part re-read after a host SaveRecord whose current row is gone (not pinned)");
        };
        rec.borrow_mut().fields = current.clone();
        let mut after = Vec::new();
        while self.nav_next(child, 1)? {
            after.push(rec.borrow().fields.clone());
            if after.len() > 4 {
                break;
            }
        }
        rec.borrow_mut().fields = current.clone();
        let mut before = Vec::new();
        while self.nav_next(child, -1)? {
            before.push(rec.borrow().fields.clone());
            if before.len() > 4 {
                break;
            }
        }
        let rows = 1 + after.len() + before.len();
        if !(2..=4).contains(&rows) {
            return unsupported(format!(
                "a part re-read after a host SaveRecord with {rows} rows in its view (only 2 to 4 are pinned)"
            ));
        }
        if stale.is_some() {
            rec.borrow_mut().fields = current.clone();
            self.populate_current(child)?;
            child.borrow_mut().agcr_xrec = Some(current.clone());
        }
        let mut order = vec![current.clone()];
        order.extend(after.iter().cloned());
        order.extend(before.iter().cloned());
        order.push(current.clone());
        order.push(current.clone());
        let mut shown_current = current.clone();
        for (i, row) in order.iter().enumerate() {
            rec.borrow_mut().fields = row.clone();
            self.populate_current(child)?;
            if i + 1 == order.len() {
                shown_current = rec.borrow().fields.clone();
            }
        }
        let mut view: Vec<Vec<Value>> = before.into_iter().rev().collect();
        view.push(current.clone());
        view.extend(after);
        let window = view
            .into_iter()
            .map(|stored| al_builtins::value::WindowRow {
                key: table.primary_key(&stored),
                shown: if Self::same_key(&table.primary_key(&stored), &key) {
                    shown_current.clone()
                } else {
                    stored.clone()
                },
                stored,
            })
            .collect();
        rec.borrow_mut().fields = shown_current;
        self.set_xrec_fields(child, current);
        {
            let mut c = child.borrow_mut();
            c.window = window;
            c.window_from_start = true;
            c.window_eof = true;
            c.dirty = false;
        }
        self.page_trigger(child, "onaftergetcurrrecord", vec![])?;
        self.page_event(child, "onaftergetcurrrecordevent", None, None)?;
        Ok(())
    }

    /// The open parts of `st` (shown, and hidden by a Visible expression) whose `CurrPage.Update` waits for its refresh.
    pub(super) fn parts_waiting_refresh(&self, st: &St) -> Vec<(usize, St)> {
        let s = st.borrow();
        s.parts
            .iter()
            .chain(s.hidden_parts.iter())
            .filter(|(_, c)| c.borrow().open && c.borrow().refresh)
            .cloned()
            .collect()
    }

    /// The host's own code (its OnAfterGetCurrRecord as it opens or moves) called a procedure of a part that ran
    /// `CurrPage.Update`: once the host's parts have followed its row, such a part fills its rows again from the first, as
    /// it does when it opens (pages/lt-hidden-part-currpage/HiddenPartCurrPageUpdateFalse.al:196-198 and
    /// HiddenPartCurrPageUpdateTrue.al:196-198: `part-OnAfterGetRecord:1 | :2 | :1` after `host-end`, twice at open, once
    /// after Next). `before` are the parts that waited already before the host's code ran. Pinned for a part without a
    /// SubPageLink hidden by a Visible expression, standing on no row typed into; any other part is refused.
    /// `pinned` is false for a host move other than Next; `skipped_unlinked` says the move left the parts without a link alone.
    pub(super) fn refresh_parts_asked_by_host(
        &mut self,
        st: &St,
        before: &[(usize, St)],
        pinned: bool,
        skipped_unlinked: bool,
    ) -> Result<(), Signal> {
        let asked = self.parts_waiting_refresh(st);
        if asked.is_empty() {
            return Ok(());
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        for (i, child) in asked {
            let newly = !before.iter().any(|(b, _)| *b == i);
            let hidden = st.borrow().hidden_parts.iter().any(|(h, _)| *h == i);
            let linked = page.controls[i]
                .part_link
                .as_ref()
                .is_some_and(|l| !l.entries.is_empty() || l.provider.is_some());
            let (draft, dirty) = {
                let c = child.borrow();
                (c.draft, c.dirty)
            };
            let shape_pinned = newly
                && pinned
                && hidden
                && !linked
                && !draft
                && !dirty
                && !self.declares(&child, "onfindrecord")
                && !self.declares(&child, "onnextrecord");
            if !shape_pinned {
                // Kept, not pinned: before this refresh was pinned such a request was not carried out on its own. Refusing it
                // loses baseline cases, so the earlier behavior stays: at open nothing more happens (handlers/
                // TestPageActionRunObjectKindsPrecompiled_Tests.al:101, Base Application's Receivables-Payables); after a
                // host move that left a shown part without a link alone, that part fills again, as every move made it do
                // before (crates/al-test-cli/tests/list_read_order.rs, Microsoft's ERM Base Calendar 134233).
                if skipped_unlinked && !hidden && !linked {
                    child.borrow_mut().refresh = false;
                    self.sync_part(st, i, &child)?;
                }
                continue;
            }
            child.borrow_mut().refresh = false;
            self.sync_part(st, i, &child)?;
        }
        Ok(())
    }

    /// Whether the part control's SubPageView restricts, with a `const` condition, a field its SubPageLink also names.
    /// Such a part reads its rows last to first, once each, when the host opens (pages/lt-part-reads/PartReadsOpen.al:
    /// OpenView_SixParts_ReadsPerPartLogged `open-Same`, 3 rows; misc/lt-probes-needed-7/PnGPartTriggers.al:
    /// OpenOrderOfHostWithSixParts, 2 rows). Only `const` on a linked field is pinned: another condition on a linked field
    /// is refused, since the reads it gives are not known.
    fn view_names_link_field(&self, host: &St, control: usize, child: &St) -> Result<bool, Signal> {
        let engine = self.engine();
        let page = page_of(&engine, host);
        let Some(link) = page.controls[control].part_link.as_ref() else {
            return Ok(false);
        };
        let Some(view) = link.view.as_ref() else {
            return Ok(false);
        };
        if !self.has_source(child) {
            return Ok(false);
        }
        let table = self.table(&self.page_rec(child))?;
        let view = view.to_ascii_lowercase();
        let mut found = false;
        for entry in &link.entries {
            let ir::LinkEntry::Field(pf, _) = entry else { continue };
            let name = table.fields[*pf].name.to_ascii_lowercase();
            for form in [format!("\"{name}\""), name.clone()] {
                let Some(at) = view
                    .find(&format!("{form} ="))
                    .or_else(|| view.find(&format!("{form}=")))
                else {
                    continue;
                };
                let after = view[at + form.len()..]
                    .trim_start()
                    .trim_start_matches('=')
                    .trim_start();
                if !after.starts_with("const(") {
                    return unsupported(format!(
                        "a SubPageView condition other than const on the linked field {name} (not pinned)"
                    ));
                }
                found = true;
                break;
            }
        }
        Ok(found)
    }

    /// Whether a part linked on host field `pos` links a field whose TableRelation names a table other than the host's.
    fn part_link_relates_elsewhere(&self, st: &St, pos: usize) -> Result<bool, Signal> {
        let engine = self.engine();
        let page = page_of(&engine, st);
        let host_table = self.table(&self.page_rec(st))?.name.to_ascii_lowercase();
        let parts = st.borrow().parts.clone();
        for (i, child) in &parts {
            let Some(link) = page.controls[*i].part_link.as_ref() else {
                continue;
            };
            if link.provider.is_some() || !self.has_source(child) {
                continue;
            }
            let ti = self.page_rec(child).borrow().table as usize;
            let Some(table) = engine.program.objects[ti].table.as_ref() else {
                continue;
            };
            for e in &link.entries {
                if let ir::LinkEntry::Field(pf, hf) = e
                    && *hf == pos
                    && table.fields[*pf]
                        .relation_table
                        .as_ref()
                        .is_some_and(|t| *t != host_table)
                {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    /// Refuses a call on a page, or on a part of a page, that reached a state whose behavior is not pinned.
    pub(super) fn refuse_unpinned_state(&self, st: &St) -> Result<(), Signal> {
        let host = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade());
        let why = st
            .borrow()
            .unpinned_state
            .clone()
            .or_else(|| host.and_then(|h| h.borrow().unpinned_state.clone()));
        match why {
            Some(why) => unsupported(why),
            None => Ok(()),
        }
    }

    /// A write into part `child` moves the focus out of its sibling parts: a row typed into one of them is saved first
    /// (pages/lt-part-link-unsaved-host/PagePartLinkUnsavedHost.al:242, the line typed into LinesG is stored once LinesH is
    /// written, LinesH's own line is not).
    pub(super) fn save_sibling_parts(&mut self, child: &St) -> Result<(), Signal> {
        let Some(host) = child.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(());
        };
        let siblings: Vec<St> = host
            .borrow()
            .parts
            .iter()
            .map(|(_, c)| c.clone())
            .filter(|c| !Rc::ptr_eq(c, child) && c.borrow().open && c.borrow().dirty)
            .collect();
        for sibling in siblings {
            self.save_row(&sibling)?;
        }
        Ok(())
    }

    /// A write into part `child` (SetValue, New) while its host stands on a typed row that is not stored yet: the host row is
    /// saved first, and the parts link to it (pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:406
    /// `HP.OnInsertRecord:D2 | H.OnInsert:D2 | L.OnValidate(Document No.):D2 | ...` for New(); handlers/
    /// TestPageOpenNewPartLink.al:100 OpenNew_NumberTyped_PartWriteCarriesTheNumber for a write).
    pub(super) fn save_unsaved_host_first(&mut self, child: &St) -> Result<(), Signal> {
        let Some(host) = child.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(());
        };
        let unsaved = {
            let h = host.borrow();
            h.open && h.host.is_none() && h.draft && !h.draft_stored && h.dirty && h.cursor != Cursor::None
        };
        if !unsaved || !self.has_source(&host) {
            return Ok(());
        }
        // A new host whose fields were typed before its key: what BC stores then is not known (pages/
        // lt-new-line-stamps-filters/NewLineStampsFilters.al:264 PartB_OpenNewHeader_TypeThenNo stores the line with a
        // blank link though the key was typed; the rule is not found).
        if host.borrow().typed_before_key {
            return unsupported(
                "a part written into while its new host row had a field typed before its key (what is stored is not pinned)",
            );
        }
        self.save_row(&host)
    }

    /// Last() or New() on the only part of a new Document host that nothing was typed into yet, the part showing no row: the
    /// client inserts the host's blank row first, and the part, linked afresh, lands on its blank line three times as at open
    /// (pages/lt-qj-part-propagation/QjPartPropagation.al:4415 QT_EmptyPart_HostOpenNew_LastOnly `heads-before-close = [] [H1]
    /// [H2] [H3]`, `log-last` = five `S.NEW(x=0 x.ln=0) S.AGCR(0 r=0 x=0)`; :4441 QT_EmptyPart_HostOpenNew_NewOnly `log-new` = four
    /// of them; :4466 QT_EmptyPart_HostOpenNew_LastThenNew `log-new` = one after the Last; :2596 QN_EmptyPart_HostOpenNew; :4393
    /// _CloseOnly and :4495 _ReadHostOnly store nothing). The pinned host declares no trigger and its table has no insert code:
    /// a host or table that could observe the insert or a read after it, and a host with more than one part, are refused.
    /// Answers whether the host row was inserted.
    pub(super) fn insert_untouched_new_host(&mut self, child: &St) -> Result<bool, Signal> {
        let Some(host) = child.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(false);
        };
        let untouched = {
            let h = host.borrow();
            h.open && h.host.is_none() && h.draft && !h.draft_stored && !h.dirty && h.cursor != Cursor::None
        };
        let part_empty = {
            let c = child.borrow();
            c.open && c.cursor == Cursor::NewLine && c.draft && !c.dirty && c.window.is_empty()
        };
        if !untouched || !part_empty || !self.has_source(&host) || self.is_list(&host) {
            return Ok(false);
        }
        let o = host.borrow().object as usize;
        if self.engine.object_facts[o].page_type != Some(al_ir::PageType::Document) {
            return unsupported(
                "Last() or New() on an empty part of a new host other than a Document page that nothing was typed into (whether the host row is inserted is not pinned)",
            );
        }
        let rec = self.page_rec(&host);
        let table = self.table(&rec)?;
        let host_observes = !page_of(&self.engine(), &host).triggers.is_empty()
            || host.borrow().parts.len() != 1
            || self.table_trigger(&table, "OnInsert").is_some()
            || ["OnBeforeInsert", "OnInsert", "OnAfterInsert"]
                .iter()
                .any(|n| !self.ext_triggers(&table, n).is_empty())
            || self.listened(&table, "onbeforeinsertevent", None)?
            || self.listened(&table, "onafterinsertevent", None)?;
        if host_observes {
            return unsupported(
                "Last() or New() on an empty part of a new Document host that nothing was typed into, whose host has page triggers, insert code or more than one part (only the plain insert is pinned)",
            );
        }
        host.borrow_mut().dirty = true;
        // The insert links the part to the stored row afresh (`insert_draft` syncs the parts).
        self.insert_draft(&host)?;
        Ok(true)
    }

    /// After part `child` moved, the parts that take their link from it follow.
    pub(super) fn sync_dependents(&mut self, child: &St) -> Result<(), Signal> {
        let Some((host, idx)) = child.borrow().host.clone() else {
            return Ok(());
        };
        let Some(host) = host.upgrade() else { return Ok(()) };
        let engine = self.engine();
        let page = page_of(&engine, &host);
        let parts = host.borrow().parts.clone();
        for (i, sibling) in &parts {
            let provider = page.controls[*i].part_link.as_ref().and_then(|l| l.provider);
            if provider == Some(idx) {
                self.sync_part(&host, *i, sibling)?;
            }
        }
        Ok(())
    }

    /// The page part `control` of `host` reads its link from: the Provider part, else the host.
    fn link_page(&self, host: &St, control: usize) -> Option<St> {
        let engine = self.engine();
        let page = page_of(&engine, host);
        match page.controls[control].part_link.as_ref().and_then(|l| l.provider) {
            Some(pi) => host
                .borrow()
                .parts
                .iter()
                .find(|(i, _)| *i == pi)
                .map(|(_, c)| c.clone()),
            None => Some(host.clone()),
        }
    }

    /// Points part `child` (control `control` of `host`) at the host's current row and loads its
    /// first row.
    fn sync_part(&mut self, host: &St, control: usize, child: &St) -> Result<(), Signal> {
        if !child.borrow().open {
            return Ok(());
        }
        // What was typed into the row the part stands on is saved before it moves on.
        self.save_row(child)?;
        // The filter `SetSelectionFilter(Rec)` left on the part's own record is gone once the part refreshes with its host
        // (HiddenPartSelectionFilter.al:SelectionFilter_VisiblePart_FilterAndCount `rec-before=/3` after Next).
        if std::mem::take(&mut child.borrow_mut().selection_on_rec) {
            self.page_rec(child)
                .borrow_mut()
                .filters
                .retain(|f| f.group == LINK_GROUP);
        }
        let engine = self.engine();
        let page = page_of(&engine, host);
        if let (Some(link), true) = (page.controls[control].part_link.as_ref(), self.has_source(child)) {
            let source = self.link_page(host, control).and_then(|p| {
                self.has_source(&p).then(|| {
                    let t = self.page_rec(&p).borrow().table as usize;
                    (p, t)
                })
            });
            let rec = self.page_rec(child);
            let table = self.table(&rec)?;
            let mut fresh: Vec<FieldFilter> = Vec::new();
            let mut fields = Vec::new();
            for entry in &link.entries {
                match entry {
                    ir::LinkEntry::Field(pf, hf) => {
                        fields.push(*pf);
                        let ty = table.fields[*pf].ty.clone();
                        let Some((src_page, src_table)) = source.as_ref() else {
                            return unsupported("a SubPageLink to a page without a source table");
                        };
                        // The row the link reads, else the type's blank value: a page that shows
                        // no row links to nothing (corpus testpart/TestPartProviderMove.al:
                        // ProviderLinkedPart_HostMovesToAnEmptyProvider_ReadsBlank).
                        // A host row that is not stored yet links to nothing either, also after its key was typed: the
                        // part's blank line takes the blank value and validates it (pages/lt-part-line-before-host-key/
                        // PartLineBeforeHostKey.al:398, :400 `H.OnValidate(No.):D2 | L.OnValidate(Document No.): | ...`).
                        let unsaved_host = {
                            let p = src_page.borrow();
                            p.host.is_none() && p.draft && !p.draft_stored
                        };
                        let value = if src_page.borrow().cursor == Cursor::None || unsaved_host {
                            let host_ty = engine.program.objects[*src_table]
                                .table
                                .as_ref()
                                .map(|t| t.fields[*hf].ty.clone())
                                .ok_or_else(|| Signal::Unsupported("a link to a table without a schema".into()))?;
                            Value::default_for(&host_ty)
                        } else {
                            self.page_rec(src_page).borrow().fields[*hf].clone()
                        };
                        let value = ops::convert(value, &ty)?;
                        fresh.push(FieldFilter {
                            field: *pf,
                            expr: FilterExpr::Range(Some(value.clone()), Some(value)),
                            group: LINK_GROUP,
                        });
                    }
                    ir::LinkEntry::Filter(pf, text) => {
                        if text == "''" {
                            al_builtins::assumptions::assume(
                                al_builtins::assumptions::Assumption::SubPageLinkEmptyTerm,
                            );
                        }
                        fields.push(*pf);
                        let ty = table.fields[*pf].ty.clone();
                        let expr = self
                            .parse_filter_text(text, &ty)
                            .map_err(|e| Signal::Unsupported(format!("SubPageLink filter: {e}")))?;
                        fresh.push(FieldFilter {
                            field: *pf,
                            expr,
                            group: LINK_GROUP,
                        });
                    }
                }
            }
            let mut r = rec.borrow_mut();
            r.filters
                .retain(|f| !(f.group == LINK_GROUP && fields.contains(&f.field)));
            r.filters.extend(fresh);
        }
        {
            let mut s = child.borrow_mut();
            s.cursor = Cursor::None;
            s.draft = false;
            s.dirty = false;
            s.window = Vec::new();
            s.empty_part_xrec = None;
            s.focus = None;
            s.focus_in_row = false;
            s.refused = false;
            s.pending = None;
        }
        let mode = child.borrow().mode;
        // The refill runs the CaptionClass expressions with its first row only (QdCaptionClassAreas.al:1022-1066
        // QO_Doc_OpenEdit_HostSteps `host_next = rest=70@3;71@3;`, `host_next_exprA = Line 3`).
        {
            let mut c = child.borrow_mut();
            c.field_captions_settled = false;
            c.field_captions_fill_first = true;
        }
        let loaded = self.load_first_row(child, mode, false);
        {
            let mut c = child.borrow_mut();
            c.field_captions_settled = true;
            c.field_captions_fill_first = false;
        }
        loaded
    }

    /// A field the host just changed is read by the link of some parts: they follow the new value
    /// (corpus handlers/TestPageOpenNewPartLink.al:OpenNew_NumberTyped_PartWriteCarriesTheNumber).
    pub(super) fn resync_parts_linked_by(&mut self, st: &St, control: usize) -> Result<(), Signal> {
        if st.borrow().parts.is_empty() || !st.borrow().open {
            return Ok(());
        }
        let engine = self.engine();
        let page = page_of(&engine, st);
        let ir::Binding::Field(pos) = page.controls[control].binding else {
            return Ok(());
        };
        // A key field typed on a host row that is stored (a new row the host inserted when a field outside the key was
        // typed first): BC keeps the parts' blank lines on the old link value (pages/lt-new-line-stamps-filters/
        // NewLineStampsFilters.al:264 PartB_OpenNewHeader_TypeThenNo, the line is stored with a blank "Document No.");
        // the rule is not found, so it is refused.
        let key_of_stored_row = {
            let s = st.borrow();
            !s.draft && s.cursor == Cursor::Row
        } && self.has_source(st)
            && self.table(&self.page_rec(st))?.keys[0].contains(&pos)
            && st.borrow().typed_before_key;
        // A new Card row whose key is typed while two or more parts link on it: BC stores the row at once
        // (pages/lt-part-link-unsaved-host/PagePartLinkUnsavedHost.al:178, :201, :218 `a_hosts = 1`), and then shows a blank key
        // and parts without rows (:181-183); what the page does after that is not understood (the shape conflicts with
        // pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:401, a Document page that stores nothing). The row is
        // stored, and every later call on the page or its parts but Close is refused.
        let linked_on_key = parts_linked_on(page, &st.borrow().parts, pos);
        let new_card = {
            let s = st.borrow();
            s.host.is_none() && s.draft && !s.draft_stored && s.cursor != Cursor::None
        } && matches!(
            self.engine.object_facts[st.borrow().object as usize].page_type,
            Some(al_ir::PageType::Card)
        ) && self.has_source(st)
            && self.table(&self.page_rec(st))?.keys[0].contains(&pos);
        // Only the shape of that file: one of the parts links a field whose TableRelation names another table than the
        // host's (its "Group Code"). A Card whose two parts both relate to the host (pages/lt-new-line-stamps-filters/
        // NewLineStampsFilters.al:289 PartB_OpenNewHeader_NoThenType) carries on as before.
        // A Card with a SourceTableView: typing the key stores a header with a blank key, and the card and its parts' lines keep
        // the blank key (NewLineStampsFilters.al:534-597 QO_HostB_OpenNew_DelayedPart_NoThenType, QO_HostB_OpenNew_PartB_NoThenType:
        // `after_no.headers = 1 [|A]`, `typed = |A|A`, corpus bump 45). The rule is not understood: refused.
        if new_card && linked_on_key >= 1 && self.engine.object_facts[st.borrow().object as usize].has_source_table_view
        {
            return unsupported(
                "a key field typed on a new Card row of a page with a SourceTableView, with parts linked on it (BC stores a row with a blank key; not modelled)",
            );
        }
        let foreign_relation = new_card && linked_on_key >= 2 && self.part_link_relates_elsewhere(st, pos)?;
        if foreign_relation {
            st.borrow_mut().dirty = true;
            self.save_row(st)?;
            st.borrow_mut().unpinned_state = Some(
                "a new Card row whose key was typed while two parts link on it (what the page shows afterwards is not pinned)"
                    .into(),
            );
            return Ok(());
        }
        let parts = st.borrow().parts.clone();
        for (i, child) in &parts {
            if key_of_stored_row
                && page.controls[*i].part_link.as_ref().is_some_and(|l| {
                    l.entries
                        .iter()
                        .any(|e| matches!(e, ir::LinkEntry::Field(_, hf) if *hf == pos))
                })
            {
                return unsupported(
                    "a key field typed on a new host row stored after a field outside its key was typed, with parts linked on it (not pinned)",
                );
            }
            let reads = page.controls[*i].part_link.as_ref().is_some_and(|l| {
                l.provider.is_none()
                    && l.entries
                        .iter()
                        .any(|e| matches!(e, ir::LinkEntry::Field(_, hf) if *hf == pos))
            });
            if reads {
                child.borrow_mut().field_captions_link_refill = true;
                let r = self.sync_part(st, *i, child);
                child.borrow_mut().field_captions_link_refill = false;
                r?;
            }
        }
        Ok(())
    }

    /// A part list that shows rows of the host's own table, edited through SetValue on the host's current row: the host card reads that
    /// row's new values and runs its OnAfterGetCurrRecord once more, so its fields show the edit, whatever `UpdatePropagation` says
    /// (pages/lt-page-controls-aa/AaPartsLists.al:AA_UpdatePropagation_Both line 191, _SubPart line 218, _Default line 245). The part
    /// saves its row later, when its cursor leaves it (testpart/TestPartCloseLinkEditableContracts.al:Parts_EnabledReadsTheControl_
    /// EditableFalseDoesNotBlockWrites). Pinned for a host card over the same table, without OnFindRecord; any other pairing keeps the
    /// earlier behavior.
    pub(super) fn refresh_host_after_part_edit(&mut self, st: &St) -> Result<(), Signal> {
        let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(());
        };
        if host.borrow().open
            && self.is_list(&host)
            && self.is_list(st)
            && self.has_source(&host)
            && self.has_source(st)
        {
            let (a, b) = (self.page_rec(&host), self.page_rec(st));
            if self.table(&a)?.object == self.table(&b)?.object {
                return unsupported("a part edit on a list host over the same table (not pinned)");
            }
        }
        if !host.borrow().open
            || self.is_list(&host)
            || !self.is_list(st)
            || !self.has_source(&host)
            || !self.has_source(st)
            || host.borrow().cursor != Cursor::Row
            || host.borrow().draft
            || self.declares(&host, "onfindrecord")
            || self.declares(&host, "onnextrecord")
        {
            return Ok(());
        }
        let (a, b) = (self.page_rec(&host), self.page_rec(st));
        let table = self.table(&a)?;
        if table.object != self.table(&b)?.object {
            return Ok(());
        }
        if !Self::same_key(
            &table.primary_key(&a.borrow().fields),
            &table.primary_key(&b.borrow().fields),
        ) {
            return Ok(());
        }
        let fields = b.borrow().fields.clone();
        a.borrow_mut().fields = fields;
        let r = (|| {
            self.populate_current(&host)?;
            self.page_trigger(&host, "onaftergetcurrrecord", vec![])?;
            self.page_event(&host, "onaftergetcurrrecordevent", None, None)
        })();
        self.torn_down_on_error(&host, r).map(|_| ())
    }

    /// A part ran `CurrPage.Update` in a trigger of a SetValue, and its code changed the host's stored row (another table): the host
    /// reads its row again in every UpdatePropagation mode, no 'changed by another user' follows, and the host's later save keeps the
    /// change. Over a stored part row the part's typed edit is dropped (Update(false) reads the line again; the part's
    /// OnAfterGetCurrRecord sees the typed row as xRec). With Both the host's triggers run first and then its parts read their rows;
    /// with SubPart or none the part reads its rows first, then the host's triggers run. Pinned by
    /// pages/lt-qj-part-propagation/QjPartPropagation.al:508 (QJ_Both_), :545 (QJ_SubPart_) and :582 (QJ_Default_PartModifiesHost_
    /// ThenHostSave `log-part-edit`). A new part row with Both is pinned below; with SubPart or none it is refused (QjPartPropagation.al:
    /// 1034 and :1071 pin it, but their first observation, Last and New on the part, is not built yet).
    /// Returns whether the part's own refresh was done here.
    pub(super) fn host_after_part_update(&mut self, st: &St) -> Result<bool, Signal> {
        let Some((host, c)) = st
            .borrow()
            .host
            .as_ref()
            .and_then(|(h, c)| h.upgrade().map(|h| (h, *c)))
        else {
            return Ok(false);
        };
        let both = {
            let engine = self.engine();
            matches!(page_of(&engine, &host).controls[c].prop("updatepropagation"), Some(ir::PropVal::Lit(v)) if v.eq_ignore_ascii_case("both"))
        };
        if !host.borrow().open
            || !self.has_source(&host)
            || !self.has_source(st)
            || host.borrow().cursor != Cursor::Row
            || host.borrow().draft
        {
            return Ok(false);
        }
        let hrec = self.page_rec(&host);
        if hrec.borrow().temporary {
            return Ok(false);
        }
        // A part over the host's own table: `refresh_host_after_part_edit` re-reads the host once, and only once
        // (pages/lt-page-controls-aa/AaPartsLists.al:AA_UpdatePropagation_Both line 191: loads-after-part-edit = 2).
        if self.table(&hrec)?.object == self.table(&self.page_rec(st))?.object {
            return Ok(false);
        }
        let Some(stored) = self.stored_row(&hrec)? else {
            return Ok(false);
        };
        let read = self.page_xrec_fields(&host);
        if Self::same_values(&stored, &read) {
            return self.part_update_host_unchanged(st, &host, c, both);
        }
        if host.borrow().dirty {
            return unsupported(
                "a part's CurrPage.Update after its code changed the host's stored row while the host holds an unsaved edit (not pinned)",
            );
        }
        let prec = self.page_rec(st);
        let part_stored = !st.borrow().draft && st.borrow().cursor == Cursor::Row && self.stored_row(&prec)?.is_some();
        if !part_stored && !both {
            return self.new_part_row_update_without_both(st, &host, false, true);
        }
        // A new part row with Both: the part keeps its typed row (no read of the part, the row is inserted at Close) and the host
        // reads its row again, its triggers seeing the row read before as xRec (pages/lt-qj-part-propagation/QjPartPropagation.al:997
        // QL_Both_NewPartRow_PartModifiesHost `log-part-edit` = `L.Qty.V(5 x=0) L.Qty.V.end H.AGR(t=6 ... x.t=1) H.AGCR(t=6 ... x.t=1)`,
        // `part-qty-after-part` 5.00, `lines-after-close` with 20000:q=5).
        if !part_stored {
            return self.new_part_row_update_both(st, &host);
        }
        {
            let mut p = st.borrow_mut();
            p.refresh = false;
            p.dirty = false;
        }
        self.set_xrec(st);
        if !both {
            self.sync_part(&host, c, st)?;
        }
        // The host's triggers still see the row it read before as xRec (`H.AGR(t=5 ... x.t=1)`); the next read sees the new one.
        let mut h = host.borrow_mut();
        h.refresh = true;
        h.refresh_by_update = false;
        h.refresh_without_parts = !both;
        h.read_keeps_xrec = true;
        drop(h);
        let r = self.run_refresh(&host);
        host.borrow_mut().read_keeps_xrec = false;
        r?;
        self.set_xrec(&host);
        Ok(true)
    }

    /// A new part row with Both: the part keeps its typed row (no read of the part, the row is inserted at Close) and the host
    /// reads its row again, its triggers seeing the row read before as xRec (QjPartPropagation.al QL_Both_NewPartRow_PartModifiesHost,
    /// QN_Both_NewPartRow_PartUpdate_HostRowUnchanged).
    fn new_part_row_update_both(&mut self, st: &St, host: &St) -> Result<bool, Signal> {
        st.borrow_mut().refresh = false;
        let mut h = host.borrow_mut();
        h.refresh = true;
        h.refresh_by_update = false;
        h.refresh_without_parts = true;
        h.read_keeps_xrec = true;
        drop(h);
        let r = self.run_refresh(host);
        host.borrow_mut().read_keeps_xrec = false;
        r?;
        self.set_xrec(host);
        Ok(true)
    }

    /// A part ran `CurrPage.Update(false)` in a trigger of a SetValue on a stored part row, and its code left the host's stored row
    /// as the host read it. On a Document host the part reads its rows again and drops the typed value; with Both the host reads its
    /// row first (and its parts after it), with SubPart or none the host is not read again (pages/lt-qj-part-propagation/
    /// QjPartPropagation.al:1106 QL_Both_PartUpdate_HostRowUnchanged `log-part-edit = N.Amt.V(5 x=1) N.Amt.V.end H.AGR(t=1 x.t=1)
    /// H.AGCR(t=1 x.t=1) N.AGR(10000 a=1 x.a=1) N.AGR(10000 a=1 x.a=1) N.AGCR(10000 a=1 x.a=5)`, :1134 _SubPart_ and :1162
    /// _Default_ `N.Amt.V(5 x=1) N.Amt.V.end N.AGR(10000 a=1 x.a=1) N.AGR(10000 a=1 x.a=1) N.AGCR(10000 a=1 x.a=5)`, `lines-after-
    /// close = 10000:q=1,a=1`). Another host type, a new part row and a host with an unsaved edit are refused.
    ///
    /// Card and ListPlus hosts behave as the Document host on a stored part row (QjPartPropagation.al:2123
    /// QN_Card_PartUpdate_HostRowUnchanged, :2151 QN_CardBoth_, :2179 QN_ListPlus_, :2207 QN_ListPlusBoth_). On a new part row the
    /// part does what it does when its code changed the host row: with Both on a Document or ListPlus host the host reads its row
    /// again and the part keeps the typed row (:2030 QN_Both_NewPartRow_PartUpdate_HostRowUnchanged `log-part-edit = N.Amt.V(5 x=0)
    /// N.Amt.V.end H.AGR(t=1 x.t=1) H.AGCR(t=1 x.t=1)`, `lines-after-close` with 20000:q=0,a=5; :2266 QN_ListPlusBoth_); with
    /// SubPart or not set on a Document host, or not set on a Card host, the part drops the typed row (:2061 QN_SubPart_, :2092
    /// QN_Default_, :2235 QN_Card_NewPartRow_). A new row of a Card host with Both, or of a ListPlus host without it, is not pinned.
    fn part_update_host_unchanged(&mut self, st: &St, host: &St, c: usize, both: bool) -> Result<bool, Signal> {
        let o = host.borrow().object as usize;
        let prec = self.page_rec(st);
        let part_stored = !st.borrow().draft && st.borrow().cursor == Cursor::Row && self.stored_row(&prec)?.is_some();
        let page_type = self.engine.object_facts[o].page_type.clone();
        let host_type_pinned = matches!(
            page_type,
            Some(al_ir::PageType::Document | al_ir::PageType::Card | al_ir::PageType::ListPlus)
        );
        if !host_type_pinned || host.borrow().dirty || self.is_list(host) {
            return unsupported(
                "a part's CurrPage.Update when the host row did not change, other than on a Document, Card or ListPlus host without an unsaved edit (not pinned)",
            );
        }
        // On a stored part row Card and ListPlus hosts read as a Document host does, at open and after the part's update
        // (QjPartPropagation.al:2128 QN_Card_PartUpdate_HostRowUnchanged, :2156 QN_CardBoth_, :2184 QN_ListPlus_, :2212
        // QN_ListPlusBoth_; the host's reads at open alone: :3963 QT_HostCard_OpenEdit_GoToKey_PartUpdate `log-openedit = H.OPEN()
        // H.AGR(H1 t=1 x=) H.AGCR(H1 t=1 x=) N.AGR(10000 a=1 x.a=0) N.AGR(10000 a=1 x.a=1) N.AGCR(10000 a=1 x.a=0)`, `log-gotokey =
        // H.AGR(H1 t=1 x=H1) H.AGCR(H1 t=1 x=H1)`, the same on :4049 QT_HostCardBoth_, :4135 QT_HostListPlus_, :4221
        // QT_HostListPlusBoth_ and :4307 QT_HostDoc_; without the GoToKey :3991, :4077, :4163, :4249).
        if !part_stored {
            let card = page_type == Some(al_ir::PageType::Card);
            let list_plus = page_type == Some(al_ir::PageType::ListPlus);
            if both && !card {
                return self.new_part_row_update_both(st, host);
            }
            if !both && !list_plus {
                return self.new_part_row_update_without_both(st, host, card, false);
            }
            return unsupported(
                "a part's CurrPage.Update on a new part row when the host row did not change, on a Card host with UpdatePropagation Both or a ListPlus host without it (not pinned)",
            );
        }
        {
            let mut p = st.borrow_mut();
            p.refresh = false;
            p.dirty = false;
        }
        self.set_xrec(st);
        if !both {
            self.sync_part(host, c, st)?;
            return Ok(true);
        }
        let mut h = host.borrow_mut();
        h.refresh = true;
        h.refresh_by_update = false;
        h.refresh_without_parts = false;
        h.read_keeps_xrec = true;
        drop(h);
        let r = self.run_refresh(host);
        host.borrow_mut().read_keeps_xrec = false;
        r?;
        self.set_xrec(host);
        Ok(true)
    }

    /// A new part row whose code changed the host's stored row and ran `CurrPage.Update(false)`, UpdatePropagation SubPart or not
    /// set: the part reads its rows again and drops the typed row (it is never inserted), comes back to its blank line, then the
    /// host reads its row again, its triggers seeing the row read before as xRec (pages/lt-qj-part-propagation/
    /// QjPartPropagation.al:1032 QL_SubPart_NewPartRow_PartModifiesHost and :1069 QL_Default_ `log-part-edit = L.Qty.V(5 x=0)
    /// L.Qty.V.end L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1) L.AGCR(0 q=0
    /// x.q=0) H.AGR(t=6 ... x.t=1) H.AGCR(t=6 ... x.t=1)`, `part-qty-after-part = 0.00`, `lines-after-close = 10000:q=1,a=1`).
    /// The pinned part declares no OnNewRecord: whether it runs on the way back to the blank line is not pinned, so a part that
    /// can observe it is refused, and so is a part whose whole view the client does not hold or that shows more than one row (one row is pinned).
    /// `card_ok`: a Card host is pinned too; `read_host`: the host reads its row again after the part (not when the host row did
    /// not change: QjPartPropagation.al:2061 QN_SubPart_NewPartRow_PartUpdate_HostRowUnchanged, :2092, :2235 end their `log-part-edit`
    /// with the part's `N.AGCR(0 a=0 x.a=0)`).
    fn new_part_row_update_without_both(
        &mut self,
        st: &St,
        host: &St,
        card_ok: bool,
        read_host: bool,
    ) -> Result<bool, Signal> {
        let o = host.borrow().object as usize;
        let part_object = st.borrow().object as usize;
        let observes_new =
            self.declares(st, "onnewrecord") || self.engine().events.is_subscribed(part_object, "onnewrecordevent");
        let page_type = self.engine.object_facts[o].page_type.clone();
        if !(page_type == Some(al_ir::PageType::Document) || card_ok && page_type == Some(al_ir::PageType::Card))
            || self.is_list(host)
            || observes_new
            || !self.part_view_held(st)
            || st.borrow().window.len() != 1
        {
            return unsupported(
                "a part's CurrPage.Update on a new part row after its code changed the host's stored row, UpdatePropagation not Both, other than on a Document host with a part without OnNewRecord (not pinned)",
            );
        }
        // The typed row is dropped: the part reads its one row three times, the first OnAfterGetRecord seeing the new line as it
        // was set up as xRec, runs OnAfterGetCurrRecord on it and comes back to its blank line.
        let set_up = self.page_xrec_fields(st);
        {
            let mut p = st.borrow_mut();
            p.refresh = false;
            p.draft = false;
            p.dirty = false;
            p.cursor = Cursor::Row;
            p.agr_xrec = Some(set_up);
        }
        self.reread_held_row(st, 0)?;
        for _ in 0..3 {
            self.populate_current(st)?;
        }
        let shown = self.page_rec(st).borrow().fields.clone();
        if let Some(w) = st.borrow_mut().window.get_mut(0) {
            w.shown = shown;
        }
        self.current_record_trigger(st, None)?;
        self.land_on_new_line(st)?;
        if !read_host {
            return Ok(true);
        }
        let mut h = host.borrow_mut();
        h.refresh = true;
        h.refresh_by_update = false;
        h.refresh_without_parts = true;
        h.read_keeps_xrec = true;
        drop(h);
        let r = self.run_refresh(host);
        host.borrow_mut().read_keeps_xrec = false;
        r?;
        self.set_xrec(host);
        Ok(true)
    }

    /// A part control is about to take a value while its Document/card host holds an unsaved edit of a stored row: the host saves
    /// it first (its OnModifyRecord and the table's OnModify run), then the part's validation runs, and the typed host field stays
    /// (pages/lt-qj-part-propagation/QjPartPropagation.al:623, :661, :699 QJ_Both_ / QJ_SubPart_ / QJ_Default_HostEdited_
    /// PartModifiesHost_ThenHostSave `log-part-edit=H.MODR(...) T.H.MOD(...) L.Qty.V(...)`).
    /// A new host row, or a list host, keeps the earlier behavior.
    pub(super) fn save_host_before_part_edit(&mut self, st: &St) -> Result<(), Signal> {
        let Some(host) = st.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(());
        };
        if !host.borrow().open
            || !host.borrow().dirty
            || host.borrow().draft
            || host.borrow().cursor != Cursor::Row
            || self.is_list(&host)
            || !self.has_source(&host)
        {
            return Ok(());
        }
        let hrec = self.page_rec(&host);
        if hrec.borrow().temporary || self.stored_row(&hrec)?.is_none() {
            return Ok(());
        }
        self.save_row(&host)?;
        // The host's next read sees the saved row as xRec (QjPartPropagation.al:2297 QN_Both_HostEdited_PartUpdate_HostRowUnchanged
        // `H.MODR(t=9 x.t=1) ... H.AGR(t=9 x.t=9)`; :1332 QJ_Both_HostEdited_PartModifiesHost_ThenHostSave `H.AGR(t=5 ... x.t=1)`
        // after saving t=1): the saved row counts as the host's last read.
        let saved = hrec.borrow().fields.clone();
        host.borrow_mut().agr_xrec = Some(saved);
        Ok(())
    }

    /// A host control is about to take a value: the open parts that hold a typed row that is not saved save it first
    /// (testpart/QiHostSaveParts.al:500, :554 on a Document host; other hosts: assumption part-row-saved-before-host-set-value).
    pub(super) fn save_parts_before_host_edit(&mut self, st: &St) -> Result<(), Signal> {
        let parts = st.borrow().parts.clone();
        for (_, child) in parts {
            let pending = {
                let c = child.borrow();
                c.open && c.dirty
            };
            if pending {
                let o = st.borrow().object as usize;
                if self.engine.object_facts[o].page_type != Some(al_ir::PageType::Document) {
                    al_builtins::assumptions::assume(
                        al_builtins::assumptions::Assumption::PartRowSavedBeforeHostSetValue,
                    );
                    self.save_row(&child)?;
                    continue;
                }
                let inserted = child.borrow().draft;
                self.save_row(&child)?;
                self.siblings_read_saved_row(st, &child, inserted)?;
            }
        }
        Ok(())
    }

    /// Part `child` saved a row just before a host SetValue: an unlinked ListPart of the host over the same table reads that
    /// row, twice for an inserted row and once for a modified one, then runs OnAfterGetCurrRecord with a blank xRec
    /// (testpart/QiHostSaveParts.al:500 QI_PartNewRow_ThenHostSetValue `U.AGR(D1/30000 x=D1/10000);U.AGR(D1/30000 x=D1/30000);
    /// U.AGCR(D1/30000 x=/0)`, :554 QI_PartEditedRow_ThenHostSetValue `U.AGR(D1/10000 x=D1/10000);U.AGCR(D1/10000 x=/0)`, the row
    /// the unlinked part stood on). Its cursor stays on its own row: the next refresh or the host's Close reads it again (:504,
    /// :580). Any other sibling over that table is refused.
    fn siblings_read_saved_row(&mut self, host: &St, child: &St, inserted: bool) -> Result<(), Signal> {
        if !self.has_source(child) {
            return Ok(());
        }
        let saved = self.page_rec(child).borrow().fields.clone();
        let table = self.page_rec(child).borrow().table;
        let engine = self.engine();
        let page = page_of(&engine, host);
        let siblings: Vec<(usize, St)> = {
            let h = host.borrow();
            h.parts
                .iter()
                .chain(h.hidden_parts.iter())
                .filter(|(_, c)| !Rc::ptr_eq(c, child) && c.borrow().open)
                .cloned()
                .collect()
        };
        for (i, sib) in siblings {
            if !self.has_source(&sib) || self.page_rec(&sib).borrow().table != table {
                continue;
            }
            let o = sib.borrow().object as usize;
            let list_part =
                matches!(&self.engine.object_facts[o].page_type, Some(al_ir::PageType::Other(t)) if t == "listpart");
            let linked = page.controls[i]
                .part_link
                .as_ref()
                .is_some_and(|l| !l.entries.is_empty() || l.provider.is_some());
            let (cursor, draft, dirty, stale, hidden) = {
                let c = sib.borrow();
                let hidden = host.borrow().hidden_parts.iter().any(|(h, _)| *h == i);
                (c.cursor, c.draft, c.dirty, c.shows_sibling_save.is_some(), hidden)
            };
            if linked
                || !list_part
                || hidden
                || cursor != Cursor::Row
                || draft
                || dirty
                || stale
                || self.declares(&sib, "onfindrecord")
                || self.declares(&sib, "onnextrecord")
            {
                return unsupported(
                    "a part row saved before a host SetValue while another part over the same table is not an unlinked ListPart on a stored row (its reads are not pinned)",
                );
            }
            let srec = self.page_rec(&sib);
            let tdef = self.table(&srec)?;
            let at = srec.borrow().fields.clone();
            let same_row = Self::same_key(&tdef.primary_key(&at), &tdef.primary_key(&saved));
            if inserted == same_row {
                return unsupported(
                    "a part row saved before a host SetValue that is a modified row other than the one an unlinked sibling part stands on (not pinned)",
                );
            }
            for _ in 0..if inserted { 2 } else { 1 } {
                srec.borrow_mut().fields = saved.clone();
                self.populate_current(&sib)?;
            }
            let blank = self.blank_fetch_xrec(&sib, true)?;
            sib.borrow_mut().agcr_xrec = Some(blank);
            self.page_trigger(&sib, "onaftergetcurrrecord", vec![])?;
            self.page_event(&sib, "onaftergetcurrrecordevent", None, None)?;
            if inserted {
                let mut s = sib.borrow_mut();
                s.shows_sibling_save = Some(at);
                s.unpinned_state = Some(
                    "a part that shows a row another part saved, other than the row it stands on (what it reads next is not pinned)".into(),
                );
            } else {
                self.set_xrec(&sib);
            }
        }
        Ok(())
    }

    /// The host closes after it saved its row: an unlinked part that shows a row a sibling saved reads the row it stands on
    /// once (testpart/QiHostSaveParts.al:504 `H.MOD(D1);T.HeadModify(D1);U.AGR(D1/10000 x=D1/30000)`). A close that saves no
    /// host row is refused.
    pub(super) fn stale_parts_read_at_close(&mut self, host: &St, host_saved: bool) -> Result<(), Signal> {
        let parts: Vec<St> = host.borrow().parts.iter().map(|(_, c)| c.clone()).collect();
        for part in parts {
            let Some(at) = part.borrow_mut().shows_sibling_save.take() else {
                continue;
            };
            part.borrow_mut().unpinned_state = None;
            if !host_saved {
                return unsupported(
                    "closing a host that saves nothing while a part shows a row another part saved (not pinned)",
                );
            }
            self.page_rec(&part).borrow_mut().fields = at;
            self.populate_current(&part)?;
        }
        Ok(())
    }

    /// A call reaches part `child` of a Document host that is a new, unsaved record: a read of a part field or the part's First
    /// inserts the host (page OnInsertRecord and table OnInsert, nothing typed), and the parts link to it and read their rows
    /// again, with or without rows of the blank key (pages/lt-part-line-before-host-key/PartLineBeforeHostKey.al:1748
    /// QH_BlankRows_ReadPart_Stored, :2032 QH_NoRows_ReadPartControl, :2057 QH_OtherKeyRows_ReadPartControl, :2083
    /// QH_BlankRows_PartFirst, :2142 QH_BlankRows_ReadPartControlTwice_TraceHost). Pinned for a field-linked part of a host
    /// nothing was typed into, reached the first time; any other call on such a part while it shows rows of the blank key is
    /// refused. `Ok(true)`: the call was handled here.
    fn new_host_on_part_access(
        &mut self,
        host: &St,
        child: &St,
        m: PageMethod,
        field_read: bool,
        reached_before: bool,
    ) -> Result<bool, Signal> {
        let new_host = {
            let h = host.borrow();
            h.open && h.host.is_none() && h.draft && !h.draft_stored && h.cursor != Cursor::None
        };
        if !new_host || !self.has_source(host) {
            return Ok(false);
        }
        let shows_rows = host.borrow().new_host_part_rows;
        let o = host.borrow().object as usize;
        let engine = self.engine();
        let idx = child.borrow().host.as_ref().map(|(_, c)| *c);
        let field_linked = idx.is_some_and(|i| {
            page_of(&engine, host).controls[i]
                .part_link
                .as_ref()
                .is_some_and(|l| l.entries.iter().any(|e| matches!(e, ir::LinkEntry::Field(..))))
        });
        let pinned_call = field_read || m == PageMethod::First;
        if self.engine.object_facts[o].page_type != Some(al_ir::PageType::Document) || !field_linked {
            if shows_rows {
                return unsupported(format!(
                    "{m} on a part while its new host shows rows of the blank key in another part (not pinned)"
                ));
            }
            return Ok(false);
        }
        if !pinned_call || reached_before || host.borrow().dirty {
            if shows_rows {
                return unsupported(format!(
                    "{m} on a part whose new, unsaved host shows rows of the blank key, other than a first field read or First of a host nothing was typed into (not pinned)"
                ));
            }
            // Kept, not pinned: a call other than a field read or First on the part of a new host whose part shows no rows
            // inserts nothing, as before (the write paths save the host themselves: `save_unsaved_host_first`).
            return Ok(false);
        }
        let used = self.ret_used;
        host.borrow_mut().dirty = true;
        let r = self.save_row(host);
        self.ret_used = used;
        r?;
        host.borrow_mut().new_host_part_rows = false;
        Ok(true)
    }

    /// The first call that reaches part `child` since its host opened, while the Document host holds an unsaved edit of a
    /// stored row: a read of a part field saves the host first, its OnModifyRecord and the table's OnModify, and nothing reads
    /// again (testpart/QiHostSaveParts.al:441 QI_HostModify_ChangedValue and :469 QI_HostPlain_ChangedValue
    /// `after-part-read=H.MOD(D1);T.HeadModify(D1);`). A part reached before saves nothing (:495 QI_PartNewRow_ThenHostSetValue
    /// `after-part-read=` after Lines.Last, New and SetValue, the host saved at Close). A SetValue goes its own way
    /// (`save_host_before_part_edit`); any other first call, a host of another type, and a first call while another part was
    /// reached before are refused.
    pub(super) fn first_part_access(&mut self, child: &St, m: PageMethod, field_read: bool) -> Result<(), Signal> {
        let Some(host) = child.borrow().host.as_ref().and_then(|(h, _)| h.upgrade()) else {
            return Ok(());
        };
        let reached_before = std::mem::replace(&mut child.borrow_mut().part_accessed, true);
        if self.new_host_on_part_access(&host, child, m, field_read, reached_before)? {
            return Ok(());
        }
        if reached_before {
            return Ok(());
        }
        let edited = {
            let h = host.borrow();
            h.open && h.host.is_none() && h.dirty && !h.draft && h.cursor == Cursor::Row
        };
        if !edited || !self.has_source(&host) || m == PageMethod::SetValue {
            return Ok(());
        }
        let hrec = self.page_rec(&host);
        if hrec.borrow().temporary || self.stored_row(&hrec)?.is_none() {
            return Ok(());
        }
        let o = host.borrow().object as usize;
        let other_reached = host
            .borrow()
            .parts
            .iter()
            .chain(host.borrow().hidden_parts.iter())
            .any(|(_, c)| !Rc::ptr_eq(c, child) && c.borrow().part_accessed);
        if !field_read {
            // Kept, not pinned: a first New, First, Last or GoToKey on a part while the stored host holds an edit saves
            // nothing, as before the field read was pinned (baseline cases 60868 OpenNew_NumberAssignedOnInsert_
            // FirstThenWriteCarriesTheNumber, _LinesInsertedInCode_LastThenWriteLandsOnTheLastLine; 71494, 71495 P182b_*;
            // 78622 DocumentHost_HeaderEditAndPartEdit; 60412 HostEdit_SurvivesAPartSavingItsOwnRow).
            return Ok(());
        }
        if self.engine.object_facts[o].page_type != Some(al_ir::PageType::Document) || other_reached {
            return unsupported(
                "a first field read on a part while its host holds an unsaved edit, other than on the only part reached of a Document page (whether the host saves first is not pinned)",
            );
        }
        let used = self.ret_used;
        let r = self.save_row(&host);
        self.ret_used = used;
        r
    }

    /// The client's whole view of a part that offers the blank line, held from its first row and asked of no page trigger: the
    /// shape the Last, First and New counts of pages/lt-qj-part-propagation/QjPartPropagation.al pin (a ListPart with
    /// AutoSplitKey and DelayedInsert in a Document host opened for edit, without OnFindRecord and OnNextRecord).
    pub(super) fn part_view_held(&self, st: &St) -> bool {
        let s = st.borrow();
        s.host.is_some()
            && s.open
            && s.new_line
            && s.window_from_start
            && (s.window.len() < Self::WINDOW || s.window_whole)
            && self.is_list(st)
            && self.has_source(st)
            && !self.declares(st, "onfindrecord")
            && !self.declares(st, "onnextrecord")
    }

    /// Last() on a part that shows no row runs its blank line twice; whether the link value is validated each time is not
    /// pinned (the pinned table has no validation code), so a part whose table validates a key field in code is refused.
    fn empty_part_new_line_pinned(&self, st: &St) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let table = self.table(&rec)?;
        let t = rec.borrow().table as usize;
        let engine = self.engine();
        let events = &engine.events;
        let Some(schema) = engine.program.objects[t].table.as_ref() else {
            return unsupported("a part over a table without a schema");
        };
        let validates = table.keys[0].iter().any(|&pos| {
            let f = &schema.fields[pos];
            f.on_validate.is_some() || !f.ext_before_validate.is_empty() || !f.ext_after_validate.is_empty()
        }) || events.is_subscribed(t, "onbeforevalidateevent")
            || events.is_subscribed(t, "onaftervalidateevent");
        if validates {
            return unsupported(
                "Last() or New() on a part that shows no row whose table validates a key field in code (whether the link value is validated then is not pinned)",
            );
        }
        Ok(())
    }

    fn observes_new_record(&self, st: &St) -> bool {
        let object = st.borrow().object as usize;
        self.declares(st, "onnewrecord") || self.engine().events.is_subscribed(object, "onnewrecordevent")
    }

    /// `Last()` and `First()` on a part whose whole view the client holds (`part_view_held`). `None` for any other call.
    ///
    /// - Last() from a stored row: the client runs the blank line below the last row once (OnNewRecord with the last row as
    ///   xRec, OnAfterGetCurrRecord on the blank row with a blank xRec) and comes back to the last row, read as a move from the
    ///   row it stood on (QjPartPropagation.al:1290 QL_PartLastNew_TwoLines `log-1-last = S.NEW(x=4 x.ln=40000) S.AGCR(0 r=0
    ///   x=0) S.AGCR(40000 r=4 x=3)`, from row 30000; :1374 QL_PartLastNewNew and :925 QJ_SubformSaveRecord_NewRow on the only
    ///   row; :995 QL_Both_NewPartRow_PartModifiesHost `log-new` with OnAfterGetRecord: `L.AGCR(0 q=0 x.q=0) L.AGR(10000 q=1
    ///   x.q=0) L.AGR(10000 q=1 x.q=1) L.AGCR(10000 q=1 x.q=1)`). It answers Yes.
    /// - First() on the first row of a part without OnAfterGetRecord reads nothing and answers Yes (:1318
    ///   QL_PartFirstNew_TwoLines `log-1-first` empty).
    /// - Last() on a part that shows no row runs OnNewRecord and OnAfterGetCurrRecord twice, OnNewRecord with the xRec of
    ///   `PageState::empty_part_xrec`, and answers No (:1346 QL_PartLastNew_NoLines `log-1-last = S.NEW(x=3 x.ln=30000)
    ///   S.AGCR(0 r=0 x=0) S.NEW(x=3 x.ln=30000) S.AGCR(0 r=0 x=0)`, `last-result = No`).
    pub(super) fn part_last_first(&mut self, st: &St, how: PageMethod) -> Result<Option<bool>, Signal> {
        if how == PageMethod::Last
            && !self.part_view_held(st)
            && st.borrow().host.is_some()
            && self.observes_new_record(st)
        {
            let empty_line = {
                let s = st.borrow();
                s.cursor == Cursor::NewLine && s.draft && !s.dirty && s.window.is_empty()
            };
            // Last() on a part whose view holds no row runs its blank line again and answers No: twice on a stored host
            // (QjPartPropagation.al:2441 QN_EmptyPart_OpenEdit_FirstHostEmpty `log-last = S.NEW(x=0 x.ln=0) S.AGCR(0 r=0 x=0)` two
            // times), OnNewRecord seeing the xRec the part held as it
            // landed on the blank line (`PageState::empty_part_xrec`, blank in both probes).
            let view_empty = empty_line && {
                let rec = self.page_rec(st);
                let probe = Rc::new(RefCell::new(rec.borrow().clone()));
                self.ret_used = true;
                !matches!(
                    self.record_op(RecordOp::FindFirst, &probe, &mut [])?,
                    Value::Boolean(true)
                )
            };
            if view_empty {
                let x = st.borrow().empty_part_xrec.clone();
                let Some(x) = x else {
                    return unsupported(
                        "Last() on a part that shows no row it did not land on through a host move or open (the xRec OnNewRecord sees is not pinned)",
                    );
                };
                self.empty_part_new_line_pinned(st)?;
                // On a new host the host row is inserted first and the part lands on its blank line three more times
                // (`insert_untouched_new_host`: five runs in all on :2596 QN_EmptyPart_HostOpenNew).
                self.insert_untouched_new_host(st)?;
                let x = st.borrow().empty_part_xrec.clone().unwrap_or(x);
                for _ in 0..2 {
                    self.set_xrec_fields(st, x.clone());
                    self.land_on_new_line(st)?;
                }
                return Ok(Some(false));
            }
        }
        if !matches!(how, PageMethod::Last | PageMethod::First) || !self.part_view_held(st) {
            return Ok(None);
        }
        let (cursor, draft, dirty, empty) = {
            let s = st.borrow();
            (s.cursor, s.draft, s.dirty, s.window.is_empty())
        };
        if cursor == Cursor::NewLine && draft && !dirty && empty && how == PageMethod::Last {
            let Some(x) = st.borrow().empty_part_xrec.clone() else {
                if self.observes_new_record(st) {
                    return unsupported(
                        "Last() on a part that shows no row it did not land on through a host move or open (the xRec OnNewRecord sees is not pinned)",
                    );
                }
                return Ok(None);
            };
            self.empty_part_new_line_pinned(st)?;
            for _ in 0..2 {
                self.set_xrec_fields(st, x.clone());
                self.land_on_new_line(st)?;
            }
            return Ok(Some(false));
        }
        if cursor != Cursor::Row || draft || dirty || empty {
            return Ok(None);
        }
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        let window = st.borrow().window.clone();
        let Some(at) = window.iter().position(|w| Self::same_key(&w.key, &key)) else {
            return Ok(None);
        };
        if how == PageMethod::First {
            if at != 0 {
                return Ok(None);
            }
            // Also with OnAfterGetRecord or without OnAfterGetCurrRecord (QjPartPropagation.al:2824 QN_PartAGR_First_OnFirstRow
            // `log-first` empty, :2860 QN_PartNoAGCR_First_OnFirstRow).
            return Ok(Some(true));
        }
        // Without OnAfterGetCurrRecord too: OnNewRecord runs once with the last row as xRec and the part lands on the last row
        // (QjPartPropagation.al:2644 QN_PartNoAGCR_Last_TwoLines `log-last = A.NEW(x=4 x.ln=40000 b=Yes)`, :2674 _OneLine, :2734
        // QN_PartNewOnly_Last_TwoLines, :2764 _OneLine).
        let before = self.page_xrec_fields(st);
        let last = window.len() - 1;
        self.set_xrec_fields(st, window[last].stored.clone());
        self.land_on_new_line(st)?;
        {
            let mut s = st.borrow_mut();
            s.draft = false;
            s.cursor = Cursor::Row;
        }
        // The last row is read twice; the first OnAfterGetRecord sees the blank line as xRec, the row fetched before it
        // (`L.AGR(10000 q=1 x.q=0) L.AGR(10000 q=1 x.q=1)`), and OnAfterGetCurrRecord the row the move came from.
        // Without OnAfterGetCurrRecord the client reads nothing on its way back to the last row: no OnAfterGetRecord
        // (QjPartPropagation.al:2644 QN_PartNoAGCR_Last_TwoLines on a part that declares OnAfterGetRecord).
        if !self.declares(st, "onaftergetcurrrecord") {
            self.reread_held_row(st, last)?;
            self.set_xrec(st);
            let shown = self.page_rec(st).borrow().fields.clone();
            if let Some(w) = st.borrow_mut().window.get_mut(last) {
                w.shown = shown;
            }
            return Ok(Some(true));
        }
        let landed_xrec = self.page_xrec_fields(st);
        st.borrow_mut().agr_xrec = Some(landed_xrec);
        self.reread_held_row(st, last)?;
        self.populate_current(st)?;
        self.populate_current(st)?;
        let shown = self.page_rec(st).borrow().fields.clone();
        if let Some(w) = st.borrow_mut().window.get_mut(last) {
            w.shown = shown;
        }
        self.current_record_trigger(st, Some(before))?;
        Ok(Some(true))
    }

    /// The xRec OnNewRecord of `New()` on a part sees: the last row of the part's view, also when the cursor stands on another
    /// row or on the untouched blank line (QjPartPropagation.al:1318 QL_PartFirstNew_TwoLines `log-2-new = S.NEW(x=4 x.ln=40000)`
    /// after First on row 30000; :1374 QL_PartLastNewNew `log-3-new = S.NEW(x=1 x.ln=10000)` from the blank line; :1266
    /// QL_PartNew_WithoutLast); on a part that shows no row, `PageState::empty_part_xrec` (:1346 QL_PartLastNew_NoLines
    /// `log-2-new = S.NEW(x=3 x.ln=30000)`). `None` for any other page or state.
    pub(super) fn part_new_xrec(&mut self, st: &St) -> Result<Option<Vec<Value>>, Signal> {
        if !self.part_view_held(st) {
            return Ok(None);
        }
        let (cursor, draft, dirty) = {
            let s = st.borrow();
            (s.cursor, s.draft, s.dirty)
        };
        let on_row = cursor == Cursor::Row && !draft;
        let on_blank = cursor == Cursor::NewLine && draft && !dirty;
        if !on_row && !on_blank {
            return Ok(None);
        }
        let last = st.borrow().window.last().map(|w| w.stored.clone());
        if let Some(last) = last {
            // Every part shape: AutoSplitKey and DelayedInsert (the `LT QJ Save Lines` part), DelayedInsert only, AutoSplitKey
            // only, neither, with OnAfterGetRecord, without OnAfterGetCurrRecord (QjPartPropagation.al:2921
            // QN_PartDelayedOnly_New_FromFirstRow `D.NEW(x=4 x.ln=40000 b=Yes)` from row 30000, :3027 QN_PartSplitOnly_*, :3133
            // QN_PartPlain_*, :3239 QN_PartAGR_*, :3345 QN_PartNoAGCR_*; each also after Next, Last and a typed New).
            return Ok(Some(last));
        }
        // Only OnNewRecord's xRec comes from here; the link value is stamped and validated as for any New() (`start_draft`).
        let x = st.borrow().empty_part_xrec.clone().filter(|_| on_blank);
        if x.is_none() && on_blank && self.observes_new_record(st) {
            return unsupported(
                "New() on a part that shows no row it did not land on through a host move or open (the xRec OnNewRecord sees is not pinned)",
            );
        }
        Ok(x)
    }

    /// Saves what was typed into the rows the parts stand on.
    pub(super) fn save_parts(&mut self, st: &St) -> Result<(), Signal> {
        let parts = st.borrow().parts.clone();
        for (_, child) in parts {
            if child.borrow().open {
                self.save_row(&child)?;
            }
        }
        Ok(())
    }
}

/// How many of `parts` take a SubPageLink value from host field `pos`.
fn parts_linked_on(page: &ir::Page, parts: &[(usize, St)], pos: usize) -> usize {
    parts
        .iter()
        .filter(|(i, _)| {
            page.controls[*i].part_link.as_ref().is_some_and(|l| {
                l.provider.is_none()
                    && l.entries
                        .iter()
                        .any(|e| matches!(e, ir::LinkEntry::Field(_, hf) if *hf == pos))
            })
        })
        .count()
}
