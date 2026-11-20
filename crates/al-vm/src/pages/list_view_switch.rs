//! TestPage View() and Edit() on a list without a card (al-runner issues 6977, 8501). On a list opened for edit View switches the
//! page to view mode and drops the new-row line (testpage/R6977ListViewNext.al:138, :156, :174, :191; R6977ListAfterView.al:125,
//! :142, :164); Edit then switches it back (R6977ListAfterView.al:187). After that second switch only what the probes read is
//! measured; the rest is refused here. On the list of corpus PR 1467 (open, `reopen_probe_list`) each switch reads the row again,
//! Edit also switches the list opened for view, and the page opens again after a Close.

use super::pinned_shape::PinnedCode;
use super::state::ListViewSwitch;
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

const EDIT_BACK: &str =
    "after the built-in View and then Edit switched a list without a card back to edit mode (not pinned)";

const EDIT_FROM_VIEW: &str =
    "after the built-in Edit switched a list without a card opened for view to edit mode (not pinned)";

/// The code of the list of testpage/lifecycle/ReopenAfterListModeSwitch.al (corpus PR 1467, open): page 75641 "RLS Lines" logs
/// its five triggers, its fields show Rec's two fields and a page variable, its table runs no code.
const PROBE_LIST: PinnedCode = PinnedCode {
    triggers: &[
        "onopenpage",
        "onaftergetrecord",
        "onaftergetcurrrecord",
        "onnewrecord",
        "onclosepage",
    ],
    control_triggers: &[],
    properties: &[],
    action_props: None,
    part_props: None,
    table_triggers: &[],
};

impl Session {
    /// The enabled built-in Edit or View of a list without a card, invoked. View on the list opened for edit switches the page to
    /// view: a First/Next walk then visits only the rows, no blank new-row line (testpage/R6977ListViewNext.al:138, :174, :191
    /// `n=3`; :156 without View `0/; n=4`). The page and its fields are then not editable, SetValue still types into the row,
    /// and Edit is enabled, View not (R6977ListAfterView.al:125, :142, :164): view mode serves these. Edit after that View
    /// switches the page back to edit on the same row (R6977ListAfterView.al:187 `back|page=Yes;back|description=Yes;
    /// back|editEnabled=No;back|viewEnabled=Yes;back|row=10000`); whether the new-row line comes back is not pinned
    /// (`refuse_after_list_edit_back`). On the probe's list each switch reads the row again, and Edit switches the list opened
    /// for view to edit (`measured_switch_read`); Edit on another list opened for view is refused.
    pub(super) fn switch_list_without_card(&mut self, st: &St, edit: bool) -> Result<Value, Signal> {
        let state = st.borrow().list_view;
        let to = match (edit, state) {
            (false, _) => {
                if st.borrow().cursor != Cursor::Row || st.borrow().draft {
                    return unsupported(
                        "TestAction View of a list without a card that stands on the new-row line, a typed row or no row",
                    );
                }
                PageMode::View
            }
            // Edit is enabled only in view mode: after View, or on the list opened for view.
            (true, ListViewSwitch::InView | ListViewSwitch::EditBackThenView | ListViewSwitch::None) => PageMode::Edit,
            (true, ListViewSwitch::EditBack | ListViewSwitch::EditFromView) => return Ok(Value::Null),
        };
        let read = self.reopen_probe_list(st)?;
        let row = if read {
            self.measured_switch_read(st, edit)?
        } else {
            None
        };
        if read && row.is_none() {
            return unsupported(format!(
                "TestAction.Invoke of the built-in {} of a list without a card that reads its row again, not on the first \
                 stored, unchanged row of a page the test opened, or in a state not measured (corpus PR 1467)",
                if edit { "Edit" } else { "View" }
            ));
        }
        if (edit, state, read) == (true, ListViewSwitch::None, false) {
            return unsupported(
                "TestAction.Invoke of the built-in Edit of a list without a card opened for view (measured on one list only, \
                 corpus PR 1467)",
            );
        }
        {
            let mut s = st.borrow_mut();
            match (edit, state) {
                (false, _) => {
                    s.new_line = false;
                    s.list_view.switched_to_view();
                }
                (true, ListViewSwitch::None) => s.list_view.edited_from_view(),
                (true, _) => {
                    s.list_view.edit_invoked();
                }
            }
        }
        let switched = self.switch_mode(st, to)?;
        if let Some(stored) = row {
            assume(Assumption::ListModeSwitchReadsRow);
            self.read_row_on_switch(st, stored)?;
        }
        Ok(switched)
    }

    /// Whether `st` is the list of corpus PR 1467 (open; testpage/lifecycle/ReopenAfterListModeSwitch.al, measured on a local BC
    /// of each supported major): PageType List, no CardPageId, not a part, the code `PROBE_LIST` names. The rules of
    /// `list-mode-switch-reads-row`, `list-reopen-after-mode-switch` and `mode-actions-bound-to-earlier-open` are measured on it
    /// only.
    pub(super) fn reopen_probe_list(&self, st: &St) -> Result<bool, Signal> {
        Ok(self.is_list_page(st)
            && st.borrow().host.is_none()
            && self.list_card_page(st)?.is_none()
            && self.page_code_bound_is(st, &PROBE_LIST, true)?)
    }

    /// The switch of `switch_list_without_card` on the probe's list reads its row the way the probe measured: View on the page
    /// the test opened for edit, before any switch (`view-1` of every cell: `[agr(10000) agcr(10000)]`); Edit after that View
    /// (`edit-1`); Edit on the page the test opened for view (OpenView_Edit_Close_OpenEdit_Reads `edit-1`, `edit1|page = Yes`).
    /// Each with the page on its first row, which is stored and unchanged since the page showed it, nothing typed. The stored
    /// row when it does.
    fn measured_switch_read(&mut self, st: &St, edit: bool) -> Result<Option<Vec<Value>>, Signal> {
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        let stored = self.stored_row(&rec)?;
        let s = st.borrow();
        let state = match (edit, s.list_view) {
            (false, ListViewSwitch::None) => s.mode == PageMode::Edit && !s.mode_switched_since_open,
            (true, ListViewSwitch::InView) => s.mode == PageMode::View,
            (true, ListViewSwitch::None) => s.mode == PageMode::View && !s.mode_switched_since_open,
            (
                _,
                ListViewSwitch::InView
                | ListViewSwitch::EditBack
                | ListViewSwitch::EditBackThenView
                | ListViewSwitch::EditFromView,
            ) => false,
        };
        let first = s.window_from_start
            && s.window.first().is_some_and(|w| {
                Self::same_key(&w.key, &key) && stored.as_ref().is_some_and(|v| Self::same_values(&w.stored, v))
            });
        let measured = state && first && s.opened_by_test && s.cursor == Cursor::Row && !s.draft && !s.dirty;
        Ok(stored.filter(|_| measured))
    }

    /// The row the page stands on, read again from the table (`stored`): OnAfterGetRecord, then OnAfterGetCurrRecord
    /// (ReopenAfterListModeSwitch.al `view-1`, `edit-1`: `[agr(10000) agcr(10000)]`). The window keeps what the read showed.
    fn read_row_on_switch(&mut self, st: &St, stored: Vec<Value>) -> Result<(), Signal> {
        let rec = self.page_rec(st);
        let key = self.row_key(&rec)?;
        rec.borrow_mut().fields = stored.clone();
        self.populate_current(st)?;
        let shown = rec.borrow().fields.clone();
        if let Some(w) = st.borrow_mut().window.iter_mut().find(|w| Self::same_key(&w.key, &key)) {
            w.stored = stored;
            w.shown = shown;
        }
        self.current_record_trigger(st, None)
    }

    /// A page call after View and then Edit on a list without a card: Editable, ValidationErrorCount, Close and taking the
    /// built-in Edit and View actions are pinned (R6977ListAfterView.al:178, :102, :184, :180). After Edit on a list opened for
    /// view Editable, Close and taking the actions are measured (ReopenAfterListModeSwitch.al OpenView_Edit_Close_OpenEdit_Reads).
    /// The opens `reopen_after_switch_measured` names pass. A walk, New and the rest are not: whether the new-row line comes
    /// back is unmeasured.
    pub(super) fn refuse_after_list_edit_back(&self, st: &St, m: PageMethod) -> Result<(), Signal> {
        let state = st.borrow().list_view;
        let (allowed, after) = match state {
            ListViewSwitch::EditBack | ListViewSwitch::EditBackThenView => (
                matches!(
                    m,
                    PageMethod::Editable
                        | PageMethod::ValidationErrorCount
                        | PageMethod::Close
                        | PageMethod::Edit
                        | PageMethod::View
                ),
                EDIT_BACK,
            ),
            ListViewSwitch::EditFromView => (
                matches!(
                    m,
                    PageMethod::Editable | PageMethod::Close | PageMethod::Edit | PageMethod::View
                ),
                EDIT_FROM_VIEW,
            ),
            ListViewSwitch::None | ListViewSwitch::InView => return Ok(()),
        };
        if allowed {
            return Ok(());
        }
        if self.reopen_after_switch_measured(st, m)? {
            assume(Assumption::ListReopenAfterModeSwitch);
            return Ok(());
        }
        unsupported(format!("TestPage.{m} {after}"))
    }

    /// The opens of the probe's list after a switch that ReopenAfterListModeSwitch.al measured (corpus PR 1467, open): after View
    /// and Edit and a clean `TestPage.Close()`, OpenEdit, OpenView and OpenNew open a fresh page (ViewEdit_Close_OpenEdit_Reads,
    /// _Walk, _SwitchAgain, ViewEdit_SetValue_Close_OpenEdit_ShowsValue, ViewEdit_Close_OpenView_Reads, _WalkThenEdit,
    /// ViewEdit_Close_OpenNew_Reads; `list_view` is reset at open); OpenEdit on that page still open raises 'The TestPage is
    /// already open.' and changes nothing (ViewEdit_OpenEditWithoutClose, `page_open`); after Edit on the list opened for view
    /// and a clean Close, OpenEdit opens a fresh page (OpenView_Edit_Close_OpenEdit_Reads).
    fn reopen_after_switch_measured(&self, st: &St, m: PageMethod) -> Result<bool, Signal> {
        let measured = {
            let s = st.borrow();
            let after_close = !s.open && s.closed_by == Some(ClosedBy::TestClose);
            match (s.list_view, m) {
                (ListViewSwitch::EditBack, PageMethod::OpenEdit) => after_close || s.open,
                (ListViewSwitch::EditBack, PageMethod::OpenView | PageMethod::OpenNew) => after_close,
                (ListViewSwitch::EditFromView, PageMethod::OpenEdit) => after_close,
                _ => false,
            }
        };
        Ok(measured && self.reopen_probe_list(st)?)
    }

    /// A control call in the same states. After View and Edit the fields are pinned (Editable, Value, SetValue:
    /// R6977ListAfterView.al:179, :182, :100) and so are Enabled reads of the built-in actions (:180, :181); invoking Edit or
    /// View again is not. After Edit on a list opened for view only Editable of a field and Enabled of the built-in actions are
    /// measured (ReopenAfterListModeSwitch.al OpenView_Edit_Close_OpenEdit_Reads `edit1|desc`, `edit1|editEnabled`).
    pub(super) fn refuse_after_list_edit_back_control(&self, c: &ControlRef, m: PageMethod) -> Result<(), Signal> {
        let state = c.page.borrow().list_view;
        match (state, &c.target) {
            (ListViewSwitch::EditBack | ListViewSwitch::EditBackThenView, ControlTarget::BuiltIn(name))
                if m == PageMethod::Invoke && matches!(name.as_str(), "Edit" | "View") =>
            {
                unsupported(format!("TestAction.{m} of the built-in {name} {EDIT_BACK}"))
            }
            (ListViewSwitch::EditFromView, ControlTarget::Control(_)) if m == PageMethod::Editable => Ok(()),
            (ListViewSwitch::EditFromView, ControlTarget::BuiltIn(name))
                if m == PageMethod::Enabled && matches!(name.as_str(), "Edit" | "View") =>
            {
                Ok(())
            }
            (ListViewSwitch::EditFromView, _) => unsupported(format!("{m} of a control {EDIT_FROM_VIEW}")),
            _ => Ok(()),
        }
    }
}
