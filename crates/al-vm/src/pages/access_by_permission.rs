//! A page control guarded by `AccessByPermission` (ADR-062 stage 3, Y24PermExecute.al Y24_P21_*).

use super::*;
use al_builtins::page_op::PageMethod;

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

    /// Whether AccessByPermission leaves control `idx` off the page, for a TestPage call of `m` on it. A control that is off answers
    /// only `Value`, `Visible` and `Enabled` (Y24PermExecute.al Y24_P21_AccessByPermissionPage) with the error of a removed
    /// control; any other method is not pinned and refuses.
    pub(super) fn access_by_permission_guard(&self, st: &St, idx: usize, m: PageMethod) -> Result<bool, Signal> {
        let off = self.access_by_permission_off(st, idx)?;
        if off && !matches!(m, PageMethod::Value | PageMethod::Visible | PageMethod::Enabled) {
            return unsupported(format!(
                "TestPage.{}() of a control that AccessByPermission leaves off the page (only Value, Visible and Enabled are pinned)",
                m.name()
            ));
        }
        Ok(off)
    }
}
