//! A page control over a FlowField whose source table the session's permission sets cannot read is not on the page (ADR-062
//! stage 2; corpus permissions/lt-y19-perm-probes/Y24PermReads.al Y24_P15_FlowFieldsNothingOnSource `testpage_field`: with rows in
//! the source table, `Card.SumCtl.Value()` raises "The field with ID = n is not found on the page.", code TestFieldNotFound; with
//! the table readable the value is there).

use super::*;
use al_builtins::page_op::PageMethod;

impl Session {
    /// Of the FlowField `positions` the page shows, those the session may calculate; the others (a certain denial of
    /// the source table, which has rows) are noted in the page state, and their controls are off the page. Any other
    /// outcome of the permission check (a shape that is not pinned) refuses.
    pub(super) fn leave_off_denied_flowfields(
        &mut self,
        st: &St,
        rec: &Rec,
        table: usize,
        positions: Vec<usize>,
    ) -> Result<Vec<usize>, Signal> {
        st.borrow_mut().hidden_flowfields.clear();
        if self.permissions.is_none() {
            return Ok(positions);
        }
        let engine = self.engine();
        let Some(def) = engine.program.objects[table].table.as_ref() else {
            return Ok(positions);
        };
        let mut kept = Vec::new();
        for p in positions {
            let Some(calc) = def.fields[p].calc.as_ref() else {
                kept.push(p);
                continue;
            };
            match self.permission_gate_flowfield(rec, calc.table) {
                Ok(()) => kept.push(p),
                Err(Signal::Error(_)) => st.borrow_mut().hidden_flowfields.push(p),
                Err(other) => return Err(other),
            }
        }
        Ok(kept)
    }

    /// Whether control `idx` shows a FlowField the session cannot read (see [`Self::leave_off_denied_flowfields`]).
    pub(super) fn flowfield_hidden(&self, st: &St, idx: usize) -> bool {
        let hidden = &st.borrow().hidden_flowfields;
        if hidden.is_empty() {
            return false;
        }
        let engine = self.engine();
        matches!(page_of(&engine, st).controls[idx].binding, ir::Binding::Field(p) if hidden.contains(&p))
    }

    /// The checks before a TestPage call of `m` on control `idx`: the application areas, and the FlowField controls left off the
    /// page, which answer only `Value` (Y24_P15 `testpage_field`); any other method of such a control is not pinned and refuses.
    pub(super) fn check_removed_pinned(&self, st: &St, idx: usize, m: PageMethod) -> Result<(), Signal> {
        self.check_areas_pinned(st, idx)?;
        if m != PageMethod::Value && self.flowfield_hidden(st, idx) {
            return unsupported(format!(
                "TestPage.{}() of a control over a FlowField the session cannot read, which is not on the page (only Value is pinned)",
                m.name()
            ));
        }
        Ok(())
    }
}
