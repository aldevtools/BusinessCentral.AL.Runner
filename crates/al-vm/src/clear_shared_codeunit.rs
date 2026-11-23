//! `Clear` and `ClearAll()` of a codeunit variable whose instance an interface, a second codeunit variable or a parameter
//! shares (assumption clear-shared-codeunit, open corpus PR 1560, tests/al-language/codeunits/clear-instance/
//! ClearSharedCodeunit.al). `Clear` gives such a variable nothing used yet a fresh instance, and `ClearAll()` resets such an
//! instance in place in the measured shape; the shapes the probe does not measure refuse. An instance nothing else holds
//! keeps main's answer. Entry points: `Session::clear_shared_codeunit` (from the `Clear` site, `exec/run.rs`) and
//! `Session::clear_all_in_place` (from `clear_all`).

use super::*;

impl Session {
    /// Whether object `object` is a codeunit that is not SingleInstance: a variable of it holds an instance of its own,
    /// which `Clear` replaces and an assignment shares (ClearSharedCodeunit.al codeunit 75580 "CLX Store"). A SingleInstance
    /// codeunit variable reads the one instance whatever is cleared (ClearSharedCodeunitSI.al).
    pub(crate) fn plain_codeunit(&self, object: usize) -> bool {
        self.engine.program.objects[object].kind == al_ir::ObjKind::Codeunit
            && !self.engine.object_facts[object].single_instance
    }

    /// The globals of `owner` (not in `keep`) that `ClearAll()` in `owner` resets in place: each holds an instance of a
    /// codeunit in the measured shape ([`Session::reset_in_place`]) that another holder shares; its globals go back to their
    /// declared values and the variable keeps the instance, so the holder sees the reset and later writes through either
    /// side (ClearSharedCodeunit.al ClearAll_GlobalUnusedIface, ClearAll_GlobalUsedAlias_AliasWroteBefore,
    /// ClearAll_GlobalUsedValueParam, X unused and used once alike). An instance nothing else holds keeps main's answer (the
    /// variable gets a fresh one, which holds the same). A shared instance of any other shape refuses: main's fresh instance
    /// would detach it, which BC does not do in the measured shape.
    ///
    /// `ClearAll` takes a copy of the globals, so the instance of a global nothing shares has two holders; an element of an
    /// array of codeunits has one, the array.
    pub(crate) fn clear_all_in_place(
        &mut self,
        owner: usize,
        globals: &[Value],
        keep: &[usize],
    ) -> Result<Vec<usize>, Signal> {
        let mut kept = Vec::new();
        for (i, v) in globals.iter().enumerate() {
            if keep.contains(&i) {
                continue;
            }
            match v {
                Value::Codeunit(rc) if self.plain_codeunit(rc.borrow().object as usize) && Rc::strong_count(rc) > 2 => {
                    if !(self.clears_in_place(owner) && self.reset_in_place(rc)) {
                        return Err(Signal::unsupported(
                            "ClearAll of a global codeunit variable whose instance another variable shares, in a shape \
                             the corpus does not pin",
                        ));
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
                Value::Array(items) if items.borrow().iter().any(|e| self.shared_element(e)) => {
                    return Err(Signal::unsupported(
                        "ClearAll of a global array whose codeunit element another variable shares (not pinned)",
                    ));
                }
                _ => {}
            }
        }
        Ok(kept)
    }

    /// Whether array element `e` holds an instance of a codeunit that is not SingleInstance that another holder shares (the
    /// array is its one own holder).
    fn shared_element(&self, e: &Value) -> bool {
        matches!(e, Value::Codeunit(rc) if self.plain_codeunit(rc.borrow().object as usize) && Rc::strong_count(rc) > 1)
    }

    /// Whether `ClearAll()` in `owner` may reset a global codeunit's instance in place: `owner` is a codeunit that is not
    /// SingleInstance, states no Subtype and has no TableNo, as "CLX Holder" (ClearSharedCodeunit.al codeunit 75581).
    fn clears_in_place(&self, owner: usize) -> bool {
        let o = &self.engine.program.objects[owner];
        self.plain_codeunit(owner)
            && o.properties.iter().all(|(k, _)| !k.eq_ignore_ascii_case("Subtype"))
            && o.rec_slot.is_none()
    }

    /// Whether the instance `rc` is of the measured shape: of a codeunit that is not SingleInstance, has no TableNo and holds
    /// exactly one global, an Integer, as "CLX Store" (ClearSharedCodeunit.al codeunit 75580, `N: Integer`); not bound by
    /// BindSubscription (`clear_all_bound_globals` owns those) and with no routine of its codeunit running.
    fn reset_in_place(&self, rc: &Rc<RefCell<Instance>>) -> bool {
        let object = rc.borrow().object as usize;
        let o = &self.engine.program.objects[object];
        self.plain_codeunit(object)
            && o.rec_slot.is_none()
            && matches!(o.globals.as_slice(), [g] if matches!(g.ty, RtType::Integer))
            && !self.holds_bound_codeunit(&Value::Codeunit(rc.clone()))
            && !self.frames.contains(&object)
    }

    /// `Clear` of place `p`, whose value `v` the native clears next (al-builtins `native/values.rs`, which keeps an unused
    /// codeunit variable as it is). For a Codeunit variable that holds an instance nothing used yet, of a codeunit that is
    /// not SingleInstance, that another holder shares, `v` becomes a fresh instance first: the variable gets it and the
    /// holder keeps the old one (ClearSharedCodeunit.al Clear_LocalUnusedIface, Clear_GlobalUnusedAlias,
    /// Clear_LocalUnusedAliasClearedThroughVarParam, Clear_GlobalUnusedValueParam: X reads 0, the holder 5). Measured for a
    /// declared local, a global and a var parameter ([`ClearPlace::Measured`]); any other place refuses, a by-value
    /// parameter among them (the probe never clears one itself, ClearThenSet clears the caller's variable). So does an array
    /// of codeunits whose unused element another holder shares. An instance nothing else holds keeps main's answer with no
    /// mark: a fresh unused instance reads the same. An Interface or Variant place keeps its own path (`exec/run.rs`).
    pub(crate) fn clear_shared_codeunit(&mut self, p: &ir::Place, v: &mut Value, f: &Frame) -> Result<(), Signal> {
        match (self.place_type(p, f), &*v) {
            (RtType::Codeunit(_), Value::Codeunit(rc))
                if !rc.borrow().initialized && self.plain_codeunit(rc.borrow().object as usize) =>
            {
                let place = self.clear_place(p, f);
                if Rc::strong_count(rc) <= place.own_copies() {
                    return Ok(());
                }
                if place == ClearPlace::Other {
                    return Err(Signal::unsupported(
                        "Clear of a codeunit variable that nothing used yet and another variable shares, in a place \
                         the corpus does not pin (a by-value parameter, an array element)",
                    ));
                }
                al_builtins::assumptions::assume(al_builtins::assumptions::Assumption::ClearSharedCodeunit);
                let object = rc.borrow().object;
                *v = Value::Codeunit(al_builtins::instance_cycles::shared(Instance {
                    object,
                    ..Default::default()
                }));
                Ok(())
            }
            (RtType::Array(_, el), Value::Array(items))
                if matches!(*el, RtType::Codeunit(_))
                    && items.borrow().iter().any(|e| {
                        self.shared_element(e) && matches!(e, Value::Codeunit(rc) if !rc.borrow().initialized)
                    }) =>
            {
                Err(Signal::unsupported(
                    "Clear of an array whose unused codeunit element another variable shares (not pinned)",
                ))
            }
            _ => Ok(()),
        }
    }

    /// The kind of place `p` a `Clear` names, in the routine `f` runs.
    fn clear_place(&self, p: &ir::Place, f: &Frame) -> ClearPlace {
        let r = &self.engine.program.objects[f.object].routines[f.routine];
        match p {
            ir::Place::Local(slot) if Some(*slot) == r.ret_local => ClearPlace::Other,
            ir::Place::Local(slot) if *slot < r.params => match r.by_var.get(*slot) {
                Some(true) => ClearPlace::VarParameter,
                _ => ClearPlace::Other,
            },
            ir::Place::Local(_) | ir::Place::Global(_) => ClearPlace::Measured,
            _ => ClearPlace::Other,
        }
    }
}

/// The kind of place a `Clear` of a codeunit variable names ([`Session::clear_shared_codeunit`]).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ClearPlace {
    /// A declared local or a global (ClearSharedCodeunit.al Clear_LocalUnusedIface, Clear_GlobalUnusedIface).
    Measured,
    /// A var parameter (Clear_LocalUnusedAliasClearedThroughVarParam, ClearVar).
    VarParameter,
    /// A by-value parameter, an element, the named return value or any other place.
    Other,
}

impl ClearPlace {
    /// How many holders of the instance are copies of the variable itself: the variable and the copy `Clear` takes of it
    /// (the VM evaluates its argument), and for a var parameter the copy the parameter's slot keeps of the caller's variable.
    fn own_copies(self) -> usize {
        2 + usize::from(self == ClearPlace::VarParameter)
    }
}
