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
        // An actionref of a guarded action that the guard leaves off the page: the corpus reaches the action only by its own name
        // (Y94AbpActionRef.al, Y90PermActionRef.al), so whether the TestPage still finds the actionref is not pinned.
        let target = match page_of(&self.engine(), st).controls[idx].action_ref {
            Some(ir::ActionRef::Target(t)) => Some(t),
            _ => None,
        };
        if let Some(t) = target
            && self.access_by_permission_off(st, t)?
        {
            let engine = self.engine();
            let page = page_of(&engine, st);
            return Err(Signal::unsupported(format!(
                "TestPage {} on actionref `{}` of action `{}`, which AccessByPermission leaves off the page under the session's permission sets (not pinned, ADR-062 probe P21)",
                m.name(),
                page.controls[idx].name,
                page.controls[t].name
            )));
        }
        Ok(())
    }

    /// Whether part `idx` is not on the page: its application area is not enabled or its AccessByPermission is not granted
    /// (Y84PermAbp.al: a guarded part is left off, and `GuardPart.IdCol.Visible()` raises "The part with ID = n was not found on
    /// the page."). An undecidable guard refuses.
    pub(super) fn part_removed(&self, st: &St, idx: usize) -> Result<bool, Signal> {
        if self.area_removed(st, idx) || self.access_by_permission_off(st, idx)? {
            return Ok(true);
        }
        // A part whose page the session may not execute is left off too. Guarded part: Y84PermAbp.al Y84_Abp_ObjectKinds `GOP`,
        // Y84PermAbp.al:132 (set X Some: GOP=N with the guard held) against :160 (Set: GOP=Y): guard `codeunit C = X` is held under
        // the exact sets that name C, yet the part is off unless the session executes the part's page, as `Set` does through All
        // Objects. Unguarded part: it needs Execute on the part page and on the part page's source table
        // (Y90PermParts.al Y90_Part_ExactA..C, Y90_Part_Set, BC 28 and up).
        let engine = self.engine();
        let ctl = &page_of(&engine, st).controls[idx];
        self.permission_part_page_off(ctl.part_page, ctl.prop("accessbypermission").is_none())
    }

    /// Whether the application areas or AccessByPermission take control `idx` off the page (after
    /// [`access_by_permission_check`](Self::access_by_permission_check) passed).
    pub(super) fn off_page(&self, st: &St, idx: usize) -> bool {
        self.area_removed(st, idx) || self.access_by_permission_off(st, idx).unwrap_or(false)
    }
}
