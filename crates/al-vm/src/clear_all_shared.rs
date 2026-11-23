//! `ClearAll()` of a global codeunit variable whose instance an interface, a second codeunit variable or a parameter may
//! share: the instance is reset in place and stays in the variable (assumption clear-shared-codeunit, open corpus PR 1560,
//! tests/al-language/codeunits/clear-instance/ClearSharedCodeunit.al). Entry point: `Session::clear_all_in_place`.

use super::*;

impl Session {
    /// The globals of `owner` (not in `keep`) that `ClearAll()` in `owner` resets in place: each holds an instance of a codeunit
    /// in the measured shape ([`Session::reset_in_place`]); its globals go back to their declared values and the variable keeps
    /// the instance, so a holder that shares it sees the reset and later writes through either side (ClearSharedCodeunit.al
    /// ClearAll_GlobalUnusedIface, ClearAll_GlobalUsedAlias_AliasWroteBefore, ClearAll_GlobalUsedValueParam, X unused and used
    /// once alike). Every other global gets a fresh value from `clear_all`, as before.
    pub(crate) fn clear_all_in_place(&mut self, owner: usize, globals: &[Value], keep: &[usize]) -> Vec<usize> {
        if !self.clears_in_place(owner) {
            return Vec::new();
        }
        let mut kept = Vec::new();
        for (i, v) in globals.iter().enumerate() {
            let Value::Codeunit(rc) = v else { continue };
            if keep.contains(&i) || !self.reset_in_place(rc) {
                continue;
            }
            al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
            if rc.borrow().initialized {
                let engine = self.engine.clone();
                let object = rc.borrow().object as usize;
                let fresh = self.slot_defaults(&engine.program.objects[object].globals);
                rc.borrow_mut().globals = fresh;
            }
            kept.push(i);
        }
        kept
    }

    /// Whether `ClearAll()` in `owner` may reset a global codeunit's instance in place: `owner` is a codeunit that is not
    /// SingleInstance, states no Subtype and has no TableNo, as "CLX Holder" (ClearSharedCodeunit.al codeunit 75581).
    fn clears_in_place(&self, owner: usize) -> bool {
        let o = &self.engine.program.objects[owner];
        o.kind == al_ir::ObjKind::Codeunit
            && !self.engine.object_facts[owner].single_instance
            && o.properties.iter().all(|(k, _)| !k.eq_ignore_ascii_case("Subtype"))
            && o.rec_slot.is_none()
    }

    /// Whether the instance `rc` is of the measured shape: of a codeunit that is not SingleInstance, has no TableNo and holds
    /// exactly one global, an Integer, as "CLX Store" (ClearSharedCodeunit.al codeunit 75580, `N: Integer`); not bound by
    /// BindSubscription (`clear_all_bound_globals` owns those) and with no routine of its codeunit running.
    fn reset_in_place(&self, rc: &Rc<RefCell<Instance>>) -> bool {
        let object = rc.borrow().object as usize;
        let o = &self.engine.program.objects[object];
        o.kind == al_ir::ObjKind::Codeunit
            && !self.engine.object_facts[object].single_instance
            && o.rec_slot.is_none()
            && matches!(o.globals.as_slice(), [g] if matches!(g.ty, RtType::Integer))
            && !self.holds_bound_codeunit(&Value::Codeunit(rc.clone()))
            && !self.frames.contains(&object)
    }
}
