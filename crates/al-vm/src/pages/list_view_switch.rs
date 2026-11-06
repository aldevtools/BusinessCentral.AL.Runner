//! TestPage View() on a list without a card, opened for edit (al-runner issue 6977): the page switches to view mode and drops
//! the new-row line, so a First/Next walk visits only the rows (testpage/R6977ListViewNext.al:138, :156, :174, :191). Nothing
//! else of the switch is pinned; what a probe has not measured is refused here.

use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// After the built-in View switched a list without a card to view mode only the walk over its rows is pinned
    /// (testpage/R6977ListViewNext.al:138, :174, :191). A field's Editable and SetValue, and Enabled and Invoke of the built-in
    /// Edit and View actions, are not pinned: refused until a probe measures them (docs/probes-needed.md, al-runner issue 6977).
    pub(super) fn refuse_after_list_view_switch(&self, c: &ControlRef, m: PageMethod) -> Result<(), Signal> {
        if !c.page.borrow().list_view_switched {
            return Ok(());
        }
        let refused = match &c.target {
            ControlTarget::Control(idx) => {
                matches!(m, PageMethod::Editable | PageMethod::SetValue)
                    && page_of(&self.engine(), &c.page).controls[*idx].kind == ir::ControlKind::Field
            }
            ControlTarget::BuiltIn(name) => {
                // Edit().Invoke() after View is pinned: it opens nothing and raises nothing
                // (testpage/TestPageModeSwitchAction_Tests.al:200, PlainListWithoutCardPageIdOffersBothActions...).
                match name.as_str() {
                    "Edit" => m == PageMethod::Enabled,
                    "View" => matches!(m, PageMethod::Enabled | PageMethod::Invoke),
                    _ => false,
                }
            }
            _ => false,
        };
        if refused {
            return unsupported(format!(
                "TestField.{m} after the built-in View switched a list without a card to view mode (not pinned)"
            ));
        }
        Ok(())
    }
}
