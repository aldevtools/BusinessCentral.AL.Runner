//! TestPage View() and then Edit() on a list without a card, opened for edit (al-runner issue 6977). View switches the page to
//! view mode and drops the new-row line (testpage/R6977ListViewNext.al:138, :156, :174, :191; R6977ListAfterView.al:125, :142,
//! :164); Edit then switches it back (R6977ListAfterView.al:187). After that second switch only what the probe reads is pinned;
//! the rest is refused here.

use super::*;
use al_builtins::page_op::PageMethod;

const EDIT_BACK: &str =
    "after the built-in View and then Edit switched a list without a card back to edit mode (not pinned)";

impl Session {
    /// The enabled built-in Edit or View of a list without a card, invoked. View on the list opened for edit switches the page to
    /// view: a First/Next walk then visits only the rows, no blank new-row line (testpage/R6977ListViewNext.al:138, :174, :191
    /// `n=3`; :156 without View `0/; n=4`). The page and its fields are then not editable, SetValue still types into the row,
    /// and Edit is enabled, View not (R6977ListAfterView.al:125, :142, :164): view mode serves these. Edit after that View
    /// switches the page back to edit on the same row (R6977ListAfterView.al:187 `back|page=Yes;back|description=Yes;
    /// back|editEnabled=No;back|viewEnabled=Yes;back|row=10000`); whether the new-row line comes back is not pinned
    /// (`refuse_after_list_edit_back`). Edit on such a list opened for view opens nothing (TestPageModeSwitchAction_Tests.al:200).
    pub(super) fn switch_list_without_card(&mut self, st: &St, edit: bool) -> Result<Value, Signal> {
        if !edit {
            if st.borrow().cursor != Cursor::Row || st.borrow().draft {
                return unsupported(
                    "TestAction View of a list without a card that stands on the new-row line, a typed row or no row",
                );
            }
            st.borrow_mut().new_line = false;
            st.borrow_mut().list_view_switched = true;
            return self.switch_mode(st, PageMode::View);
        }
        if !st.borrow().list_view_switched {
            return Ok(Value::Null);
        }
        {
            let mut s = st.borrow_mut();
            s.list_view_switched = false;
            s.list_edit_back = true;
        }
        self.switch_mode(st, PageMode::Edit)
    }

    /// A page call after View and then Edit on a list without a card: Editable, ValidationErrorCount, Close and taking the
    /// built-in Edit and View actions are pinned (R6977ListAfterView.al:178, :102, :184, :180). A walk, New and the rest are
    /// not: whether the new-row line comes back is unmeasured.
    pub(super) fn refuse_after_list_edit_back(&self, st: &St, m: PageMethod) -> Result<(), Signal> {
        if !st.borrow().list_edit_back
            || matches!(
                m,
                PageMethod::Editable
                    | PageMethod::ValidationErrorCount
                    | PageMethod::Close
                    | PageMethod::Edit
                    | PageMethod::View
            )
        {
            return Ok(());
        }
        unsupported(format!("TestPage.{m} {EDIT_BACK}"))
    }

    /// A control call in the same state: the fields are pinned (Editable, Value, SetValue: R6977ListAfterView.al:179, :182,
    /// :100) and so are Enabled reads of the built-in actions (:180, :181); invoking Edit or View again is not.
    pub(super) fn refuse_after_list_edit_back_control(&self, c: &ControlRef, m: PageMethod) -> Result<(), Signal> {
        if !c.page.borrow().list_edit_back {
            return Ok(());
        }
        match &c.target {
            ControlTarget::BuiltIn(name) if m == PageMethod::Invoke && matches!(name.as_str(), "Edit" | "View") => {
                unsupported(format!("TestAction.{m} of the built-in {name} {EDIT_BACK}"))
            }
            _ => Ok(()),
        }
    }
}
