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

    /// The checks before a TestPage call of `m` on control `idx`: the application areas, and AccessByPermission. A control that
    /// AccessByPermission leaves off the page answers only `Value`, `Visible` and `Enabled` (Y24PermExecute.al
    /// Y24_P21_AccessByPermissionPage) with the error of a removed control; any other method is not pinned and refuses.
    pub(super) fn access_by_permission_check(&self, st: &St, idx: usize, m: PageMethod) -> Result<(), Signal> {
        self.check_areas_pinned(st, idx)?;
        let off = self.access_by_permission_off(st, idx)?;
        if off && !matches!(m, PageMethod::Value | PageMethod::Visible | PageMethod::Enabled) {
            return unsupported(format!(
                "TestPage.{}() of a control that AccessByPermission leaves off the page (only Value, Visible and Enabled are pinned)",
                m.name()
            ));
        }
        Ok(())
    }

    /// Whether the application areas or AccessByPermission take control `idx` off the page (after
    /// [`access_by_permission_check`](Self::access_by_permission_check) passed).
    pub(super) fn off_page(&self, st: &St, idx: usize) -> bool {
        self.area_removed(st, idx) || self.access_by_permission_off(st, idx).unwrap_or(false)
    }
}
