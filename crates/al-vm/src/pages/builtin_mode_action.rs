//! The built-in View and Edit actions of a TestPage (`P.View()`, `P.Edit()`): taking one from the variable, and what Visible,
//! Enabled and Invoke of it answer. A list without a card switches its own mode (`list_view_switch.rs`); an action the variable
//! handed out on an earlier open stays bound to it (`state/mode_actions.rs`).

use super::state::EarlierOpen;
use super::*;
use al_builtins::assumptions::{Assumption, assume};
use al_builtins::page_op::PageMethod;

impl Session {
    /// `P.View()` or `P.Edit()` (`m`) on the open page `st`: the action. The variable binds it to this open when it had not handed
    /// it out yet (`ModeActions::taken`).
    pub(super) fn take_mode_action(&mut self, st: &St, m: PageMethod) -> Result<Value, Signal> {
        self.require_open(st)?;
        st.borrow_mut().mode_actions.taken(m == PageMethod::Edit);
        Ok(test_control(st.clone(), ControlTarget::BuiltIn(m.to_string())))
    }

    /// Visible, Enabled and Invoke of the built-in Edit or View action (`name`) of page `st`. An action the variable handed out on
    /// an earlier open answers for that open (`bound_mode_action`).
    pub(super) fn builtin_mode_action(&mut self, st: &St, name: &str, m: PageMethod) -> Result<Value, Signal> {
        let edit = name == "Edit";
        let bound = st.borrow().mode_actions.bound(edit);
        if let Some(earlier) = bound {
            return self.bound_mode_action(st, edit, earlier, m);
        }
        // On a card the built-in Edit and View switch the open page's mode in place; both stay
        // visible, and only the one the current mode does not already have is enabled (corpus
        // testpage/TestPageModeSwitchAction_Tests.al:Claim 1 and 2). A list opens its card
        // instead, like a RunObject action with RunPageOnRec, answered by a page handler (corpus
        // testpage/TestPageListViewEditAction_Tests.al:ListViewActionOpensTheCardOnTheListsCurrentRowReadOnly,
        // ListEditActionOpensTheCardOnTheListsCurrentRowEditable).
        if self.is_list(st) {
            // Both actions are visible on a list whether or not it names a card; on a list that names one, View is
            // enabled, Edit only when that card can be edited, and a disabled action does nothing (corpus
            // testpage/TestPageModeSwitchAction_Tests.al:
            // PlainListWithoutCardPageIdOffersBothActionsAndEnablesOnlyView,
            // ListWhoseCardIsReadOnlyLeavesTheEditActionVisibleButNotEnabled,
            // ListWithAnEditableCardPageIdEnablesBothActions).
            let card = self.list_card_page(st)?;
            // A list without a card switches the mode of its own page, as a card does: opened for view,
            // Edit is enabled and View is not; opened for edit, the other way round. A list that names a
            // card with no row to open it for has both disabled (testpage/W2ListActions.al:NoCard_WithRows,
            // List_NoRows, BC 27.5 and 29.0).
            let no_rows = st.borrow().cursor == Cursor::None;
            let editable = st.borrow().mode != PageMode::View;
            let enabled = match (edit, card) {
                (_, None) => edit == !editable,
                (_, Some(_)) if no_rows => false,
                (false, _) => true,
                (true, Some(target)) => self.card_allows_edit(target)?,
            };
            return match m {
                PageMethod::Visible => Ok(Value::Boolean(true)),
                PageMethod::Enabled => Ok(Value::Boolean(enabled)),
                // A list without a card switches its own mode (`switch_list_without_card`).
                PageMethod::Invoke if enabled && card.is_none() => self.switch_list_without_card(st, edit),
                PageMethod::Invoke if !enabled || card.is_none() => Ok(Value::Null),
                PageMethod::Invoke => self.list_card_action(st, edit),
                other => unsupported(format!(
                    "TestAction.{other} of the built-in {name} action of a list page"
                )),
            };
        }
        let editable = st.borrow().mode != PageMode::View;
        let applies = edit == !editable;
        match m {
            PageMethod::Visible => Ok(Value::Boolean(true)),
            PageMethod::Enabled => Ok(Value::Boolean(applies)),
            PageMethod::Invoke => {
                if applies {
                    let mode = if edit { PageMode::Edit } else { PageMode::View };
                    return self.switch_mode(st, mode);
                }
                Ok(Value::Null)
            }
            other => unsupported(format!("TestAction.{other}")),
        }
    }

    /// The built-in Edit (`edit`) or View action of `st`, taken on the `earlier` open of the variable (assumption
    /// `mode-actions-bound-to-earlier-open`, corpus PR 1467, open: testpage/lifecycle/ReopenAfterListModeSwitch.al, measured on a
    /// local BC of each supported major). Measured on the probe's list (`reopen_probe_list`), reopened by the test in the test
    /// method of the earlier open, after an earlier page that a clean `TestPage.Close()` closed in edit or view mode: Enabled
    /// answers what it answered when that page closed (`reopened|editEnabled`, `reopened|viewEnabled` of every reopen cell;
    /// Close_Reopen_Modes_PerOpenMode `view then openNew = Yes|Yes|Yes|No`), and Invoke, after an earlier page that closed in edit
    /// mode, on the reopened page in edit or view mode standing on a row, raises 'Server page is not open.' and runs nothing
    /// (ViewEdit_Close_OpenEdit_SwitchAgain `view-2`, `edit-2`; ReadViewEnabled_Close_OpenEdit_View;
    /// ViewEdit_Close_OpenView_WalkThenEdit `edit-2`). The text is the platform's (reports/request-page/P105RpClose.al:93); its
    /// code is not measured here. Every other shape refuses.
    fn bound_mode_action(&mut self, st: &St, edit: bool, earlier: EarlierOpen, m: PageMethod) -> Result<Value, Signal> {
        let mode = earlier.mode;
        let measured = earlier.test_close
            && earlier.test_method == self.pages.test_methods_started
            && st.borrow().opened_by_test
            && matches!(mode, PageMode::Edit | PageMode::View)
            && self.reopen_probe_list(st)?;
        if measured {
            let (now, cursor) = (st.borrow().mode, st.borrow().cursor);
            match m {
                PageMethod::Enabled => {
                    assume(Assumption::ModeActionsBoundToEarlierOpen);
                    return Ok(Value::Boolean(edit == (mode == PageMode::View)));
                }
                PageMethod::Invoke
                    if mode == PageMode::Edit
                        && matches!(now, PageMode::Edit | PageMode::View)
                        && cursor == Cursor::Row =>
                {
                    assume(Assumption::ModeActionsBoundToEarlierOpen);
                    return Err(Signal::Error(Box::new(AlError {
                        message: "Server page is not open.".into(),
                        code: al_builtins::native::UNPINNED_ERROR.into(),
                        call_stack: String::new(),
                        ..Default::default()
                    })));
                }
                _ => {}
            }
        }
        let name = if edit { "Edit" } else { "View" };
        unsupported(format!(
            "TestAction.{m} of the built-in {name} action taken on an earlier open of the TestPage (measured only for Enabled and \
             Invoke on one list without a card, corpus PR 1467)"
        ))
    }
}
