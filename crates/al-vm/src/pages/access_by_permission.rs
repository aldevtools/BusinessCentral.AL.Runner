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
    /// AccessByPermission leaves off the page answers every method with the error of a removed control (Y84PermAbp.al
    /// Y84_Abp_HiddenMethods: Value, Editable, Caption, Enabled, SetValue, Lookup, AssertEquals, Activate and an action's Enabled
    /// and Invoke), so the call goes on to the shared "not found" error; this only lets a guard that cannot be decided refuse.
    pub(super) fn access_by_permission_check(&self, st: &St, idx: usize, m: PageMethod) -> Result<(), Signal> {
        self.check_removed_pinned(st, idx, m)?;
        self.access_by_permission_off(st, idx)?;
        Ok(())
    }

    /// Whether part `idx` is not on the page: its application area is not enabled or its AccessByPermission is not granted
    /// (Y84PermAbp.al: a guarded part is left off, and `GuardPart.IdCol.Visible()` raises "The part with ID = n was not found on
    /// the page."). An undecidable guard refuses.
    pub(super) fn part_removed(&self, st: &St, idx: usize) -> Result<bool, Signal> {
        if self.area_removed(st, idx) || self.access_by_permission_off(st, idx)? {
            return Ok(true);
        }
        // A guarded part whose page the session may not execute is left off too (Y84PermAbp.al Y84_Abp_ObjectKinds `GOP`: guard
        // `codeunit C = X` is held under the exact sets that name C, yet the part is off unless the session executes the part's page,
        // as `Set` does through All Objects). Read from guarded parts only: an unguarded part under a narrow set keeps showing, which
        // no probe pins either way.
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        if ctl.prop("accessbypermission").is_none() {
            return Ok(false);
        }
        self.permission_part_page_off(ctl.part_page)
    }

    /// Whether the application areas or AccessByPermission take control `idx` off the page (after
    /// [`access_by_permission_check`](Self::access_by_permission_check) passed).
    pub(super) fn off_page(&self, st: &St, idx: usize) -> bool {
        self.area_removed(st, idx) || self.access_by_permission_off(st, idx).unwrap_or(false)
    }
}
