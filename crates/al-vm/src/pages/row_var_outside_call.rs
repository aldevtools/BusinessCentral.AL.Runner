//! A page-variable control of a repeater after code of another page changed the variable through a procedure of the page:
//! the client still shows what the variable held at the row's last read. Pinned for a ListPart without a link whose host
//! hands it a value in the host's OnAfterGetRecord (`CurrPage.<part>.Page.Initialize`): the host moves, the part reads no
//! row, and the part's control shows the value of its last read (testpage/parts/PartFilterExpressionControl.al:392
//! Periods_ReadBeforeFilter_ThenFilter_ThenFirst `before` `item=I1` after the host moved to I2, :450
//! Periods_FilterOneDay_HostMovesToNextItem `I3 no move` `item=I2` after the host moved to I3).

use super::*;

impl Session {
    /// A procedure of page `st` is called through a `Page` value from code running on `caller` (the calling frame's instance).
    pub(crate) fn note_outside_page_call(&self, st: &St, caller: &Rc<RefCell<al_builtins::value::Instance>>) {
        let mut s = st.borrow_mut();
        let own = s.instance.as_ref().is_some_and(|i| Rc::ptr_eq(i, caller))
            || s.ext_instances.iter().any(|(_, i)| Rc::ptr_eq(i, caller));
        if !own && s.host.is_some() {
            s.row_vars_behind_outside_call = true;
        }
    }

    /// Whether a page-variable control of `st` on the row OnAfterGetRecord ran with last shows what `row_vars` noted rather
    /// than the variable as it is.
    pub(super) fn row_var_shows_noted(&self, st: &St) -> bool {
        st.borrow().row_vars_behind_outside_call
    }
}
