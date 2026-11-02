//! A page control guarded by `AccessByPermission` (ADR-062 stage 3, Y24PermExecute.al Y24_P21_*).

use super::*;

impl Session {
    /// Whether the control's AccessByPermission guard is not granted by the permission sets the session held when the page opened
    /// (`Session::access_by_permission_hidden`); false for a control with no guard.
    pub(super) fn access_by_permission_off(&self, st: &St, idx: usize) -> Result<bool, Signal> {
        let engine = self.engine();
        let Some(ir::PropVal::Lit(guard)) = page_of(&engine, st).controls[idx].prop("accessbypermission") else {
            return Ok(false);
        };
        let opened_under = st.borrow().permission_roles.clone();
        self.access_by_permission_hidden(guard, &opened_under)
    }
}
